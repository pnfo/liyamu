#!/bin/sh
# Builds the engine (libliyamu.a) for Android. Needs rustup with the Android targets:
#   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
# ABIS chooses which (default: arm64 only, enough for phones and the emulator on Apple Silicon).
set -eu
cd "$(dirname "$0")/.."
PATH="/opt/homebrew/opt/rustup/bin:$PATH"
for abi in ${ABIS:-arm64-v8a}; do
    case $abi in
        arm64-v8a) triple=aarch64-linux-android ;;
        armeabi-v7a) triple=armv7-linux-androideabi ;;
        x86_64) triple=x86_64-linux-android ;;
        *) echo "unknown ABI $abi"; exit 2 ;;
    esac
    cargo +stable rustc --release -p liyamu-ffi --target $triple --crate-type staticlib
done
