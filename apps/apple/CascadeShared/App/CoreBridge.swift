import Foundation

/// Wraps the UniFFI-generated `CascadeBridge` in something idiomatic for the
/// macOS shell: JSON in, JSON out, decoded into typed Swift values.
///
/// Same shape as the Android `CascadeBridgeHolder` and the web `useCascade`
/// hook. Errors come back as a typed `BridgeError`, never strings.
@MainActor
final class CoreBridge {
    enum BridgeError: Error {
        case decode(String)
        case core(String)
    }

    private let bridge: CascadeBridge
    private let encoder: JSONEncoder
    private let decoder: JSONDecoder

    init(persistedSettings: String?) {
        if let json = persistedSettings, !json.isEmpty {
            self.bridge = CascadeBridge.restoreOrNew(settingsJson: json)
        } else {
            self.bridge = CascadeBridge()
        }
        let enc = JSONEncoder()
        enc.outputFormatting = [.withoutEscapingSlashes]
        self.encoder = enc
        self.decoder = JSONDecoder()
    }

    func snapshot() throws -> Snapshot {
        do {
            let json = try bridge.snapshot()
            return try decode(Snapshot.self, from: json)
        } catch let error as CascadeError {
            throw BridgeError.core(message(for: error))
        }
    }

    func dispatch(_ command: Command) throws -> Update {
        let commandData: Data
        do {
            commandData = try encoder.encode(command)
        } catch {
            throw BridgeError.decode("encode command: \(error.localizedDescription)")
        }
        let commandJSON = String(data: commandData, encoding: .utf8) ?? "{}"
        do {
            let updateJSON = try bridge.dispatch(commandJson: commandJSON)
            return try decode(Update.self, from: updateJSON)
        } catch let error as CascadeError {
            throw BridgeError.core(message(for: error))
        }
    }

    private func decode<T: Decodable>(_ type: T.Type, from json: String) throws -> T {
        guard let data = json.data(using: .utf8) else {
            throw BridgeError.decode("invalid UTF-8")
        }
        do {
            return try decoder.decode(type, from: data)
        } catch {
            throw BridgeError.decode("\(type): \(error.localizedDescription) — payload: \(json.prefix(200))")
        }
    }

    // UniFFI renames the inner `reason` field to `message` on the Swift side
    // for both error variants (its convention for single-String error enums).
    private func message(for error: CascadeError) -> String {
        switch error {
        case .BadJson(let message): return "bad JSON: \(message)"
        case .Core(let message): return "core: \(message)"
        }
    }
}
