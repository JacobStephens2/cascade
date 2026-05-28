package page.stephens.cascade.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

// Match the web shell's deep-teal-into-foam palette.
private val CascadeMidnight = Color(0xFF0B1A24)
private val CascadeDeep = Color(0xFF112E3F)
private val CascadeMist = Color(0xFFB6DDE8)
private val CascadeFoam = Color(0xFFEDF6F9)
private val CascadeAccent = Color(0xFF7CC7D6)
private val CascadeError = Color(0xFFE07A5F)

private val DarkColors = darkColorScheme(
    primary = CascadeAccent,
    onPrimary = CascadeMidnight,
    primaryContainer = CascadeDeep,
    onPrimaryContainer = CascadeFoam,
    background = CascadeMidnight,
    onBackground = CascadeFoam,
    surface = CascadeDeep,
    onSurface = CascadeFoam,
    surfaceVariant = Color(0xFF1A3D52),
    onSurfaceVariant = CascadeMist,
    error = CascadeError,
)

private val LightColors = lightColorScheme(
    primary = Color(0xFF1E607A),
    onPrimary = Color.White,
    primaryContainer = CascadeFoam,
    onPrimaryContainer = CascadeMidnight,
    background = Color(0xFFF7FBFD),
    onBackground = CascadeMidnight,
    surface = Color.White,
    onSurface = CascadeMidnight,
    surfaceVariant = Color(0xFFD8E9EF),
    onSurfaceVariant = Color(0xFF335968),
)

@Composable
fun CascadeTheme(content: @Composable () -> Unit) {
    val colors = if (isSystemInDarkTheme()) DarkColors else LightColors
    MaterialTheme(colorScheme = colors, content = content)
}
