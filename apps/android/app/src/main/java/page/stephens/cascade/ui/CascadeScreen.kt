package page.stephens.cascade.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import page.stephens.cascade.core.TimerKind
import page.stephens.cascade.sync.SyncUiState

private val PillShape = RoundedCornerShape(percent = 50)
private val CardShape = RoundedCornerShape(14.dp)

@Composable
fun CascadeScreen(viewModel: CascadeViewModel) {
    val snapshot by viewModel.snapshot.collectAsStateWithLifecycle()
    val syncState by viewModel.syncState.collectAsStateWithLifecycle()

    Surface(modifier = Modifier.fillMaxSize(), color = CascadeColors.BgDeep) {
        Box(modifier = Modifier.fillMaxSize()) {
            WaterfallBackdrop(active = snapshot.isPlaying)
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .systemBarsPadding()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 24.dp, vertical = 16.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                // Mirror the web's centered 480px shell.
                Column(
                    modifier = Modifier.widthIn(max = 440.dp).fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Header(subtitle = snapshot.subtitle)
                    Spacer(Modifier.height(32.dp))
                    TimerReadout(
                        kind = snapshot.timer.kind,
                        label = snapshot.timer.remainingLabel,
                        progress = snapshot.timer.progress,
                    )
                    Spacer(Modifier.height(32.dp))
                    PlayButton(
                        isPlaying = snapshot.isPlaying,
                        onClick = viewModel::togglePlayback,
                    )
                    Spacer(Modifier.height(32.dp))
                    VolumeSlider(
                        percent = snapshot.volumePercent,
                        isMuted = snapshot.isMuted,
                        onChange = viewModel::setVolume,
                        onToggleMute = viewModel::toggleMute,
                    )
                    Spacer(Modifier.height(20.dp))
                    ListeningStats(
                        totalLabel = snapshot.listening.totalLabel,
                        trackingEnabled = snapshot.listening.trackingEnabled,
                        onToggleTracking = viewModel::setListeningTracking,
                    )
                    Spacer(Modifier.height(28.dp))
                    TimerControls(
                        activeKind = snapshot.timer.kind,
                        onStartPomodoro = viewModel::startPomodoro,
                        onStartSleep = viewModel::startSleepTimer,
                        onStartStopwatch = viewModel::startStopwatch,
                        onCancel = viewModel::cancelTimer,
                    )
                    if (syncState.available) {
                        Spacer(Modifier.height(28.dp))
                        AccountControls(
                            state = syncState,
                            onSignIn = viewModel::signIn,
                            onCompleteSignIn = viewModel::completeSignInFromLink,
                            onSignOut = viewModel::signOut,
                            onDeleteData = viewModel::deleteListeningData,
                            onDeleteAccount = viewModel::deleteAccount,
                        )
                    }
                    snapshot.errorMessage?.let {
                        Spacer(Modifier.height(12.dp))
                        Text(
                            text = it,
                            color = CascadeColors.Danger,
                            style = MaterialTheme.typography.bodyMedium,
                            textAlign = TextAlign.Center,
                        )
                    }
                    Spacer(Modifier.height(32.dp))
                    Footer()
                }
            }
        }
    }
}

@Composable
private fun Header(subtitle: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.Bottom,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        // The web mark is uppercased + tracked + accent-glow via CSS.
        Text(
            text = "CASCADE",
            style = TextStyle(
                color = CascadeColors.Accent,
                fontSize = 18.sp,
                fontWeight = FontWeight.SemiBold,
                letterSpacing = 0.18.em,
                shadow = Shadow(color = CascadeColors.AccentGlow, blurRadius = 24f),
            ),
        )
        Text(
            text = subtitle,
            fontSize = 13.sp,
            letterSpacing = 0.04.em,
            color = CascadeColors.InkDim,
        )
    }
}

@Composable
private fun Footer() {
    val uriHandler = LocalUriHandler.current
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "CASCADE · LOOPED",
            fontSize = 11.sp,
            letterSpacing = 0.18.em,
            color = CascadeColors.InkFaint,
        )
        Spacer(Modifier.width(14.dp))
        Text(
            text = "ARCHITECTURE",
            fontSize = 11.sp,
            letterSpacing = 0.18.em,
            color = CascadeColors.InkFaint,
            modifier = Modifier.clickable {
                uriHandler.openUri("https://cascade.stephens.page/architecture/")
            },
        )
    }
}

@Composable
private fun TimerReadout(kind: TimerKind, label: String, progress: Float) {
    AnimatedVisibility(visible = kind != TimerKind.OFF) {
        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = label.ifEmpty { "—" },
                style = TextStyle(
                    color = CascadeColors.Ink,
                    fontWeight = FontWeight.Light,
                    fontSize = 52.sp,
                    letterSpacing = 0.04.em,
                ),
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(8.dp))
            // Thin progress bar — same proportion language as the web readout.
            Box(
                modifier = Modifier
                    .fillMaxWidth(0.5f)
                    .height(2.dp)
                    .clip(RoundedCornerShape(1.dp))
                    .background(CascadeColors.PaperStrong),
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth(progress.coerceIn(0f, 1f))
                        .height(2.dp)
                        .background(CascadeColors.Accent),
                )
            }
        }
    }
}

@Composable
private fun PlayButton(isPlaying: Boolean, onClick: () -> Unit) {
    // The web play control is a dark disc inside glowing accent rings, not a
    // filled button. Recreate that: a dashed outer ring that rotates while
    // playing, an inner glow, and a thin accent rim around a faint-lit disc.
    val ringSpin = rememberInfiniteTransition(label = "ring")
    val angle by ringSpin.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(tween(60_000, easing = LinearEasing)),
        label = "angle",
    )
    val rim = if (isPlaying) CascadeColors.Accent else CascadeColors.Accent.copy(alpha = 0.35f)

    Box(
        modifier = Modifier
            .size(200.dp)
            .clip(CircleShape)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        // Dashed outer ring — rotates only while playing.
        Box(
            modifier = Modifier
                .matchParentSize()
                .rotate(if (isPlaying) angle else 0f)
                .drawBehind {
                    val sw = 1.dp.toPx()
                    drawCircle(
                        color = CascadeColors.Accent.copy(alpha = if (isPlaying) 0.28f else 0.12f),
                        radius = size.minDimension / 2f - sw,
                        style = Stroke(
                            width = sw,
                            pathEffect = PathEffect.dashPathEffect(floatArrayOf(6f, 14f)),
                        ),
                    )
                },
        )
        if (isPlaying) {
            Box(
                modifier = Modifier
                    .size(180.dp)
                    .background(
                        brush = Brush.radialGradient(
                            colors = listOf(CascadeColors.Accent.copy(alpha = 0.22f), Color.Transparent),
                        ),
                        shape = CircleShape,
                    ),
            )
        }
        Box(
            modifier = Modifier
                .size(172.dp)
                .clip(CircleShape)
                .background(
                    Brush.radialGradient(
                        colors = listOf(Color.White.copy(alpha = 0.08f), Color.Transparent),
                    ),
                )
                .border(1.dp, rim, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            PlayPauseGlyph(isPlaying = isPlaying, tint = CascadeColors.Ink)
        }
    }
}

@Composable
private fun PlayPauseGlyph(isPlaying: Boolean, tint: Color) {
    Canvas(modifier = Modifier.size(46.dp)) {
        if (isPlaying) {
            val barW = size.width * 0.24f
            val gap = size.width * 0.18f
            val h = size.height * 0.72f
            val top = (size.height - h) / 2f
            val corner = CornerRadius(barW * 0.25f)
            drawRoundRect(
                color = tint,
                topLeft = Offset(size.width / 2f - gap / 2f - barW, top),
                size = Size(barW, h),
                cornerRadius = corner,
            )
            drawRoundRect(
                color = tint,
                topLeft = Offset(size.width / 2f + gap / 2f, top),
                size = Size(barW, h),
                cornerRadius = corner,
            )
        } else {
            val w = size.width * 0.6f
            val h = size.height * 0.66f
            val left = (size.width - w) / 2f + size.width * 0.05f
            val top = (size.height - h) / 2f
            val path = androidx.compose.ui.graphics.Path().apply {
                moveTo(left, top)
                lineTo(left + w, size.height / 2f)
                lineTo(left, top + h)
                close()
            }
            drawPath(path, tint)
        }
    }
}

@Composable
private fun VolumeSlider(
    percent: Int,
    isMuted: Boolean,
    onChange: (Int) -> Unit,
    onToggleMute: () -> Unit,
) {
    Column(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Surface(
                    onClick = onToggleMute,
                    shape = CircleShape,
                    color = if (isMuted) CascadeColors.AccentSoft else Color.Transparent,
                    border = BorderStroke(1.dp, if (isMuted) CascadeColors.Accent else CascadeColors.PaperStrong),
                    modifier = Modifier.size(34.dp),
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Text(if (isMuted) "🔇" else "🔊", fontSize = 14.sp)
                    }
                }
                Spacer(Modifier.width(10.dp))
                SectionLabel("Volume")
            }
            Text(
                text = if (isMuted) "MUTED" else "$percent%",
                fontSize = 13.sp,
                letterSpacing = 0.05.em,
                color = CascadeColors.Ink,
            )
        }
        Slider(
            value = percent.toFloat(),
            onValueChange = { onChange(it.toInt()) },
            valueRange = 0f..100f,
            colors = SliderDefaults.colors(
                thumbColor = CascadeColors.Ink,
                activeTrackColor = CascadeColors.Accent,
                inactiveTrackColor = CascadeColors.PaperStrong,
            ),
        )
    }
}

@Composable
private fun ListeningStats(
    totalLabel: String,
    trackingEnabled: Boolean,
    onToggleTracking: (Boolean) -> Unit,
) {
    Surface(
        shape = CardShape,
        color = CascadeColors.Paper,
        border = BorderStroke(1.dp, CascadeColors.PaperStrong),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(18.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = totalLabel,
                fontSize = 26.sp,
                fontWeight = FontWeight.Light,
                letterSpacing = 0.02.em,
                color = if (trackingEnabled) CascadeColors.Ink else CascadeColors.InkFaint,
            )
            Spacer(Modifier.height(4.dp))
            SectionLabel(if (trackingEnabled) "Lifetime listening" else "Tracking paused")
            Spacer(Modifier.height(12.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Switch(
                    checked = trackingEnabled,
                    onCheckedChange = onToggleTracking,
                    colors = SwitchDefaults.colors(
                        checkedThumbColor = CascadeColors.BgDeep,
                        checkedTrackColor = CascadeColors.Accent,
                        checkedBorderColor = CascadeColors.Accent,
                    ),
                )
                Spacer(Modifier.width(10.dp))
                Text("Track listening", color = CascadeColors.InkDim, fontSize = 13.sp)
            }
        }
    }
}

@Composable
private fun TimerControls(
    activeKind: TimerKind,
    onStartPomodoro: (Int) -> Unit,
    onStartSleep: (Int) -> Unit,
    onStartStopwatch: () -> Unit,
    onCancel: () -> Unit,
) {
    Column(modifier = Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
        SectionLabel("Focus session")
        Spacer(Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(30, 60, 480).forEach { minutes ->
                val label = if (minutes < 60) "${minutes}m"
                            else if (minutes == 60) "1h"
                            else "${minutes / 60}h"
                PillChip(
                    text = label,
                    selected = activeKind == TimerKind.POMODORO,
                    onClick = { onStartPomodoro(minutes) },
                )
            }
        }
        Spacer(Modifier.height(18.dp))
        SectionLabel("Sleep timer")
        Spacer(Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(15, 30, 60).forEach { minutes ->
                PillChip(
                    text = "${minutes}m",
                    selected = activeKind == TimerKind.SLEEP,
                    onClick = { onStartSleep(minutes) },
                )
            }
        }

        Spacer(Modifier.height(18.dp))
        SectionLabel("Stopwatch")
        Spacer(Modifier.height(8.dp))
        PillChip(
            text = "Start stopwatch",
            selected = activeKind == TimerKind.STOPWATCH,
            onClick = onStartStopwatch,
        )

        Spacer(Modifier.height(18.dp))
        CustomDurationSection(onStartPomodoro = onStartPomodoro, onStartSleep = onStartSleep)

        if (activeKind != TimerKind.OFF) {
            Spacer(Modifier.height(14.dp))
            PillChip(text = "Cancel timer", selected = false, onClick = onCancel)
        }
    }
}

@Composable
private fun CustomDurationSection(
    onStartPomodoro: (Int) -> Unit,
    onStartSleep: (Int) -> Unit,
) {
    var minutesText by remember { mutableStateOf("45") }
    // false = Focus session, true = Sleep timer.
    var sleepMode by remember { mutableStateOf(false) }

    SectionLabel("Custom")
    Spacer(Modifier.height(8.dp))
    Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        PillChip(text = "Focus", selected = !sleepMode, onClick = { sleepMode = false })
        PillChip(text = "Sleep", selected = sleepMode, onClick = { sleepMode = true })
    }
    Spacer(Modifier.height(8.dp))
    Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        OutlinedTextField(
            value = minutesText,
            onValueChange = { new -> minutesText = new.filter { it.isDigit() }.take(4) },
            label = { Text("Minutes") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.size(width = 120.dp, height = 64.dp),
        )
        Button(
            shape = PillShape,
            onClick = {
                val m = minutesText.toIntOrNull() ?: return@Button
                if (m in 1..1440) {
                    if (sleepMode) onStartSleep(m) else onStartPomodoro(m)
                }
            },
        ) {
            Text(if (sleepMode) "Start sleep" else "Start focus")
        }
    }
}

@Composable
private fun AccountControls(
    state: SyncUiState,
    onSignIn: (String) -> Unit,
    onCompleteSignIn: (String) -> Unit,
    onSignOut: () -> Unit,
    onDeleteData: () -> Unit,
    onDeleteAccount: () -> Unit,
) {
    var email by remember { mutableStateOf("") }
    var link by remember { mutableStateOf("") }
    var showManage by remember { mutableStateOf(false) }
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        val account = state.account
        if (account != null) {
            SectionLabel("Syncing · ${account.email}")
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(onClick = onSignOut) { Text("Sign out") }
                TextButton(onClick = { showManage = !showManage }) { Text("Manage data") }
            }
            if (showManage) {
                TextButton(
                    onClick = onDeleteData,
                    enabled = !state.busy,
                ) { Text("Delete listening data", color = CascadeColors.Danger) }
                TextButton(
                    onClick = onDeleteAccount,
                    enabled = !state.busy,
                ) { Text("Delete account", color = CascadeColors.Danger) }
            }
        } else {
            SectionLabel("Sync across devices")
            Spacer(Modifier.height(6.dp))
            Text(
                "1. Email yourself a link.  2. Paste it back here to sign in.",
                fontSize = 13.sp,
                textAlign = TextAlign.Center,
                color = CascadeColors.InkDim,
            )
            Spacer(Modifier.height(10.dp))
            // Step 1 — request the magic link.
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = email,
                    onValueChange = { email = it },
                    label = { Text("you@example.com") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
                    modifier = Modifier.weight(1f),
                )
                Button(
                    shape = PillShape,
                    onClick = { if (email.isNotBlank()) onSignIn(email.trim()) },
                    enabled = !state.busy && email.isNotBlank(),
                ) { Text("Email link") }
            }
            Spacer(Modifier.height(8.dp))
            // Step 2 — paste the link from the email to finish.
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = link,
                    onValueChange = { link = it },
                    label = { Text("paste the sign-in link") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                    modifier = Modifier.weight(1f),
                )
                Button(
                    shape = PillShape,
                    onClick = { if (link.isNotBlank()) onCompleteSignIn(link.trim()) },
                    enabled = !state.busy && link.isNotBlank(),
                ) { Text("Sign in") }
            }
        }
        state.status?.let {
            Spacer(Modifier.height(8.dp))
            Text(
                it,
                fontSize = 13.sp,
                textAlign = TextAlign.Center,
                color = CascadeColors.InkDim,
            )
        }
    }
}

/** Tiny uppercase tracked label — the web's section-heading / caption type. */
@Composable
private fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text.uppercase(),
        modifier = modifier,
        color = CascadeColors.InkFaint,
        fontSize = 10.sp,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.22.em,
    )
}

/** Pill-shaped ghost chip matching the web `.chip--ghost` (accent when active). */
@Composable
private fun PillChip(text: String, selected: Boolean, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = PillShape,
        color = if (selected) CascadeColors.AccentSoft else Color.Transparent,
        border = BorderStroke(1.dp, if (selected) CascadeColors.Accent else CascadeColors.PaperStrong),
    ) {
        Text(
            text = text,
            color = if (selected) CascadeColors.Accent else CascadeColors.InkDim,
            fontSize = 13.sp,
            letterSpacing = 0.06.em,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 9.dp),
        )
    }
}

@Composable
private fun WaterfallBackdrop(active: Boolean) {
    // The web shell paints a radial accent glow at 50% 0% over a bg→bg-deep
    // wash, brighter while playing. Approximate it with a top-centered radial.
    val glow = if (active) 0.16f else 0.07f
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(CascadeColors.BgDeep)
            .drawBehind {
                drawRect(
                    brush = Brush.radialGradient(
                        colors = listOf(CascadeColors.Accent.copy(alpha = glow), Color.Transparent),
                        center = Offset(size.width / 2f, 0f),
                        radius = size.maxDimension * 0.7f,
                    ),
                )
            },
    )
}
