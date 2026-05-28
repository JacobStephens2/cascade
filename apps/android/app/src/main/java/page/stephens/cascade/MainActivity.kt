package page.stephens.cascade

import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
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
        CascadeViewModel.factory(app.bridgeHolder)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        installSplashScreen()
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val app = application as CascadeApp
        playback = PlaybackController(this, app.bridgeHolder)
        playback.start()

        setContent {
            CascadeTheme {
                CascadeScreen(viewModel)
            }
        }
    }

    override fun onDestroy() {
        playback.stop()
        super.onDestroy()
    }
}
