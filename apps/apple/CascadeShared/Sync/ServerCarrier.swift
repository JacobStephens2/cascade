import Foundation

/// Base URL of cascade-sync-server. Empty disables the sync feature.
enum SyncConfig {
    static let apiBase = "https://sync.cascade.stephens.page"
    static var available: Bool { !apiBase.isEmpty }
}

/// Carries the core's `serverRequest` effects to the sync server (URLSession).
/// It decides nothing per request: it sends what it is given and answers with
/// the `serverResponse` that settles it, holding the status and body verbatim.
/// The core reads them.
struct ServerCarrier {
    private let session = URLSession.shared

    /// Send one request and return its settle. Status 0 when it was not sent
    /// or got no response: no sync server configured, a network error, a
    /// timeout or a cancelled task.
    func send(_ request: ServerRequest) async -> Command {
        let unsent = Command.serverResponse(id: request.id, status: 0, body: "")
        guard SyncConfig.available, let url = URL(string: SyncConfig.apiBase + request.path) else {
            return unsent
        }
        var req = URLRequest(url: url)
        req.httpMethod = request.method.rawValue
        req.timeoutInterval = 15
        if let token = request.bearerToken {
            req.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        if let body = request.body {
            req.setValue("application/json", forHTTPHeaderField: "Content-Type")
            req.httpBody = Data(body.utf8)
        }

        do {
            let (data, response) = try await session.data(for: req)
            guard let http = response as? HTTPURLResponse else { return unsent }
            return .serverResponse(
                id: request.id,
                status: UInt16(clamping: http.statusCode),
                body: String(decoding: data, as: UTF8.self))
        } catch {
            return unsent
        }
    }
}
