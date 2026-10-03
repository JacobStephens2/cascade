package page.stephens.cascade.sync

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.core.Command
import page.stephens.cascade.core.Effect
import page.stephens.cascade.core.SyncReason
import java.util.UUID
import kotlin.coroutines.cancellation.CancellationException

/**
 * The account and listening-sync transport on Android. The core holds the
 * account and decides what to send — the threshold, the in-flight guard, the
 * device id, the session, the status copy and the 401 rule all live there.
 * This dispatches the account commands and `BeginListeningSync`, carries each
 * request effect it is handed over HTTP, and settles it with its outcome
 * command. The shell decides only when it can talk (lifecycle triggers).
 */
class SyncManager(private val bridge: CascadeBridgeHolder) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    init {
        if (syncAvailable) {
            // On launch, fetch the cross-device total straight away. The core
            // sends nothing while signed out, and answers a fresh sign-in with
            // its own refresh.
            sync(SyncReason.REFRESH)
            // Routine check on every snapshot; the core only answers with a
            // push once enough unsynced time has accrued.
            scope.launch {
                bridge.snapshot.collect { sync(SyncReason.THRESHOLD) }
            }
        }
    }

    fun requestSignInLink(email: String) = dispatchAndCarry(Command.RequestSignInLink(email))

    /** Finish a magic-link sign-in from an opened or pasted link, or a bare token. */
    fun submitSignInLink(input: String) = dispatchAndCarry(Command.SubmitSignInLink(input))

    fun signOut() = dispatchAndCarry(Command.SignOut)
    fun deleteListeningData() = dispatchAndCarry(Command.DeleteListeningData)
    fun deleteAccount() = dispatchAndCarry(Command.DeleteAccount)

    /** Flush recent listening, e.g. when the activity is going to the background. */
    fun flush() = sync(SyncReason.FLUSH)

    /** Ask the core whether there is anything to send; it answers with a push or nothing. */
    private fun sync(reason: SyncReason) = dispatchAndCarry(Command.BeginListeningSync(reason))

    private fun dispatchAndCarry(command: Command) {
        if (syncAvailable) carry(bridge.dispatch(command))
    }

    /**
     * Carry every request effect in [effects] over HTTP and settle it with the
     * core. A settle's own answer is carried too (a sign-in answers with a
     * refresh push). Every request but `RevokeSession` must be settled, or the
     * core won't start another.
     */
    private fun carry(effects: List<Effect>) {
        for (effect in effects) {
            when (effect) {
                is Effect.PushListening -> settle(
                    call = {
                        SyncApi.putListening(effect.sessionToken, effect.deviceId, effect.deviceTotalMs)
                    },
                    succeeded = { Command.ListeningSyncSucceeded(serverTotalMs = it.serverTotalMs) },
                    failed = { Command.ListeningSyncFailed(unauthorized = it) },
                )
                is Effect.SendSignInLink -> settle(
                    call = { SyncApi.requestLink(effect.email) },
                    succeeded = { Command.SignInLinkSent },
                    failed = ::accountRequestFailed,
                )
                is Effect.VerifySignInToken -> settle(
                    call = { SyncApi.verify(effect.token) },
                    succeeded = { Command.SignInVerified(sessionToken = it.sessionToken, email = it.email) },
                    failed = ::accountRequestFailed,
                )
                // Already gone server-side or offline — local sign-out stands.
                is Effect.RevokeSession -> scope.launch { runCatching { SyncApi.logout(effect.sessionToken) } }
                is Effect.DeleteServerListening -> settle(
                    call = { SyncApi.deleteListening(effect.sessionToken) },
                    succeeded = { Command.ListeningDataDeleted(newDeviceId = UUID.randomUUID().toString()) },
                    failed = ::accountRequestFailed,
                )
                is Effect.DeleteServerAccount -> settle(
                    call = { SyncApi.deleteAccount(effect.sessionToken) },
                    succeeded = { Command.AccountDeleted(newDeviceId = UUID.randomUUID().toString()) },
                    failed = ::accountRequestFailed,
                )
                else -> {}
            }
        }
    }

    /**
     * Run one request and settle it with the command its result maps to (an
     * HTTP 401 is `unauthorized`), then carry what the core answers.
     */
    private fun <T> settle(
        call: suspend () -> T,
        succeeded: (T) -> Command,
        failed: (unauthorized: Boolean) -> Command,
    ) {
        scope.launch {
            val outcome = try {
                succeeded(call())
            } catch (e: Throwable) {
                // Every request must be settled, or the core never starts
                // another — including on cancellation, which is rethrown after.
                val settled = bridge.dispatch(failed(e is SyncException && e.status == 401))
                if (e is CancellationException) throw e
                carry(settled)
                return@launch
            }
            carry(bridge.dispatch(outcome))
        }
    }

    private fun accountRequestFailed(unauthorized: Boolean) = Command.AccountRequestFailed(unauthorized)
}
