import Cocoa

/// The settings and help window, opened from the input menu or by opening Liyamu.app.
final class SettingsWindow: NSObject, NSWindowDelegate {
    static let shared = SettingsWindow()
    private var window: NSWindow?
    private var shortcutsView: NSTextView?
    private var checks: [(NSButton, ReferenceWritableKeyPath<Settings, Bool>)] = []
    private var layout: NSPopUpButton?
    private var output: NSPopUpButton?

    func show() {
        if window == nil { build() }
        refresh()
        NSApp.activate(ignoringOtherApps: true)
        window?.center()
        window?.makeKeyAndOrderFront(nil)
    }

    private func refresh() {
        let s = Settings.shared
        layout?.selectItem(at: Int(s.layout))
        output?.selectItem(at: s.output.rawValue)
        for (b, key) in checks { b.state = s[keyPath: key] ? .on : .off }
        shortcutsView?.string = s.shortcuts
    }

    // MARK: building

    private func build() {
        let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 560, height: 480),
                         styleMask: [.titled, .closable, .miniaturizable], backing: .buffered, defer: false)
        w.title = "Liyamu ලියමු"
        w.isReleasedWhenClosed = false
        w.delegate = self
        let tabs = NSTabView(frame: w.contentView!.bounds.insetBy(dx: 10, dy: 10))
        tabs.autoresizingMask = [.width, .height]
        tabs.addTabViewItem(tab("Typing", typing()))
        tabs.addTabViewItem(tab("Letters", letters()))
        tabs.addTabViewItem(tab("Shortcuts", shortcuts()))
        tabs.addTabViewItem(tab("Help", help()))
        w.contentView?.addSubview(tabs)
        window = w
    }

    private func tab(_ title: String, _ view: NSView) -> NSTabViewItem {
        let item = NSTabViewItem()
        item.label = title
        let holder = NSView()
        view.translatesAutoresizingMaskIntoConstraints = false
        holder.addSubview(view)
        NSLayoutConstraint.activate([
            view.topAnchor.constraint(equalTo: holder.topAnchor, constant: 16),
            view.leadingAnchor.constraint(equalTo: holder.leadingAnchor, constant: 18),
            view.trailingAnchor.constraint(equalTo: holder.trailingAnchor, constant: -18),
            view.bottomAnchor.constraint(lessThanOrEqualTo: holder.bottomAnchor, constant: -14),
        ])
        if view is NSScrollView || view.subviews.contains(where: { $0 is NSScrollView }) {
            view.bottomAnchor.constraint(equalTo: holder.bottomAnchor, constant: -14).isActive = true
        }
        item.view = holder
        return item
    }

    private func stack(_ views: [NSView]) -> NSStackView {
        let s = NSStackView(views: views)
        s.orientation = .vertical
        s.alignment = .leading
        s.spacing = 10
        return s
    }

    private func note(_ text: String) -> NSTextField {
        let l = NSTextField(wrappingLabelWithString: text)
        l.font = Fonts.ui(12.5)
        l.textColor = .secondaryLabelColor
        l.preferredMaxLayoutWidth = 480
        return l
    }

    private func heading(_ text: String) -> NSTextField {
        let l = NSTextField(labelWithString: text)
        l.font = .boldSystemFont(ofSize: 13)
        return l
    }

    private func check(_ title: String, _ key: ReferenceWritableKeyPath<Settings, Bool>, _ example: String) -> NSView {
        let b = NSButton(checkboxWithTitle: title, target: self, action: #selector(toggled(_:)))
        b.tag = checks.count
        checks.append((b, key))
        let n = note(example)
        let s = NSStackView(views: [b, n])
        s.orientation = .vertical
        s.alignment = .leading
        s.spacing = 2
        s.edgeInsets = NSEdgeInsets(top: 0, left: 0, bottom: 4, right: 0)
        n.translatesAutoresizingMaskIntoConstraints = false
        n.leadingAnchor.constraint(equalTo: s.leadingAnchor, constant: 20).isActive = true
        return s
    }

    private func popup(_ items: [String], _ action: Selector) -> NSPopUpButton {
        let p = NSPopUpButton(frame: .zero, pullsDown: false)
        p.addItems(withTitles: items)
        p.target = self
        p.action = action
        return p
    }

    private func row(_ label: String, _ control: NSView) -> NSView {
        let l = NSTextField(labelWithString: label)
        l.alignment = .right
        l.widthAnchor.constraint(equalToConstant: 90).isActive = true
        let s = NSStackView(views: [l, control])
        s.spacing = 8
        return s
    }

    private func typing() -> NSView {
        layout = popup(["Wijesekara", "Easy Singlish", "Helakuru keys"], #selector(layoutChosen(_:)))
        output = popup(["Unicode", "FM fonts (FM Abhaya …)", "Isi fonts"], #selector(outputChosen(_:)))
        return stack([
            row("Layout:", layout!),
            row("Write as:", output!),
            note("FM and Isi are for documents set in the old fonts; choose that font in the app you type in."),
            heading("Suggestions"),
            check("Word suggestions", \.suggestions,
                  "A row of words under what you type. → ← choose, Space takes the highlighted one, 1–9 take that one."),
            check("Learn from the words I pick", \.learn,
                  "A word you pick twice for the same letters is offered first from then on. Kept only on this Mac."),
            NSButton(title: "Forget what was learned", target: self, action: #selector(forgetLearned(_:))),
            check("Suggest Pali words", \.paliWords,
                  "Also offer words found only in Pali: ධම්මං, භික්ඛූනං. For typing suttas and chanting."),
        ])
    }

    private func letters() -> NSView {
        stack([
            heading("How joined letters are written"),
            check("Repaya", \.repaya, "ර් before a letter is written above it: ධර්ම → ධර්‍ම, කර්ම → කර්‍ම"),
            check("Pali joined letters (බැඳි අකුරු)", \.conjuncts, "බුද්ධ → බුද්‍ධ, තත්ථ → තත්‍ථ, නන්ද → නන්‍ද, ද්වාර → ද්‍වාර"),
            check("Pali touching letters", \.pali,
                  "ධම්ම → ධම‍්ම, සඤ්ඤා → සඤ‍්ඤා. In Wijesekara, hal after a letter touches the next one and X types ඞ."),
            heading("Wijesekara"),
            check("Kombuva before the letter, as on paper", \.traditionalOrder,
                  "Type ෙ then ක for කෙ (as in FM fonts). Off: ක then ෙ, the Windows Unicode order."),
        ])
    }

    private func shortcuts() -> NSView {
        let intro = note("Your own short forms, one on a line: the keys you type, =, then what they stand for. "
            + "When you type exactly those keys the words are offered first and Space takes them. "
            + "The keys are as typed in the layout you use (in Wijesekara, iaj for ස්ව).")
        let example = note("bs=බුදු සරණයි\niaj=ස්වාමීන් වහන්ස,")
        let (scroll, text) = textArea(editable: true)
        shortcutsView = text
        scroll.heightAnchor.constraint(greaterThanOrEqualToConstant: 170).isActive = true
        let save = NSButton(title: "Save", target: self, action: #selector(saveShortcuts(_:)))
        save.keyEquivalent = "\r"
        let open = NSButton(title: "Open in TextEdit", target: self, action: #selector(openShortcuts(_:)))
        let buttons = NSStackView(views: [save, open])
        let s = stack([intro, example, scroll, buttons])
        scroll.widthAnchor.constraint(equalTo: s.widthAnchor).isActive = true
        return s
    }

    private func help() -> NSView {
        let (scroll, text) = textArea(editable: false)
        text.string = Help.text
        text.textContainerInset = NSSize(width: 6, height: 8)
        return scroll
    }

    private func textArea(editable: Bool) -> (NSScrollView, NSTextView) {
        let scroll = NSTextView.scrollableTextView()
        scroll.borderType = .bezelBorder
        let text = scroll.documentView as! NSTextView
        text.font = Fonts.ui(14)
        text.isEditable = editable
        text.isRichText = false
        text.isAutomaticQuoteSubstitutionEnabled = false
        text.isAutomaticDashSubstitutionEnabled = false
        text.isAutomaticTextReplacementEnabled = false
        text.isAutomaticSpellingCorrectionEnabled = false
        return (scroll, text)
    }

    // MARK: actions

    @objc private func toggled(_ sender: NSButton) {
        Settings.shared[keyPath: checks[sender.tag].1] = sender.state == .on
    }

    @objc private func forgetLearned(_ sender: NSButton) {
        Settings.shared.forgetLearned()
        sender.title = "Forgotten"
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { sender.title = "Forget what was learned" }
    }

    @objc private func layoutChosen(_ sender: NSPopUpButton) {
        Settings.shared.layout = UInt32(sender.indexOfSelectedItem)
    }

    @objc private func outputChosen(_ sender: NSPopUpButton) {
        Settings.shared.output = Settings.Output(rawValue: sender.indexOfSelectedItem) ?? .unicode
    }

    @objc private func saveShortcuts(_ sender: Any?) {
        if let text = shortcutsView?.string { Settings.shared.shortcuts = text }
    }

    @objc private func openShortcuts(_ sender: Any?) {
        saveShortcuts(nil)
        NSWorkspace.shared.open([Settings.shortcutsFile], withApplicationAt: URL(fileURLWithPath: "/System/Applications/TextEdit.app"),
                                configuration: NSWorkspace.OpenConfiguration())
    }

    func windowWillClose(_ notification: Notification) {
        saveShortcuts(nil)
    }
}
