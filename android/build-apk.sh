#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

if [ ! -f local.properties ]; then
    SDK_DIR="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
    if [ -n "$SDK_DIR" ]; then
        printf 'sdk.dir=%s\n' "$SDK_DIR" > local.properties
    fi
fi

for abi in arm64-v8a armeabi-v7a x86_64; do
    if [ ! -f "app/src/main/jniLibs/$abi/librget.so" ]; then
        echo "error: missing app/src/main/jniLibs/$abi/librget.so" >&2
        echo "Run ./android/build-android.sh once, then retry." >&2
        exit 1
    fi
done

AAPT2_OVERRIDE=""
case "$(uname -m)" in
    aarch64|arm64)
        if [ -n "${PREFIX:-}" ] && [ -x "$PREFIX/bin/aapt2" ]; then
            AAPT2_OVERRIDE="-Pandroid.aapt2FromMavenOverride=$PREFIX/bin/aapt2"
        fi
        ;;
esac

if [ -n "$AAPT2_OVERRIDE" ]; then
    gradle :app:assembleDebug "$AAPT2_OVERRIDE"
else
    gradle :app:assembleDebug
fi

echo "APK: $SCRIPT_DIR/app/build/outputs/apk/debug/app-debug.apk"
