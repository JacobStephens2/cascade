import ServiceManagement
import SwiftUI

struct SettingsView: View {
    @Environment(AppStore.self) private var store
    @State private var launchAtLogin: Bool = SettingsView.currentLaunchAtLogin()
    @State private var lastError: String?

    var body: some View {
        TabView {
            Form {
                Section("Playback") {
                    LabeledContent("Current volume") {
                        Text("\(store.snapshot.volumePercent)%")
                            .monospacedDigit()
                    }
                    LabeledContent("Status") {
                        Text(store.snapshot.isPlaying ? "Playing" : "Paused")
                    }
                }
                Section("System") {
                    Toggle("Launch Cascade at login", isOn: $launchAtLogin)
                        .onChange(of: launchAtLogin) { _, new in
                            applyLaunchAtLogin(new)
                        }
                    if let lastError {
                        Text(lastError)
                            .foregroundStyle(.red)
                            .font(.caption)
                    }
                }
                Section("Settings file") {
                    LabeledContent("Location") {
                        Text(SettingsStore().fileURL.path)
                            .font(.caption.monospaced())
                            .textSelection(.enabled)
                    }
                    Button("Reveal in Finder") {
                        NSWorkspace.shared.activateFileViewerSelecting([SettingsStore().fileURL])
                    }
                }
            }
            .padding(20)
            .tabItem { Label("General", systemImage: "gearshape") }
        }
        .frame(width: 460, height: 360)
    }

    private static func currentLaunchAtLogin() -> Bool {
        SMAppService.mainApp.status == .enabled
    }

    private func applyLaunchAtLogin(_ enabled: Bool) {
        do {
            if enabled {
                try SMAppService.mainApp.register()
            } else {
                try SMAppService.mainApp.unregister()
            }
            lastError = nil
        } catch {
            lastError = "Couldn't update login item: \(error.localizedDescription)"
            launchAtLogin = SettingsView.currentLaunchAtLogin()
        }
    }
}
