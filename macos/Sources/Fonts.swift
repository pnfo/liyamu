import Cocoa
import CoreText

/// The Sinhala font that comes with the app (Abhaya), for the suggestions and the settings window.
enum Fonts {
    private static var name: String?

    /// Makes the fonts in the app's Resources usable by this process only (nothing is installed).
    static func register() {
        let urls = Bundle.main.urls(forResourcesWithExtension: "ttf", subdirectory: nil) ?? []
        for url in urls {
            CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
            if name == nil, let d = (CTFontManagerCreateFontDescriptorsFromURL(url as CFURL) as? [CTFontDescriptor])?.first {
                name = CTFontDescriptorCopyAttribute(d, kCTFontNameAttribute) as? String
            }
        }
    }

    /// The system font, with Sinhala in Abhaya.
    static func ui(_ size: CGFloat) -> NSFont {
        guard let name else { return NSFont.systemFont(ofSize: size) }
        let d = NSFont.systemFont(ofSize: size).fontDescriptor
            .addingAttributes([.cascadeList: [NSFontDescriptor(fontAttributes: [.name: name])]])
        return NSFont(descriptor: d, size: size) ?? NSFont.systemFont(ofSize: size)
    }

    static func sinhala(_ size: CGFloat) -> NSFont {
        name.flatMap { NSFont(name: $0, size: size) } ?? NSFont.systemFont(ofSize: size * 0.85)
    }
}
