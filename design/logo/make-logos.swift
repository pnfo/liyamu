// Draws the logo concepts: swift design/logo/make-logos.swift design/logo
// a-key.png, b-cursor.png, c-shield.png, d-wordmark.png (1024 px) and sheet.png with all four.
import Cocoa
import CoreText

let out = CommandLine.arguments[1]
let S: CGFloat = 1024
let fontURL = URL(fileURLWithPath: NSString(string: "~/Library/Fonts/AbhayaUni-Bold.ttf").expandingTildeInPath)
CTFontManagerRegisterFontsForURL(fontURL as CFURL, .process, nil)
let fontName = ((CTFontManagerCreateFontDescriptorsFromURL(fontURL as CFURL) as? [CTFontDescriptor])?.first)
    .flatMap { CTFontDescriptorCopyAttribute($0, kCTFontNameAttribute) as? String } ?? "Sinhala MN Bold"

func rgb(_ hex: Int) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255, green: CGFloat((hex >> 8) & 255) / 255, blue: CGFloat(hex & 255) / 255, alpha: 1)
}

/// Draws `text` with its ink centred on `centre`, as large as fits in `box`; returns the ink rectangle.
@discardableResult
func ink(_ text: String, _ colour: NSColor, centre: NSPoint, box: NSSize) -> NSRect {
    var size: CGFloat = 900
    var s: NSAttributedString, r: NSRect, font: NSFont
    repeat {
        font = NSFont(name: fontName, size: size)!
        s = NSAttributedString(string: text, attributes: [.font: font, .foregroundColor: colour])
        r = s.boundingRect(with: NSSize(width: 5000, height: 5000), options: [.usesDeviceMetrics])
        size -= 8
    } while r.width > box.width || r.height > box.height
    let at = NSPoint(x: centre.x - r.width / 2 - r.minX, y: centre.y - r.height / 2 - r.minY + font.descender)
    s.draw(at: at)
    return NSRect(x: centre.x - r.width / 2, y: centre.y - r.height / 2, width: r.width, height: r.height)
}

func image(_ draw: () -> Void) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(S), pixelsHigh: Int(S), bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                               bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    draw()
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

func tile(_ colour: NSColor) {
    colour.setFill()
    NSBezierPath(roundedRect: NSRect(x: 0, y: 0, width: S, height: S), xRadius: S * 0.225, yRadius: S * 0.225).fill()
}

let concepts: [(String, () -> Void)] = [
    // A: a keycap with ලි
    ("a-key", {
        tile(rgb(0x0F4C5C))
        rgb(0x0A3540).setFill() // the key's side
        NSBezierPath(roundedRect: NSRect(x: 152, y: 132, width: 720, height: 720), xRadius: 150, yRadius: 150).fill()
        rgb(0xF4F1E8).setFill() // its top
        NSBezierPath(roundedRect: NSRect(x: 152, y: 192, width: 720, height: 680), xRadius: 150, yRadius: 150).fill()
        ink("ලි", rgb(0x0F4C5C), centre: NSPoint(x: 512, y: 532), box: NSSize(width: 470, height: 440))
    }),
    // B: ලි with the typing cursor after it
    ("b-cursor", {
        tile(rgb(0xE8912D))
        let r = ink("ලි", .white, centre: NSPoint(x: 462, y: 512), box: NSSize(width: 520, height: 520))
        rgb(0x6B2D0F).setFill()
        NSBezierPath(roundedRect: NSRect(x: r.maxX + 46, y: 262, width: 46, height: 500), xRadius: 23, yRadius: 23).fill()
    }),
    // C: a shield (private) with ලි
    ("c-shield", {
        tile(rgb(0xF6EFE3))
        let p = NSBezierPath()
        p.move(to: NSPoint(x: 512, y: 900))
        p.curve(to: NSPoint(x: 842, y: 780), controlPoint1: NSPoint(x: 630, y: 850), controlPoint2: NSPoint(x: 740, y: 800))
        p.curve(to: NSPoint(x: 512, y: 124), controlPoint1: NSPoint(x: 860, y: 470), controlPoint2: NSPoint(x: 750, y: 240))
        p.curve(to: NSPoint(x: 182, y: 780), controlPoint1: NSPoint(x: 274, y: 240), controlPoint2: NSPoint(x: 164, y: 470))
        p.curve(to: NSPoint(x: 512, y: 900), controlPoint1: NSPoint(x: 284, y: 800), controlPoint2: NSPoint(x: 394, y: 850))
        rgb(0x7A1F2B).setFill()
        p.fill()
        ink("ලි", rgb(0xF6EFE3), centre: NSPoint(x: 512, y: 560), box: NSSize(width: 400, height: 380))
    }),
    // D: the whole word
    ("d-wordmark", {
        tile(rgb(0x1F2A44))
        let r = ink("ලියමු", rgb(0xFFD166), centre: NSPoint(x: 512, y: 540), box: NSSize(width: 800, height: 520))
        rgb(0xFFD166).setFill()
        NSBezierPath(roundedRect: NSRect(x: r.minX, y: r.minY - 90, width: r.width, height: 30), xRadius: 15, yRadius: 15).fill()
    }),
]

var reps: [NSBitmapImageRep] = []
for (name, draw) in concepts {
    let rep = image(draw)
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(out)/\(name).png"))
    reps.append(rep)
}
// the four side by side, large and at menu/app-list size
let W = 4 * 300 + 5 * 40, H: CGFloat = 480
let sheet = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: W, pixelsHigh: Int(H), bitsPerSample: 8, samplesPerPixel: 4,
                             hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: sheet)
NSColor.white.setFill()
NSRect(x: 0, y: 0, width: CGFloat(W), height: H).fill()
for (i, rep) in reps.enumerated() {
    let x = CGFloat(40 + i * 340)
    rep.draw(in: NSRect(x: x, y: 140, width: 300, height: 300))
    rep.draw(in: NSRect(x: x + 118, y: 40, width: 64, height: 64))
    NSAttributedString(string: ["A", "B", "C", "D"][i], attributes: [.font: NSFont.boldSystemFont(ofSize: 28), .foregroundColor: NSColor.black])
        .draw(at: NSPoint(x: x, y: 56))
}
NSGraphicsContext.restoreGraphicsState()
try! sheet.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(out)/sheet.png"))
