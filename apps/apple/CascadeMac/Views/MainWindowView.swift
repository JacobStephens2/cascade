import SwiftUI

struct MainWindowView: View {
    @Environment(AppStore.self) private var store

    var body: some View {
        let snapshot = store.snapshot
        ZStack {
            CascadeBackdrop(isPlaying: snapshot.isPlaying, progress: snapshot.timer.progress)
                .ignoresSafeArea()

            VStack(spacing: 24) {
                Header(subtitle: snapshot.subtitle)
                Spacer(minLength: 0)
                TimerReadout(timer: snapshot.timer)
                PlayButton(isPlaying: snapshot.isPlaying, label: snapshot.primaryButtonLabel) {
                    store.dispatch(.togglePlayback)
                }
                VolumeSlider(
                    percent: snapshot.volumePercent,
                    isMuted: snapshot.isMuted,
                    onChange: { store.dispatch(.setVolume(percent: $0)) },
                    onToggleMute: { store.dispatch(.toggleMute) }
                )
                .padding(.horizontal, 4)
                ListeningRow(listening: snapshot.listening) {
                    store.dispatch(.setListeningTracking(enabled: !snapshot.listening.trackingEnabled))
                }
                .padding(.horizontal, 4)
                AccountControlsView()
                    .padding(.horizontal, 4)
                TimerControls()
                Spacer(minLength: 0)
                if let message = snapshot.errorMessage ?? store.lastError {
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .multilineTextAlignment(.center)
                }
            }
            .padding(.horizontal, 32)
            .padding(.vertical, 28)
        }
        // The backdrop is always a dark gradient, so pin the window to dark
        // mode — otherwise Light-mode label colors (.primary/.secondary)
        // render dark-on-dark and become unreadable.
        .preferredColorScheme(.dark)
    }
}

private struct Header: View {
    let subtitle: String
    var body: some View {
        HStack {
            Text("Cascade")
                .font(.title2.weight(.semibold))
            Spacer()
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
        VStack(spacing: 6) {
            if timer.kind == .off {
                Text("NO TIMER RUNNING")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .tracking(3)
                    .frame(height: 38)
            } else {
                Text(timer.remainingLabel)
                    .font(.system(size: 40, weight: .light, design: .rounded))
                    .monospacedDigit()
                ProgressView(value: Double(timer.progress.clamped(0, 1)))
                    .progressViewStyle(.linear)
                    .frame(maxWidth: 220)
            }
        }
        .frame(minHeight: 64)
    }
}

private struct PlayButton: View {
    let isPlaying: Bool
    let label: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            ZStack {
                Circle()
                    .fill(isPlaying
                          ? AnyShapeStyle(.tint.opacity(0.18))
                          : AnyShapeStyle(.tint))
                Circle()
                    .stroke(.tint.opacity(0.5), lineWidth: 1.5)
                Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 36, weight: .medium))
                    .foregroundStyle(isPlaying ? AnyShapeStyle(.tint) : AnyShapeStyle(.white))
            }
            .frame(width: 132, height: 132)
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
                .buttonStyle(.borderless)
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
                    .font(.caption)
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
    /// `nil` until the user types, so the field shows the core's pre-fill
    /// for the chosen mode.
    @State private var typedMinutes: String?
    @State private var showCustom = false
    @State private var customMode: CustomMode = .focus

    private var options: TimerOptions { store.snapshot.timerOptions }

    private var customMinutesText: String {
        typedMinutes
            ?? String(customMode == .focus ? options.customFocusMinutes : options.customSleepMinutes)
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
            if showCustom {
                VStack(alignment: .leading, spacing: 8) {
                    Picker("Mode", selection: $customMode) {
                        ForEach(CustomMode.allCases) { mode in
                            Text(mode.rawValue).tag(mode)
                        }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .frame(maxWidth: 200)
                    .onChange(of: customMode) { typedMinutes = nil }
                    HStack {
                        TextField(
                            "Minutes",
                            text: Binding(get: { customMinutesText }, set: { typedMinutes = $0 })
                        )
                            .frame(maxWidth: 80)
                            .textFieldStyle(.roundedBorder)
                        Button(customMode == .focus ? "Start focus" : "Start sleep") {
                            // Only a positive whole number crosses; the core
                            // clamps it into the limits.
                            if let m = Int(customMinutesText), m > 0 {
                                typedMinutes = nil
                                switch customMode {
                                case .focus: store.dispatch(.startPomodoro(minutes: m))
                                case .sleep: store.dispatch(.startSleepTimer(minutes: m))
                                }
                                showCustom = false
                            }
                        }
                        Spacer()
                    }
                }
            }
            HStack {
                Button(showCustom ? "Hide custom" : "Custom…") { showCustom.toggle() }
                    .buttonStyle(.link)
                if store.snapshot.timer.isActive {
                    Spacer()
                    Button("Cancel timer") { store.dispatch(.cancelTimer) }
                        .buttonStyle(.link)
                }
            }
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
                        .controlSize(.regular)
                }
            }
        }
    }
}

private struct CascadeBackdrop: View {
    let isPlaying: Bool
    let progress: Float

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
        .overlay(alignment: .bottom) {
            // Subtle progress wash for active sessions.
            if progress > 0 {
                Rectangle()
                    .fill(.tint.opacity(0.08))
                    .frame(height: 80 * CGFloat(progress.clamped(0, 1)))
            }
        }
    }
}

private extension Float {
    func clamped(_ lower: Float, _ upper: Float) -> Float {
        min(max(self, lower), upper)
    }
}
