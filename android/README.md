# Rget Android App

The Android app is a demo frontend for the shared Rust downloader in this
repository. It uses the same Rust engine as the CLI for downloads, resume
handling, filename resolution, and progress formatting.

## Features

- URL input with download and cancel controls
- Live progress percentage, downloaded/total size, speed, and ETA
- Automatic light/dark themes, with an AMOLED-friendly true black dark theme
- Public Downloads storage under `Download/<application-id>/`
- `MediaStore` on Android 10+
- Legacy public Downloads directory on Android 5–9 with the
  `WRITE_EXTERNAL_STORAGE` permission
- Filenames and content types resolved by the Rust core from the final response

## How it fits together

- `android/app/src/main/java/dev/jt/rget/MainActivity.kt` contains the UI and
  download flow.
- `Rget.kt` is the Kotlin wrapper around the native downloader.
- `src/jni_bridge.rs` in the repository root exposes the Rust core to Kotlin.
- `android/app/src/main/jniLibs/` contains the compiled `librget.so` files for
  each Android ABI.
- The Rust download engine lives in `src/` and is shared with the CLI.

The demo downloads into private app storage first. After the transfer finishes,
`DownloadsStore.kt` publishes the completed file to public Downloads using the
filename and content type returned by Rust. The app then deletes its temporary
copy.

## Requirements

- Android SDK Platform 34
- Android NDK with `ANDROID_NDK_HOME` or `ANDROID_NDK_ROOT` set
- JDK 17 or newer
- Gradle 9.x available as `gradle`
- [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk)
- Rust targets for the Android ABIs

## Build

Install the Rust targets and `cargo-ndk` once:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk
```

Set the NDK location:

```bash
export ANDROID_NDK_HOME=/path/to/android-ndk
```

Build the native libraries from the repository root:

```bash
./android/build-android.sh
```

This creates `librget.so` for `arm64-v8a`, `armeabi-v7a`, and `x86_64`. Override
the output directory with `JNI_LIBS_DIR` or the minimum API with
`ANDROID_API_LEVEL` (default `21`).

Build the debug APK:

```bash
./android/build-apk.sh
```

The APK is written to:

```text
android/app/build/outputs/apk/debug/app-debug.apk
```

`build-apk.sh` only runs Gradle and reuses the existing native libraries. Run
`build-android.sh` again whenever the Rust code or JNI bridge changes.

On Termux/aarch64, `build-apk.sh` automatically uses the system `aapt2` when
available.

## Storage behavior

- Android 10+ writes through `MediaStore` and does not require a storage
  permission.
- Android 5–9 writes to the legacy public Downloads directory and requests
  `WRITE_EXTERNAL_STORAGE` at runtime where required.
- Files are saved under `Download/<application-id>/`.
- Existing filenames are sanitized by the Rust core.

## Networking

The demo manifest enables cleartext HTTP for convenience. Use HTTPS in a
production app, or remove `usesCleartextTraffic` from
`app/src/main/AndroidManifest.xml` and add a network security configuration only
where plain HTTP is required.

## Rebuilding after Rust changes

Changes to the Rust core or JNI bridge require rebuilding the native libraries
before rebuilding the APK:

```bash
./android/build-android.sh
./android/build-apk.sh
```

Changes limited to Kotlin, resources, or the manifest only require:

```bash
./android/build-apk.sh
```
