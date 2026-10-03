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
    @Serializable @SerialName("restore") data class Restore(
        val settingsJson: String,
        val listeningJson: String,
        val fallbackDeviceId: String,
        /** The stored account blob, verbatim; `""` for none. Always sent: the core holds the account. */
        val accountJson: String,
    ) : Command()
    @Serializable @SerialName("beginListeningSync") data class BeginListeningSync(val reason: SyncReason) : Command()
    @Serializable @SerialName("listeningSyncSucceeded") data class ListeningSyncSucceeded(val serverTotalMs: Long) : Command()
    @Serializable @SerialName("listeningSyncFailed") data class ListeningSyncFailed(val unauthorized: Boolean) : Command()
    @Serializable @SerialName("resetListeningData") data class ResetListeningData(val newDeviceId: String) : Command()
    @Serializable @SerialName("requestSignInLink") data class RequestSignInLink(val email: String) : Command()
    @Serializable @SerialName("submitSignInLink") data class SubmitSignInLink(val input: String) : Command()
    @Serializable @SerialName("signOut") data object SignOut : Command()
    @Serializable @SerialName("deleteListeningData") data object DeleteListeningData : Command()
    @Serializable @SerialName("deleteAccount") data object DeleteAccount : Command()
    @Serializable @SerialName("signInLinkSent") data object SignInLinkSent : Command()
    @Serializable @SerialName("signInVerified") data class SignInVerified(val sessionToken: String, val email: String) : Command()
    @Serializable @SerialName("listeningDataDeleted") data class ListeningDataDeleted(val newDeviceId: String) : Command()
    @Serializable @SerialName("accountDeleted") data class AccountDeleted(val newDeviceId: String) : Command()
    @Serializable @SerialName("accountRequestFailed") data class AccountRequestFailed(val unauthorized: Boolean) : Command()
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
    /** [gain] is the final output level, 0–1, mute and curve already applied. */
    @Serializable @SerialName("startPlayback") data class StartPlayback(val gain: Float) : Effect()
    @Serializable @SerialName("pausePlayback") data object PausePlayback : Effect()
    @Serializable @SerialName("setPlatformVolume") data class SetPlatformVolume(val gain: Float) : Effect()
    @Serializable @SerialName("persistSettings") data class PersistSettings(val json: String) : Effect()
    @Serializable @SerialName("persistListening") data class PersistListening(val json: String) : Effect()
    /** [sessionToken] is null only for a shell that keeps its own account; never for this one. */
    @Serializable @SerialName("pushListening") data class PushListening(
        val deviceId: String,
        val deviceTotalMs: Long,
        val sessionToken: String? = null,
    ) : Effect()
    @Serializable @SerialName("sendSignInLink") data class SendSignInLink(val email: String) : Effect()
    @Serializable @SerialName("verifySignInToken") data class VerifySignInToken(val token: String) : Effect()
    /** Fire-and-forget: no settle command. */
    @Serializable @SerialName("revokeSession") data class RevokeSession(val sessionToken: String) : Effect()
    @Serializable @SerialName("deleteServerListening") data class DeleteServerListening(val sessionToken: String) : Effect()
    @Serializable @SerialName("deleteServerAccount") data class DeleteServerAccount(val sessionToken: String) : Effect()
    /** Store verbatim; `""` means delete the stored account. */
    @Serializable @SerialName("persistAccount") data class PersistAccount(val json: String) : Effect()
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
    /** A timer is running (something to cancel); false when off or just completed. */
    val isActive: Boolean,
    /** One-line status, e.g. "Playing · 12:34 left". */
    val statusLabel: String,
)

/** A ready-made timer length the core offers. */
@Serializable
data class TimerPreset(
    val minutes: Int,
    val label: String,
    val shortLabel: String,
)

/** The timer choices the core offers: presets, limits and the custom pre-fill. */
@Serializable
data class TimerOptions(
    val focusPresets: List<TimerPreset>,
    val sleepPresets: List<TimerPreset>,
    val minMinutes: Int,
    val maxMinutes: Int,
    val customFocusMinutes: Int,
    val customSleepMinutes: Int,
)

@Serializable
data class ListeningSnapshot(
    val trackingEnabled: Boolean,
    val deviceTotalMs: Long,
    val displayedTotalMs: Long,
    val unsyncedMs: Long,
    val totalLabel: String,
)

/** The account view. Never carries the session token. */
@Serializable
data class AccountSnapshot(
    /** Null when signed out. */
    val email: String? = null,
    /** "Syncing · {email}"; null when signed out. */
    val signedInLabel: String? = null,
    /** What the user was last told; null when there is nothing to say. */
    val statusLabel: String? = null,
    /** An account request is out; every account control but sign-out waits. */
    val busy: Boolean,
)

@Serializable
data class Snapshot(
    val title: String,
    val subtitle: String,
    val isPlaying: Boolean,
    val volumePercent: Int,
    val isMuted: Boolean,
    /** Gain to output right now, 0–1: 0 while muted, else the curve applied. */
    val outputGain: Float,
    val primaryButtonLabel: String,
    val timer: TimerSnapshot,
    val timerOptions: TimerOptions,
    val errorMessage: String? = null,
    val listening: ListeningSnapshot,
    val account: AccountSnapshot,
    /** How often to send [Command.Tick], in ms; 0 means stop. The core owns the cadence. */
    val tickIntervalMs: Long,
)

@Serializable
data class Update(
    val snapshot: Snapshot,
    val effects: List<Effect>,
)
