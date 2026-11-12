#!/bin/sh
# Builds macos/build/Liyamu.app (ad-hoc signed, for this Mac's architecture).
# The release pipeline should build the Rust library for both arm64 and x86_64, lipo them, and sign with
# a Developer ID + notarize instead.
set -eu
cd "$(dirname "$0")"
ROOT=..
APP=build/Liyamu.app

cargo build --release -p liyamu-ffi --manifest-path "$ROOT/Cargo.toml"

rm -rf build
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
swiftc -O -swift-version 5 -module-name Liyamu \
    -target "$(uname -m)-apple-macos12" \
    -import-objc-header "$ROOT/core/liyamu-ffi/include/liyamu.h" \
    Sources/*.swift \
    -L "$ROOT/target/release" -lliyamu \
    -framework Cocoa -framework InputMethodKit -framework Carbon \
    -o "$APP/Contents/MacOS/Liyamu"

cp Resources/Info.plist "$APP/Contents/"
cp -R Resources/en.lproj Resources/si.lproj "$APP/Contents/Resources/"
cp Resources/fonts/*.ttf "$APP/Contents/Resources/"
swift make-icon.swift "$APP/Contents/Resources/icon.tiff" "ලි"
codesign --force --sign - "$APP"
echo "built $APP"
