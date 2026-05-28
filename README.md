# Cascade

A waterfall white-noise player built as a "Clave architecture kata": a headless
Rust core driving a React PWA (and, eventually, an Android Compose shell) via
two different binding mechanisms (`wasm-bindgen` and UniFFI). The core owns
state, time (via explicit `Tick` events), pomodoro/sleep timers, volume, and
serialized settings — but never touches an audio API. Each platform shell is
responsible for the actual sound.

The audio is a 13-minute recording of a waterfall (`The_Falls_v3.1`). Cascade
is the daily-use player for that file across web and Android.

## Layout

```
cascade/
├── crates/
│   ├── cascade-core/      # Rust state machine + reducer + snapshot
│   └── cascade-wasm/      # wasm-bindgen wrapper around cascade-core
├── apps/
│   └── web/               # React + Vite PWA, Web Audio API engine
├── assets/                # source audio (mp3 / ogg / wav)
└── docs/                  # architecture brief + Clave tech stack
```

## Quick start (web)

```bash
# 1. Build the WASM bindings (once, or whenever core changes)
cd crates/cascade-wasm
wasm-pack build --target web --out-dir ../../apps/web/src/wasm

# 2. Run the web app
cd ../../apps/web
npm install
npm run dev
```

## Core principles (mirroring Clave)

- **Headless core.** The Rust crate owns intent and state; the platform owns
  side effects (audio output, OS integration, UI).
- **Coarse-grained API.** One `dispatch(Command) -> Update { snapshot, effects }`
  call. No tiny getters across the FFI boundary.
- **Snapshots out, commands in.** UIs render from `Snapshot`; user actions and
  platform events flow back as `Command`s.
- **Effects, not callbacks.** The core returns a list of `Effect`s the platform
  layer executes — easier to test, replay, and reason about.
- **Time is an input.** The core never reads the system clock. The UI ticks
  the core with wall-clock deltas.

See [`docs/architecture-brief.md`](docs/architecture-brief.md) for the full
design rationale.
