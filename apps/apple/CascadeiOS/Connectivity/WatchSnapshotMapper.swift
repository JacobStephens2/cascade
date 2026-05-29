import Foundation

/// Maps a full `Snapshot` to the watch-friendly wire format. Pre-formats the
/// status line so the watch never has to localize or do date math.
enum WatchSnapshotMapper {
    static func map(_ snapshot: Snapshot) -> PhoneSnapshotForWatch {
        let statusLine: String
        switch snapshot.timer.kind {
        case .off:
            statusLine = snapshot.isMuted ? "Muted · no timer"
                : (snapshot.isPlaying ? "Playing · no timer" : "Paused")
        case .sleep, .pomodoro:
            let verb = snapshot.isMuted ? "Muted" : (snapshot.isPlaying ? "Playing" : "Paused")
            statusLine = "\(verb) · \(snapshot.timer.remainingLabel) left"
        case .stopwatch:
            statusLine = "Stopwatch · \(snapshot.timer.remainingLabel)"
        case .justCompleted:
            statusLine = snapshot.timer.remainingLabel
        }
        return PhoneSnapshotForWatch(
            isPlaying: snapshot.isPlaying,
            volumePercent: snapshot.volumePercent,
            isMuted: snapshot.isMuted,
            statusLine: statusLine,
            timerProgress: max(0, min(1, snapshot.timer.progress)),
            timerRemainingLabel: snapshot.timer.remainingLabel
        )
    }
}
