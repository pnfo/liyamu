// Draws the logo studies: swift design/logo/make-logos.swift design/logo
// The mark is the letter ලි with a typing cursor after it, on a rounded tile (like the arutha.lk icon:
// one letter, two colours). fonts.png shows it in several fonts, dark tile above and light tile below;
// logo-<font>-dark.png / -light.png are the 1024 px tiles.
import Cocoa
import CoreText

let out = CommandLine.arguments[1]
let S: CGFloat = 1024
let fontDir = NSString(string: "~/Library/Fonts").expandingTildeInPath
let fonts: [(label: String, file: String)] = [
    ("Abhaya Bold", "AbhayaUni-Bold.ttf"),
    ("Abhaya", "AbhayaUni-Regular.ttf"),
    ("Gurulugomi", "UN-Gurulugomi.ttf"),
    ("Malithi", "UN-Malithi.ttf"),
    ("Alakamanda", "UN-Alakamanda.ttf"),
    ("Arundathee", "UN-Arundathee.ttf"),
    ("Davasa", "UN-Davasa.ttf"),
    ("Noto Sans", "NotoSansSinhala-VariableFont_wdth,wght.ttf"),
    ("Noto Serif", "NotoSerifSinhala-VariableFont_wdth,wght.ttf"),
]

func rgb(_ hex: Int) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255, green: CGFloat((hex >> 8) & 255) / 255, blue: CGFloat(hex & 255) / 255, alpha: 1)
}
let teal = rgb(0x0F4C5C), tealDeep = rgb(0x0B3A47), cream = rgb(0xF6F1E4), creamDeep = rgb(0xEADFC6), saffron = rgb(0xF2A541)

func fontName(_ file: String) -> String? {
    let url = URL(fileURLWithPath: "\(fontDir)/\(file)")
    CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
    return ((CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor])?.first)
        .flatMap { CTFontDescriptorCopyAttribute($0, kCTFontNameAttribute) as? String }
}

/// The tile: ලි and the cursor, together centred.
func mark(_ font: String, dark: Bool) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(S), pixelsHigh: Int(S), bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                               bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let tile = NSBezierPath(roundedRect: NSRect(x: 0, y: 0, width: S, height: S), xRadius: 224, yRadius: 224)
    NSGradient(starting: dark ? teal : cream, ending: dark ? tealDeep : creamDeep)!.draw(in: tile, angle: -90)
    // the letter, 500 px of ink high
    var size: CGFloat = 900
    var s: NSAttributedString, r: NSRect, f: NSFont
    repeat {
        // variable fonts (Noto) at their bold weight
        let d = NSFontDescriptor(fontAttributes: [.name: font, .variation: [0x77676874: 700]])
        f = NSFont(descriptor: d, size: size) ?? NSFont(name: font, size: size)!
        s = NSAttributedString(string: "ලි", attributes: [.font: f, .foregroundColor: dark ? cream : teal])
        r = s.boundingRect(with: NSSize(width: 5000, height: 5000), options: [.usesDeviceMetrics])
        size -= 6
    } while r.height > 500 || r.width > 560
    let gap: CGFloat = 56, bar: CGFloat = 44
    let x = (S - (r.width + gap + bar)) / 2, y = (S - r.height) / 2
    s.draw(at: NSPoint(x: x - r.minX, y: y - r.minY + f.descender))
    saffron.setFill()
    NSBezierPath(roundedRect: NSRect(x: x + r.width + gap, y: S / 2 - 270, width: bar, height: 540), xRadius: bar / 2, yRadius: bar / 2).fill()
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

var tiles: [(String, NSBitmapImageRep, NSBitmapImageRep)] = []
for (label, file) in fonts {
    guard let name = fontName(file) else { print("no font \(file)"); continue }
    let d = mark(name, dark: true), l = mark(name, dark: false)
    let slug = label.lowercased().replacingOccurrences(of: " ", with: "-")
    try! d.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(out)/logo-\(slug)-dark.png"))
    try! l.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(out)/logo-\(slug)-light.png"))
    tiles.append((label, d, l))
}
// sheet: each font a column; dark and light tiles, then both at app-list size
let cw = 230, W = tiles.count * cw + 30, H: CGFloat = 640
let sheet = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: W, pixelsHigh: Int(H), bitsPerSample: 8, samplesPerPixel: 4,
                             hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: sheet)
NSColor.white.setFill()
NSRect(x: 0, y: 0, width: CGFloat(W), height: H).fill()
for (i, (label, d, l)) in tiles.enumerated() {
    let x = CGFloat(30 + i * cw)
    d.draw(in: NSRect(x: x, y: 400, width: 200, height: 200))
    l.draw(in: NSRect(x: x, y: 170, width: 200, height: 200))
    d.draw(in: NSRect(x: x + 30, y: 80, width: 56, height: 56))
    l.draw(in: NSRect(x: x + 110, y: 80, width: 56, height: 56))
    NSAttributedString(string: "\(i + 1)  \(label)", attributes: [.font: NSFont.boldSystemFont(ofSize: 20), .foregroundColor: NSColor.black])
        .draw(at: NSPoint(x: x, y: 30))
}
NSGraphicsContext.restoreGraphicsState()
try! sheet.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(out)/fonts.png"))
