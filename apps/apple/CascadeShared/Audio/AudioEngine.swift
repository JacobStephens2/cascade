import AVFoundation
import Foundation

/// AVAudioEngine + AVAudioPlayerNode for sample-accurate gapless looping.
///
/// We decode the bundled asset *once* into a PCM buffer and schedule it with
/// `.loops`. That sidesteps MP3/AAC encoder padding entirely — the loop seam
/// is exactly the buffer's frame count, no silent frames before/after.
///
/// Volume goes through `mixerNode.outputVolume` with a short linear ramp so
/// the slider doesn't click.
@MainActor
final class AudioEngine {
    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private var mixer: AVAudioMixerNode { engine.mainMixerNode }

    private var buffer: AVAudioPCMBuffer?
    private var assetLoaded = false

    init() {
        engine.attach(player)
        engine.connect(player, to: mixer, format: nil)
    }

    /// Start (or resume) the loop at the given volume. Idempotent.
    func start(volumePercent: Int) {
        do {
            try loadAssetIfNeeded()
            configureSessionForPlayback()
            if !engine.isRunning {
                engine.prepare()
                try engine.start()
            }
            if !player.isPlaying {
                if let buffer {
                    player.scheduleBuffer(buffer, at: nil, options: [.loops, .interrupts])
                }
                player.play()
            }
            setVolume(volumePercent: volumePercent, rampMs: 400)
        } catch {
            NSLog("[Cascade] AudioEngine.start failed: \(error)")
        }
    }

    func pause() {
        // Quick fade so pause doesn't pop, then stop the node.
        setVolume(volumePercent: 0, rampMs: 300)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) { [weak self] in
            guard let self else { return }
            self.player.pause()
        }
    }

    func setVolume(volumePercent: Int, rampMs: Int = 80) {
        let target = perceptualVolume(volumePercent)
        let steps = max(1, rampMs / 16)
        let startValue = mixer.outputVolume
        let delta = (target - startValue) / Float(steps)
        // Tiny CADisplayLink-style linear ramp on the main run loop. Cheap and
        // good enough for a non-game app.
        var stepIndex = 0
        Timer.scheduledTimer(withTimeInterval: 0.016, repeats: true) { [weak self] timer in
            guard let self else { timer.invalidate(); return }
            stepIndex += 1
            if stepIndex >= steps {
                self.mixer.outputVolume = target
                timer.invalidate()
            } else {
                self.mixer.outputVolume = startValue + delta * Float(stepIndex)
            }
        }
    }

    // MARK: - Asset loading

    private func loadAssetIfNeeded() throws {
        guard !assetLoaded else { return }
        guard let url = assetURL() else {
            throw NSError(domain: "Cascade", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "waterfall asset not found in bundle"
            ])
        }
        let file = try AVAudioFile(forReading: url)
        let format = file.processingFormat
        let frameCount = AVAudioFrameCount(file.length)
        guard let buf = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frameCount) else {
            throw NSError(domain: "Cascade", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "could not allocate PCM buffer"
            ])
        }
        try file.read(into: buf)
        self.buffer = buf
        self.assetLoaded = true
    }

    /// Look in a couple of likely places. Xcode flat-bundles resources by
    /// default; the build script writes the converted `waterfall.m4a` into
    /// the app bundle's `Resources/`.
    private func assetURL() -> URL? {
        let candidates: [(String, String)] = [
            ("waterfall", "m4a"),
            ("waterfall", "caf"),
            ("waterfall", "wav"),
        ]
        for (name, ext) in candidates {
            if let url = Bundle.main.url(forResource: name, withExtension: ext) {
                return url
            }
        }
        return nil
    }

    /// Match the web app's square-law curve so 50% feels like ~half volume.
    private func perceptualVolume(_ percent: Int) -> Float {
        let clamped = Float(min(max(percent, 0), 100)) / 100.0
        return clamped * clamped
    }

    /// iOS requires us to opt into background-capable audio playback through
    /// `AVAudioSession.playback`; macOS has no equivalent session concept.
    /// Called every `start()` because route changes / interruptions can quietly
    /// deactivate the session and there's no harm in re-asserting it.
    private func configureSessionForPlayback() {
        #if os(iOS)
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback, mode: .default,
                                 options: [.allowAirPlay, .allowBluetoothA2DP])
        try? session.setActive(true)
        #endif
    }
}
