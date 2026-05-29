import SwiftUI
import WatchKit

/// Primary screen: status text on top, oversized play/pause in the middle,
/// Digital Crown for volume. The watch hardware's strongest affordances are
/// the tap target and the crown; everything else is glanceable polish.
struct WatchPlayerView: View {
    @Environment(WatchConnectivityClient.self) private var conn
    @State private var crownVolume: Double = 60
    @State private var didSyncFromSnapshot = false

    var body: some View {
        VStack(spacing: 6) {
            Text(conn.snapshot.statusLine)
                .font(.caption.monospacedDigit())
                .lineLimit(1)
                .minimumScaleFactor(0.6)
                .foregroundStyle(.secondary)

            Button(action: togglePlayback) {
                Image(systemName: conn.snapshot.isPlaying ? "pause.fill" : "play.fill")
                    .resizable()
                    .scaledToFit()
                    .frame(width: 44, height: 44)
                    .padding(20)
            }
            .buttonStyle(.borderedProminent)
            .tint(.cyan)
            .clipShape(Circle())

            HStack(spacing: 4) {
                Image(systemName: "speaker.fill")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                ProgressView(value: Double(conn.snapshot.volumePercent), total: 100)
                    .tint(.cyan)
                Text("\(conn.snapshot.volumePercent)%")
                    .font(.caption2.monospacedDigit())
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 4)
        .focusable()
        .digitalCrownRotation(
            $crownVolume,
            from: 0,
            through: 100,
            by: 1,
            sensitivity: .medium,
            isContinuous: false,
            isHapticFeedbackEnabled: true
        )
        .onChange(of: crownVolume) { _, new in
            // Avoid echoing back the value the iPhone just told us about.
            let intValue = Int(new.rounded())
            guard intValue != conn.snapshot.volumePercent else { return }
            conn.send(.setVolume(percent: intValue))
        }
        .onChange(of: conn.snapshot.volumePercent) { _, new in
            // Reflect remote changes (user adjusted volume on the phone).
            crownVolume = Double(new)
        }
        .onAppear {
            if !didSyncFromSnapshot {
                crownVolume = Double(conn.snapshot.volumePercent)
                didSyncFromSnapshot = true
            }
        }
    }

    private func togglePlayback() {
        WatchHaptics.tap()
        conn.send(.togglePlayback)
    }
}
