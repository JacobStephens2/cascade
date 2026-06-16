package page.stephens.cascade.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/**
 * The web shell's design tokens (apps/web/src/styles.css `:root`), mirrored so
 * the native UI reads as the same product. Material's ColorScheme has no slot
 * for the dim/faint ink ramp or the translucent "paper" surfaces, so they live
 * here and are used directly, the way the web uses CSS custom properties.
 */
object CascadeColors {
    val Bg = Color(0xFF050B10)          // --bg
    val BgDeep = Color(0xFF02060A)      // --bg-deep
    val Ink = Color(0xFFE6F1F6)         // --ink
    val InkDim = Color(0xFF8AA3B0)      // --ink-dim
    val InkFaint = Color(0xFF4F6B78)    // --ink-faint
    val Accent = Color(0xFF6EC9E2)      // --accent
    val AccentGlow = Color(0x736EC9E2)  // --accent-glow  (~0.45)
    val AccentSoft = Color(0x1F6EC9E2)  // --accent-soft  (~0.12)
    val Paper = Color(0x0AFFFFFF)       // --paper        (white 0.04)
    val PaperStrong = Color(0x12FFFFFF) // --paper-strong (white 0.07)
    val Danger = Color(0xFFFF8A72)      // --danger
}

// The web app is dark-only (color-scheme: dark); match it rather than tracking
// the system light/dark setting, so the brand reads identically everywhere.
private val DarkColors = darkColorScheme(
    primary = CascadeColors.Accent,
    onPrimary = Color(0xFF03161E),          // --chip--primary text
    primaryContainer = Color(0xFF0B2530),
    onPrimaryContainer = CascadeColors.Ink,
    background = CascadeColors.Bg,
    onBackground = CascadeColors.Ink,
    surface = CascadeColors.BgDeep,
    onSurface = CascadeColors.Ink,
    surfaceVariant = Color(0xFF0E2029),
    onSurfaceVariant = CascadeColors.InkDim,
    outline = CascadeColors.InkFaint,
    error = CascadeColors.Danger,
)

@Composable
fun CascadeTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = DarkColors, content = content)
}
