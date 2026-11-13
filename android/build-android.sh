#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"

ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-21}"
JNI_LIBS_DIR="${JNI_LIBS_DIR:-$SCRIPT_DIR/app/src/main/jniLibs}"

if ! command -v cargo-ndk >/dev/null 2>&1; then
    echo "error: cargo-ndk is required. Install it with: cargo install cargo-ndk" >&2
    exit 1
fi

if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -z "${ANDROID_NDK_ROOT:-}" ]; then
    echo "error: ANDROID_NDK_HOME (or ANDROID_NDK_ROOT) must point to an Android NDK" >&2
    exit 1
fi

ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_NDK_ROOT}"
export ANDROID_NDK_HOME

MISSING=""

for target in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
    if [ ! -d "$(rustc --print sysroot)/lib/rustlib/${target}/lib" ]; then
        MISSING="$MISSING $target"
    fi
done

if [ -n "$MISSING" ]; then
    echo "error: missing Rust standard library for:$MISSING" >&2
    echo "Install it with:" >&2
    echo "  rustup target add$MISSING" >&2
    exit 1
fi

mkdir -p "$JNI_LIBS_DIR"

cargo ndk \
    -t arm64-v8a \
    -t armeabi-v7a \
    -t x86_64 \
    -P "$ANDROID_API_LEVEL" \
    -o "$JNI_LIBS_DIR" \
    build --release --lib

for abi in arm64-v8a armeabi-v7a x86_64; do
    echo "built: $JNI_LIBS_DIR/$abi/librget.so"
done
