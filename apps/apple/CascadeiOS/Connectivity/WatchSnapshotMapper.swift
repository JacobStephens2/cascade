import Foundation

/// Maps a full `Snapshot` to the watch-friendly wire format. The core already
/// formats the status line, so the watch never has to localize or do date math.
enum WatchSnapshotMapper {
    static func map(_ snapshot: Snapshot) -> PhoneSnapshotForWatch {
        return PhoneSnapshotForWatch(
            isPlaying: snapshot.isPlaying,
            volumePercent: snapshot.volumePercent,
            isMuted: snapshot.isMuted,
            statusLine: snapshot.timer.statusLabel,
            timerProgress: max(0, min(1, snapshot.timer.progress)),
            timerRemainingLabel: snapshot.timer.remainingLabel,
            isTimerActive: snapshot.timer.isActive,
            focusPresets: snapshot.timerOptions.focusPresets.map {
                WatchTimerPreset(minutes: $0.minutes, label: $0.label)
            },
            minMinutes: snapshot.timerOptions.minMinutes,
            maxMinutes: snapshot.timerOptions.maxMinutes,
            customFocusMinutes: snapshot.timerOptions.customFocusMinutes
        )
    }
}
