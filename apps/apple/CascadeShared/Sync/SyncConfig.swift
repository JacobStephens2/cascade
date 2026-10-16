import Foundation

/// Base URL of cascade-sync-server. Empty disables the sync feature.
enum SyncConfig {
    static let apiBase = "https://sync.cascade.stephens.page"
    static var available: Bool { !apiBase.isEmpty }
}
