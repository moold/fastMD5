use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read},
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
    thread,
    sync::{Arc, atomic::{AtomicUsize, Ordering}},
};

use blake3::{hash, Hasher};
use crossbeam_channel::bounded;
use walkdir::WalkDir;

use mimalloc::MiMalloc;
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod option;
use option::Option as Opt;
 
const MIN_READ_SIZE: usize = 1 << 20; // 1 MiB

fn hash_full_md5(path: &Path) -> io::Result<String> {//here, buffer reader is slower
    use md5::{Digest, Md5};
    thread::scope(|work| {
        let (in_s, in_r) = bounded(4);
        work.spawn(move || -> io::Result<()> {
            let mut f = File::open(path)?;
            let mut buf = vec![0u8; 2 * MIN_READ_SIZE];

            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                in_s.send(buf[..n].to_owned()).unwrap();
            }
            Ok(())
        });

        let mut hasher = Md5::new();
        while let Ok(chunk) = in_r.recv() {
            hasher.update(&chunk);
        }
        Ok(format!("{:x}", hasher.finalize()))
    })
}

fn hash_full_openssl_md5(path: &Path) -> io::Result<String> {
    use openssl::hash::{Hasher, MessageDigest};
    thread::scope(|work| {
        let (in_s, in_r) = bounded(4);
        work.spawn(move || -> io::Result<()> {
            let mut f = File::open(path)?;
            let mut buf = vec![0u8; MIN_READ_SIZE];

            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                in_s.send(buf[..n].to_owned()).unwrap();
            }
            Ok(())
        });

        let mut hasher = Hasher::new(MessageDigest::md5()).unwrap();
        while let Ok(chunk) = in_r.recv() {
            hasher.update(&chunk)?;
        }
        Ok(hex::encode(hasher.finish().unwrap()))
    })
}

fn hash_full_blake3(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut hasher = Hasher::new();
    hasher.update_reader(file)?;
    Ok(hasher.finalize().to_hex().to_string())
}

#[derive(Clone, Copy, Debug)]
struct SampleBlock {
    offset: u64,
    len: usize,
}

fn make_sample_offsets(file_len: u64, speed: usize) -> Vec<SampleBlock> {

    if file_len == 0 {
        return Vec::new();
    }

    let mut blocks = (10 - speed.max(2).min(9)) * 128;
    let max_blocks_by_size = ((file_len + MIN_READ_SIZE as u64 - 1) / MIN_READ_SIZE as u64) as usize;
    blocks = blocks.min(max_blocks_by_size).min(8192).max(1);

    let block_size = MIN_READ_SIZE;

    if blocks == 1 {
        return vec![SampleBlock {offset: 0, len: block_size.min(file_len as usize) }];
    }

    let step = if file_len > block_size as u64 {
        (file_len - block_size as u64) / (blocks as u64 - 1)
    } else {
        0
    };

    let mut offsets = Vec::with_capacity(blocks);
    for i in 0..blocks {
        let mut pos = i as u64 * step;
        if pos + block_size as u64 > file_len {
            pos = file_len.saturating_sub(block_size as u64);
        }
        let len = block_size.min((file_len - pos) as usize);
        offsets.push(SampleBlock {offset: pos, len });
    }

    offsets
}

fn hash_sampled_blake3_pread(path: &Path, speed: usize, threads: usize) -> io::Result<String> {
    let file = File::open(path)?;
    let meta = file.metadata()?;
    let file_len = meta.len();

    if file_len == 0 {
        return Ok(hash(&[]).to_hex().to_string());
    }

    let jobs = &make_sample_offsets(file_len, speed);
    thread::scope(|scope| {
        let counter = Arc::new(AtomicUsize::new(0));
        let (ou_s, ou_r) = bounded(threads * 2);
        for _ in 0..threads {
            let counter = counter.clone();
            let ou_s = ou_s.clone();
            let f = file.try_clone().expect("clone file handle");
            scope.spawn(move || {
                let mut buf = vec![0u8; MIN_READ_SIZE];
                loop {
                    let i = counter.fetch_add(1, Ordering::Relaxed);
                    if i >= jobs.len() {
                         break;
                    }
                    let job = &jobs[i];
                    let n = FileExt::read_at(&f, &mut buf[..job.len], job.offset).unwrap_or(0);
                    let mut local_hasher = Hasher::new();
                    local_hasher.update(&buf[..n]);
                    let hash_bytes = local_hasher.finalize().as_bytes().to_vec();
                    ou_s.send((i, hash_bytes)).ok();
                }
            });
        }
        drop(ou_s);

        let mut chunks: Vec<Option<Vec<u8>>> = vec![None; jobs.len()];
        while let Ok((idx, hash_bytes)) = ou_r.recv() {
            chunks[idx] = Some(hash_bytes);
        }

        let mut hasher = Hasher::new();
        hasher.update(&file_len.to_be_bytes());
        for h in chunks.into_iter().flatten() {
            hasher.update(&h);
        }
        Ok(hasher.finalize().to_hex().to_string())
    })
}

fn hash_file(path: &Path, speed: usize, thread: usize) -> String {
    if speed == 0 {
        match hash_full_openssl_md5(path) {
            Ok(d) => d,
            Err(_) => String::new(),
        }
    }else if speed == 1 {
        match hash_full_blake3(path) {
            Ok(d) => d,
            Err(_) => String::new(),
        }
    } else {
        let r = hash_sampled_blake3_pread(
            path,
            speed,                                 
            thread.max(1),                      
        );
        match r {
            Ok(d) => d,
            Err(_) => String::new(),
        }
    }
}

fn get_hash_workers(opt: &Opt) {
    thread::scope(|work| {
        let (in_s, in_r) = bounded(opt.thread * 4);
        work.spawn(move || {
            for file in opt.dest.iter(){
                for entry in WalkDir::new(file)
                    .follow_links(opt.link)
                    .into_iter()
                    .filter_entry(|e| {
                        (opt.link || !e.path_is_symlink())
                            && (opt.hidden
                                || !e
                                    .file_name()
                                    .to_str()
                                    .map(|s| s.starts_with('.') && s != "." && !s.starts_with("./") && !s.starts_with(".."))
                                    .unwrap_or(false))
                    })
                    .filter_map(|x| x.ok())
                {
                    if entry.file_type().is_file() {
                        in_s.send(entry.path().to_path_buf()).ok();
                    }
                }
            }
        });

        let (ou_s, ou_r) = bounded::<(PathBuf, String)>(opt.thread * 4);

        for _ in 0..opt.thread {
            let in_r = in_r.clone();
            let ou_s = ou_s.clone();
            work.spawn(move || {
                while let Ok(path) = in_r.recv() {
                    let digest = hash_file(&path, opt.speed as usize, opt.thread);
                    ou_s.send((path, digest)).ok();
                }
            });
        }
        drop(ou_s);

        work.spawn(move || {
            while let Ok((path, digest)) = ou_r.recv() {
                if digest.is_empty() {
                    println!("{}: FAILED open or read", path.display());
                } else if opt.speed == 0 {
                        println!("{digest}  {}", path.display());
                } else {
                    println!("{}s{}  {}", digest, opt.speed, path.display());
                }
            }
        });
    });
}

fn check_hash_workers(opt: &Opt) -> bool {
    thread::scope(|work| {
        let (in_s, in_r) = bounded(opt.thread * 4);
        let (ou_s, ou_r) = bounded(opt.thread * 4);

        work.spawn(move || {
            for file in opt.dest.iter(){
                for line in BufReader::new(File::open(file).expect("Unable to read file")).lines() {
                    if let Ok(l) = line {
                        if !l.trim().is_empty() {
                            in_s.send(l).ok();
                        }
                    }
                }
            }
        });

        for _ in 0..opt.thread {
            let (in_r, ou_s) = (in_r.clone(), ou_s.clone());
            work.spawn(move || {
                while let Ok(line) = in_r.recv() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() < 2 {
                        eprintln!("Improperly formatted checksum line: {}", line);
                        if opt.strict {
                            std::process::exit(1);
                        }
                        continue;
                    }

                    let (tag, path) = (parts[0], parts[1]);
                    let (hex, sampled_speed) = if let Some(pos) = tag.rfind('s') {
                        let (h, s) = tag.split_at(pos);
                        if let Ok(sp) = s[1..].parse::<usize>() {
                            (h.to_string(), Some(sp))
                        } else {
                            eprintln!("Improperly formatted checksum line: {}", line);
                            if opt.strict {
                                std::process::exit(1);
                            }
                            continue;
                        }
                    } else {
                        (tag.to_string(), None)
                    };

                    let res = if sampled_speed.is_none() {
                        hash_full_openssl_md5(Path::new(path)).map(|d| d == hex)
                    } else if sampled_speed == Some(1) {
                        hash_full_blake3(Path::new(path)).map(|d| d == hex)
                    }else if let Some(sp) = sampled_speed {
                        hash_sampled_blake3_pread(Path::new(path), sp, opt.thread.max(1)).map(|d| d == hex)
                    } else {
                        Err(io::Error::new(io::ErrorKind::InvalidData, "unknown digest length"))
                    };

                    match res {
                        Ok(ok) => {
                            ou_s.send((path.to_string(), Ok(ok))).ok();
                        }
                        Err(_) => {
                            ou_s.send((path.to_string(), Err(()))).ok();
                        }
                    }
                }
            });
        }
        drop(ou_s);

        work.spawn(move || {
            let mut has_failed = false;
            while let Ok((path, status)) = ou_r.recv() {
                match status {
                    Ok(true) => {
                        if !opt.quiet {
                            println!("{path}: OK");
                        }
                    }
                    Ok(false) => {
                        println!("{path}: FAILED");
                        has_failed = true;
                        if opt.status {
                            std::process::exit(1);
                        }
                    }
                    Err(_) => {
                        println!("{path}: FAILED open or read");
                        has_failed = true;
                        if opt.status {
                            std::process::exit(1);
                        }
                    }
                }
            }
            has_failed
        })
        .join()
        .unwrap()
    })
}


fn main() {

    //exit on a thread to immediately end the main thread
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        std::process::exit(1);
    }));

    let opt = Opt::from_args();
    if opt.check {
        if check_hash_workers(&opt) {
            std::process::exit(1);
        }
    } else {
        get_hash_workers(&opt);
    }
}
