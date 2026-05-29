import SwiftUI

/// iOS root view. Same controls as the macOS main window but laid out for a
/// phone: edge-to-edge backdrop, big play button in the center, controls
/// stacked beneath, no separate Settings scene (iOS apps don't ship one by
/// convention — settings live inline).
struct CascadeScreen: View {
    @Environment(AppStore.self) private var store

    var body: some View {
        let snapshot = store.snapshot
        ZStack {
            CascadeBackdrop(isPlaying: snapshot.isPlaying)
                .ignoresSafeArea()
            VStack(spacing: 24) {
                Header(subtitle: snapshot.subtitle)
                Spacer(minLength: 8)
                TimerReadout(timer: snapshot.timer)
                PlayButton(
                    isPlaying: snapshot.isPlaying,
                    label: snapshot.primaryButtonLabel
                ) {
                    store.dispatch(.togglePlayback)
                }
                Spacer(minLength: 8)
                VolumeSlider(
                    percent: snapshot.volumePercent,
                    isMuted: snapshot.isMuted,
                    onChange: { store.dispatch(.setVolume(percent: $0)) },
                    onToggleMute: { store.dispatch(.toggleMute) }
                )
                TimerControls()
                if let message = snapshot.errorMessage ?? store.lastError {
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal)
                }
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 16)
        }
    }
}

private struct Header: View {
    let subtitle: String
    var body: some View {
        VStack(spacing: 4) {
            Text("Cascade")
                .font(.title.weight(.semibold))
            Text(subtitle)
                .font(.caption)
                .foregroundStyle(.secondary)
                .textCase(.uppercase)
                .tracking(2)
        }
    }
}

private struct TimerReadout: View {
    let timer: TimerSnapshot
    var body: some View {
        Group {
            if timer.kind == .off {
                Text("NO TIMER RUNNING")
                    .font(.caption)
                    .tracking(3)
                    .foregroundStyle(.secondary)
                    .frame(height: 40)
            } else {
                VStack(spacing: 6) {
                    Text(timer.remainingLabel)
                        .font(.system(size: 44, weight: .light, design: .rounded))
                        .monospacedDigit()
                    ProgressView(value: Double(max(0, min(1, timer.progress))))
                        .progressViewStyle(.linear)
                        .frame(maxWidth: 220)
                }
            }
        }
    }
}

private struct PlayButton: View {
    let isPlaying: Bool
    let label: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            ZStack {
                Circle().fill(isPlaying
                              ? AnyShapeStyle(.tint.opacity(0.18))
                              : AnyShapeStyle(.tint))
                Circle().stroke(.tint.opacity(0.5), lineWidth: 1.5)
                Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 40, weight: .medium))
                    .foregroundStyle(isPlaying
                                     ? AnyShapeStyle(.tint)
                                     : AnyShapeStyle(.background))
            }
            .frame(width: 160, height: 160)
            .shadow(color: .black.opacity(0.25), radius: 12, y: 6)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label)
    }
}

private struct VolumeSlider: View {
    let percent: Int
    let isMuted: Bool
    let onChange: (Int) -> Void
    let onToggleMute: () -> Void
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Button(action: onToggleMute) {
                    Image(systemName: isMuted ? "speaker.slash.fill" : "speaker.wave.2.fill")
                }
                .buttonStyle(.plain)
                .foregroundStyle(isMuted ? AnyShapeStyle(.tint) : AnyShapeStyle(.secondary))
                .accessibilityLabel(isMuted ? "Unmute" : "Mute")
                Text("Volume")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .tracking(2)
                Spacer()
                Text(isMuted ? "Muted" : "\(percent)%")
                    .font(.caption.monospacedDigit())
            }
            Slider(
                value: Binding(
                    get: { Double(percent) },
                    set: { onChange(Int($0.rounded())) }
                ),
                in: 0 ... 100,
                step: 1
            )
        }
    }
}

private enum CustomMode: String, CaseIterable, Identifiable {
    case focus = "Focus"
    case sleep = "Sleep"
    var id: String { rawValue }
}

private struct TimerControls: View {
    @Environment(AppStore.self) private var store
    @State private var customMinutes: Int = 45
    @State private var customMode: CustomMode = .focus

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            section(
                title: "Focus session",
                presets: [30, 60, 480],
                labelFor: { mins in
                    if mins < 60 { return "\(mins) min" }
                    if mins == 60 { return "1 hr" }
                    return "\(mins / 60) hr"
                },
                action: { store.dispatch(.startPomodoro(minutes: $0)) }
            )
            section(
                title: "Sleep timer",
                presets: [15, 30, 60],
                labelFor: { "\($0) min" },
                action: { store.dispatch(.startSleepTimer(minutes: $0)) }
            )
            customSection
            if store.snapshot.timer.kind != .off {
                Button("Cancel timer") { store.dispatch(.cancelTimer) }
                    .buttonStyle(.bordered)
                    .controlSize(.regular)
            }
        }
    }

    private var customSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Custom")
                .font(.caption)
                .foregroundStyle(.secondary)
                .tracking(2)
            Picker("Mode", selection: $customMode) {
                ForEach(CustomMode.allCases) { Text($0.rawValue).tag($0) }
            }
            .pickerStyle(.segmented)
            // Stepper keeps numeric entry keyboard-free, which suits a
            // one-handed "set it and forget it" interaction.
            Stepper(value: $customMinutes, in: 1 ... 1440, step: 5) {
                Text("\(customMinutes) min").monospacedDigit()
            }
            Button(customMode == .focus ? "Start focus" : "Start sleep") {
                switch customMode {
                case .focus: store.dispatch(.startPomodoro(minutes: customMinutes))
                case .sleep: store.dispatch(.startSleepTimer(minutes: customMinutes))
                }
            }
            .buttonStyle(.borderedProminent)
        }
    }

    @ViewBuilder
    private func section(
        title: String,
        presets: [Int],
        labelFor: @escaping (Int) -> String,
        action: @escaping (Int) -> Void
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
                .tracking(2)
            HStack(spacing: 8) {
                ForEach(presets, id: \.self) { mins in
                    Button(labelFor(mins)) { action(mins) }
                        .buttonStyle(.bordered)
                }
            }
        }
    }
}

private struct CascadeBackdrop: View {
    let isPlaying: Bool
    var body: some View {
        LinearGradient(
            colors: [
                Color(red: 0.04, green: 0.10, blue: 0.14),
                isPlaying
                    ? Color(red: 0.10, green: 0.32, blue: 0.45)
                    : Color(red: 0.06, green: 0.16, blue: 0.22),
                Color(red: 0.04, green: 0.10, blue: 0.14),
            ],
            startPoint: .top,
            endPoint: .bottom
        )
    }
}
