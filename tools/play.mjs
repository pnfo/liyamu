// Google Play: the listing (texts of docs/store/listing.md, screenshots, feature graphic, icon) and the signed
// bundle, through the Play Developer API with the service account key of the Play console (PLAY_KEY, a file
// outside the repo). The app must already exist in the console: the API cannot create one.
//   PLAY_KEY=/path/key.json node tools/play.mjs listing [--dry]
//   PLAY_KEY=/path/key.json node tools/play.mjs release [--dry]     TRACK=internal (default) | production …
//       the bundle: cd android && ABIS="arm64-v8a armeabi-v7a x86_64" ./build-rust.sh &&
//                   ./gradlew bundleRelease -Pabis=arm64-v8a,armeabi-v7a,x86_64
// --dry: listing prints what would be sent; release uploads the bundle and then drops the edit.
import { createSign } from 'node:crypto'
import { readFileSync, readdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const PACKAGE = 'lk.arutha.liyamu'
const [command, ...flags] = process.argv.slice(2)
const dry = flags.includes('--dry')

// ---- the texts, from docs/store/listing.md
const md = readFileSync(join(ROOT, 'docs/store/listing.md'), 'utf8')
const play = md.slice(md.indexOf('## Google Play'), md.indexOf('**Data safety form'))
const short = lang => play.match(new RegExp(`^- ${lang}[^:]*: \`(.+)\`$`, 'm'))[1]
const full = lang => play.split(`**Full description (${lang})**`)[1].match(/```\n([\s\S]*?)\n```/)[1]
  .replace(/([^\n])\n(?=[^\n•A-Z])/g, '$1 ') // lines wrapped in the file are one paragraph
const listing = {
  'en-US': { title: 'Liyamu – Sinhala Keyboard', short: short('en'), full: full('en') },
  'si-LK': { title: 'ලියමු – සිංහල යතුරුපුවරුව', short: short('si'), full: full('si') },
}
for (const [lang, l] of Object.entries(listing)) {
  if ([...l.title].length > 30) throw new Error(`${lang}: title over 30`)
  if ([...l.short].length > 80) throw new Error(`${lang}: short description over 80 (${[...l.short].length})`)
  if ([...l.full].length > 4000) throw new Error(`${lang}: full description over 4000`)
}
const shots = join(ROOT, 'docs/store/screenshots/android')
const IMAGES = [
  ['phoneScreenshots', readdirSync(shots).filter(f => f.endsWith('.png')).sort().map(f => join(shots, f))],
  ['featureGraphic', [join(ROOT, 'design/logo/feature-1024x500.png')]],
  ['icon', [join(ROOT, 'design/logo/play-512.png')]],
]

// ---- the API
let token
async function auth() {
  if (token) return token
  const key = JSON.parse(readFileSync(process.env.PLAY_KEY))
  const b64 = o => Buffer.from(JSON.stringify(o)).toString('base64url')
  const now = Math.floor(Date.now() / 1000)
  const unsigned = `${b64({ alg: 'RS256', typ: 'JWT' })}.${b64({ iss: key.client_email,
    scope: 'https://www.googleapis.com/auth/androidpublisher', aud: key.token_uri, iat: now, exp: now + 3600 })}`
  const jwt = `${unsigned}.${createSign('RSA-SHA256').update(unsigned).sign(key.private_key).toString('base64url')}`
  const r = await fetch(key.token_uri, { method: 'POST',
    body: new URLSearchParams({ grant_type: 'urn:ietf:params:oauth:grant-type:jwt-bearer', assertion: jwt }) })
  const json = await r.json()
  if (!r.ok) throw new Error(`token: ${JSON.stringify(json)}`)
  return token = json.access_token
}

/** body: an object (json) or { bytes, type } (an upload) */
async function api(method, path, body) {
  const upload = body?.bytes
  const url = `https://androidpublisher.googleapis.com/${upload ? 'upload/' : ''}androidpublisher/v3/applications/${PACKAGE}${path}${upload ? '?uploadType=media' : ''}`
  const r = await fetch(url, { method, body: upload ? body.bytes : body && JSON.stringify(body),
    headers: { Authorization: `Bearer ${await auth()}`, 'Content-Type': upload ? body.type : 'application/json' } })
  const text = await r.text()
  if (!r.ok) throw new Error(`${method} ${path} -> ${r.status} ${text.slice(0, 1500)}`)
  return text ? JSON.parse(text) : {}
}

async function edit(work) {
  const e = `/edits/${(await api('POST', '/edits')).id}`
  try {
    if (await work(e) === false) return api('DELETE', e)
    // Play sometimes will not send an edit for review itself: it is then sent from the console
    await api('POST', `${e}:commit`).catch(err => {
      if (!/changesNotSentForReview/.test(err.message)) throw err
      return api('POST', `${e}:commit?changesNotSentForReview=true`).then(() => console.log('not sent for review: send it in the Play console'))
    })
    console.log('committed')
  } catch (err) {
    await api('DELETE', e).catch(() => {})
    throw err
  }
}

if (command == 'listing') {
  if (dry) {
    for (const [lang, l] of Object.entries(listing)) console.log(`== ${lang}\n${l.title}\n${l.short}\n\n${l.full}\n`)
    for (const [type, files] of IMAGES) console.log(type, files.length)
  } else await edit(async e => {
    for (const [language, l] of Object.entries(listing)) {
      await api('PUT', `${e}/listings/${language}`, { language, title: l.title, shortDescription: l.short, fullDescription: l.full })
      for (const [type, files] of IMAGES) {
        await api('DELETE', `${e}/listings/${language}/${type}`)
        for (const f of files) await api('POST', `${e}/listings/${language}/${type}`, { bytes: readFileSync(f), type: 'image/png' })
      }
      console.log('listing', language)
    }
  })
} else if (command == 'release') {
  const aab = process.env.AAB || join(ROOT, 'android/app/build/outputs/bundle/release/app-release.aab')
  const track = process.env.TRACK || 'internal'
  await edit(async e => {
    const bundle = await api('POST', `${e}/bundles`, { bytes: readFileSync(aab), type: 'application/octet-stream' })
    console.log('bundle', bundle.versionCode)
    // a draft until the app's first review is through; the console rolls it out
    await api('PUT', `${e}/tracks/${track}`, { track, releases: [{ status: process.env.STATUS || 'draft', versionCodes: [String(bundle.versionCode)] }] })
    if (dry) { console.log('dry run: edit dropped'); return false }
    console.log(track)
  })
} else {
  console.log('node tools/play.mjs listing|release [--dry]')
}
