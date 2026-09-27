package page.stephens.cascade.sync

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.core.Command
import page.stephens.cascade.core.Effect
import page.stephens.cascade.core.SyncReason
import java.util.UUID
import kotlin.coroutines.cancellation.CancellationException

data class SyncUiState(
    val available: Boolean,
    val account: Account?,
    val status: String?,
    val busy: Boolean,
)

/**
 * Owns the optional account and the HTTP transport for listening sync on
 * Android. The shell decides when it can talk (signed in, lifecycle triggers);
 * the core decides whether there is anything to say, and what — the threshold,
 * the in-flight guard, the device id and the 401 rule all live there. This asks
 * with `BeginListeningSync`, PUTs exactly the `PushListening` it gets back, and
 * settles every begun sync with `ListeningSyncSucceeded`/`ListeningSyncFailed`.
 */
class SyncManager(
    private val bridge: CascadeBridgeHolder,
    private val accountStore: AccountStore,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _state = MutableStateFlow(
        SyncUiState(available = syncAvailable, account = null, status = null, busy = false),
    )
    val state: StateFlow<SyncUiState> = _state.asStateFlow()

    init {
        if (syncAvailable) {
            scope.launch {
                val account = accountStore.readAccount()
                if (account != null) {
                    _state.value = _state.value.copy(account = account)
                    sync(account, SyncReason.REFRESH)
                }
            }
            // Routine check on every snapshot; the core only answers with a
            // push once enough unsynced time has accrued.
            scope.launch {
                bridge.snapshot.collect {
                    val account = _state.value.account ?: return@collect
                    sync(account, SyncReason.THRESHOLD)
                }
            }
        }
    }

    fun signIn(email: String) {
        _state.value = _state.value.copy(busy = true, status = null)
        scope.launch {
            try {
                SyncApi.requestLink(email)
                _state.value = _state.value.copy(busy = false, status = "Check $email for a sign-in link.")
            } catch (_: Exception) {
                _state.value = _state.value.copy(busy = false, status = "Couldn't send the sign-in link.")
            }
        }
    }

    /** Complete a magic-link sign-in from a deep-link token. */
    fun completeSignIn(token: String) {
        _state.value = _state.value.copy(busy = true, status = "Signing in…")
        scope.launch {
            try {
                val res = SyncApi.verify(token)
                val account = Account(res.sessionToken, res.email)
                accountStore.writeAccount(account)
                _state.value = _state.value.copy(account = account, busy = false, status = "Signed in as ${res.email}.")
                sync(account, SyncReason.REFRESH)
            } catch (_: Exception) {
                _state.value = _state.value.copy(busy = false, status = "That sign-in link was invalid or expired.")
            }
        }
    }

    /** Complete sign-in from a pasted link (…/auth?token=XYZ) or a raw token. */
    fun completeSignInFromLink(input: String) {
        val token = extractToken(input)
        if (token == null) {
            _state.value = _state.value.copy(status = "Paste the full sign-in link.")
            return
        }
        completeSignIn(token)
    }

    fun signOut() {
        val account = _state.value.account
        _state.value = _state.value.copy(account = null, status = null)
        scope.launch {
            accountStore.clearAccount()
            if (account != null) runCatching { SyncApi.logout(account.sessionToken) }
        }
    }

    fun deleteData() {
        val account = _state.value.account ?: return
        _state.value = _state.value.copy(busy = true)
        scope.launch {
            try {
                SyncApi.deleteListening(account.sessionToken)
                // One dispatch zeroes the slot and rotates the id atomically.
                bridge.dispatch(Command.ResetListeningData(newDeviceId = UUID.randomUUID().toString()))
                _state.value = _state.value.copy(busy = false, status = "Listening data deleted.")
            } catch (_: Exception) {
                _state.value = _state.value.copy(busy = false, status = "Couldn't delete listening data.")
            }
        }
    }

    fun deleteAccount() {
        val account = _state.value.account ?: return
        _state.value = _state.value.copy(busy = true)
        scope.launch {
            try {
                SyncApi.deleteAccount(account.sessionToken)
                bridge.dispatch(Command.ResetListeningData(newDeviceId = UUID.randomUUID().toString()))
                accountStore.clearAccount()
                _state.value = _state.value.copy(account = null, busy = false, status = "Account deleted.")
            } catch (_: Exception) {
                _state.value = _state.value.copy(busy = false, status = "Couldn't delete the account.")
            }
        }
    }

    /**
     * Ask the core whether there is anything to send; if so, PUT exactly what it
     * handed back and report the outcome. No effect means nothing to send, or a
     * sync is already in flight.
     */
    private suspend fun sync(account: Account, reason: SyncReason) {
        val push = bridge.dispatch(Command.BeginListeningSync(reason))
            .filterIsInstance<Effect.PushListening>()
            .firstOrNull() ?: return
        val res = try {
            SyncApi.putListening(account.sessionToken, push.deviceId, push.deviceTotalMs)
        } catch (e: Throwable) {
            // Every begun sync must be settled, or the core never starts
            // another — including on cancellation, which is rethrown after.
            val unauthorized = e is SyncException && e.status == 401
            val effects = bridge.dispatch(Command.ListeningSyncFailed(unauthorized = unauthorized))
            if (e is CancellationException) throw e
            if (effects.any { it is Effect.ClearSession }) {
                accountStore.clearAccount()
                _state.value = _state.value.copy(account = null, status = "Signed out — sign in again to sync.")
            }
            // Otherwise offline / transient — try again on the next trigger.
            return
        }
        bridge.dispatch(Command.ListeningSyncSucceeded(serverTotalMs = res.serverTotalMs))
    }

    /** Flush recent listening, e.g. when the activity is going to the background. */
    fun flush() {
        val account = _state.value.account ?: return
        scope.launch { sync(account, SyncReason.FLUSH) }
    }

    /** Pull the token out of a pasted sign-in URL, or accept a raw token. */
    private fun extractToken(input: String): String? {
        val s = input.trim()
        if (s.isEmpty()) return null
        val idx = s.indexOf("token=")
        if (idx >= 0) {
            val rest = s.substring(idx + "token=".length)
            val amp = rest.indexOf('&')
            return if (amp >= 0) rest.substring(0, amp) else rest
        }
        return s
    }
}
