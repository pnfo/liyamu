import Carbon
import Cocoa
import InputMethodKit

/// One controller per text field (client). It owns an engine session, sends it each key, and shows the
/// response: `commit` is inserted, `preedit` is shown as underlined marked text, suggestions in the bar.
@objc(LiyamuInputController)
final class LiyamuInputController: IMKInputController {
    private let session: OpaquePointer
    private var settingsVersion = -1
    private var hasMarkedText = false

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        var opts = Settings.shared.options()
        session = liyamu_session_new(&opts)
        super.init(server: server, delegate: delegate, client: inputClient)
    }

    deinit {
        liyamu_session_free(session)
    }

    // MARK: keys

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask.keyDown.rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown, let client = sender as? IMKTextInput else { return false }
        syncOptions(client)
        let flags = event.modifierFlags
        let ctrl = flags.contains(.command) || flags.contains(.control)
        let altgr = flags.contains(.option)
        let (kind, ch) = KeyMap.translate(event)
        guard let r = liyamu_session_key(session, kind, ch, altgr, ctrl) else { return false }
        defer { liyamu_response_free(r) }
        show(r.pointee, client)
        return r.pointee.handled
    }

    /// Focus moved, the user clicked elsewhere, or the input source changed: keep what was typed.
    override func commitComposition(_ sender: Any!) {
        guard let client = sender as? IMKTextInput, let r = liyamu_session_commit(session) else { return }
        defer { liyamu_response_free(r) }
        show(r.pointee, client)
    }

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        Settings.shared.loadShortcuts()
        settingsVersion = -1
    }

    override func deactivateServer(_ sender: Any!) {
        commitComposition(sender)
        CandidateWindow.shared.hide()
        super.deactivateServer(sender)
    }

    private func syncOptions(_ client: IMKTextInput) {
        guard settingsVersion != Settings.shared.version else { return }
        settingsVersion = Settings.shared.version
        var opts = Settings.shared.options()
        if let r = liyamu_session_set_options(session, &opts) {
            show(r.pointee, client)
            liyamu_response_free(r)
        }
    }

    private func show(_ r: LiyamuResponse, _ client: IMKTextInput) {
        let notFound = NSRange(location: NSNotFound, length: 0)
        let commit = String(cString: r.commit)
        let preedit = String(cString: r.preedit)
        if !commit.isEmpty {
            client.insertText(commit, replacementRange: notFound) // replaces the marked text
            hasMarkedText = false
        }
        if !preedit.isEmpty || hasMarkedText {
            let marked = NSAttributedString(string: preedit, attributes: [
                .underlineStyle: NSUnderlineStyle.single.rawValue,
            ])
            client.setMarkedText(marked, selectionRange: NSRange(location: preedit.utf16.count, length: 0),
                                 replacementRange: notFound)
            hasMarkedText = !preedit.isEmpty
        }
        showCandidates(r, client)
    }

    // MARK: suggestions

    private func showCandidates(_ r: LiyamuResponse, _ client: IMKTextInput) {
        let items = (0..<Int(r.candidate_count)).map { String(cString: r.candidates[$0]!) }
        let window = CandidateWindow.shared
        guard items.count > 1 else {
            window.hide()
            return
        }
        var caret = NSRect.zero
        client.attributes(forCharacterIndex: 0, lineHeightRectangle: &caret)
        if caret.isEmpty && caret.origin == .zero {
            // the app does not say where the text is: near the mouse pointer
            let m = NSEvent.mouseLocation
            caret = NSRect(x: m.x, y: m.y - 20, width: 1, height: 18)
        }
        window.onPick = { [weak self] i in self?.pick(i, client) }
        window.show(items, selected: Int(r.selected), caret: caret)
    }

    /// A suggestion was clicked.
    private func pick(_ i: Int, _ client: IMKTextInput) {
        guard let r = liyamu_session_pick(session, UInt32(i)) else { return }
        defer { liyamu_response_free(r) }
        show(r.pointee, client)
    }

    // MARK: input menu

    override func menu() -> NSMenu! {
        let m = NSMenu(title: "Liyamu")
        let s = Settings.shared
        func item(_ title: String, _ action: Selector, _ on: Bool) {
            let i = NSMenuItem(title: title, action: action, keyEquivalent: "")
            i.target = self
            i.state = on ? .on : .off
            m.addItem(i)
        }
        item("Wijesekara", #selector(useWijesekara(_:)), s.layout == 0)
        item("Easy Singlish", #selector(useSinglish(_:)), s.layout == 1)
        item("Helakuru keys", #selector(useHelakuru(_:)), s.layout == 2)
        m.addItem(.separator())
        item("Unicode", #selector(useUnicode(_:)), s.output == .unicode)
        item("FM fonts (FM Abhaya …)", #selector(useFM(_:)), s.output == .fm)
        item("Isi fonts", #selector(useIsi(_:)), s.output == .isi)
        m.addItem(.separator())
        item("Word suggestions", #selector(toggleSuggestions(_:)), s.suggestions)
        m.addItem(.separator())
        item("Settings and Help…", #selector(openSettings(_:)), false)
        return m
    }

    @objc func useWijesekara(_ sender: Any?) { setLayout(0) }
    @objc func useSinglish(_ sender: Any?) { setLayout(1) }
    @objc func useHelakuru(_ sender: Any?) { setLayout(2) }

    private func setLayout(_ layout: UInt32) {
        Settings.shared.layout = layout
        if let client = client() { syncOptions(client) } // commits the word being typed
    }

    @objc func useUnicode(_ sender: Any?) { Settings.shared.output = .unicode }
    @objc func useFM(_ sender: Any?) { Settings.shared.output = .fm }
    @objc func useIsi(_ sender: Any?) { Settings.shared.output = .isi }
    @objc func toggleSuggestions(_ sender: Any?) { Settings.shared.suggestions.toggle() }
    @objc func openSettings(_ sender: Any?) { SettingsWindow.shared.show() }
}
