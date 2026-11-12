import Cocoa

/// Maps a key to what the US QWERTY layout would type, whatever keyboard layout macOS has, so that
/// Wijesekara and Singlish always follow the physical keys.
enum KeyMap {
    // ANSI virtual key codes → (plain, shifted)
    private static let table: [UInt16: (Character, Character)] = [
        0: ("a", "A"), 1: ("s", "S"), 2: ("d", "D"), 3: ("f", "F"), 4: ("h", "H"), 5: ("g", "G"),
        6: ("z", "Z"), 7: ("x", "X"), 8: ("c", "C"), 9: ("v", "V"), 11: ("b", "B"), 12: ("q", "Q"),
        13: ("w", "W"), 14: ("e", "E"), 15: ("r", "R"), 16: ("y", "Y"), 17: ("t", "T"),
        18: ("1", "!"), 19: ("2", "@"), 20: ("3", "#"), 21: ("4", "$"), 22: ("6", "^"), 23: ("5", "%"),
        24: ("=", "+"), 25: ("9", "("), 26: ("7", "&"), 27: ("-", "_"), 28: ("8", "*"), 29: ("0", ")"),
        30: ("]", "}"), 31: ("o", "O"), 32: ("u", "U"), 33: ("[", "{"), 34: ("i", "I"), 35: ("p", "P"),
        37: ("l", "L"), 38: ("j", "J"), 39: ("'", "\""), 40: ("k", "K"), 41: (";", ":"), 42: ("\\", "|"),
        43: (",", "<"), 44: ("/", "?"), 45: ("n", "N"), 46: ("m", "M"), 47: (".", ">"), 50: ("`", "~"),
    ]

    /// (kind, character) for liyamu_session_key
    static func translate(_ event: NSEvent) -> (UInt32, UInt32) {
        switch event.keyCode {
        case 49: return (UInt32(LIYAMU_KEY_SPACE), 0)
        case 51: return (UInt32(LIYAMU_KEY_BACKSPACE), 0)
        case 36, 76: return (UInt32(LIYAMU_KEY_ENTER), 0)
        case 48: return (UInt32(LIYAMU_KEY_TAB), 0)
        case 53: return (UInt32(LIYAMU_KEY_ESCAPE), 0)
        case 123: return (UInt32(LIYAMU_KEY_LEFT), 0)
        case 124: return (UInt32(LIYAMU_KEY_RIGHT), 0)
        case 125: return (UInt32(LIYAMU_KEY_DOWN), 0)
        case 126: return (UInt32(LIYAMU_KEY_UP), 0)
        default: break
        }
        if let (plain, shifted) = table[event.keyCode] {
            let c = event.modifierFlags.contains(.shift) ? shifted : plain
            return (UInt32(LIYAMU_KEY_CHAR), c.unicodeScalars.first!.value)
        }
        // keypad digits and other printable keys: what the key types
        if let s = event.characters, s.unicodeScalars.count == 1, let u = s.unicodeScalars.first,
           u.value >= 0x20, u.value < 0xF700 {
            return (UInt32(LIYAMU_KEY_CHAR), u.value)
        }
        return (UInt32(LIYAMU_KEY_OTHER), 0)
    }
}
