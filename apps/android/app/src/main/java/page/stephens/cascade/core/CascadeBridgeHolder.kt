package page.stephens.cascade.core

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import page.stephens.cascade.settings.SettingsStore
import uniffi.cascade_uniffi.CascadeBridge
import java.util.UUID

/**
 * Owns the single `CascadeBridge` (the UniFFI handle to `cascade-core`) and
 * exposes its current [Snapshot] as a [StateFlow] the UI can collect.
 *
 * The bridge itself is thread-safe (Rust `Mutex` inside), so we can dispatch
 * from any coroutine context.
 *
 * [legacyDeviceId] reads the device id older builds stored in the shell. It is
 * offered to the core once, at restore, so an existing server slot carries over;
 * the core owns the id from then on.
 */
class CascadeBridgeHolder(
    private val settingsStore: SettingsStore,
    private val legacyDeviceId: suspend () -> String?,
) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val bridge: CascadeBridge = run {
        // Block once during app startup to load persisted settings. The reads
        // are local DataStore IO and complete in single-digit ms; doing this
        // async would just mean the first render shows defaults.
        val json = runBlocking { settingsStore.read() }
        if (json.isNullOrEmpty()) CascadeBridge() else CascadeBridge.restoreOrNew(json)
    }

    private val _snapshot = MutableStateFlow(cascadeJson.decodeFromString<Snapshot>(bridge.snapshot()))
    val snapshot: StateFlow<Snapshot> = _snapshot.asStateFlow()

    /** Latest effects emitted by the most recent dispatch — consumers
     *  (PlaybackController, settings persister) collect this. */
    private val _effects = MutableStateFlow<List<Effect>>(emptyList())
    val effects: StateFlow<List<Effect>> = _effects.asStateFlow()

    init {
        // Restore the listening ledger once at startup — always, even with no
        // blob, because the core owns the device id and needs a fallback to
        // adopt when the blob has none. The core ignores a missing/incompatible
        // blob and never lets a restore lower the counter. (Declared after
        // [_effects]: the restore can emit PersistListening.)
        val listeningJson = runBlocking { settingsStore.readListening() }.orEmpty()
        val fallbackDeviceId = runBlocking { legacyDeviceId() } ?: UUID.randomUUID().toString()
        dispatch(Command.RestoreListening(listeningJson, fallbackDeviceId))
    }

    /** Synchronous dispatch. Updates the snapshot and returns this update's
     *  effects, so a caller can act on the ones it asked for (e.g. the sync
     *  loop reading PushListening). Effect handlers also receive them via
     *  [effects]. */
    fun dispatch(command: Command): List<Effect> {
        val commandJson = cascadeJson.encodeToString(Command.serializer(), command)
        val updateJson = bridge.dispatch(commandJson)
        val update = cascadeJson.decodeFromString<Update>(updateJson)
        _snapshot.value = update.snapshot
        if (update.effects.isNotEmpty()) {
            _effects.value = update.effects
            // Persist any settings effect immediately — DataStore handles its
            // own coalescing, so flooding it on every slider tick is fine.
            for (effect in update.effects) {
                when (effect) {
                    is Effect.PersistSettings -> scope.launch { settingsStore.write(effect.json) }
                    is Effect.PersistListening -> scope.launch { settingsStore.writeListening(effect.json) }
                    else -> {}
                }
            }
        }
        return update.effects
    }
}
