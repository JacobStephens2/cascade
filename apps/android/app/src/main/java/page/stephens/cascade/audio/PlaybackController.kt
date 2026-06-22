package page.stephens.cascade.audio

import android.content.ComponentName
import android.content.Context
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.google.common.util.concurrent.MoreExecutors
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.core.Command
import page.stephens.cascade.core.Effect
import page.stephens.cascade.core.Snapshot
import page.stephens.cascade.core.TimerKind

/**
 * Translates the Rust core's [Effect]s into Media3 commands.
 *
 * The Rust core says "start playback at 42% volume" — this class is the
 * thing that maps that to `mediaController.volume = 0.42f; mediaController.play()`.
 *
 * Volume curve note: the web shell applies a perceptual square-law curve
 * to its slider. We do the same here so both platforms feel identical at
 * the same percentage value.
 */
class PlaybackController(
    context: Context,
    private val bridge: CascadeBridgeHolder,
) {
    private val appContext = context.applicationContext
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    private var controller: MediaController? = null
    private var collectJob: Job? = null
    private var metadataJob: Job? = null
    // Last artist line pushed to the session, so we only refresh the lock-screen
    // metadata when the text actually changes (the snapshot ticks far more often
    // than the displayed mm:ss does).
    private var lastArtist: String? = null

    // Report *confirmed* playback back to the core. Listening time accrues on
    // these signals, not on intent, so an audio-focus loss or a failed start
    // never counts as listening.
    private val playerListener = object : Player.Listener {
        override fun onIsPlayingChanged(isPlaying: Boolean) {
            bridge.dispatch(
                if (isPlaying) Command.PlatformPlaybackStarted else Command.PlatformPlaybackPaused,
            )
        }

        override fun onPlayerError(error: PlaybackException) {
            bridge.dispatch(Command.PlatformPlaybackError(error.message ?: "playback error"))
        }
    }

    fun start() {
        val sessionToken = SessionToken(
            appContext,
            ComponentName(appContext, CascadePlaybackService::class.java),
        )
        val future = MediaController.Builder(appContext, sessionToken).buildAsync()
        future.addListener({
            controller = future.get().also { it.addListener(playerListener) }
            // Drain any effects we missed before the controller existed.
            applyCurrentState()
            collectJob = scope.launch {
                bridge.effects.collectLatest { applyEffects(it) }
            }
            // Keep the lock-screen now-playing text in sync with the core: the
            // track name and, while a timer runs, its remaining time.
            metadataJob = scope.launch {
                bridge.snapshot.collect { updateNowPlaying(it) }
            }
        }, MoreExecutors.directExecutor())
    }

    fun stop() {
        collectJob?.cancel()
        collectJob = null
        metadataJob?.cancel()
        metadataJob = null
        lastArtist = null
        controller?.removeListener(playerListener)
        controller?.release()
        controller = null
    }

    /**
     * Refresh the session's now-playing metadata from the snapshot. The artist
     * line carries the track name plus the live timer, so it shows on the lock
     * screen / media shade. Replacing the item with the same URI updates the
     * metadata without interrupting the loop.
     */
    private fun updateNowPlaying(snap: Snapshot) {
        val c = controller ?: return
        val artist = when (snap.timer.kind) {
            TimerKind.OFF -> snap.subtitle
            TimerKind.STOPWATCH -> "${snap.subtitle} · ${snap.timer.remainingLabel}"
            else -> "${snap.subtitle} · ${snap.timer.remainingLabel} left"
        }
        if (artist == lastArtist) return
        lastArtist = artist
        c.replaceMediaItem(0, CascadePlaybackService.nowPlayingItem(artist))
    }

    /** Replay the snapshot's intent so a freshly-built controller catches up. */
    private fun applyCurrentState() {
        val snap = bridge.snapshot.value
        val c = controller ?: return
        c.volume = perceptualVolume(snap.volumePercent)
        if (snap.isPlaying && !c.isPlaying) c.play() else if (!snap.isPlaying && c.isPlaying) c.pause()
    }

    private suspend fun applyEffects(effects: List<Effect>) = withContext(Dispatchers.Main) {
        val c = controller ?: return@withContext
        for (effect in effects) {
            when (effect) {
                is Effect.StartPlayback -> {
                    c.volume = perceptualVolume(effect.volumePercent)
                    c.play()
                }
                Effect.PausePlayback -> c.pause()
                is Effect.SetPlatformVolume -> c.volume = perceptualVolume(effect.volumePercent)
                is Effect.PersistSettings -> { /* handled by CascadeBridgeHolder */ }
                is Effect.PersistListening -> { /* handled by CascadeBridgeHolder */ }
            }
        }
    }

    private fun perceptualVolume(percent: Int): Float {
        val clamped = percent.coerceIn(0, 100) / 100f
        return clamped * clamped
    }
}
