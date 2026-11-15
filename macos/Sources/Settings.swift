import Foundation

/// Options shared by every text field, chosen in the input menu or the settings window and saved in the
/// user's defaults.
final class Settings {
    static let shared = Settings()
    private let d = UserDefaults.standard
    /// bumped on every change so open sessions pick it up
    private(set) var version = 0

    /// 0 Wijesekara, 1 Easy Singlish, 2 Helakuru keys
    var layout: UInt32 {
        // Easy Singlish until one is chosen, as on Android and Windows
        get { UInt32(max(0, min(2, d.object(forKey: "layout") as? Int ?? 1))) }
        set { d.set(Int(newValue), forKey: "layout"); version += 1 }
    }

    enum Output: Int { case unicode = 0, fm = 1, isi = 2 }

    var output: Output {
        get { Output(rawValue: d.integer(forKey: "output")) ?? .unicode }
        set { d.set(newValue.rawValue, forKey: "output"); version += 1 }
    }
    /// kombuva typed before its letter, repaya after (Wijesekara)
    var traditionalOrder: Bool {
        get { d.object(forKey: "traditionalOrder") as? Bool ?? true }
        set { d.set(newValue, forKey: "traditionalOrder"); version += 1 }
    }
    var pali: Bool {
        get { d.bool(forKey: "pali") }
        set { d.set(newValue, forKey: "pali"); version += 1 }
    }

    /// word suggestions under the word being typed
    var suggestions: Bool {
        get { d.object(forKey: "suggestions") as? Bool ?? true }
        set { d.set(newValue, forKey: "suggestions"); version += 1 }
    }

    /// ර් before a consonant as repaya
    var repaya: Bool {
        get { d.bool(forKey: "repaya") }
        set { d.set(newValue, forKey: "repaya"); version += 1 }
    }
    /// Pali pairs as joined letters
    var conjuncts: Bool {
        get { d.bool(forKey: "conjuncts") }
        set { d.set(newValue, forKey: "conjuncts"); version += 1 }
    }
    /// suggest words that are Pali only
    var paliWords: Bool {
        get { d.bool(forKey: "paliWords") }
        set { d.set(newValue, forKey: "paliWords"); version += 1 }
    }

    /// learn from the suggestions picked; kept in a file on this Mac only
    var learn: Bool {
        get { d.object(forKey: "learn") as? Bool ?? true }
        set { d.set(newValue, forKey: "learn"); applyLearning() }
    }

    static let learnedFile: URL = shortcutsFile.deletingLastPathComponent().appendingPathComponent("learned.tsv")

    func applyLearning() {
        liyamu_set_learning(Settings.learnedFile.path, learn)
    }

    func forgetLearned() {
        liyamu_forget_learned()
    }

    func options() -> LiyamuOptions {
        LiyamuOptions(layout: layout, output: UInt32(output.rawValue),
                      order: traditionalOrder ? 0 : 1, pali: pali, suggest: suggestions,
                      repaya: repaya, conjuncts: conjuncts, pali_words: paliWords)
    }

    // MARK: the user's own shortcuts, a text file of keys=words lines

    static let shortcutsFile: URL = {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("Liyamu")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("shortcuts.txt")
    }()
    private var shortcutsRead: Date?

    var shortcuts: String {
        get { (try? String(contentsOf: Settings.shortcutsFile, encoding: .utf8)) ?? "" }
        set {
            try? newValue.write(to: Settings.shortcutsFile, atomically: true, encoding: .utf8)
            loadShortcuts(force: true)
        }
    }

    /// Hands the shortcuts to the engine if the file changed (it may be edited in another app).
    func loadShortcuts(force: Bool = false) {
        let changed = (try? Settings.shortcutsFile.resourceValues(forKeys: [.contentModificationDateKey]))?.contentModificationDate
        guard force || changed != shortcutsRead else { return }
        shortcutsRead = changed
        liyamu_set_shortcuts(shortcuts)
    }
}
