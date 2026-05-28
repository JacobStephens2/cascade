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
    @Serializable @SerialName("startSleepTimer") data class StartSleepTimer(val minutes: Int) : Command()
    @Serializable @SerialName("startPomodoro") data class StartPomodoro(val minutes: Int) : Command()
    @Serializable @SerialName("cancelTimer") data object CancelTimer : Command()
    @Serializable @SerialName("tick") data class Tick(val elapsedMs: Long) : Command()
    @Serializable @SerialName("platformPlaybackStarted") data object PlatformPlaybackStarted : Command()
    @Serializable @SerialName("platformPlaybackPaused") data object PlatformPlaybackPaused : Command()
    @Serializable @SerialName("platformPlaybackError") data class PlatformPlaybackError(val message: String) : Command()
}

@Serializable
@JsonClassDiscriminator("type")
sealed class Effect {
    @Serializable @SerialName("startPlayback") data class StartPlayback(val volumePercent: Int) : Effect()
    @Serializable @SerialName("pausePlayback") data object PausePlayback : Effect()
    @Serializable @SerialName("setPlatformVolume") data class SetPlatformVolume(val volumePercent: Int) : Effect()
    @Serializable @SerialName("persistSettings") data class PersistSettings(val json: String) : Effect()
}

@Serializable
enum class TimerKind {
    @SerialName("off") OFF,
    @SerialName("sleep") SLEEP,
    @SerialName("pomodoro") POMODORO,
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
data class Snapshot(
    val title: String,
    val subtitle: String,
    val isPlaying: Boolean,
    val volumePercent: Int,
    val primaryButtonLabel: String,
    val timer: TimerSnapshot,
    val errorMessage: String? = null,
)

@Serializable
data class Update(
    val snapshot: Snapshot,
    val effects: List<Effect>,
)
