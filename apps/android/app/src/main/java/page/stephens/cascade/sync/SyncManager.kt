package page.stephens.cascade.sync

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.core.Command

data class SyncUiState(
    val available: Boolean,
    val account: Account?,
    val status: String?,
    val busy: Boolean,
)

/**
 * Owns the optional account and the listening-sync loop on Android. The core
 * stays pure: this reads `snapshot.listening` and decides when to talk to the
 * server, then feeds the result back via `ApplySyncedTotal`. Sync cadence lives
 * here (the shell), never in the core.
 */
class SyncManager(
    private val bridge: CascadeBridgeHolder,
    private val accountStore: AccountStore,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private val syncMutex = Mutex()

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
                    sync(account)
                }
            }
            // Threshold-driven sync: push once enough unsynced time has accrued.
            scope.launch {
                bridge.snapshot.collect { snap ->
                    val account = _state.value.account ?: return@collect
                    if (snap.listening.unsyncedMs >= SYNC_THRESHOLD_MS) sync(account)
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
                sync(account)
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
                accountStore.rotateDeviceId()
                bridge.dispatch(Command.ResetListeningData)
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
                accountStore.rotateDeviceId()
                bridge.dispatch(Command.ResetListeningData)
                accountStore.clearAccount()
                _state.value = _state.value.copy(account = null, busy = false, status = "Account deleted.")
            } catch (_: Exception) {
                _state.value = _state.value.copy(busy = false, status = "Couldn't delete the account.")
            }
        }
    }

    /** Push this device's slot and fold the server aggregate back into the core. */
    private suspend fun sync(account: Account) {
        if (!syncMutex.tryLock()) return
        try {
            val deviceTotalMs = bridge.snapshot.value.listening.deviceTotalMs
            val deviceId = accountStore.deviceId()
            val res = SyncApi.putListening(account.sessionToken, deviceId, deviceTotalMs)
            bridge.dispatch(
                Command.ApplySyncedTotal(
                    syncedThroughMs = deviceTotalMs,
                    serverTotalMs = res.serverTotalMs,
                ),
            )
        } catch (e: SyncException) {
            if (e.status == 401) {
                accountStore.clearAccount()
                _state.value = _state.value.copy(account = null, status = "Signed out — sign in again to sync.")
            }
        } catch (_: Exception) {
            // Offline / transient — try again on the next trigger.
        } finally {
            syncMutex.unlock()
        }
    }

    /** Flush recent listening, e.g. when the activity is going to the background. */
    fun flush() {
        val account = _state.value.account ?: return
        scope.launch { sync(account) }
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

    companion object {
        private const val SYNC_THRESHOLD_MS = 30_000L
    }
}
