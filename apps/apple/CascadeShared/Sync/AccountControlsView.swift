import SwiftUI

/// Optional account controls for syncing listening time across devices. Renders
/// nothing when no sync backend is configured. Shared by the macOS and iOS UIs.
struct AccountControlsView: View {
    @Environment(AppStore.self) private var store
    @State private var email = ""
    @State private var link = ""

    var body: some View {
        if store.syncAvailable {
            VStack(alignment: .leading, spacing: 8) {
                Text("SYNC ACROSS DEVICES")
                    .font(.caption)
                    .foregroundStyle(CascadeTheme.inkDim)
                    .tracking(2)

                if let account = store.account {
                    Text(account.email).font(.callout)
                    HStack(spacing: 12) {
                        Button("Sign out") { Task { await store.signOut() } }
                        Button("Delete data") { Task { await store.deleteListeningData() } }
                        Button("Delete account", role: .destructive) { Task { await store.deleteAccount() } }
                    }
                    .buttonStyle(.borderless)
                } else {
                    HStack {
                        TextField("you@example.com", text: $email)
                            .textFieldStyle(.roundedBorder)
                            .frame(maxWidth: 220)
                        Button("Email me a link") {
                            Task { await store.signIn(email: email.trimmingCharacters(in: .whitespaces)) }
                        }
                    }
                    HStack {
                        TextField("paste the sign-in link", text: $link)
                            .textFieldStyle(.roundedBorder)
                            .frame(maxWidth: 220)
                        Button("Sign in") {
                            Task {
                                await store.completeSignIn(fromLinkOrToken: link)
                                link = ""
                            }
                        }
                    }
                }

                if let status = store.syncStatus {
                    Text(status).font(.caption).foregroundStyle(CascadeTheme.inkDim)
                }
            }
        }
    }
}
