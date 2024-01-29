use std::{
    fs::File,
    io:: {BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};
use md5::{Md5, Digest};
use crossbeam_channel::bounded;
use crossbeam_utils::thread;
use walkdir::WalkDir;

mod option;
use option::Option as Opt;

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

const MIN_READ_SIZE: u64 = 1024000;

pub fn md5<P: AsRef<Path>>(path: P, speed: u64) -> String {//here, buffer reader is slower
    let mut f = File::open(&path).unwrap_or_else(|_| panic!("Failed open file: {}", path.as_ref().display()));
    thread::scope(|work| {
        let (in_s, in_r) = bounded(0);
        if speed != 0 {
            work.spawn(move |_| {
                let mut seek = 0;
                let mut buffer = [0; MIN_READ_SIZE as usize * 2];

                let mut has_reach_end = false;
                let file_size = f.seek(SeekFrom::End(0)).unwrap();
                let offset = file_size / 100_u64.checked_sub(speed * 10).expect("speed should be < 10") + MIN_READ_SIZE;
                loop {
                    f.seek(SeekFrom::Start(seek)).expect("Failed to seek!");
                    let bytes_read = f.read(&mut buffer).unwrap();
                    if bytes_read == 0 {
                        break;
                    }
                    in_s.send(buffer[..bytes_read].to_owned()).unwrap();
                    if bytes_read as u64 >= file_size{
                        break;
                    }
                    seek += std::cmp::max(bytes_read as u64, offset);
                    if seek >= file_size && !has_reach_end {
                        seek = file_size - MIN_READ_SIZE;
                        has_reach_end = true;
                    }
                }
            });
        }else{
            work.spawn(move |_| {
                let mut buffer = [0; MIN_READ_SIZE as usize * 2];
                loop {
                    let bytes_read = f.read(&mut buffer).unwrap();
                    if bytes_read == 0 {
                        break;
                    }
                    in_s.send(buffer[..bytes_read].to_owned()).unwrap();
                }
            });
        }

        work.spawn(move |_| {
            let mut md5 = Md5::new();
            while let Ok(buffer) = in_r.recv() {
                md5.update(buffer);
            }
            if speed == 0 {
                format!("{:x}", md5.finalize())
            }else {
                format!("{:x}{}", md5.finalize(), speed)
            }
        }).join().unwrap() 
    }).unwrap()
}

fn get_md5_workers(opt: &Opt) {
    thread::scope(|work| {
        let (in_s, in_r) = bounded(opt.thread);
        let (ou_s, ou_r) = bounded(opt.thread);
        work.spawn(move |_| {
            for entry in WalkDir::new(&opt.dest).follow_links(opt.link).into_iter().filter_entry(|e|
                    (opt.link || !e.path_is_symlink()) && 
                    (opt.hidden || !e.file_name().to_str().map(|s| 
                            s.starts_with('.') &&
                            s != "." && !s.starts_with("./") && 
                            !s.starts_with("..")
                        ).unwrap_or(false)
                    )
                ).filter_map(|x| {
                    let x = x.unwrap();
                    if x.file_type().is_file() {
                        Some(x)
                    }else {
                        None
                    }
                })
            {
                in_s.send(entry.path().to_owned()).unwrap();
            }
        });

        (0..opt.thread).for_each(|_| {
            let in_r = in_r.clone();
            let ou_s = ou_s.clone();
            work.spawn(move |_| {
                while let Ok(path) = in_r.recv() {
                    let md5 = md5(&path, opt.speed);
                    ou_s.send((path, md5)).unwrap();
                }
            });
        });

        work.spawn(move |_| {
            while let Ok((path, md5)) = ou_r.recv() {
                println!("{md5}  {}", path.display());
            }
        });
    }).unwrap();
}

fn check_md5_workers(opt: &Opt) -> bool {
    thread::scope(|work| {
        let (in_s, in_r) = bounded(opt.thread);
        let (ou_s, ou_r) = bounded(opt.thread);
        work.spawn(move |_| {
            for line in BufReader::new(File::open(&opt.dest).expect("Unable to read file")).lines() 
            {
                in_s.send(line.unwrap()).unwrap();
            }
        });

        (0..opt.thread).for_each(|_| {
            let in_r = in_r.clone();
            let ou_s = ou_s.clone();
            work.spawn(move |_| {
                while let Ok(line) = in_r.recv() {
                    let lines: Vec<&str> = line.split_ascii_whitespace().collect();
                    let (md5_str, path) = (lines[0], lines[1].to_owned());
                    let md5 = if md5_str.len() == 32 {
                        md5(&path, 0)
                    }else if md5_str.len() == 33 {
                        let speed_pos = md5_str.len() - 1;
                        md5(&path, md5_str[speed_pos..=speed_pos].parse::<u64>().unwrap())
                    }else {
                        eprintln!("Improperly formatted MD5 checksum line: {}", line);
                        if opt.strict {
                            std::process::exit(1);
                        }
                        continue;
                    };
                    ou_s.send((path, md5 == md5_str)).unwrap();
                }
            });
        });
        drop(ou_s);

        work.spawn(move |_| {
            let mut has_failed = false;
            while let Ok((path, status)) = ou_r.recv() {
                if opt.status {
                    if !status {
                        std::process::exit(1);
                    }
                }else if status {
                    if !opt.quiet {
                        println!("{path}: OK");
                    }
                }else {
                    println!("{path}: FAILED");
                    has_failed = true;
                }
            }
            has_failed
        }).join().unwrap()
    }).unwrap()
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
        if check_md5_workers(&opt) {
            std::process::exit(1);
        }
    }else {
        get_md5_workers(&opt);
    }
}
