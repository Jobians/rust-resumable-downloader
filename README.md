# 📥 Rget

A **fast, resumable downloader** written in Rust. Rget ships as a command-line
tool and an Android app, both powered by the same Rust core for downloads, resume
handling, filename resolution, and progress formatting.

---

## ✨ Features

* ✅ Resume interrupted downloads
* 📊 Live progress, speed, ETA, and size display
* 📱 Android app with download and cancel controls
* 🌓 Automatic light/dark themes, including AMOLED-friendly true black
* 💾 Saves to public Downloads on Android
* 🔗 Follows redirects and resolves filenames from `Content-Disposition` or the
  final URL
* ⚡ Lightweight, with TLS provided by `rustls` instead of OpenSSL
* 💻 CLI available from Cargo and Termux

---

## 🛠 Installation

### CLI

#### From Source

Make sure you have [Rust](https://www.rust-lang.org/tools/install) installed.

```bash
git clone https://github.com/jobians/rust-resumable-downloader.git
cd rust-resumable-downloader
cargo build --release
```

The compiled binary will be located in `target/release/rget`.

#### Termux (Android)

```bash
curl -LO https://github.com/jobians/rust-resumable-downloader/releases/download/v0.1.1/rget_0.1.1-1_aarch64.deb
dpkg -i rget_0.1.1-1_aarch64.deb
```

### Android App

The Android app is included in [`android/`](android/) and uses the same Rust
engine as the CLI. It provides download and cancel controls, live progress,
light/dark themes, and public Downloads storage.

Requirements and architecture are documented in
[`android/README.md`](android/README.md).

Build the native libraries and debug APK from the repository root:

```bash
./android/build-android.sh
./android/build-apk.sh
```

The APK is written to
`android/app/build/outputs/apk/debug/app-debug.apk`.

---

## 🚀 CLI Usage

```bash
rget [OPTIONS] <URL> [OUTPUT]
```

Options:

```text
-h, --help     Show help
-V, --version  Show version
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

## 📊 CLI Output

While downloading, `rget` displays progress percentage, downloaded/total size,
current speed, and estimated time remaining. Size, speed, and ETA strings are
formatted by the Rust core.

Example:

```
URL:    https://example.com/file.zip
Output: file.zip
45.23%  4.5 MB / 10.0 MB  500 KB/s  ETA 10s
Downloaded to file.zip (10.0 MB) in 2.41s
```

---

## 💖 Donate

If you like this project and want to support development, you can donate using crypto:

[Donate here](https://cwallet.com/t/TE6A6KMV)

---

## 📜 License

MIT License © 2025 [JOBIANSTECHIE](https://github.com/jobians)
