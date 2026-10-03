package page.stephens.cascade.core

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.ReceiveChannel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import page.stephens.cascade.settings.SettingsStore
import page.stephens.cascade.sync.AccountStore
import uniffi.cascade_uniffi.CascadeBridge
import java.util.UUID

/**
 * Owns the single `CascadeBridge` (the UniFFI handle to `cascade-core`) and
 * exposes its current [Snapshot] as a [StateFlow] the UI can collect.
 *
 * The bridge itself is thread-safe (Rust `Mutex` inside), so we can dispatch
 * from any coroutine context.
 *
 * [accountStore] holds the core's account blob, and the device id older builds
 * stored in the shell. That id is offered to the core once, at restore, so an
 * existing server slot carries over; the core owns the id from then on.
 */
class CascadeBridgeHolder(
    private val settingsStore: SettingsStore,
    private val accountStore: AccountStore,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /** Persist writes, run one at a time in dispatch order. A later blob must
     *  never be overwritten by an earlier one — in particular the single write
     *  that rotates the device id and zeroes the slot (ResetListeningData) must
     *  not be undone by a tick's write landing after it. */
    private val writes = Channel<suspend () -> Unit>(Channel.UNLIMITED).also { queue ->
        scope.launch { for (write in queue) write() }
    }
    private val dispatchLock = Any()

    /** Request effects from every dispatch, in the order the core produced
     *  them, for the one request carrier (SyncManager). A channel rather than a
     *  StateFlow, so none is overwritten or dropped: each is received, and so
     *  settled, exactly once — whether a tick, a playback report, a UI command
     *  or a settle produced it. */
    private val _requests = Channel<Effect>(Channel.UNLIMITED)
    val requests: ReceiveChannel<Effect> = _requests

    private val bridge = CascadeBridge()

    /** Boot is one step: restore the persisted blobs (empty = none) and the
     *  fallback device id in a single dispatch. Blocks once during app startup;
     *  the reads are local DataStore IO and complete in single-digit ms, and
     *  doing this async would just mean the first render shows defaults. */
    private val boot: Update = run {
        val restore = runBlocking {
            Command.Restore(
                settingsJson = settingsStore.read().orEmpty(),
                listeningJson = settingsStore.readListening().orEmpty(),
                fallbackDeviceId = accountStore.legacyDeviceId() ?: UUID.randomUUID().toString(),
                accountJson = accountStore.read(),
            )
        }
        send(restore)
    }

    private val _snapshot = MutableStateFlow(boot.snapshot)
    val snapshot: StateFlow<Snapshot> = _snapshot.asStateFlow()

    /** Playback effects from the most recent dispatch that had any —
     *  PlaybackController collects this. */
    private val _effects = MutableStateFlow(forHandlers(boot.effects))
    val effects: StateFlow<List<Effect>> = _effects.asStateFlow()

    /** Synchronous dispatch. Updates the snapshot, hands playback effects to
     *  the handlers via [effects] and request effects to the carrier via
     *  [requests]. */
    fun dispatch(command: Command) {
        // Locked so writes and requests are queued in the same order the core
        // produced them, even when dispatches race in from different threads.
        synchronized(dispatchLock) {
            val update = send(command)
            _snapshot.value = update.snapshot
            val handled = forHandlers(update.effects)
            if (handled.isNotEmpty()) _effects.value = handled
        }
    }

    /** Run [command] through the core and queue its persist and request effects. */
    private fun send(command: Command): Update {
        val commandJson = cascadeJson.encodeToString(Command.serializer(), command)
        val update = cascadeJson.decodeFromString<Update>(bridge.dispatch(commandJson))
        // Persist immediately — DataStore handles its own coalescing, so
        // flooding it on every slider tick is fine.
        for (effect in update.effects) {
            when (effect) {
                is Effect.PersistSettings -> writes.trySend { settingsStore.write(effect.json) }
                is Effect.PersistListening -> writes.trySend { settingsStore.writeListening(effect.json) }
                is Effect.PersistAccount -> writes.trySend { accountStore.write(effect.json) }
                else -> if (effect.isRequest) _requests.trySend(effect)
            }
        }
        return update
    }

    /** Only what the effect handlers act on. [effects] is a StateFlow and keeps
     *  just the latest value, so a dispatch with only persist or request
     *  effects (a tick's push, a sync settle) must not replace a StartPlayback
     *  before it's applied. Persist effects are written and request effects
     *  queued in [send]. */
    private fun forHandlers(effects: List<Effect>): List<Effect> = effects.filter {
        when (it) {
            is Effect.PersistSettings, is Effect.PersistListening, is Effect.PersistAccount -> false
            else -> !it.isRequest
        }
    }

    /** An HTTP request for the carrier to run and settle. */
    private val Effect.isRequest: Boolean
        get() = when (this) {
            is Effect.PushListening,
            is Effect.SendSignInLink, is Effect.VerifySignInToken, is Effect.RevokeSession,
            is Effect.DeleteServerListening, is Effect.DeleteServerAccount -> true
            else -> false
        }
}
