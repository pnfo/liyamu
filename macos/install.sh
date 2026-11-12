#!/bin/sh
# Builds and installs Liyamu for the current user and registers it. Then add it in System Settings ›
# Keyboard › Text Input › Input Sources › Edit › + › Sinhala › Liyamu (macOS does not let an installer
# switch input sources on), and pick the layout in its menu. After the first install, or if it is not
# listed, log out and back in: System Settings and the input menu only read new input methods at login.
set -eu
cd "$(dirname "$0")"
./build.sh
DIR="$HOME/Library/Input Methods"
mkdir -p "$DIR"
pkill -x Liyamu 2>/dev/null || true
# before 0.4 it was called Akkhara
pkill -x Akkhara 2>/dev/null || true
rm -rf "$DIR/Akkhara.app" "$DIR/Liyamu.app"
cp -R build/Liyamu.app "$DIR/"
"$DIR/Liyamu.app/Contents/MacOS/Liyamu" --register
echo "installed $DIR/Liyamu.app"
