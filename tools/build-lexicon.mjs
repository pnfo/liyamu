// Builds what suggestions know about words, from data/corpus/train.txt (tools/build-corpus.mjs):
//   data/lexicon/words.tsv   every word seen at least MIN times (MIN_SUBS if only in subtitles, which have
//                            more misspellings), with its count
//   data/lexicon/ngram.bin   which word follows which (bigrams), for suggestions in context
// ngram.bin: "AKNG1\0\0\0", u32 entry count, then sorted u64 entries (little-endian), each
//   (48-bit FNV-1a hash of the key) << 16 | min(value, 65535)
// with keys "w1 w2" → how often w2 followed w1, and "w1\t" → how many different words followed w1.
// Usage: node tools/build-lexicon.mjs
import fs from 'node:fs'
import path from 'node:path'
import readline from 'node:readline'
import { headwords } from './headwords.mjs'
import { execFileSync } from 'node:child_process'

const repo = path.resolve(import.meta.dirname, '..')
const MIN = 3        // word count
const MIN_SUBS = 6   // word count, for words the books never use
const MIN_PAIR = 3   // bigram count

const words = new Map(), pairs = new Map(), inBooks = new Set(), inSubs = new Set()
const rl = readline.createInterface({ input: fs.createReadStream(path.join(repo, 'data/corpus/train.txt')) })
for await (const line of rl) {
    const [source, text] = line.split('\t')
    const ws = text.split(' ')
    for (let i = 0; i < ws.length; i++) {
        if (source === 'books') inBooks.add(ws[i])
        else inSubs.add(ws[i])
        words.set(ws[i], (words.get(ws[i]) || 0) + 1)
        if (i > 0) {
            const k = ws[i - 1] + ' ' + ws[i]
            pairs.set(k, (pairs.get(k) || 0) + 1)
        }
    }
}
// ---- misspellings: a word that differs from another only in ල/ළ, න/ණ, ශ/ෂ, where the dictionary knows the
// other spelling (the word, or a headword it starts with that covers the letters in question) and not this
// one (nor a headword this one could be a form of). Its counts go to the right spelling.
const heads = headwords()
const norm = w => w.replace(/ළ/g, 'ල').replace(/ණ/g, 'න').replace(/ෂ/g, 'ශ')
const groups = new Map()
for (const w of new Set([...words.keys(), ...heads])) {
    const n = norm(w)
    if (!groups.has(n)) groups.set(n, [])
    groups.get(n).push(w)
}
// is the spelling of the first `upto` letters of w what a headword has?
function backed(w, upto) {
    for (let n = Math.max(upto, 2); n <= w.length; n++) if (heads.has(w.slice(0, n))) return true
    return false
}
// the same, more loosely, for the spelling in doubt: w without its last two letters starts a headword
// (පෙනේ: පෙනෙනවා), so it may be a form of that word
const starts = new Set()
for (const h of heads) for (let n = 2; n <= h.length; n++) starts.add(h.slice(0, n))
const maybe = (w, upto) => backed(w, upto) || starts.has(w.slice(0, Math.max(upto, w.length - 2, 2)))
const fixes = new Map()
for (const group of groups.values()) {
    if (group.length < 2) continue
    for (const w of group) {
        if (!words.has(w)) continue
        let best = null
        for (const v of group) {
            if (v === w) continue
            const diff = [...w].map((c, i) => c !== v[i] ? i : -1).filter(i => i >= 0)
            // ශ/ෂ at the start of borrowed words and names (ෂර්ට්) needs a longer headword to decide
            const least = diff.some(i => 'ශෂ'.includes(w[i])) ? 4 : 2
            if (maybe(w, diff[0] + 1) || !backed(v, Math.max(diff.at(-1) + 1, least))) continue
            // the right spelling is itself a headword, or not much rarer than the wrong one
            if (!heads.has(v) && (words.get(v) || 0) * 20 < words.get(w)) continue
            if (!best || (words.get(v) || 0) > (words.get(best) || 0)) best = v
        }
        if (best) fixes.set(w, best)
    }
}
// a rakaransaya or yansaya typed without its joiner (ප්රශ්නය), when the joined word is the commoner one
for (const [w, c] of words) {
    if (fixes.has(w) || !/(?<!ර)\u0DCA[රය]/.test(w)) continue
    const v = w.replace(/(?<!ර)\u0DCA(?=[රය])/g, '\u0DCA\u200D')
    if ((words.get(v) || 0) > c) fixes.set(w, fixes.get(v) || v)
}
for (const [w, v] of fixes) {
    words.set(v, (words.get(v) || 0) + words.get(w))
    if (inBooks.has(w)) inBooks.add(v)
}
const fixed = [...fixes].sort((a, b) => words.get(b[0]) - words.get(a[0]))
fs.writeFileSync(path.join(repo, 'data/lexicon/misspellings.tsv'),
    '# wrong\tright\tcount — spellings in the corpus that the dictionary headwords show to be wrong (ල/ළ, න/ණ, ශ/ෂ, a missing joiner); they are\n'
    + '# counted as the right word and are not suggested. Built by tools/build-lexicon.mjs.\n'
    + fixed.map(([w, v]) => `${w}\t${v}\t${words.get(w)}`).join('\n') + '\n')
console.log(`misspellings.tsv: ${fixed.length} spellings, ${fixed.reduce((n, [w]) => n + words.get(w), 0)} uses`)
for (const w of fixes.keys()) words.delete(w)
for (const [k, c] of [...pairs]) {
    const [a, b] = k.split(' ')
    if (!fixes.has(a) && !fixes.has(b)) continue
    pairs.delete(k)
    const k2 = (fixes.get(a) || a) + ' ' + (fixes.get(b) || b)
    pairs.set(k2, (pairs.get(k2) || 0) + c)
}

const rows = [...words].filter(([w, c]) => c >= (inBooks.has(w) ? MIN : MIN_SUBS)).sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1))
const out = path.join(repo, 'data/lexicon/words.tsv')
fs.writeFileSync(out, `# word\tcount — from the pitaka.lk books and OpenSubtitles (95% of the sentences), words seen at least ${MIN} times. Built by tools/build-lexicon.mjs.\n`
    + rows.map(([w, c]) => `${w}\t${c}`).join('\n') + '\n')
console.log(`${out}: ${rows.length} words`)

// ---- Pali-only words: forms the Digital Pāḷi Dictionary knows (arutha.lk's db) that everyday Sinhala does not
// use (not a Sinhala dictionary headword, never in the subtitles, not in everyday.tsv). Suggestions leave
// them out unless Pali words are switched on.
const ROMAN = { ක: 'k', ඛ: 'kh', ග: 'g', ඝ: 'gh', ඞ: 'ṅ', ච: 'c', ඡ: 'ch', ජ: 'j', ඣ: 'jh', ඤ: 'ñ', ට: 'ṭ', ඨ: 'ṭh', ඩ: 'ḍ', ඪ: 'ḍh',
    ණ: 'ṇ', ත: 't', ථ: 'th', ද: 'd', ධ: 'dh', න: 'n', ප: 'p', ඵ: 'ph', බ: 'b', භ: 'bh', ම: 'm', ය: 'y', ර: 'r', ල: 'l', ව: 'v',
    ස: 's', හ: 'h', ළ: 'ḷ' }
const VOWEL = { අ: 'a', ආ: 'ā', ඉ: 'i', ඊ: 'ī', උ: 'u', ඌ: 'ū', එ: 'e', ඒ: 'e', ඔ: 'o', ඕ: 'o' }
const SIGN = { 'ා': 'ā', 'ි': 'i', 'ී': 'ī', 'ු': 'u', 'ූ': 'ū', 'ෙ': 'e', 'ේ': 'e', 'ො': 'o', 'ෝ': 'o' }
function roman(word) {
    let out = ''
    const cs = [...word.replace(/\u200D/g, '')]
    for (let i = 0; i < cs.length; i++) {
        const c = cs[i]
        if (ROMAN[c]) {
            out += ROMAN[c]
            const n = cs[i + 1]
            if (n === '\u0DCA') i++
            else if (SIGN[n]) { out += SIGN[n]; i++ } else out += 'a'
        } else if (VOWEL[c]) out += VOWEL[c]
        else if (c === 'ං') out += 'ṃ'
        else return null
    }
    return out
}
const dpd = new Set(execFileSync('sqlite3', [path.resolve(repo, '../../arutha.lk/db/dict.db'), 'select word from dpd_lookup'],
    { maxBuffer: 1 << 28, encoding: 'utf8' }).split('\n'))
const everyday = new Set(fs.readFileSync(path.join(repo, 'data/lexicon/everyday.tsv'), 'utf8').split('\n').map(l => l.split('\t')[0]))
const pali = rows.map(r => r[0]).filter(w => !heads.has(w) && !inSubs.has(w) && !everyday.has(w) && dpd.has(roman(w))).sort()
fs.writeFileSync(path.join(repo, 'data/lexicon/pali.txt'),
    '# Words of the corpus that are Pali only: forms in the Digital Pāḷi Dictionary that are not Sinhala dictionary headwords\n'
    + '# and that the subtitles never use. Suggested only when Pali words are on. Built by tools/build-lexicon.mjs.\n'
    + pali.join('\n') + '\n')
console.log(`pali.txt: ${pali.length} words`)

// bigrams between known words; followers counts every different follower, kept or not
const known = new Set(rows.map(r => r[0]))
const followers = new Map(), entries = []
const MASK = (1n << 48n) - 1n
function hash(s) {
    let h = 0xcbf29ce484222325n
    for (const b of Buffer.from(s, 'utf8')) h = ((h ^ BigInt(b)) * 0x100000001b3n) & 0xffffffffffffffffn
    return (h ^ (h >> 48n)) & MASK
}
const entry = (key, value) => (hash(key) << 16n) | BigInt(Math.min(value, 65535))
for (const [k, c] of pairs) {
    const [a, b] = k.split(' ')
    if (!known.has(a)) continue
    followers.set(a, (followers.get(a) || 0) + 1)
    if (c >= MIN_PAIR && known.has(b)) entries.push(entry(k, c))
}
const kept = entries.length
for (const [a, t] of followers) entries.push(entry(a + '\t', t))
entries.sort((x, y) => (x < y ? -1 : x > y ? 1 : 0))
const buf = Buffer.alloc(12 + entries.length * 8)
buf.write('AKNG1\0\0\0', 0, 'latin1')
buf.writeUInt32LE(entries.length, 8)
entries.forEach((e, i) => buf.writeBigUInt64LE(e, 12 + i * 8))
const bin = path.join(repo, 'data/lexicon/ngram.bin')
fs.writeFileSync(bin, buf)
console.log(`${bin}: ${kept} word pairs, ${followers.size} contexts, ${(buf.length / 1e6).toFixed(1)} MB`)
