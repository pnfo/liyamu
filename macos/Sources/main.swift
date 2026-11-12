import Cocoa
import InputMethodKit

// One Liyamu input method (Wijesekara, Singlish or Helakuru keys, from Info.plist). `--register` registers
// it with macOS (used by install.sh); otherwise it runs as the input method server macOS launches on demand.

let connectionName = Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String ?? "Liyamu_Connection"

if CommandLine.arguments.contains("--register") {
    Registration.register()
    exit(0)
}

guard let server = IMKServer(name: connectionName, bundleIdentifier: Bundle.main.bundleIdentifier) else {
    NSLog("Liyamu: could not start the input method server")
    exit(1)
}
Fonts.register()
Settings.shared.loadShortcuts(force: true)

/// Opening Liyamu.app (Finder, Spotlight) while it runs as the input method shows the settings window.
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        SettingsWindow.shared.show()
        return false
    }
}
let delegate = AppDelegate()
NSApplication.shared.delegate = delegate
withExtendedLifetime(server) {
    NSApplication.shared.run()
}
