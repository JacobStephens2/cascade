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
            ScrollView {
                VStack(spacing: 24) {
                    Header(subtitle: snapshot.subtitle)
                    TimerReadout(timer: snapshot.timer)
                    PlayButton(
                        isPlaying: snapshot.isPlaying,
                        label: snapshot.primaryButtonLabel
                    ) {
                        store.dispatch(.togglePlayback)
                    }
                    VolumeSlider(
                        percent: snapshot.volumePercent,
                        isMuted: snapshot.isMuted,
                        onChange: { store.dispatch(.setVolume(percent: $0)) },
                        onToggleMute: { store.dispatch(.toggleMute) }
                    )
                    ListeningRow(listening: snapshot.listening) {
                        store.dispatch(.setListeningTracking(enabled: !snapshot.listening.trackingEnabled))
                    }
                    AccountControlsView()
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
                .padding(.vertical, 32)
                .frame(maxWidth: 620)
                .frame(maxWidth: .infinity)
            }
            .scrollIndicators(.hidden)
        }
        .preferredColorScheme(.dark)
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
                                     : AnyShapeStyle(.white))
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

private struct ListeningRow: View {
    let listening: ListeningSnapshot
    let onToggle: () -> Void
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(listening.totalLabel)
                    .font(.title3.weight(.light))
                    .monospacedDigit()
                Text(listening.trackingEnabled ? "LIFETIME LISTENING" : "TRACKING PAUSED")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .tracking(2)
            }
            Spacer()
            Toggle(
                "Track listening",
                isOn: Binding(get: { listening.trackingEnabled }, set: { _ in onToggle() })
            )
            .labelsHidden()
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
    /// `nil` until the user steps, so the stepper shows the core's pre-fill
    /// for the chosen mode.
    @State private var steppedMinutes: Int?
    @State private var customMode: CustomMode = .focus

    private var options: TimerOptions { store.snapshot.timerOptions }

    private var customMinutes: Int {
        steppedMinutes
            ?? (customMode == .focus ? options.customFocusMinutes : options.customSleepMinutes)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            section(
                title: "Focus session",
                presets: options.focusPresets,
                action: { store.dispatch(.startPomodoro(minutes: $0)) }
            )
            section(
                title: "Sleep timer",
                presets: options.sleepPresets,
                action: { store.dispatch(.startSleepTimer(minutes: $0)) }
            )
            VStack(alignment: .leading, spacing: 6) {
                Text("Stopwatch")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .tracking(2)
                Button("Start stopwatch") { store.dispatch(.startStopwatch) }
                    .buttonStyle(.bordered)
            }
            customSection
            if store.snapshot.timer.isActive {
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
            .onChange(of: customMode) { steppedMinutes = nil }
            // Stepper keeps numeric entry keyboard-free, which suits a
            // one-handed "set it and forget it" interaction.
            Stepper(
                value: Binding(get: { customMinutes }, set: { steppedMinutes = $0 }),
                in: options.minMinutes ... options.maxMinutes,
                step: 5
            ) {
                Text("\(customMinutes) min").monospacedDigit()
            }
            Button(customMode == .focus ? "Start focus" : "Start sleep") {
                let minutes = customMinutes
                steppedMinutes = nil
                switch customMode {
                case .focus: store.dispatch(.startPomodoro(minutes: minutes))
                case .sleep: store.dispatch(.startSleepTimer(minutes: minutes))
                }
            }
            .buttonStyle(.borderedProminent)
        }
    }

    @ViewBuilder
    private func section(
        title: String,
        presets: [TimerPreset],
        action: @escaping (Int) -> Void
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
                .tracking(2)
            HStack(spacing: 8) {
                ForEach(presets, id: \.minutes) { preset in
                    Button(preset.label) { action(preset.minutes) }
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
