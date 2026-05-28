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
                VolumeSlider(percent: snapshot.volumePercent) { newPercent in
                    store.dispatch(.setVolume(percent: newPercent))
                }
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
                    .foregroundStyle(isPlaying ? AnyShapeStyle(.tint) : AnyShapeStyle(.background))
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
    let onChange: (Int) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Volume")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .tracking(2)
                Spacer()
                Text("\(percent)%")
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

private struct TimerControls: View {
    @Environment(AppStore.self) private var store
    @State private var customMinutesText: String = "45"
    @State private var showCustom = false

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
            if showCustom {
                HStack {
                    TextField("Minutes", text: $customMinutesText)
                        .frame(maxWidth: 80)
                        .textFieldStyle(.roundedBorder)
                    Button("Start focus") {
                        if let m = Int(customMinutesText), m > 0 {
                            store.dispatch(.startPomodoro(minutes: m))
                            showCustom = false
                        }
                    }
                    Spacer()
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
        labelFor: (Int) -> String,
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
