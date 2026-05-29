package page.stephens.cascade.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AssistChip
import androidx.compose.material3.AssistChipDefaults
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import page.stephens.cascade.core.TimerKind

@Composable
fun CascadeScreen(viewModel: CascadeViewModel) {
    val snapshot by viewModel.snapshot.collectAsStateWithLifecycle()

    Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Box(modifier = Modifier.fillMaxSize()) {
            WaterfallBackdrop(active = snapshot.isPlaying, progress = snapshot.timer.progress)
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .systemBarsPadding()
                    .padding(horizontal = 24.dp, vertical = 16.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Header(subtitle = snapshot.subtitle)
                Spacer(Modifier.height(24.dp))
                TimerReadout(
                    kind = snapshot.timer.kind,
                    label = snapshot.timer.remainingLabel,
                    progress = snapshot.timer.progress,
                )
                Spacer(Modifier.weight(1f))
                PlayButton(
                    isPlaying = snapshot.isPlaying,
                    label = snapshot.primaryButtonLabel,
                    onClick = viewModel::togglePlayback,
                )
                Spacer(Modifier.height(32.dp))
                VolumeSlider(
                    percent = snapshot.volumePercent,
                    isMuted = snapshot.isMuted,
                    onChange = viewModel::setVolume,
                    onToggleMute = viewModel::toggleMute,
                )
                Spacer(Modifier.height(24.dp))
                TimerControls(
                    activeKind = snapshot.timer.kind,
                    onStartPomodoro = viewModel::startPomodoro,
                    onStartSleep = viewModel::startSleepTimer,
                    onCancel = viewModel::cancelTimer,
                )
                Spacer(Modifier.height(12.dp))
                snapshot.errorMessage?.let {
                    Text(
                        text = it,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            }
        }
    }
}

@Composable
private fun Header(subtitle: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            text = "Cascade",
            style = MaterialTheme.typography.titleLarge.copy(fontWeight = FontWeight.SemiBold),
        )
        Text(
            text = subtitle,
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.6f),
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
                style = MaterialTheme.typography.displayMedium.copy(
                    fontWeight = FontWeight.Light,
                    fontSize = 56.sp,
                ),
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(8.dp))
            // Thin progress bar — same proportion language as the web backdrop.
            Box(
                modifier = Modifier
                    .fillMaxWidth(0.5f)
                    .height(2.dp)
                    .clip(RoundedCornerShape(1.dp))
                    .background(MaterialTheme.colorScheme.onBackground.copy(alpha = 0.15f)),
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth(progress.coerceIn(0f, 1f))
                        .height(2.dp)
                        .background(MaterialTheme.colorScheme.primary),
                )
            }
        }
    }
}

@Composable
private fun PlayButton(isPlaying: Boolean, label: String, onClick: () -> Unit) {
    val container = if (isPlaying)
        MaterialTheme.colorScheme.primaryContainer
    else
        MaterialTheme.colorScheme.primary

    Surface(
        modifier = Modifier
            .size(168.dp)
            .shadow(elevation = 24.dp, shape = CircleShape, clip = false),
        shape = CircleShape,
        color = container,
        onClick = onClick,
    ) {
        Box(contentAlignment = Alignment.Center) {
            Text(
                text = label,
                style = MaterialTheme.typography.headlineMedium.copy(fontWeight = FontWeight.Medium),
                color = if (isPlaying) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onPrimary,
            )
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
                TextButton(
                    onClick = onToggleMute,
                    contentPadding = PaddingValues(horizontal = 8.dp),
                ) {
                    Text(if (isMuted) "🔇" else "🔊")
                }
                Text("Volume", style = MaterialTheme.typography.labelLarge)
            }
            Text(
                if (isMuted) "Muted" else "$percent%",
                style = MaterialTheme.typography.labelLarge,
            )
        }
        Slider(
            value = percent.toFloat(),
            onValueChange = { onChange(it.toInt()) },
            valueRange = 0f..100f,
            steps = 0,
            colors = SliderDefaults.colors(
                thumbColor = MaterialTheme.colorScheme.primary,
                activeTrackColor = MaterialTheme.colorScheme.primary,
            ),
        )
    }
}

@Composable
private fun TimerControls(
    activeKind: TimerKind,
    onStartPomodoro: (Int) -> Unit,
    onStartSleep: (Int) -> Unit,
    onCancel: () -> Unit,
) {
    Column(modifier = Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
        // Pomodoro chips.
        Text("Focus session", style = MaterialTheme.typography.labelMedium)
        Spacer(Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(30, 60, 480).forEach { minutes ->
                val label = if (minutes < 60) "${minutes}m"
                            else if (minutes == 60) "1h"
                            else "${minutes / 60}h"
                FilterChip(
                    selected = activeKind == TimerKind.POMODORO,
                    onClick = { onStartPomodoro(minutes) },
                    label = { Text(label) },
                    colors = FilterChipDefaults.filterChipColors(),
                )
            }
        }
        Spacer(Modifier.height(16.dp))
        Text("Sleep timer", style = MaterialTheme.typography.labelMedium)
        Spacer(Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(15, 30, 60).forEach { minutes ->
                FilterChip(
                    selected = activeKind == TimerKind.SLEEP,
                    onClick = { onStartSleep(minutes) },
                    label = { Text("${minutes}m") },
                )
            }
        }

        Spacer(Modifier.height(16.dp))
        CustomDurationSection(onStartPomodoro = onStartPomodoro, onStartSleep = onStartSleep)

        if (activeKind != TimerKind.OFF) {
            Spacer(Modifier.height(12.dp))
            TextButton(onClick = onCancel) {
                Text("Cancel timer")
            }
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

    Text("Custom", style = MaterialTheme.typography.labelMedium)
    Spacer(Modifier.height(8.dp))
    Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        FilterChip(
            selected = !sleepMode,
            onClick = { sleepMode = false },
            label = { Text("Focus") },
        )
        FilterChip(
            selected = sleepMode,
            onClick = { sleepMode = true },
            label = { Text("Sleep") },
        )
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
private fun WaterfallBackdrop(active: Boolean, progress: Float) {
    // Lightweight static gradient that intensifies when playing. The web app
    // does a fancier animated waterfall — leaving that as a polish pass.
    val intensity = if (active) 1f else 0.35f
    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(
                Brush.verticalGradient(
                    colors = listOf(
                        MaterialTheme.colorScheme.background,
                        MaterialTheme.colorScheme.primary.copy(alpha = 0.18f * intensity),
                        MaterialTheme.colorScheme.background,
                    ),
                ),
            ),
    )
}
