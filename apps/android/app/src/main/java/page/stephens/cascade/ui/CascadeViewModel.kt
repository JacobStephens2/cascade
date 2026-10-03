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
import page.stephens.cascade.sync.SyncManager
import page.stephens.cascade.sync.SyncUiState

class CascadeViewModel(
    private val bridge: CascadeBridgeHolder,
    private val syncManager: SyncManager,
) : ViewModel() {
    val snapshot: StateFlow<Snapshot> = bridge.snapshot
    val syncState: StateFlow<SyncUiState> = syncManager.state

    private var tickJob: Job? = null
    private var tickInterval = 0L

    init {
        // Tick at the cadence the core asks for (0 = stop). The Rust core never
        // reads the clock; it relies on these ticks.
        viewModelScope.launch {
            bridge.snapshot.collect { snap ->
                val want = snap.tickIntervalMs
                if (want != tickInterval) {
                    stopTicking()
                    if (want > 0L) startTicking(want)
                }
            }
        }
    }

    fun togglePlayback() { bridge.dispatch(Command.TogglePlayback) }
    fun setVolume(percent: Int) { bridge.dispatch(Command.SetVolume(percent.coerceIn(0, 100))) }
    fun toggleMute() { bridge.dispatch(Command.ToggleMute) }
    fun startSleepTimer(minutes: Int) { bridge.dispatch(Command.StartSleepTimer(minutes)) }
    fun startPomodoro(minutes: Int) { bridge.dispatch(Command.StartPomodoro(minutes)) }
    fun startStopwatch() { bridge.dispatch(Command.StartStopwatch) }
    fun cancelTimer() { bridge.dispatch(Command.CancelTimer) }
    fun setListeningTracking(enabled: Boolean) { bridge.dispatch(Command.SetListeningTracking(enabled)) }

    fun signIn(email: String) = syncManager.signIn(email)
    fun completeSignInFromLink(input: String) = syncManager.completeSignInFromLink(input)
    fun signOut() = syncManager.signOut()
    fun deleteListeningData() = syncManager.deleteData()
    fun deleteAccount() = syncManager.deleteAccount()

    private fun startTicking(intervalMs: Long) {
        tickInterval = intervalMs
        tickJob = viewModelScope.launch {
            var last = System.currentTimeMillis()
            while (true) {
                delay(intervalMs)
                val now = System.currentTimeMillis()
                bridge.dispatch(Command.Tick(elapsedMs = now - last))
                last = now
            }
        }
    }

    private fun stopTicking() {
        tickJob?.cancel()
        tickJob = null
        tickInterval = 0L
    }

    companion object {
        fun factory(bridge: CascadeBridgeHolder, syncManager: SyncManager) =
            object : ViewModelProvider.Factory {
                override fun <T : ViewModel> create(modelClass: Class<T>): T {
                    @Suppress("UNCHECKED_CAST")
                    return CascadeViewModel(bridge, syncManager) as T
                }
            }
    }
}
