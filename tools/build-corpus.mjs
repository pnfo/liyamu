// Builds the sentence corpus that suggestions learn from, into data/corpus/ (not committed):
//   train.txt    95% of the sentences, one per line: "books" or "subs", a tab, Sinhala words separated by spaces
//   heldout.txt  the other 5% (every 20th sentence), same format, for `liyamu eval`
// Sources: the pitaka.lk books (formal, with Pali) and data/corpus/opensubtitles-si.txt.gz (everyday speech):
//   curl -L -o data/corpus/opensubtitles-si.txt.gz https://object.pouta.csc.fi/OPUS-OpenSubtitles/v2018/mono/si.txt.gz
// Usage: node tools/build-corpus.mjs [path/to/pitaka.lk]
import fs from 'node:fs'
import path from 'node:path'
import zlib from 'node:zlib'

const repo = path.resolve(import.meta.dirname, '..')
const pitaka = path.resolve(process.argv[2] || process.env.PITAKA_DIR || path.join(repo, '../../pitaka.lk'))
const dir = path.join(repo, 'data/corpus')

// words are NFC; ZWNJ is noise, and so is a ZWJ that is not part of a joined (්‍ර ්‍ය) or touching (‍්) letter
const clean = w => w.normalize('NFC').replace(/\u200c/g, '').replace(/(?<!\u0dca)\u200d(?!\u0dca)/g, '')
function* sentences(text) {
    for (const s of text.split(/[.?!:;|\n]+/)) {
        const words = (s.match(/[඀-෿‌‍]+/g) || []).map(clean).filter(Boolean)
        if (words.length >= 2) yield words.join(' ')
    }
}

function* bookSentences() {
    const root = path.join(pitaka, 'books')
    for (const book of fs.readdirSync(root).sort()) {
        const d = path.join(root, book)
        if (!fs.statSync(d).isDirectory()) continue
        for (const f of fs.readdirSync(d).sort()) {
            if (!f.endsWith('.html') || f === 'index.html') continue
            const html = fs.readFileSync(path.join(d, f), 'utf8')
            const text = html.replace(/<script[\s\S]*?<\/script>|<style[\s\S]*?<\/style>/g, '\n')
                .replace(/<\/?(p|div|br|h\d|li|td|tr|blockquote)[^>]*>/gi, '\n').replace(/<[^>]+>/g, ' ')
            yield* sentences(text)
        }
    }
}

function* subtitleSentences() {
    const file = path.join(dir, 'opensubtitles-si.txt.gz')
    if (!fs.existsSync(file)) return console.warn(`missing ${file}: only the books are used`)
    yield* sentences(zlib.gunzipSync(fs.readFileSync(file)).toString('utf8'))
}

const train = fs.createWriteStream(path.join(dir, 'train.txt'))
const held = fs.createWriteStream(path.join(dir, 'heldout.txt'))
for (const [name, gen] of [['books', bookSentences()], ['subs', subtitleSentences()]]) {
    let n = 0, words = 0
    for (const s of gen) {
        if (++n % 20 === 0) held.write(`${name}\t${s}\n`)
        else train.write(`${name}\t${s}\n`)
        words += s.split(' ').length
    }
    console.log(`${name}: ${n} sentences, ${words} words`)
}
train.end()
held.end()
