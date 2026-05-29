import SwiftUI

/// Connectivity status / diagnostic page. Surfaces the reachability state so
/// the user has something to look at when commands aren't going through.
struct WatchStatusView: View {
    @Environment(WatchConnectivityClient.self) private var conn

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("CASCADE")
                .font(.caption2)
                .tracking(2)
                .foregroundStyle(.secondary)

            HStack(spacing: 6) {
                Image(systemName: conn.isReachable ? "iphone.gen3" : "iphone.slash")
                    .foregroundStyle(conn.isReachable ? .cyan : .secondary)
                Text(conn.isReachable ? "Connected" : "iPhone unreachable")
                    .font(.footnote)
            }

            Text(conn.snapshot.statusLine)
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)

            Spacer(minLength: 4)

            Button {
                conn.send(.requestSnapshot)
            } label: {
                Label("Refresh", systemImage: "arrow.clockwise")
            }
            .buttonStyle(.bordered)
        }
        .padding(.horizontal, 4)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
