# fastMD5 - print or check MD5 checksums

**fastMD5** is a high-performance MD5 checksum tool, designed as a faster and parallelized alternative to the standard `md5sum`. It offers flexible speed modes, multi-threading support, and efficient directory handling—making it especially well-suited for verifying very large files or entire directories.

## Key Features

- [x] 🚀 **Adjustable speed levels (0–9)**
  * **0** – Full sequential computation, \~20% faster than GNU `md5sum`, fully compatible with it.
  * **1** – Full-file computation with optimized performance, up to **> 4× faster** than GNU `md5sum`.
  * **2–9** – Block-based sampling for approximate checksums. For example, a file >100 GB can be processed in **under 1 second**.
- [x] 🧵 **Multi-threaded execution** – Leverages multi-core CPUs for even faster performance.
- [x] 📂 **Recursive directory support** – Optionally include hidden files and symbolic links.
- [x] ✅ **Checksum verification mode (`--check`)** – Validate files against precomputed MD5 sums.


## Getting Started

```bash
# Compute MD5 checksums at the default speed level (5), typically used to verify file integrity after copying.
fastmd5 -t 10 file1.fastq.gz file2.fastq.gz data_directory/
# Run in the most accurate mode (level 0, equivalent to `md5sum`).
fastmd5 -t 3 -s 0 file1.fastq.gz file2.fastq.gz data_directory/
# Run full-file computation with optimized performance (level 1).
fastmd5 -t 5 -s 1 file1.fastq.gz file2.fastq.gz data_directory/
# Verify checksums from a checksum file (equivalent to `md5sum -c`).
fastmd5 --check checksums.md5
```

## Table of Contents

- [Installation](#install)
- [General usage](#usage)
- [Getting help](#help)
- [Benchmarking](#benchmark)

### <a name="install"></a>Installation
<!-- 
#### Installing from bioconda
```sh
conda install nextpolish2
``` -->
#### Installing from source
##### Dependencies

`fastMD5` is written in rust, try below commands (no root required) or refer [here](https://www.rust-lang.org/tools/install) to install `Rust` first.
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

##### Download and install
国内用户请参考[这里](https://mirrors.tuna.tsinghua.edu.cn/help/crates.io-index/)设置清华源加速
```sh
git clone https://github.com/moold/fastMD5.git
cd fastMD5 && cargo build --release
```

##### Test

```sh
cd test && bash hh.sh
```

### <a name="usage"></a>General usage
#### 1. Quickly verify the integrity of copied files
```bash
# Generate checksum file (default speed level 5)
fastmd5 -t 10 file1.fastq.gz file2.fastq.gz data_directory/ > checksums.md5  

# Verify against checksum file
fastmd5 --check checksums.md5
```

#### 2. Verify integrity of modified files (full-file mode)
```bash
# Generate checksum file (speed level 1, full-file computation)
fastmd5 -t 10 -s 1 file1.fastq.gz file2.fastq.gz data_directory/ > checksums.md5  

# Verify against checksum file
fastmd5 --check checksums.md5
```

#### 3. Generate results fully compatible with GNU `md5sum`
```bash
# Generate checksum file (speed level 0, GNU-compatible mode)
fastmd5 -t 10 -s 0 file1.fastq.gz file2.fastq.gz data_directory/ > checksums.md5  
# Equivalent GNU command
ls file1.fastq.gz file2.fastq.gz data_directory/ | while read line; do md5sum $line >> checksums.md5; done  

# Verify against checksum file
fastmd5 --check checksums.md5  
# Equivalent GNU command
md5sum -c checksums.md5
```
ℹ️ Run `./target/release/fastMD5 -h` to view all available options.

### <a name="help"></a>Getting help
#### Help

   Feel free to raise an issue at the [issue page](https://github.com/moold/fastMD5/issues/new).

   ***Note:*** Please ask questions on the issue page first. They are also helpful to other users.
#### Contact
   
   For additional help, please send an email to huj1203\_at\_foxmail\_dot\_cn.

<!-- ### <a name="cite"></a>Citation -->

### <a name="benchmark"></a>Benchmarking

| Program     | File Size (GB) | Speed Level (-s) | Wall Clock (s) | Speedup (×) |
|:------------|---------------:|:----------------:|---------------:|------------:|
| **GNU `md5sum`**| 1          | –                | 1.945          | 1.0×        |
| `fastMD5`   | 1              | 0                | 1.594          | 1.22×       |
| `fastMD5`   | 1              | 1                | 0.454          | 4.29×       |
| `fastMD5`   | 1              | 2                | 0.164          | 11.85×      |
| **GNU `md5sum`**| **10**     | –                | **19.347**     | **1.0×**    |
| `fastMD5`   | 10             | 0                | 15.826         | 1.22×       |
| `fastMD5`   | 10             | 1                | 4.387          | 4.41×       |
| `fastMD5`   | 10             | 2                | 0.165          | 117.26×     |
| **GNU `md5sum`**| **40**     | –                | **79.040**     | **1.0×**    |
| `fastMD5`   | 40             | 0                | 63.150         | 1.25×       |
| `fastMD5`   | 40             | 1                | 18.738         | 4.22×       |
| `fastMD5`   | 40             | 2                | 0.173          | 456.65×     |
| **GNU `md5sum`**| **90**     | –                | **179.552**    | **1.0×**    |
| `fastMD5`   | 90             | 0                | 141.837        | 1.27×       |
| `fastMD5`   | 90             | 1                | 43.649         | 4.11×       |
| `fastMD5`   | 90             | 2                | 0.176          | 1020.27×    |

***Note:*** Wall-clock times were measured using `hyperfine` with three warm-up runs on a single CPU core. 

### Star
You can track updates by tab the **Star** button on the upper-right corner at the [github page](https://github.com/moold/fastMD5).
