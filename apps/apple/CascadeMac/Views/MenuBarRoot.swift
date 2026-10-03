import SwiftUI

/// Compact menu-bar surface — the 95%-case daily interaction. Click the
/// drop icon, see status, hit play, pick a duration.
struct MenuBarRoot: View {
    @Environment(AppStore.self) private var store
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        let snapshot = store.snapshot
        VStack(alignment: .leading, spacing: 10) {
            Text("Cascade")
                .font(.headline)
            Text(snapshot.timer.statusLabel)
                .font(.caption)
                .foregroundStyle(.secondary)

            Divider()

            // Big play / pause action.
            Button {
                store.dispatch(.togglePlayback)
            } label: {
                HStack {
                    Image(systemName: snapshot.isPlaying ? "pause.fill" : "play.fill")
                    Text(snapshot.primaryButtonLabel)
                    Spacer()
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.regular)

            // Inline volume.
            HStack {
                Image(systemName: "speaker.fill")
                    .foregroundStyle(.secondary)
                Slider(
                    value: Binding(
                        get: { Double(snapshot.volumePercent) },
                        set: { store.dispatch(.setVolume(percent: Int($0.rounded()))) }
                    ),
                    in: 0 ... 100
                )
                Image(systemName: "speaker.wave.3.fill")
                    .foregroundStyle(.secondary)
            }

            Divider()

            Text("FOCUS SESSION")
                .font(.caption2)
                .foregroundStyle(.secondary)
                .tracking(2)
            HStack(spacing: 6) {
                ForEach(snapshot.timerOptions.focusPresets, id: \.minutes) { preset in
                    presetButton(preset.shortLabel) {
                        store.dispatch(.startPomodoro(minutes: preset.minutes))
                    }
                }
            }

            Text("SLEEP TIMER")
                .font(.caption2)
                .foregroundStyle(.secondary)
                .tracking(2)
            HStack(spacing: 6) {
                ForEach(snapshot.timerOptions.sleepPresets, id: \.minutes) { preset in
                    presetButton(preset.shortLabel) {
                        store.dispatch(.startSleepTimer(minutes: preset.minutes))
                    }
                }
            }

            if snapshot.timer.isActive {
                Button("Cancel timer") { store.dispatch(.cancelTimer) }
                    .buttonStyle(.link)
            }

            Divider()

            Button("Open Cascade…") {
                NSApp.activate(ignoringOtherApps: true)
                openWindow(id: "main")
            }
            SettingsLink {
                Text("Settings…")
            }
            .keyboardShortcut(",", modifiers: [.command])
            Button("Quit Cascade") { NSApp.terminate(nil) }
                .keyboardShortcut("q", modifiers: [.command])
        }
        .padding(14)
        .frame(width: 280)
    }

    @ViewBuilder
    private func presetButton(_ label: String, action: @escaping () -> Void) -> some View {
        Button(label, action: action)
            .buttonStyle(.bordered)
            .controlSize(.small)
    }
}
