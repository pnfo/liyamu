// Draws the Liyamu icon set: swift design/logo/make-logos.swift design/logo
// The mark: the letter ලි (Abhaya Bold) with a typing cursor, in the colours of the arutha.lk icon (maroon on
// a soft gold gradient), filling the tile as that icon does.
//   icon-1024.png            rounded tile (website, macOS)
//   icon-square-1024.png     full-bleed square, no transparency (App Store, Play 512)
//   adaptive-background.png / adaptive-foreground.png   432 px Android adaptive icon layers
//   feature-1024x500.png     Play feature graphic
import Cocoa
import CoreText

let out = CommandLine.arguments[1]
let fontURL = URL(fileURLWithPath: NSString(string: "~/Library/Fonts/AbhayaUni-Bold.ttf").expandingTildeInPath)
CTFontManagerRegisterFontsForURL(fontURL as CFURL, .process, nil)
let fontName = ((CTFontManagerCreateFontDescriptorsFromURL(fontURL as CFURL) as? [CTFontDescriptor])?.first)
    .flatMap { CTFontDescriptorCopyAttribute($0, kCTFontNameAttribute) as? String }!

func rgb(_ hex: Int) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255, green: CGFloat((hex >> 8) & 255) / 255, blue: CGFloat(hex & 255) / 255, alpha: 1)
}
let goldTop = rgb(0xFBE7B5), goldBottom = rgb(0xF3CF7A), maroon = rgb(0x6E1C22)
/// the cursor: a warm saffron that sits back on the gold, so the letter stays the subject
let saffron = rgb(0xD9861C)
let maroonTop = rgb(0x7E2530), maroonBottom = rgb(0x4A1016), ivory = rgb(0xFFF8E8)

/// `alpha: false`: the picture is opaque and is saved without an alpha channel (the stores require that).
var opaque = Set<ObjectIdentifier>()
func canvas(_ w: Int, _ h: Int, alpha: Bool = true, _ draw: () -> Void) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: w, pixelsHigh: h, bitsPerSample: 8, samplesPerPixel: 4,
                               hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    if !alpha { opaque.insert(ObjectIdentifier(rep)) }
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    draw()
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

func save(_ rep: NSBitmapImageRep, _ name: String) {
    let url = URL(fileURLWithPath: "\(out)/\(name)")
    guard opaque.contains(ObjectIdentifier(rep)) else {
        try! rep.representation(using: .png, properties: [:])!.write(to: url)
        return
    }
    // redraw into a context without alpha
    let cg = rep.cgImage!
    let ctx = CGContext(data: nil, width: cg.width, height: cg.height, bitsPerComponent: 8, bytesPerRow: 0,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)!
    ctx.draw(cg, in: CGRect(x: 0, y: 0, width: cg.width, height: cg.height))
    let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil)!
    CGImageDestinationAddImage(dest, ctx.makeImage()!, nil)
    CGImageDestinationFinalize(dest)
}

func gradient(_ rect: NSRect, radius: CGFloat = 0) {
    NSGradient(starting: goldTop, ending: goldBottom)!.draw(in: NSBezierPath(roundedRect: rect, xRadius: radius, yRadius: radius), angle: -90)
}

/// ලි and the cursor, together centred on `centre`, the ink fitting in `box`.
func mark(centre: NSPoint, box: NSSize) {
    var size: CGFloat = 1400
    var s: NSAttributedString, r: NSRect, f: NSFont, gap: CGFloat, bar: CGFloat
    repeat {
        f = NSFont(name: fontName, size: size)!
        s = NSAttributedString(string: "ලි", attributes: [.font: f, .foregroundColor: maroon])
        r = s.boundingRect(with: NSSize(width: 9000, height: 9000), options: [.usesDeviceMetrics])
        gap = r.height * 0.12
        bar = r.height * 0.07
        size -= 4
    } while r.height > box.height * 0.9 || r.width + gap + bar > box.width
    let x = centre.x - (r.width + gap + bar) / 2, y = centre.y - r.height / 2
    s.draw(at: NSPoint(x: x - r.minX, y: y - r.minY + f.descender))
    saffron.setFill()
    let h = r.height / 0.9 // the cursor is a little taller than the letter
    NSBezierPath(roundedRect: NSRect(x: x + r.width + gap, y: centre.y - h / 2, width: bar, height: h), xRadius: bar / 2, yRadius: bar / 2).fill()
}

save(canvas(1024, 1024) {
    gradient(NSRect(x: 0, y: 0, width: 1024, height: 1024), radius: 224)
    mark(centre: NSPoint(x: 512, y: 512), box: NSSize(width: 700, height: 700))
}, "icon-1024.png")
save(canvas(1024, 1024, alpha: false) {
    gradient(NSRect(x: 0, y: 0, width: 1024, height: 1024))
    mark(centre: NSPoint(x: 512, y: 512), box: NSSize(width: 700, height: 700))
}, "icon-square-1024.png")
// Android adaptive icon: 108 dp layers, of which the launcher shows the middle 72 dp (a circle of 66 dp is always safe)
save(canvas(432, 432, alpha: false) { gradient(NSRect(x: 0, y: 0, width: 432, height: 432)) }, "adaptive-background.png")
save(canvas(432, 432) { mark(centre: NSPoint(x: 216, y: 216), box: NSSize(width: 196, height: 196)) }, "adaptive-foreground.png")
// Play feature graphic: the icon on deep maroon, the name, and a row of suggestions as the keyboard shows them
save(canvas(1024, 500, alpha: false) {
    NSGradient(starting: maroonTop, ending: maroonBottom)!.draw(in: NSRect(x: 0, y: 0, width: 1024, height: 500), angle: -90)
    // a soft glow behind the icon
    NSGradient(colors: [rgb(0xF3CF7A).withAlphaComponent(0.22), rgb(0xF3CF7A).withAlphaComponent(0)])!
        .draw(fromCenter: NSPoint(x: 215, y: 250), radius: 0, toCenter: NSPoint(x: 215, y: 250), radius: 330, options: [])
    let tile = NSRect(x: 80, y: 115, width: 270, height: 270)
    let shadow = NSShadow()
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.35)
    shadow.shadowBlurRadius = 30
    shadow.shadowOffset = NSSize(width: 0, height: -10)
    NSGraphicsContext.saveGraphicsState()
    shadow.set()
    goldBottom.setFill()
    NSBezierPath(roundedRect: tile, xRadius: 60, yRadius: 60).fill()
    NSGraphicsContext.restoreGraphicsState()
    gradient(tile, radius: 60)
    mark(centre: NSPoint(x: tile.midX, y: tile.midY), box: NSSize(width: 186, height: 186))

    NSAttributedString(string: "ලියමු", attributes: [.font: NSFont(name: fontName, size: 132)!, .foregroundColor: goldTop])
        .draw(at: NSPoint(x: 418, y: 278))
    NSAttributedString(string: "The private Sinhala keyboard", attributes: [
        .font: NSFont.systemFont(ofSize: 34, weight: .medium), .foregroundColor: ivory.withAlphaComponent(0.92), .kern: 0.3])
        .draw(at: NSPoint(x: 424, y: 216))
    // suggestion chips
    var x: CGFloat = 424
    for (i, w) in ["මම", "ගෙදර", "යනවා"].enumerated() {
        let on = i == 2
        let t = NSAttributedString(string: w, attributes: [.font: NSFont(name: fontName, size: 40)!, .foregroundColor: on ? maroonBottom : ivory])
        let tw = t.size().width
        let r = NSRect(x: x, y: 112, width: tw + 44, height: 70)
        (on ? goldBottom : ivory.withAlphaComponent(0.13)).setFill()
        NSBezierPath(roundedRect: r, xRadius: 18, yRadius: 18).fill()
        t.draw(at: NSPoint(x: x + 22, y: r.midY - t.size().height / 2 + 2))
        x += tw + 44 + 14
    }
    saffron.setFill()
    NSBezierPath(roundedRect: NSRect(x: x + 4, y: 118, width: 6, height: 58), xRadius: 3, yRadius: 3).fill()
    NSAttributedString(string: "Free  ·  Open source  ·  No tracking", attributes: [
        .font: NSFont.systemFont(ofSize: 22, weight: .regular), .foregroundColor: ivory.withAlphaComponent(0.6), .kern: 0.6])
        .draw(at: NSPoint(x: 426, y: 62))
}, "feature-1024x500.png")
