import Carbon
import Foundation

enum Registration {
    /// Makes the input method known to macOS. Switching it on is up to the user (System Settings), as macOS
    /// ignores TISEnableInputSource from installers.
    static func register() {
        let status = TISRegisterInputSource(Bundle.main.bundleURL as CFURL)
        print(status == noErr ? "registered \(Bundle.main.bundlePath)" : "TISRegisterInputSource failed: \(status)")
    }
}
