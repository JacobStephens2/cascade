import SwiftUI

@main
struct CascadeMacApp: App {
    @State private var store = AppStore.bootstrap()

    var body: some Scene {
        WindowGroup("Cascade", id: "main") {
            MainWindowView()
                .environment(store)
                .frame(minWidth: 420, minHeight: 520)
                .onOpenURL { store.handleOpenURL($0) }
        }
        .windowResizability(.contentSize)
        .commands { CascadeCommands(store: store) }

        MenuBarExtra {
            MenuBarRoot()
                .environment(store)
        } label: {
            Image(systemName: store.snapshot.isPlaying ? "drop.fill" : "drop")
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView()
                .environment(store)
        }
    }
}

/// Top-level command menus: Cascade ▸ Toggle Playback (Space), and
/// Session ▸ Start <focus preset> (⇧⌘1, ⇧⌘2, …). Bound at the App scene so
/// they work no matter which window has focus.
struct CascadeCommands: Commands {
    let store: AppStore

    var body: some Commands {
        CommandGroup(replacing: .newItem) {} // hide File ▸ New
        CommandMenu("Session") {
            Button("Toggle Playback") { store.dispatch(.togglePlayback) }
                .keyboardShortcut(.space, modifiers: [])
            Divider()
            let presets = Array(store.snapshot.timerOptions.focusPresets.enumerated())
            ForEach(presets, id: \.element.minutes) { index, preset in
                Button("Start \(preset.label) Focus") {
                    store.dispatch(.startPomodoro(minutes: preset.minutes))
                }
                .keyboardShortcut(KeyEquivalent(Character("\(index + 1)")), modifiers: [.command, .shift])
            }
            Divider()
            Button("Cancel Timer") { store.dispatch(.cancelTimer) }
                .keyboardShortcut(".", modifiers: [.command])
        }
    }
}
