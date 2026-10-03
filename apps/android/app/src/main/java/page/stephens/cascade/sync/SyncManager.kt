package page.stephens.cascade.sync

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.core.Command
import page.stephens.cascade.core.Effect
import page.stephens.cascade.core.RequestEffect
import page.stephens.cascade.core.SyncReason
import java.util.UUID
import kotlin.coroutines.cancellation.CancellationException

/**
 * Carries the account's and listening sync's requests on Android. The core
 * holds the account and decides what to send — the threshold, the in-flight guard, the
 * device id, the session, the status copy and the 401 rule all live there.
 * This is the one request carrier: it takes every request effect any dispatch
 * produced (a tick's threshold push included) from the bridge, carries it over
 * HTTP, and settles it with its outcome command. The shell decides only when
 * it can talk (lifecycle triggers) and what the user asked for.
 */
class SyncManager(private val bridge: CascadeBridgeHolder) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    init {
        // Always drained, even without a sync server, so a request (say, a
        // tick's push for a stored session) is still settled and never leaves
        // the core's in-flight guard set.
        scope.launch {
            for (request in bridge.requests) carry(request)
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
    fun deleteListeningData() = dispatch(Command.DeleteListeningData)
    fun deleteAccount() = dispatch(Command.DeleteAccount)

    /** Flush recent listening, e.g. when the activity is going to the background. */
    fun flush() = dispatch(Command.BeginListeningSync(SyncReason.FLUSH))

    private fun dispatch(command: Command) {
        if (syncAvailable) bridge.dispatch(command)
    }

    /**
     * Carry one request [effect] over HTTP and settle it with the core. A
     * settle's own answer comes back through [CascadeBridgeHolder.requests]
     * like any other (a sign-in answers with a refresh push). Every request
     * but `RevokeSession` must be settled, or the core won't start another.
     */
    private fun carry(effect: RequestEffect) {
        when (effect) {
            is Effect.PushListening -> carryRequest(
                call = {
                    SyncApi.putListening(effect.sessionToken, effect.deviceId, effect.deviceTotalMs)
                },
                succeeded = { Command.ListeningSyncSucceeded(serverTotalMs = it.serverTotalMs) },
                failed = { Command.ListeningSyncFailed(unauthorized = it) },
            )
            is Effect.SendSignInLink -> carryRequest(
                call = { SyncApi.requestLink(effect.email) },
                succeeded = { Command.SignInLinkSent },
                failed = { Command.AccountRequestFailed(unauthorized = it) },
            )
            is Effect.VerifySignInToken -> carryRequest(
                call = { SyncApi.verify(effect.token) },
                succeeded = { Command.SignInVerified(sessionToken = it.sessionToken, email = it.email) },
                failed = { Command.AccountRequestFailed(unauthorized = it) },
            )
            // Already gone server-side or offline — local sign-out stands.
            is Effect.RevokeSession -> if (syncAvailable) {
                scope.launch { runCatching { SyncApi.logout(effect.sessionToken) } }
            }
            is Effect.DeleteServerListening -> carryRequest(
                call = { SyncApi.deleteListening(effect.sessionToken) },
                succeeded = { Command.ListeningDataDeleted(newDeviceId = UUID.randomUUID().toString()) },
                failed = { Command.AccountRequestFailed(unauthorized = it) },
            )
            is Effect.DeleteServerAccount -> carryRequest(
                call = { SyncApi.deleteAccount(effect.sessionToken) },
                succeeded = { Command.AccountDeleted(newDeviceId = UUID.randomUUID().toString()) },
                failed = { Command.AccountRequestFailed(unauthorized = it) },
            )
        }
    }

    /**
     * Run one request and settle it with the command its result maps to (an
     * HTTP 401 is `unauthorized`). Without a sync server the request fails
     * unsent, so it is still settled.
     */
    private fun <T> carryRequest(
        call: suspend () -> T,
        succeeded: (T) -> Command,
        failed: (unauthorized: Boolean) -> Command,
    ) {
        if (!syncAvailable) {
            bridge.dispatch(failed(false))
            return
        }
        scope.launch {
            val outcome = try {
                succeeded(call())
            } catch (e: Throwable) {
                // Every request must be settled, or the core never starts
                // another — including on cancellation, which is rethrown after.
                bridge.dispatch(failed(e is SyncException && e.status == 401))
                if (e is CancellationException) throw e
                return@launch
            }
            bridge.dispatch(outcome)
        }
    }
}
