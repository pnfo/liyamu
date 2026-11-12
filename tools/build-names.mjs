// Builds the word lists that do not come from running text:
//   data/lexicon/places.tsv       words of Sri Lankan place names (provinces, districts, divisions, GN
//                                 divisions, towns) and of the names of Sri Lankan people
//   data/lexicon/dictionary.txt   headwords of the Sinhala-Sinhala dictionaries of arutha.lk that the
//                                 corpus word list does not have (so that they count as real words)
// Sources (downloaded to data/corpus/names/, not committed):
//   hdx/x/…          lka_admin_boundaries.xlsx unzipped — OCHA COD-AB Sri Lanka (Survey Department names)
//                    https://data.humdata.org/dataset/cod-ab-lka
//   osm-all.tsv      OpenStreetMap places: place, name:si, name, name:en (Overpass)
//   people.tsv       Wikidata: Sinhala labels of people with Sri Lankan citizenship
//   ../../../../arutha.lk/data/dicts/{sankshiptha,akshara_vinyasa}.txt
// Usage: node tools/build-names.mjs
import fs from 'node:fs'
import path from 'node:path'
import { clean, ok, headwords } from './headwords.mjs'

const repo = path.resolve(import.meta.dirname, '..')
const src = path.join(repo, 'data/corpus/names')
const known = new Map()
for (const l of fs.readFileSync(path.join(repo, 'data/lexicon/words.tsv'), 'utf8').split('\n')) {
    const [w, c] = l.split('\t')
    if (c) known.set(w, +c)
}

// ---- places
const MAJOR = 200, MINOR = 20, PER_USE = 10
const places = new Map() // word → {major, n}
function place(name, major) {
    // the sheet cuts names at about 17 letters: the last word of such a name may be cut
    let ws = name.trim().split(/[\s,()/\-.]+/)
    if ([...name].length >= 17) ws = ws.slice(0, -1)
    for (let w of ws) {
        // ඵ typed for ළු (කඵතර), a common mistake in the source
        w = clean(w).replace(/ඵ(?!ල)/g, 'ළු')
        if (!ok(w)) continue
        const e = places.get(w) || places.set(w, { major: false, n: 0 }).get(w)
        e.major ||= major
        e.n++
    }
}
const strings = [...fs.readFileSync(path.join(src, 'hdx/x/xl/sharedStrings.xml'), 'utf8').matchAll(/<si>(.*?)<\/si>/gs)]
    .map(m => [...m[1].matchAll(/<t[^>]*>([^<]*)<\/t>/g)].map(t => t[1]).join(''))
for (const [sheet, major] of [[2, true], [3, true], [4, true], [5, false]]) {
    const xml = fs.readFileSync(path.join(src, `hdx/x/xl/worksheets/sheet${sheet}.xml`), 'utf8')
    for (const m of xml.matchAll(/<c [^>]*t="s"[^>]*>\s*<v>(\d+)<\/v>/g)) {
        const s = strings[+m[1]]
        if (/[඀-෿]/.test(s)) place(s.replace(/දිස්ත්‍රික්?ක?ය?$/, ''), major)
    }
}
for (const l of fs.readFileSync(path.join(src, 'osm-all.tsv'), 'utf8').split('\n')) {
    const [kind, si, name] = l.split('\t')
    const major = kind === 'city' || kind === 'town'
    for (const n of [si, ...(name || '').split(/[;/]/)]) if (n && /[඀-෿]/.test(n)) place(n, major)
}

// ---- people: a name word used by at least two people
const people = new Map()
for (const l of fs.readFileSync(path.join(src, 'people.tsv'), 'utf8').split('\n').slice(1)) {
    const name = l.replace(/^"|"@si$/g, '')
    for (const w of new Set(name.split(/[\s,()/\-.]+/).map(clean))) if (ok(w)) people.set(w, (people.get(w) || 0) + 1)
}

const rows = new Map()
for (const [w, e] of places) rows.set(w, e.major ? MAJOR : MINOR + PER_USE * Math.min(e.n, 8))
for (const [w, n] of people) if (n >= 2) rows.set(w, Math.max(rows.get(w) || 0, 40 + PER_USE * Math.min(n, 10)))
// only what adds something to the corpus counts
const list = [...rows].filter(([w, c]) => c > (known.get(w) || 0)).sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1))
fs.writeFileSync(path.join(repo, 'data/lexicon/places.tsv'),
    '# word\tcount — words of Sri Lankan place names (OCHA COD-AB Sri Lanka administrative names, OpenStreetMap) and of\n'
    + '# names of Sri Lankan people (Wikidata). Counts are weights, not measurements. Built by tools/build-names.mjs.\n'
    + list.map(([w, c]) => `${w}\t${c}`).join('\n') + '\n')
console.log(`places.tsv: ${list.length} words (${places.size} place words, ${[...people.values()].filter(n => n >= 2).length} name words)`)

// ---- dictionary headwords
const heads = [...headwords()].filter(w => !known.has(w) && !rows.has(w))
const hs = heads.sort()
fs.writeFileSync(path.join(repo, 'data/lexicon/dictionary.txt'),
    '# Headwords of the Concise Sinhala Dictionary and the Spelling Dictionary (Sinhala Dictionary Office, as on arutha.lk)\n'
    + '# that the corpus word list does not have. Built by tools/build-names.mjs.\n' + hs.join('\n') + '\n')
console.log(`dictionary.txt: ${hs.length} words`)
