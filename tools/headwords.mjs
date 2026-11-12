// Headwords of the Sinhala-Sinhala dictionaries of arutha.lk (Concise Sinhala Dictionary and Spelling
// Dictionary of the Sinhala Dictionary Office), as single words.
import fs from 'node:fs'
import path from 'node:path'

const dicts = path.resolve(import.meta.dirname, '../../../arutha.lk/data/dicts')
const SINHALA = /^[඀-෿‍]+$/
export const clean = w => w.normalize('NFC').replace(/‌/g, '').replace(/‍+$/, '').replace(/^‍+/, '')
/** a word: Sinhala only, at least two letters */
export const ok = w => SINHALA.test(w) && w.replace(/[්-ෟ‍ංඃ]/g, '').length >= 2 && !/^[්-ෟ]/.test(w)

export function headwords() {
    const heads = new Set()
    for (const f of ['sankshiptha.txt', 'akshara_vinyasa.txt']) {
        for (const block of fs.readFileSync(path.join(dicts, f), 'utf8').split(/\n\s*\n/)) {
            for (const w of block.trim().split('\n')[0].split(/[\s,;()/\-.]+/).map(clean)) if (ok(w)) heads.add(w)
        }
    }
    return heads
}
