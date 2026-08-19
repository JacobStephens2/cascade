# Cascade

Cascade is a waterfall white-noise player. It runs on several platforms. The public product is the player, not the architecture that makes those platforms possible.

## Language

**Cascade**:
A waterfall white-noise player for focus, prayer, and sleep.
_Avoid_: pitching Cascade as an architecture kata in user-facing copy

**Core**:
The headless Rust state machine that owns Cascade's behaviour. It has no audio, filesystem, or clock.
_Avoid_: backend, engine (when meaning this)

**Shell**:
A native client around the core. Each shell owns side effects — audio, OS integration, and UI.
_Avoid_: using "platform" for the client; using "shell" in user-facing copy

**App**:
The user-facing Cascade product on a device — what someone opens, downloads, or installs. The word a visitor sees.
_Avoid_: using "app" for the core

**Platform**:
The operating system or device family a shell targets: web, Android, macOS, Windows, iOS, watchOS.
_Avoid_: using "platform" for the shell itself

**Watch app**:
The Apple Watch client. It is not independently installed; it comes with the iPhone app.
