#!/bin/sh
# windows/res/liyamu.ico from design/logo/icon-1024.png (macOS: sips; node packs the PNGs into an .ico)
set -e
cd "$(dirname "$0")"
tmp=$(mktemp -d)
for s in 16 20 24 32 40 48 64 256; do sips -z $s $s ../../design/logo/icon-1024.png --out "$tmp/$s.png" >/dev/null; done
node -e '
const fs = require("fs"), dir = process.argv[1], sizes = [16, 20, 24, 32, 40, 48, 64, 256]
const pngs = sizes.map(s => fs.readFileSync(`${dir}/${s}.png`))
const head = Buffer.alloc(6 + 16 * sizes.length)
head.writeUInt16LE(1, 2); head.writeUInt16LE(sizes.length, 4)
let off = head.length
sizes.forEach((s, i) => {
  const e = 6 + 16 * i
  head[e] = s % 256; head[e + 1] = s % 256
  head.writeUInt16LE(1, e + 4); head.writeUInt16LE(32, e + 6)
  head.writeUInt32LE(pngs[i].length, e + 8); head.writeUInt32LE(off, e + 12)
  off += pngs[i].length
})
fs.writeFileSync("liyamu.ico", Buffer.concat([head, ...pngs]))
' "$tmp"
rm -r "$tmp"
