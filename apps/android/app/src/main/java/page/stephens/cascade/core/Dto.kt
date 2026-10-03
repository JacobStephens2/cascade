@file:OptIn(kotlinx.serialization.ExperimentalSerializationApi::class)

package page.stephens.cascade.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonClassDiscriminator
import kotlinx.serialization.json.Json

/**
 * Mirrors the serde-tagged enums in `cascade-core`. The Rust crate is the
 * source of truth — these classes only describe the JSON wire shape so we
 * can talk to it in a typed way.
 */
val cascadeJson = Json {
    ignoreUnknownKeys = true
    classDiscriminator = "type"
    encodeDefaults = true
}

@Serializable
@JsonClassDiscriminator("type")
sealed class Command {
    @Serializable @SerialName("play") data object Play : Command()
    @Serializable @SerialName("pause") data object Pause : Command()
    @Serializable @SerialName("togglePlayback") data object TogglePlayback : Command()
    @Serializable @SerialName("setVolume") data class SetVolume(val percent: Int) : Command()
    @Serializable @SerialName("toggleMute") data object ToggleMute : Command()
    @Serializable @SerialName("startSleepTimer") data class StartSleepTimer(val minutes: Int) : Command()
    @Serializable @SerialName("startPomodoro") data class StartPomodoro(val minutes: Int) : Command()
    @Serializable @SerialName("startStopwatch") data object StartStopwatch : Command()
    @Serializable @SerialName("cancelTimer") data object CancelTimer : Command()
    @Serializable @SerialName("tick") data class Tick(val elapsedMs: Long) : Command()
    @Serializable @SerialName("platformPlaybackStarted") data object PlatformPlaybackStarted : Command()
    @Serializable @SerialName("platformPlaybackPaused") data object PlatformPlaybackPaused : Command()
    @Serializable @SerialName("platformPlaybackError") data class PlatformPlaybackError(val message: String) : Command()
    @Serializable @SerialName("setListeningTracking") data class SetListeningTracking(val enabled: Boolean) : Command()
    @Serializable @SerialName("restoreListening") data class RestoreListening(val json: String, val fallbackDeviceId: String) : Command()
    @Serializable @SerialName("beginListeningSync") data class BeginListeningSync(val reason: SyncReason) : Command()
    @Serializable @SerialName("listeningSyncSucceeded") data class ListeningSyncSucceeded(val serverTotalMs: Long) : Command()
    @Serializable @SerialName("listeningSyncFailed") data class ListeningSyncFailed(val unauthorized: Boolean) : Command()
    @Serializable @SerialName("resetListeningData") data class ResetListeningData(val newDeviceId: String) : Command()
}

/** Why the shell is asking to sync; the core decides whether it's worth a PUT. */
@Serializable
enum class SyncReason {
    @SerialName("threshold") THRESHOLD,
    @SerialName("flush") FLUSH,
    @SerialName("refresh") REFRESH,
}

@Serializable
@JsonClassDiscriminator("type")
sealed class Effect {
    @Serializable @SerialName("startPlayback") data class StartPlayback(val volumePercent: Int) : Effect()
    @Serializable @SerialName("pausePlayback") data object PausePlayback : Effect()
    @Serializable @SerialName("setPlatformVolume") data class SetPlatformVolume(val volumePercent: Int) : Effect()
    @Serializable @SerialName("persistSettings") data class PersistSettings(val json: String) : Effect()
    @Serializable @SerialName("persistListening") data class PersistListening(val json: String) : Effect()
    @Serializable @SerialName("pushListening") data class PushListening(val deviceId: String, val deviceTotalMs: Long) : Effect()
    @Serializable @SerialName("clearSession") data object ClearSession : Effect()
}

@Serializable
enum class TimerKind {
    @SerialName("off") OFF,
    @SerialName("sleep") SLEEP,
    @SerialName("pomodoro") POMODORO,
    @SerialName("stopwatch") STOPWATCH,
    @SerialName("justCompleted") JUST_COMPLETED,
}

@Serializable
data class TimerSnapshot(
    val kind: TimerKind,
    val remainingLabel: String,
    val remainingMs: Long,
    val totalMs: Long,
    val progress: Float,
)

@Serializable
data class ListeningSnapshot(
    val trackingEnabled: Boolean,
    val deviceTotalMs: Long,
    val displayedTotalMs: Long,
    val unsyncedMs: Long,
    val totalLabel: String,
)

@Serializable
data class Snapshot(
    val title: String,
    val subtitle: String,
    val isPlaying: Boolean,
    val volumePercent: Int,
    val isMuted: Boolean,
    val primaryButtonLabel: String,
    val timer: TimerSnapshot,
    val errorMessage: String? = null,
    val listening: ListeningSnapshot,
    /** How often to send [Command.Tick], in ms; 0 means stop. The core owns the cadence. */
    val tickIntervalMs: Long,
)

@Serializable
data class Update(
    val snapshot: Snapshot,
    val effects: List<Effect>,
)
