import SwiftUI

/// Optional account controls for syncing listening time across devices. Renders
/// nothing when no sync backend is configured. Shared by the macOS and iOS UIs.
/// The core holds the account; this renders its snapshot and sends its commands.
struct AccountControlsView: View {
    @Environment(AppStore.self) private var store
    @State private var email = ""
    @State private var link = ""

    var body: some View {
        if store.syncAvailable {
            let account = store.snapshot.account
            VStack(alignment: .leading, spacing: 8) {
                Text("SYNC ACROSS DEVICES")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .tracking(2)

                if let signedInLabel = account.signedInLabel {
                    Text(signedInLabel).font(.callout)
                    HStack(spacing: 12) {
                        Button("Sign out") { store.dispatch(.signOut) }
                        Button("Delete data") { store.dispatch(.deleteListeningData(newDeviceId: UUID().uuidString)) }
                            .disabled(account.busy)
                        Button("Delete account", role: .destructive) { store.dispatch(.deleteAccount(newDeviceId: UUID().uuidString)) }
                            .disabled(account.busy)
                    }
                    .buttonStyle(.borderless)
                } else {
                    HStack {
                        TextField("", text: $email, prompt: Text("you@example.com").foregroundColor(.gray))
                            .textFieldStyle(.roundedBorder)
                            .foregroundStyle(.primary)
                            .frame(maxWidth: 220)
                        Button("Email me a link") { store.dispatch(.requestSignInLink(email: email)) }
                            .disabled(account.busy)
                    }
                    HStack {
                        TextField("", text: $link, prompt: Text("paste the sign-in link").foregroundColor(.gray))
                            .textFieldStyle(.roundedBorder)
                            .foregroundStyle(.primary)
                            .frame(maxWidth: 220)
                        Button("Sign in") {
                            store.dispatch(.submitSignInLink(input: link))
                            link = ""
                        }
                        .disabled(account.busy)
                    }
                }

                if let status = account.statusLabel {
                    Text(status).font(.caption).foregroundStyle(.secondary)
                }
            }
        }
    }
}
