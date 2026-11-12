// Exports the FM and Isi tables from pitaka.lk (the single source of truth) as JSON for the Rust core,
// plus fixtures made with pitaka.lk's own converter so the Rust port can be checked against it.
// Usage: node tools/export-encodings.mjs [path/to/pitaka.lk]
import fs from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

const repo = path.resolve(import.meta.dirname, '..')
const pitaka = path.resolve(process.argv[2] || process.env.PITAKA_DIR || path.join(repo, '../../pitaka.lk'))
const load = async (p) => import(pathToFileURL(path.join(pitaka, p)).href)

const { toUnicode, fromUnicode } = await load('tools/unicode/lib/convert.js')
const { bookWords } = await load('dev/unicode/corpus.mjs')
const encs = { fm: (await load('tools/unicode/lib/encodings/fm.js')).default, isi: (await load('tools/unicode/lib/encodings/isi.js')).default }

const src = (re) => (re.flags.includes('i') ? '(?i)' : '') + re.source
const rules = (list) => (list || []).map(([re, to]) => [src(re), to])
const out = path.join(repo, 'data')
for (const [id, enc] of Object.entries(encs)) {
    const json = {
        id, label: enc.label,
        // ordered pairs: where keys give the same Unicode, the first one is what fromUnicode() writes
        map: Object.entries(enc.map),
        pre: rules(enc.pre), post: rules(enc.post), unpost: rules(enc.unpost),
        variants: (enc.variants || []).map((v) => ({ fontNames: v.fontNames.map(src), map: Object.entries(v.map) })),
    }
    fs.writeFileSync(path.join(out, 'encodings', id + '.json'), JSON.stringify(json, null, 1) + '\n')
}

// fixtures: the most common book words, written to legacy and read back by pitaka.lk's converter
const words = [...bookWords().entries()].sort((a, b) => b[1] - a[1]).slice(0, 20000).map(([w]) => w)
for (const [id, enc] of Object.entries(encs)) {
    const rows = words.map((w) => {
        const legacy = fromUnicode(w, enc)
        return [w, legacy ?? '', legacy === null ? '' : toUnicode(legacy, enc)].map(escape).join('\t')
    })
    fs.writeFileSync(path.join(out, 'fixtures', id + '-words.tsv'), '# unicode\tlegacy (empty = not writable)\tread back\n' + rows.join('\n') + '\n')
}
// tabs, newlines and backslashes inside fields are escaped
function escape(s) { return s.replace(/\\/g, '\\\\').replace(/\t/g, '\\t').replace(/\n/g, '\\n') }
console.log('wrote data/encodings/{fm,isi}.json and data/fixtures/{fm,isi}-words.tsv')
