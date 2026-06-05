import Foundation

struct SyncAccount: Codable, Equatable {
    let sessionToken: String
    let email: String
}

/// Persists the optional sync account and a stable per-device id. The device id
/// is rotated on "delete data" so a stale offline write can't resurrect a
/// deleted G-Counter slot. (UserDefaults for the scaffold; the Keychain is the
/// on-device hardening follow-up for the session token.)
final class AccountStore {
    private let defaults = UserDefaults.standard
    private let accountKey = "cascade.account.v1"
    private let deviceKey = "cascade.device.v1"

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

    func deviceId() -> String {
        if let id = defaults.string(forKey: deviceKey), !id.isEmpty { return id }
        return rotateDeviceId()
    }

    @discardableResult
    func rotateDeviceId() -> String {
        let id = UUID().uuidString
        defaults.set(id, forKey: deviceKey)
        return id
    }
}
