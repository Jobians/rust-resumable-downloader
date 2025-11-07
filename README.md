# 📥 Rget

A **fast resumable downloader** written in Rust. `rget` allows you to download files from the internet with resume support and real-time progress display.

---

## ✨ Features

* ✅ Resume interrupted downloads
* 📊 Displays download progress, speed, and ETA
* ⚡ Lightweight and fast
* 💻 Easy to use from the command line

---

## 🛠 Installation

### From Source

Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed.

```bash
git clone https://github.com/jobians/rust-resumable-downloader.git
cd rust-resumable-downloader
cargo build --release
```

The compiled binary will be located in `target/release/rget`.

### Termux (Android)

```bash
curl -LO https://github.com/Jobians/rust-resumable-downloader/releases/download/v0.1.0/rget_0.1.0-1_aarch64.deb
dpkg -i rget_0.1.0-1_aarch64.deb
```

---

## 🚀 Usage

```bash
rget <url> [output_path]
```

### Examples

Download a file to the current directory:

```bash
rget https://example.com/file.zip
```

Download a file to a specific path:

```bash
rget https://example.com/file.zip ./downloads/file.zip
```

---

## 📊 Output

While downloading, `rget` displays:

* 📈 Download progress percentage
* 📦 Downloaded size / Total size
* ⚡ Current download speed
* ⏱ Estimated time remaining (ETA)

Example:

```
📥 Starting download from: https://example.com/file.zip
🆕 Starting new download
📄 Saving to: file.zip
45.23% (4.5/10.0 MB) 500 KB/s | ETA: 00m:10s
```

---

## 💖 Donate

If you like this plugin and want to support development, you can donate using crypto:  

[Donate here](https://cwallet.com/t/TE6A6KMV)

---

## 📜 License

MIT License © 2025 [JOBIANSTECHIE](https://github.com/jobians)
