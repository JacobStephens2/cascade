import Foundation

/// Stores the cascade-core settings blob (JSON produced by `Effect.persistSettings`)
/// as `~/Library/Application Support/Cascade/settings.json`.
///
/// The JSON shape is opaque to Swift — the Rust core owns the schema. We just
/// round-trip the bytes.
final class SettingsStore {
    private let url: URL

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
    }

    /// Returns nil if the file doesn't exist or can't be read — the bridge
    /// then boots with defaults.
    func readSafely() -> String? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return String(data: data, encoding: .utf8)
    }

    func writeSafely(_ json: String) {
        guard let data = json.data(using: .utf8) else { return }
        do {
            try data.write(to: url, options: [.atomic])
        } catch {
            NSLog("[Cascade] settings write failed: \(error)")
        }
    }

    /// For the "Reveal in Finder" debug action.
    var fileURL: URL { url }
}
