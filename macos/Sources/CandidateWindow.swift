import Cocoa

/// The suggestion bar under the word being typed: "1 බලන්න  2 බලන්නා …", the selected one highlighted.
/// Our own panel rather than IMKCandidates, which does not show in some apps (TextEdit) and cannot follow
/// the selection we keep in the engine. It never takes focus; clicking a suggestion calls `onPick`.
final class CandidateWindow: NSPanel {
    static let shared = CandidateWindow()
    var onPick: ((Int) -> Void)?
    private let bar = CandidateBar()

    private init() {
        super.init(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        level = .popUpMenu
        isOpaque = false
        backgroundColor = .clear
        hasShadow = true
        hidesOnDeactivate = false
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        contentView = bar
        bar.onPick = { [weak self] i in self?.onPick?(i) }
    }

    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }

    /// Shows the suggestions under `caret` (screen coordinates, the line the word is on).
    func show(_ items: [String], selected: Int, caret: NSRect) {
        bar.items = items
        bar.selected = selected
        let size = bar.layoutItems()
        var origin = NSPoint(x: caret.minX - 6, y: caret.minY - size.height - 3)
        let screen = NSScreen.screens.first { $0.frame.contains(caret.origin) } ?? NSScreen.main
        if let vf = screen?.visibleFrame {
            if origin.y < vf.minY { origin.y = caret.maxY + 3 } // no room below: above the line
            origin.x = max(vf.minX, min(origin.x, vf.maxX - size.width))
        }
        setFrame(NSRect(origin: origin, size: size), display: true)
        bar.needsDisplay = true
        orderFrontRegardless()
    }

    func hide() {
        orderOut(nil)
    }
}

final class CandidateBar: NSView {
    var items: [String] = []
    var selected = 0
    var onPick: ((Int) -> Void)?
    private var rects: [NSRect] = []
    private let font = Fonts.ui(19)
    private let numberFont = NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .regular)
    private let pad: CGFloat = 6

    private func label(_ i: Int) -> NSAttributedString {
        let on = i == selected
        let s = NSMutableAttributedString(string: "\(i + 1) ", attributes: [
            .font: numberFont,
            .foregroundColor: on ? NSColor.alternateSelectedControlTextColor.withAlphaComponent(0.8) : NSColor.secondaryLabelColor,
        ])
        s.append(NSAttributedString(string: items[i], attributes: [
            .font: font,
            .foregroundColor: on ? NSColor.alternateSelectedControlTextColor : NSColor.labelColor,
        ]))
        return s
    }

    /// Lays the items out in a row and returns the bar's size.
    func layoutItems() -> NSSize {
        rects = []
        var x = pad
        let h = ceil(font.ascender - font.descender + font.leading) + 14
        for i in items.indices {
            let w = ceil(label(i).size().width) + 16
            rects.append(NSRect(x: x, y: pad / 2, width: w, height: h - pad))
            x += w + 2
        }
        return NSSize(width: x + pad - 2, height: h)
    }

    override func draw(_ dirtyRect: NSRect) {
        let bg = NSBezierPath(roundedRect: bounds, xRadius: 9, yRadius: 9)
        NSColor.windowBackgroundColor.setFill()
        bg.fill()
        NSColor.separatorColor.setStroke()
        bg.lineWidth = 1
        bg.stroke()
        for (i, r) in rects.enumerated() {
            if i == selected {
                NSColor.controlAccentColor.setFill()
                NSBezierPath(roundedRect: r, xRadius: 6, yRadius: 6).fill()
            }
            let s = label(i)
            let size = s.size()
            s.draw(at: NSPoint(x: r.minX + 8, y: r.midY - size.height / 2))
        }
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    override func mouseDown(with event: NSEvent) {
        let p = convert(event.locationInWindow, from: nil)
        if let i = rects.firstIndex(where: { $0.contains(p) }) { onPick?(i) }
    }
}
