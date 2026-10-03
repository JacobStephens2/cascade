import Foundation

struct SyncAccount: Codable, Equatable {
    let sessionToken: String
    let email: String
}

/// Persists the optional sync account. (UserDefaults for the scaffold; the
/// Keychain is the on-device hardening follow-up for the session token.)
///
/// The device id now lives in the core's listening blob, which also rotates it
/// on "delete data". This only reads the id older builds stored here, so an
/// existing server slot carries over on the first restore.
final class AccountStore {
    private let defaults = UserDefaults.standard
    private let accountKey = "cascade.account.v1"
    private let legacyDeviceKey = "cascade.device.v1"

    func readAccount() -> SyncAccount? {
        guard let data = defaults.data(forKey: accountKey) else { return nil }
        return try? JSONDecoder().decode(SyncAccount.self, from: data)
    }

    func writeAccount(_ account: SyncAccount) {
        if let data = try? JSONEncoder().encode(account) {
            defaults.set(data, forKey: accountKey)
        }
    }

    func clearAccount() {
        defaults.removeObject(forKey: accountKey)
    }

    /// The device id older builds generated and stored here, if any. Read-only:
    /// handed to the core once as `restore`'s `fallbackDeviceId`.
    func legacyDeviceId() -> String? {
        guard let id = defaults.string(forKey: legacyDeviceKey), !id.isEmpty else { return nil }
        return id
    }
}
