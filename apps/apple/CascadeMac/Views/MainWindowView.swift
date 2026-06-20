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
                        .foregroundStyle(CascadeTheme.danger)
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
        // Match the web shell: cyan brand accent on every control + near-white
        // ink as the default text color.
        .tint(CascadeTheme.accent)
        .foregroundStyle(CascadeTheme.ink)
    }
}

private struct Header: View {
    let subtitle: String
    var body: some View {
        HStack {
            Text("Cascade")
                .font(.title2.weight(.semibold))
                .textCase(.uppercase)
                .tracking(4)
                .foregroundStyle(CascadeTheme.accent)
            Spacer()
            Text(subtitle)
                .font(.caption)
                .foregroundStyle(CascadeTheme.inkDim)
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
                    .foregroundStyle(CascadeTheme.inkFaint)
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
            // Mirror the web play control: a translucent disc with a glowing
            // accent ring (brighter while playing) and a light glyph, rather
            // than a solid blue button.
            ZStack {
                Circle().fill(
                    RadialGradient(
                        colors: [Color.white.opacity(0.08), Color.white.opacity(0)],
                        center: UnitPoint(x: 0.5, y: 0.35),
                        startRadius: 0,
                        endRadius: 92
                    )
                )
                Circle()
                    .strokeBorder(
                        CascadeTheme.accent.opacity(isPlaying ? 0.9 : 0.35),
                        lineWidth: 1.5
                    )
                    .shadow(color: CascadeTheme.accent.opacity(isPlaying ? 0.45 : 0),
                            radius: 22)
                Circle()
                    .strokeBorder(style: StrokeStyle(lineWidth: 1, dash: [4, 6]))
                    .foregroundStyle(CascadeTheme.accent.opacity(isPlaying ? 0.28 : 0.12))
                    .padding(-12)
                Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 36, weight: .medium))
                    .foregroundStyle(CascadeTheme.ink)
                    .shadow(color: CascadeTheme.accent.opacity(0.5), radius: 14)
            }
            .frame(width: 132, height: 132)
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
                .foregroundStyle(isMuted ? AnyShapeStyle(CascadeTheme.accent) : AnyShapeStyle(CascadeTheme.inkDim))
                .accessibilityLabel(isMuted ? "Unmute" : "Mute")
                Text("Volume")
                    .font(.caption)
                    .foregroundStyle(CascadeTheme.inkDim)
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
                    .foregroundStyle(CascadeTheme.inkDim)
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
    @State private var customMinutesText: String = "45"
    @State private var showCustom = false
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
            VStack(alignment: .leading, spacing: 6) {
                Text("Stopwatch")
                    .font(.caption)
                    .foregroundStyle(CascadeTheme.inkDim)
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
                    HStack {
                        TextField("Minutes", text: $customMinutesText)
                            .frame(maxWidth: 80)
                            .textFieldStyle(.roundedBorder)
                        Button(customMode == .focus ? "Start focus" : "Start sleep") {
                            if let m = Int(customMinutesText), m > 0, m <= 1440 {
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
                if store.snapshot.timer.kind != .off {
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
        presets: [Int],
        labelFor: @escaping (Int) -> String,
        action: @escaping (Int) -> Void
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption)
                .foregroundStyle(CascadeTheme.inkDim)
                .tracking(2)
            HStack(spacing: 8) {
                ForEach(presets, id: \.self) { mins in
                    Button(labelFor(mins)) { action(mins) }
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
