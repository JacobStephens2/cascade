package page.stephens.cascade.audio

import android.content.Intent
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService

/**
 * Hosts the Media3 ExoPlayer + MediaSession. Lives in its own process-bound
 * service so playback continues when the activity is backgrounded and shows
 * media controls on the lock screen / system shade.
 *
 * The service intentionally knows nothing about timers, fades, or volume
 * curves — that lives in the Rust core. The UI tells the service what to do
 * via [androidx.media3.session.MediaController]; everything else is a thin
 * wrapper around ExoPlayer state.
 */
class CascadePlaybackService : MediaSessionService() {
    private var mediaSession: MediaSession? = null

    override fun onCreate() {
        super.onCreate()
        val player = ExoPlayer.Builder(this).build().apply {
            // The 13-minute waterfall loop is bundled as an asset; play it on repeat.
            setMediaItem(MediaItem.fromUri("asset:///sounds/waterfall.ogg"))
            repeatMode = Player.REPEAT_MODE_ONE
            playWhenReady = false
            prepare()
        }
        mediaSession = MediaSession.Builder(this, player).build()
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? =
        mediaSession

    override fun onTaskRemoved(rootIntent: Intent?) {
        val player = mediaSession?.player ?: return super.onTaskRemoved(rootIntent)
        // Match the iOS / web behavior: if the user swipes the app away while
        // it's paused, kill the service. While playing, keep going.
        if (!player.playWhenReady) stopSelf()
    }

    override fun onDestroy() {
        mediaSession?.run {
            player.release()
            release()
        }
        mediaSession = null
        super.onDestroy()
    }
}
