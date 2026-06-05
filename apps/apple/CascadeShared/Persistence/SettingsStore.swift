import Foundation

/// Stores the cascade-core settings blob (JSON produced by `Effect.persistSettings`)
/// as `~/Library/Application Support/Cascade/settings.json`.
///
/// The JSON shape is opaque to Swift — the Rust core owns the schema. We just
/// round-trip the bytes.
final class SettingsStore {
    private let url: URL
    /// Lifetime listening ledger — a separate file from settings, so the two
    /// evolve and fail independently (mirrors cascade.listening.v1 on web).
    private let listeningUrl: URL

    init() {
        let fm = FileManager.default
        let support = (try? fm.url(for: .applicationSupportDirectory,
                                   in: .userDomainMask,
                                   appropriateFor: nil,
                                   create: true))
            ?? URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Library/Application Support")
        let dir = support.appendingPathComponent("Cascade", isDirectory: true)
        try? fm.createDirectory(at: dir, withIntermediateDirectories: true)
        self.url = dir.appendingPathComponent("settings.json")
        self.listeningUrl = dir.appendingPathComponent("listening.json")
    }

    /// Returns nil if the file doesn't exist or can't be read — the bridge
    /// then boots with defaults.
    func readSafely() -> String? { Self.read(url) }

    func writeSafely(_ json: String) { Self.write(json, to: url, label: "settings") }

    func readListeningSafely() -> String? { Self.read(listeningUrl) }

    func writeListeningSafely(_ json: String) { Self.write(json, to: listeningUrl, label: "listening") }

    private static func read(_ url: URL) -> String? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    private static func write(_ json: String, to url: URL, label: String) {
        guard let data = json.data(using: .utf8) else { return }
        do {
            try data.write(to: url, options: [.atomic])
        } catch {
            NSLog("[Cascade] \(label) write failed: \(error)")
        }
    }

    /// For the "Reveal in Finder" debug action.
    var fileURL: URL { url }
}
