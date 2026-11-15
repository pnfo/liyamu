#!/bin/sh
# Builds the Windows installer on a Mac (or Linux): the keyboard DLL for x64, x86 and ARM64 and the
# settings program, cross-compiled with cargo-xwin (MSVC libraries downloaded by xwin), packed by NSIS.
#   needs: rustup targets x86_64/i686/aarch64-pc-windows-msvc, cargo install cargo-xwin,
#          brew install llvm lld makensis
# Output: windows/dist/LiyamuSetup-<version>.exe (unsigned; sign it before release)
set -e
cd "$(dirname "$0")"
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:/opt/homebrew/opt/llvm/bin:$PATH"
export XWIN_ARCH=x86,x86_64,aarch64
version=$(sed -n 's/^version = "\(.*\)"/\1/p' ../Cargo.toml | head -1)
stage=dist/stage
rm -rf "$stage" && mkdir -p "$stage/x64" "$stage/x86" "$stage/arm64" "$stage/fonts"

build() { cargo xwin build -q --release -p liyamu-windows --target "$1-pc-windows-msvc"; }
build x86_64
build i686
build aarch64
out=../target
cp "$out/x86_64-pc-windows-msvc/release/liyamu_ime.dll" "$stage/x64/"
cp "$out/i686-pc-windows-msvc/release/liyamu_ime.dll" "$stage/x86/"
cp "$out/aarch64-pc-windows-msvc/release/liyamu_ime.dll" "$stage/arm64/"
cp "$out/x86_64-pc-windows-msvc/release/liyamu-settings.exe" "$stage/liyamu-settings-x64.exe"
cp "$out/aarch64-pc-windows-msvc/release/liyamu-settings.exe" "$stage/liyamu-settings-arm64.exe"
cp ../macos/Resources/fonts/AbhayaUni-Regular.ttf "$stage/fonts/"

makensis -V2 -DVERSION="$version" -DSTAGE="$stage" -DOUTFILE="dist/LiyamuSetup-$version.exe" installer.nsi
ls -la "dist/LiyamuSetup-$version.exe"
