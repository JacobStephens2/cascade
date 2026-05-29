import WatchKit

/// Thin wrapper over `WKInterfaceDevice` haptics so the views don't import
/// WatchKit directly. Apple Watch UX leans heavily on physical feedback —
/// every play/pause / preset tap should feel like something happened.
enum WatchHaptics {
    static func tap() { WKInterfaceDevice.current().play(.click) }
    static func start() { WKInterfaceDevice.current().play(.start) }
    static func stop() { WKInterfaceDevice.current().play(.stop) }
    static func success() { WKInterfaceDevice.current().play(.success) }
}
