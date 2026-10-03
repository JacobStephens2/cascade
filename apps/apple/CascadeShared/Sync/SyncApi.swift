import Foundation

/// Base URL of cascade-sync-server. Empty disables the sync feature.
enum SyncConfig {
    static let apiBase = "https://sync.cascade.stephens.page"
    static var available: Bool { !apiBase.isEmpty }
}

struct SyncError: Error {
    let status: Int
    let message: String
}

struct VerifyResponse: Decodable {
    let sessionToken: String
    let email: String
}

struct ListeningResponse: Decodable {
    let serverTotalMs: Int64
}

/// Typed async client for the sync endpoints (URLSession).
struct SyncApi {
    private let base = SyncConfig.apiBase
    private let session = URLSession.shared

    func requestLink(email: String) async throws {
        _ = try await send("POST", "/auth/request", body: ["email": email], token: nil)
    }

    func verify(token: String) async throws -> VerifyResponse {
        let data = try await send("POST", "/auth/verify", body: ["token": token], token: nil)
        return try JSONDecoder().decode(VerifyResponse.self, from: data)
    }

    func logout(token: String) async throws {
        _ = try await send("POST", "/auth/logout", body: nil, token: token)
    }

    func putListening(token: String, deviceId: String, deviceTotalMs: Int64) async throws -> ListeningResponse {
        let data = try await send(
            "PUT", "/listening",
            body: ["deviceId": deviceId, "deviceTotalMs": deviceTotalMs],
            token: token)
        return try JSONDecoder().decode(ListeningResponse.self, from: data)
    }

    func deleteListening(token: String) async throws {
        _ = try await send("DELETE", "/listening", body: nil, token: token)
    }

    func deleteAccount(token: String) async throws {
        _ = try await send("DELETE", "/account", body: nil, token: token)
    }

    private func send(_ method: String, _ path: String, body: [String: Any]?, token: String?) async throws -> Data {
        guard let url = URL(string: base + path) else {
            throw SyncError(status: 0, message: "bad url")
        }
        var req = URLRequest(url: url)
        req.httpMethod = method
        req.timeoutInterval = 15
        req.setValue("application/json", forHTTPHeaderField: "Content-Type")
        if let token { req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization") }
        if let body { req.httpBody = try JSONSerialization.data(withJSONObject: body) }

        let (data, response) = try await session.data(for: req)
        let code = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard (200...299).contains(code) else {
            throw SyncError(status: code, message: String(data: data, encoding: .utf8) ?? "request failed (\(code))")
        }
        return data
    }
}
