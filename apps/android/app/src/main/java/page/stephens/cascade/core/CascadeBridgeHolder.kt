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

/**
 * Owns the single `CascadeBridge` (the UniFFI handle to `cascade-core`) and
 * exposes its current [Snapshot] as a [StateFlow] the UI can collect.
 *
 * The bridge itself is thread-safe (Rust `Mutex` inside), so we can dispatch
 * from any coroutine context.
 */
class CascadeBridgeHolder(private val settingsStore: SettingsStore) {
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

    /** Synchronous dispatch. Returns immediately with the new snapshot;
     *  effect handlers receive the same effects via [effects]. */
    fun dispatch(command: Command) {
        val commandJson = cascadeJson.encodeToString(Command.serializer(), command)
        val updateJson = bridge.dispatch(commandJson)
        val update = cascadeJson.decodeFromString<Update>(updateJson)
        _snapshot.value = update.snapshot
        if (update.effects.isNotEmpty()) {
            _effects.value = update.effects
            // Persist any settings effect immediately — DataStore handles its
            // own coalescing, so flooding it on every slider tick is fine.
            for (effect in update.effects) {
                if (effect is Effect.PersistSettings) {
                    scope.launch { settingsStore.write(effect.json) }
                }
            }
        }
    }
}
