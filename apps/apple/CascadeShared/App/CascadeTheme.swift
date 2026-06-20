import SwiftUI

/// Shared color palette mirroring the web app's design tokens
/// (`apps/web/src/styles.css` `:root`). Keeping the native shells on the same
/// values as the web shell is what makes them look like one product.
///
/// The web tokens, for reference:
///   --bg #050b10  --bg-deep #02060a
///   --ink #e6f1f6  --ink-dim #8aa3b0  --ink-faint #4f6b78
///   --accent #6ec9e2  --danger #ff8a72
///   --paper rgba(255,255,255,0.04)  --paper-strong rgba(255,255,255,0.07)
enum CascadeTheme {
    /// Cyan brand accent (`--accent`). Drives the app `.tint`, so it replaces
    /// the default system blue on buttons, sliders, and the play control.
    static let accent = Color(hex: 0x6EC9E2)
    /// Translucent accent fill (`--accent-soft`).
    static let accentSoft = Color(hex: 0x6EC9E2, alpha: 0.12)

    /// Primary text (`--ink`) — near-white.
    static let ink = Color(hex: 0xE6F1F6)
    /// Dimmed text (`--ink-dim`) — used for captions/labels. Bright enough to
    /// stay legible on the dark backdrop, unlike SwiftUI's `.secondary`.
    static let inkDim = Color(hex: 0x8AA3B0)
    /// Faint text (`--ink-faint`) — the quietest tier (footer, off states).
    static let inkFaint = Color(hex: 0x4F6B78)

    /// Warm error color (`--danger`).
    static let danger = Color(hex: 0xFF8A72)

    /// Card / control fills (`--paper`, `--paper-strong`).
    static let paper = Color(hex: 0xFFFFFF, alpha: 0.04)
    static let paperStrong = Color(hex: 0xFFFFFF, alpha: 0.07)

    /// Backdrop gradient stops (`--bg`, `--bg-deep`) plus the playing accent
    /// wash, shared by both shells' `CascadeBackdrop`.
    static let bg = Color(hex: 0x050B10)
    static let bgDeep = Color(hex: 0x02060A)
}

extension Color {
    /// Build a Color from a 0xRRGGBB literal (matches the CSS hex tokens).
    init(hex: UInt32, alpha: Double = 1.0) {
        let r = Double((hex >> 16) & 0xFF) / 255.0
        let g = Double((hex >> 8) & 0xFF) / 255.0
        let b = Double(hex & 0xFF) / 255.0
        self.init(.sRGB, red: r, green: g, blue: b, opacity: alpha)
    }
}
