#!/usr/bin/env python3
"""Checks that Liyamu writes each Sinhala syllable in FM with the keys that draw it correctly.

FM fonts have more than one key for some signs (two papillas, two ispillas, two hal signs) and letters with
keys of their own, and which one is right depends on the letter. This draws every consonant + sign in FM
Abhaya (as Liyamu would write it, and with every other glyph of the font in place of the sign) and in Abhaya
Uni, which has the same letter shapes, and compares the pictures. It prints the syllables where other keys
look closer than Liyamu's. Look at those (--sheet draws them) and put the right ones in
pitaka.lk's FM table (tools/unicode/lib/encodings/fm.js: `after`, `readOnly`, key order), then export it again. Not every line is a mistake: the FM ligatures (දා ඳා ඤා ...) and the two
ispillas score close to each other.

  python3 -m venv venv && venv/bin/pip install pillow numpy fonttools
  cargo build --release -p liyamu-cli
  venv/bin/python tools/fm-forms-check.py /path/to/FMABHAYA.TTF [--sheet out.png] [--margin 0.04]
"""
import argparse, os, subprocess, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont, features
from fontTools.ttLib import TTFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ap = argparse.ArgumentParser()
ap.add_argument('fm_font')
ap.add_argument('--uni', default=os.path.join(ROOT, 'macos/Resources/fonts/AbhayaUni-Regular.ttf'))
ap.add_argument('--cli', default=os.path.join(ROOT, 'target/release/liyamu'))
ap.add_argument('--sheet', help='draw the reported syllables to this PNG: Unicode, Liyamu, the closest other keys')
ap.add_argument('--margin', type=float, default=0.04, help='report when another key scores this much higher')
args = ap.parse_args()
if not features.check('raqm'):
    sys.exit('this Pillow cannot shape Sinhala (no raqm)')

SIZE, W, H, X0, Y0 = 96, 440, 240, 60, 150
fm = ImageFont.truetype(args.fm_font, SIZE, layout_engine=ImageFont.Layout.BASIC)

def draw(font, text):
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).text((X0, Y0), text, font=font, fill=255, anchor='ls')
    return np.asarray(im) > 96

def box(a):
    ys, xs = np.where(a)
    return (ys.min(), ys.max() + 1, xs.min(), xs.max() + 1) if len(xs) else (0, 1, 0, 1)

# the two fonts draw the same shapes at slightly different sizes: scale Abhaya Uni to FM by the plain letters
ratios = []
probe = ImageFont.truetype(args.uni, SIZE, layout_engine=ImageFont.Layout.RAQM)
for f, u in [('l', 'ක'), ('u', 'ම'), (';', 'ත'), ('h', 'ය'), ('i', 'ස'), ('n', 'බ'), ('j', 'ව'), ('y', 'හ')]:
    a, b = box(draw(fm, f)), box(draw(probe, u))
    ratios += [(a[1] - a[0]) / (b[1] - b[0]), (a[3] - a[2]) / (b[3] - b[2])]
uni = ImageFont.truetype(args.uni, SIZE * float(np.median(ratios)), layout_engine=ImageFont.Layout.RAQM)

def score(a, b):
    """How alike two pictures drawn from the same point are (shared ink / all ink), best of small shifts."""
    (t1, b1, l1, r1), (t2, b2, l2, r2) = box(a), box(b)
    t, bo, l, r = max(0, min(t1, t2) - 6), min(H, max(b1, b2) + 6), max(0, min(l1, l2) - 8), min(W, max(r1, r2) + 8)
    a, b = a[t:bo, l:r], b[t:bo, l:r]
    best = 0.0
    for dx in range(-5, 6):
        for dy in range(-3, 4):
            s = np.roll(np.roll(b, dx, 1), dy, 0)
            best = max(best, (a & s).sum() / max(1, (a | s).sum()))
    return best

def to_fm(text):
    return subprocess.run([args.cli, 'to-fm', text], capture_output=True, text=True, check=True).stdout.rstrip('\n')

CONS = [chr(c) for c in range(0x0D9A, 0x0DC7) if chr(c) not in '඲඼඾඿']
CLUSTERS = CONS + ['ක්‍ෂ', 'ක්‍ව', 'ත්‍ථ', 'ත්‍ව', 'න්‍ද', 'න්‍ධ', 'ද්‍ධ', 'ද්‍ව', 'ක්‍ර', 'ත්‍ර', 'ප්‍ර', 'ද්‍ර', 'ග්‍ර', 'බ්‍ර', 'ශ්‍ර',
                   'ක්‍ය', 'ත්‍ය', 'ද්‍ය', 'ව්‍ය']
SIGNS = ['', '්', 'ා', 'ැ', 'ෑ', 'ි', 'ී', 'ු', 'ූ', 'ෘ']
glyphs = [chr(c) for c in sorted(TTFont(args.fm_font).getBestCmap()) if c > 0x20]
cache = {}
def fm_pic(t):
    if t not in cache:
        cache[t] = draw(fm, t)
    return cache[t]

found = []
for cl in CLUSTERS:
    base = to_fm(cl)
    for sign in SIGNS:
        syl = cl + sign
        cur = to_fm(syl)
        ref = draw(uni, syl)
        mine = score(fm_pic(cur), ref)
        others = {base + g for g in glyphs} | set(glyphs) if sign else set(glyphs) | {a + b for a in '`C' for b in glyphs}
        top = sorted(((score(fm_pic(c), ref), c) for c in others - {cur}), reverse=True)[:3]
        if top[0][0] > mine + args.margin:
            found.append((syl, cur, mine, top))
            print(f'{syl}\tLiyamu {cur} {mine:.2f}\tcloser: ' + '  '.join(f'{c} {s:.2f}' for s, c in top), flush=True)
print(f'{len(found)} of {len(CLUSTERS) * len(SIGNS)} syllables have closer keys')

if args.sheet and found:
    label = ImageFont.truetype('/System/Library/Fonts/Menlo.ttc', 12) if os.path.exists('/System/Library/Fonts/Menlo.ttc') else ImageFont.load_default()
    cw, ch, per = 135, 125, 2
    rows = (len(found) + per - 1) // per
    im = Image.new('RGB', ((cw * 4 + 16) * per, ch * rows), 'white')
    d = ImageDraw.Draw(im)
    for n, (syl, cur, mine, top) in enumerate(found):
        x, y = (n % per) * (cw * 4 + 16), (n // per) * ch
        d.text((x + 8, y + 80), syl, font=uni, fill=(0, 90, 0), anchor='ls')
        for i, (s, t) in enumerate([(mine, cur)] + top[:2]):
            d.text((x + cw * (i + 1) + 8, y + 80), t, font=fm, fill='black', anchor='ls')
            d.text((x + cw * (i + 1) + 4, y + 106), f'{t} {s:.2f}', font=label, fill=(0, 0, 160) if i == 0 else 'gray')
        d.rectangle([x, y, x + cw * 4 + 8, y + ch - 1], outline=(215, 215, 215))
    im.save(args.sheet)
