import SwiftUI

/// Session-preset page: tap a preset → iPhone starts the timer → snapshot
/// flows back. No timer math on the watch.
struct WatchSessionView: View {
    @Environment(WatchConnectivityClient.self) private var conn

    /// `nil` until the user steps, so the stepper shows the core's pre-fill.
    @State private var steppedMinutes: Int?
    @State private var customSleep = false

    private var customMinutes: Int {
        steppedMinutes ?? conn.snapshot.customFocusMinutes
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text("FOCUS SESSION")
                    .font(.caption2)
                    .tracking(2)
                    .foregroundStyle(.secondary)

                ForEach(conn.snapshot.focusPresets, id: \.minutes) { preset($0) }

                Divider().padding(.vertical, 4)

                Button {
                    WatchHaptics.start()
                    conn.send(.startStopwatch)
                } label: {
                    HStack {
                        Text("Stopwatch")
                        Spacer()
                        Image(systemName: "stopwatch").font(.caption)
                    }
                }
                .buttonStyle(.bordered)

                Divider().padding(.vertical, 4)

                Text("CUSTOM")
                    .font(.caption2)
                    .tracking(2)
                    .foregroundStyle(.secondary)
                // Stepper drives via the Digital Crown — no keyboard on the wrist.
                Stepper(
                    value: Binding(get: { customMinutes }, set: { steppedMinutes = $0 }),
                    in: conn.snapshot.minMinutes ... conn.snapshot.maxMinutes,
                    step: 5
                ) {
                    Text("\(customMinutes) min").monospacedDigit()
                }
                Toggle("Sleep timer", isOn: $customSleep)
                Button {
                    WatchHaptics.start()
                    conn.send(.startTimer(minutes: customMinutes, sleep: customSleep))
                    steppedMinutes = nil
                } label: {
                    HStack {
                        Text(customSleep ? "Start sleep" : "Start focus")
                        Spacer()
                        Image(systemName: "play.fill").font(.caption)
                    }
                }
                .buttonStyle(.borderedProminent)

                if conn.snapshot.isTimerActive {
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

    private func preset(_ p: WatchTimerPreset) -> some View {
        Button {
            WatchHaptics.start()
            conn.send(.startTimer(minutes: p.minutes, sleep: false))
        } label: {
            HStack {
                Text(p.label)
                Spacer()
                Image(systemName: "play.fill").font(.caption)
            }
        }
        .buttonStyle(.bordered)
    }
}
