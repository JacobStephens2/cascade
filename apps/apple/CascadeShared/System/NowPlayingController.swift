import Foundation
import MediaPlayer

/// Publishes Cascade's current state to `MPNowPlayingInfoCenter` so the
/// menu-bar Now Playing widget, Control Center, and AirPods media keys can
/// drive playback.
///
/// Best-effort: macOS doesn't guarantee any particular app wins the
/// "current Now Playing" slot when multiple apps register. If it doesn't
/// stick, the in-app and menu-bar controls still work.
@MainActor
final class NowPlayingController {
    private var dispatch: ((Command) -> Void)?

    func bindRemoteCommands(_ dispatch: @escaping (Command) -> Void) {
        self.dispatch = dispatch
        let center = MPRemoteCommandCenter.shared()
        center.playCommand.addTarget { [weak self] _ in
            self?.dispatch?(.play)
            return .success
        }
        center.pauseCommand.addTarget { [weak self] _ in
            self?.dispatch?(.pause)
            return .success
        }
        center.togglePlayPauseCommand.addTarget { [weak self] _ in
            self?.dispatch?(.togglePlayback)
            return .success
        }
    }

    func update(snapshot: Snapshot) {
        var info: [String: Any] = [:]
        info[MPMediaItemPropertyTitle] = snapshot.title
        info[MPMediaItemPropertyArtist] = snapshot.subtitle
        info[MPNowPlayingInfoPropertyPlaybackRate] = snapshot.isPlaying ? 1.0 : 0.0
        info[MPNowPlayingInfoPropertyIsLiveStream] = true
        MPNowPlayingInfoCenter.default().nowPlayingInfo = info
        MPNowPlayingInfoCenter.default().playbackState = snapshot.isPlaying ? .playing : .paused
    }
}
