package page.stephens.cascade.ui

import androidx.compose.runtime.getValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import page.stephens.cascade.core.CascadeBridgeHolder
import page.stephens.cascade.core.Command
import page.stephens.cascade.core.Snapshot
import page.stephens.cascade.core.TimerKind

class CascadeViewModel(
    private val bridge: CascadeBridgeHolder,
) : ViewModel() {
    val snapshot: StateFlow<Snapshot> = bridge.snapshot

    private var tickJob: Job? = null

    init {
        // Mirror the web hook: only tick while a timer is running. The Rust
        // core never reads the system clock; it relies on these ticks.
        viewModelScope.launch {
            bridge.snapshot.collect { snap ->
                val active = snap.timer.kind == TimerKind.SLEEP || snap.timer.kind == TimerKind.POMODORO
                if (active && tickJob == null) startTicking()
                if (!active && tickJob != null) stopTicking()
            }
        }
    }

    fun togglePlayback() = bridge.dispatch(Command.TogglePlayback)
    fun setVolume(percent: Int) = bridge.dispatch(Command.SetVolume(percent.coerceIn(0, 100)))
    fun toggleMute() = bridge.dispatch(Command.ToggleMute)
    fun startSleepTimer(minutes: Int) = bridge.dispatch(Command.StartSleepTimer(minutes))
    fun startPomodoro(minutes: Int) = bridge.dispatch(Command.StartPomodoro(minutes))
    fun cancelTimer() = bridge.dispatch(Command.CancelTimer)

    private fun startTicking() {
        tickJob = viewModelScope.launch {
            var last = System.currentTimeMillis()
            while (true) {
                delay(TICK_INTERVAL_MS)
                val now = System.currentTimeMillis()
                bridge.dispatch(Command.Tick(elapsedMs = now - last))
                last = now
            }
        }
    }

    private fun stopTicking() {
        tickJob?.cancel()
        tickJob = null
    }

    companion object {
        private const val TICK_INTERVAL_MS = 250L

        fun factory(bridge: CascadeBridgeHolder) = object : ViewModelProvider.Factory {
            override fun <T : ViewModel> create(modelClass: Class<T>): T {
                @Suppress("UNCHECKED_CAST")
                return CascadeViewModel(bridge) as T
            }
        }
    }
}
