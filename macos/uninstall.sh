#!/bin/sh
# Removes Liyamu. Remove it from the input sources in System Settings too.
set -eu
pkill -x Liyamu 2>/dev/null || true
DIR="$HOME/Library/Input Methods"
rm -rf "$DIR/Liyamu.app"
echo "removed"
