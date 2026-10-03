import Foundation

/// Persists the account blob the core hands over in `persistAccount`.
/// (UserDefaults for the scaffold; the Keychain is the on-device hardening
/// follow-up for the session token.)
///
/// Before the core held the account this shell stored `{ sessionToken, email }`
/// under the same key; the core reads that shape too, so the stored string is
/// handed to `restore` unchanged.
///
/// The device id now lives in the core's listening blob, which also rotates it
/// on "delete data". This only reads the id older builds stored here, so an
/// existing server slot carries over on the first restore.
final class AccountStore {
    private let defaults = UserDefaults.standard
    private let accountKey = "cascade.account.v1"
    private let legacyDeviceKey = "cascade.device.v1"

    /// The stored account blob, verbatim; `""` if there is none.
    func readAccountJson() -> String {
        guard let data = defaults.data(forKey: accountKey) else { return "" }
        return String(data: data, encoding: .utf8) ?? ""
    }

    /// Store the blob verbatim; an empty one deletes the stored account.
    func writeAccountJson(_ json: String) {
        if json.isEmpty {
            defaults.removeObject(forKey: accountKey)
        } else {
            defaults.set(Data(json.utf8), forKey: accountKey)
        }
    }

    /// The device id older builds generated and stored here, if any. Read-only:
    /// handed to the core once as `restore`'s `fallbackDeviceId`.
    func legacyDeviceId() -> String? {
        guard let id = defaults.string(forKey: legacyDeviceKey), !id.isEmpty else { return nil }
        return id
    }
}
