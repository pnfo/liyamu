// Draws the menu-bar icon as a TIFF with 1x and 2x: swift make-icon.swift out.tiff අ
// A filled rounded badge with the letter cut out of it, like the system's "A" badge for ABC. It is a
// template image (TISIconIsTemplate in Info.plist), so macOS colours the badge for light and dark menus.
import Cocoa

let out = CommandLine.arguments[1]
let letter = CommandLine.arguments.count > 2 ? CommandLine.arguments[2] : "අ"
let w: CGFloat = 22, h: CGFloat = 16
var reps: [NSBitmapImageRep] = []
for scale in [1, 2] {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(w) * scale, pixelsHigh: Int(h) * scale,
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = NSSize(width: w, height: h)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    NSColor.black.setFill()
    NSBezierPath(roundedRect: NSRect(x: 0, y: 0, width: w, height: h), xRadius: 4, yRadius: 4).fill()
    // the largest size whose ink fits inside the badge
    var size: CGFloat = 17
    var font: NSFont, s: NSAttributedString, ink: NSRect
    repeat {
        font = NSFont(name: "Sinhala MN Bold", size: size) ?? NSFont.boldSystemFont(ofSize: size)
        s = NSAttributedString(string: letter, attributes: [.font: font, .foregroundColor: NSColor.black])
        // centre the ink, not the line box
        ink = s.boundingRect(with: NSSize(width: 100, height: 100), options: [.usesDeviceMetrics])
        size -= 0.5
    } while (ink.width > w - 6 || ink.height > h - 4) && size > 7
    // cut the letter out of the badge
    NSGraphicsContext.current!.compositingOperation = .destinationOut
    // draw(at:) takes the bottom of the line box, which is the descender below the baseline
    s.draw(at: NSPoint(x: (w - ink.width) / 2 - ink.minX, y: h / 2 - ink.height / 2 - ink.minY + font.descender))
    NSGraphicsContext.restoreGraphicsState()
    reps.append(rep)
}
let img = NSImage(size: NSSize(width: w, height: h))
reps.forEach(img.addRepresentation)
try! img.tiffRepresentation!.write(to: URL(fileURLWithPath: out))
