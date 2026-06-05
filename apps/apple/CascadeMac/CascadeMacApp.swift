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
/// Session ▸ Start 30/60 min / 8 hr. Bound at the App scene so they work
/// no matter which window has focus.
struct CascadeCommands: Commands {
    let store: AppStore

    var body: some Commands {
        CommandGroup(replacing: .newItem) {} // hide File ▸ New
        CommandMenu("Session") {
            Button("Toggle Playback") { store.dispatch(.togglePlayback) }
                .keyboardShortcut(.space, modifiers: [])
            Divider()
            Button("Start 30 min Focus") { store.dispatch(.startPomodoro(minutes: 30)) }
                .keyboardShortcut("1", modifiers: [.command, .shift])
            Button("Start 60 min Focus") { store.dispatch(.startPomodoro(minutes: 60)) }
                .keyboardShortcut("2", modifiers: [.command, .shift])
            Button("Start 8 hr Focus") { store.dispatch(.startPomodoro(minutes: 480)) }
                .keyboardShortcut("3", modifiers: [.command, .shift])
            Divider()
            Button("Cancel Timer") { store.dispatch(.cancelTimer) }
                .keyboardShortcut(".", modifiers: [.command])
        }
    }
}
