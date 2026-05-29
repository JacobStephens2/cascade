import SwiftUI

/// Session-preset page: tap a preset → iPhone starts the timer → snapshot
/// flows back. No timer math on the watch.
struct WatchSessionView: View {
    @Environment(WatchConnectivityClient.self) private var conn

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text("FOCUS SESSION")
                    .font(.caption2)
                    .tracking(2)
                    .foregroundStyle(.secondary)

                preset("30 min",  .minutes30)
                preset("60 min",  .minutes60)
                preset("8 hours", .hours8)

                if !conn.snapshot.timerRemainingLabel.isEmpty {
                    Divider()
                        .padding(.vertical, 4)

                    Text("REMAINING")
                        .font(.caption2)
                        .tracking(2)
                        .foregroundStyle(.secondary)
                    Text(conn.snapshot.timerRemainingLabel)
                        .font(.title3.monospacedDigit())
                    ProgressView(value: Double(conn.snapshot.timerProgress))
                        .tint(.cyan)

                    Button(role: .destructive) {
                        WatchHaptics.stop()
                        conn.send(.cancelTimer)
                    } label: {
                        Label("Cancel timer", systemImage: "xmark.circle")
                    }
                    .buttonStyle(.bordered)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func preset(_ label: String, _ p: WatchSessionPreset) -> some View {
        Button {
            WatchHaptics.start()
            conn.send(.startSession(p))
        } label: {
            HStack {
                Text(label)
                Spacer()
                Image(systemName: "play.fill").font(.caption)
            }
        }
        .buttonStyle(.bordered)
    }
}
