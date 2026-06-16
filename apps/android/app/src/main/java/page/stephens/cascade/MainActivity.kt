package page.stephens.cascade

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import page.stephens.cascade.audio.PlaybackController
import page.stephens.cascade.ui.CascadeScreen
import page.stephens.cascade.ui.CascadeTheme
import page.stephens.cascade.ui.CascadeViewModel

class MainActivity : ComponentActivity() {
    private lateinit var playback: PlaybackController

    private val viewModel: CascadeViewModel by viewModels {
        val app = application as CascadeApp
        CascadeViewModel.factory(app.bridgeHolder, app.syncManager)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        installSplashScreen()
        super.onCreate(savedInstanceState)
        // The app is dark-only (matching the web), so force transparent system
        // bars with light icons regardless of the device's light/dark setting.
        // The default enableEdgeToEdge() adapts to the *system* theme, which on a
        // light-mode phone gives a stark white nav bar and dark, unreadable
        // status-bar icons over our dark background.
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
        )

        val app = application as CascadeApp
        playback = PlaybackController(this, app.bridgeHolder)
        playback.start()
        handleAuthDeepLink(intent)

        setContent {
            CascadeTheme {
                CascadeScreen(viewModel)
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleAuthDeepLink(intent)
    }

    /** Complete a magic-link sign-in if we were opened via .../auth?token=... */
    private fun handleAuthDeepLink(intent: Intent?) {
        val token = intent?.data?.getQueryParameter("token") ?: return
        (application as CascadeApp).syncManager.completeSignIn(token)
    }

    override fun onStop() {
        // Flush recent listening before we risk being suspended.
        (application as CascadeApp).syncManager.flush()
        super.onStop()
    }

    override fun onDestroy() {
        playback.stop()
        super.onDestroy()
    }
}
