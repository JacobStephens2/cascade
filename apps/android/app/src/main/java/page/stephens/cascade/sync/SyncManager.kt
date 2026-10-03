package page.stephens.cascade.sync

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.core.Command
import page.stephens.cascade.core.SyncReason
import java.util.UUID

/**
 * Carries the account's and listening sync's requests on Android. The core
 * holds the account and decides what to send — the request table, the
 * threshold, the in-flight guard, the device id, the session, the status copy
 * and the 401 rule all live there. This routes the bridge's request channel
 * to the HTTP adapter: every [page.stephens.cascade.core.Effect.ServerRequest]
 * any dispatch produced (a tick's threshold push included) goes to
 * [ServerHttp], which settles it with its [Command.ServerResponse]. The
 * shell decides only when it can talk (lifecycle triggers) and what the user
 * asked for.
 */
class SyncManager(private val bridge: CascadeBridgeHolder) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val server = ServerHttp(SYNC_API_BASE)

    init {
        // Always drained, even without a sync server, so a request (say, a
        // tick's push for a stored session) is still settled — with status 0,
        // unsent — and never leaves the core's in-flight guard set. A
        // settle's own answer comes back through the same channel (a sign-in
        // answers with a refresh push).
        scope.launch {
            for (request in bridge.requests) server.carry(request, scope, bridge::dispatch)
        }
        if (syncAvailable) {
            // On launch, fetch the cross-device total straight away. The core
            // sends nothing while signed out, and answers a fresh sign-in with
            // its own refresh.
            dispatch(Command.BeginListeningSync(SyncReason.REFRESH))
        }
    }

    fun requestSignInLink(email: String) = dispatch(Command.RequestSignInLink(email))

    /** Finish a magic-link sign-in from an opened or pasted link, or a bare token. */
    fun submitSignInLink(input: String) = dispatch(Command.SubmitSignInLink(input))

    fun signOut() = dispatch(Command.SignOut)
    fun deleteListeningData() = dispatch(Command.DeleteListeningData(newDeviceId()))
    fun deleteAccount() = dispatch(Command.DeleteAccount(newDeviceId()))

    /** Flush recent listening, e.g. when the activity is going to the background. */
    fun flush() = dispatch(Command.BeginListeningSync(SyncReason.FLUSH))

    private fun dispatch(command: Command) {
        if (syncAvailable) bridge.dispatch(command)
    }

    /** The id the core rotates to once a delete is confirmed. */
    private fun newDeviceId() = UUID.randomUUID().toString()
}
