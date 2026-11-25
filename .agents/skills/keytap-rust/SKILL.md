---
name: keytap-rust
description: Build and debug Rust global background keyboard listeners, held-key tracking, push-to-talk chords, and toggle hotkeys with jamiepine/keytap. Use when implementing physical key press/release detection, left/right modifiers, Wayland-compatible evdev capture, desktop app shortcuts, or permission and shutdown handling with keytap.
---

# Keytap (Rust) development

Use [`keytap`](https://github.com/jamiepine/keytap) to observe global physical keyboard events while a Rust process runs, including when another window has focus. It is an **observe-only** input library, not a key injector, key interceptor, or text-input decoder.

## Version and source of truth

- This skill targets **`keytap = "0.4"`**, as documented for crate v0.4.0 (checked 2026-11-10). The GitHub README still uses `0.3`; prefer the resolved Cargo version and version-matched rustdoc over old examples.
- Rust **1.85+** is required by the v0.4.0 package (`edition = "2024"`).
- API docs: https://docs.rs/keytap/0.4.0/keytap/
- Repository: https://github.com/jamiepine/keytap
- See [API reference](references/api.md) and [platform troubleshooting](references/platforms.md). Check docs again when a different version is locked.

## Choose the right integration

1. **Raw key down / up / repeat:** `Tap::new()?`, then process `event.kind` using `Tap::recv`, `Tap::try_recv`, `Tap::recv_timeout`, or `Tap::iter`. Choose this for per-key state, debugging, or arbitrary combinations.
2. **A held shortcut (push-to-talk):** use the default-enabled `keytap::chord` feature; create `ChordMatcher::builder().add(id, Chord::of([...])).build()?`. On `ChordEvent::Start`, activate; on `ChordEvent::End`, deactivate. Chords ignore auto-repeat.
3. **A toggle hotkey:** use `.add_toggle(id, Chord::of([...]))` for press-on/press-off semantics. Do not implement a toggle by treating every `KeyRepeat` as another press.
4. **Polling integration:** call `recv_timeout` or `try_recv` on a dedicated worker; `keytap` does **not** offer an authoritative `is_key_pressed(key)` system-state query. Keep your own `HashSet<Key>` for derived held state.
5. **UI/Tauri/async application:** keep blocking reception off the UI or async runtime's core thread; forward high-level events through the application's message channel. Own the listener handle for the complete lifetime of the feature.

## Minimal setup

Add to `Cargo.toml`:

```toml
[dependencies]
keytap = "0.4"
```

Observe a specific background key:

```rust
use keytap::{EventKind, Key, Tap};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tap = Tap::new()?;
    for event in tap.iter() {
        match event.kind {
            EventKind::KeyDown(Key::MetaRight) => println!("Right Meta down"),
            EventKind::KeyUp(Key::MetaRight) => println!("Right Meta up"),
            _ => {}
        }
    }
    Ok(())
}
```

For complete runnable patterns, read:

- [`examples/src/bin/held_keys.rs`](examples/src/bin/held_keys.rs) — maintain a held-key set and emit transitions rather than repeated status spam.
- [`examples/src/bin/push_to_talk.rs`](examples/src/bin/push_to_talk.rs) — momentary shortcut using the built-in chord matcher.
- [`examples/src/bin/toggle_hotkey.rs`](examples/src/bin/toggle_hotkey.rs) — press-on / press-off behavior.
- [`examples/src/bin/timeout_loop.rs`](examples/src/bin/timeout_loop.rs) — periodic app work without blocking indefinitely for an event.

Run an example from the supplied Cargo package using `cargo run --bin held_keys` (or another bin name).

## Correctness rules

- `EventKind` variants are **`KeyDown(Key)`, `KeyUp(Key)`, `KeyRepeat(Key)`**. Ignore repeat events when tracking key-held transitions or triggering shortcuts.
- `Key` represents **physical keys**, not text characters or layout-translated glyphs. Preserve distinctions such as `MetaLeft` vs. `MetaRight`; there is no generic `Key::Meta`.
- A `HashSet<Key>` derived from events is not ground truth if a key was already held before startup or a release event is missed. Resynchronize on lifecycle interruptions if the application's state matters; fail safe (e.g., deactivate push-to-talk) when event processing stops.
- The chord matcher treats a chord as an **unordered set**. It resolves overlapping matches by the longest chord; chord transitions emit `End` before `Start`. A toggle stays active between releases, and suppresses other registered chords while active.
- A `Tap` consumes events from an internal channel. **Do not have multiple consumers compete for the same channel.** An existing `Tap` passed to `ChordMatcherBuilder::with_tap(tap)` is consumed by the matcher and cannot also be read for raw events; use independent taps or an explicit app-level fan-out.
- `Tap` is `Send + Sync`, but cloning/sharing a receiver is not a broadcast mechanism.
- Keep event handlers small; move expensive work to a worker or queue to avoid backlog. Inspect bounded queue settings when necessary.
- `Tap` unregisters its platform listener when the handle is dropped. Preserve its ownership until monitoring is intentionally stopped. Do not invent an undocumented `close()` method.

## OS permissions and limitations

- **macOS:** Input Monitoring / Accessibility authorization may be required; handle `Error::PermissionDenied` and instruct the user to allow the application in macOS privacy settings, then relaunch if needed.
- **Linux (X11 or Wayland):** uses **evdev** device access, so the process must have read access to relevant `/dev/input/event*` devices. Handle `Error::PermissionDenied` and `Error::NoDevices` explicitly. Never suggest `chmod 666` on input devices or broad privileged execution as the default fix; use least-privilege device policy and explain that evdev access exposes sensitive input.
- **Windows:** uses a low-level keyboard hook; it is not a promise to capture input from every security boundary (e.g., secure desktop).
- No mouse capture, key simulation, key suppression, global per-key polling API, or character/text decoding.

## Verification workflow

1. Inspect `Cargo.toml`/`Cargo.lock`; align examples to the installed `keytap` version and Rust toolchain.
2. Implement the minimum suitable API (raw tap vs. momentary/toggle chord).
3. Preserve listener ownership; handle startup error and graceful shutdown.
4. Test focused and unfocused windows, both left/right modifier keys, key repeats, and release transitions.
5. Test the actual target OS and permission model. Cross-compilation is not equivalent to live behavioral validation.
6. In a GUI app, test startup, hide/minimize, app quit, sleep/resume, and a lost-event case. Never claim key state is infallible.
7. Keep collected data minimal and purpose-specific. Do not persist arbitrary global keystroke streams; provide clear notice/consent for background monitoring.

## Sources

- https://docs.rs/keytap/0.4.0/keytap/
- https://docs.rs/keytap/0.4.0/keytap/struct.Tap.html
- https://docs.rs/keytap/0.4.0/keytap/chord/index.html
- https://docs.rs/keytap/0.4.0/keytap/chord/struct.ChordMatcherBuilder.html
- https://docs.rs/keytap/0.4.0/keytap/enum.Error.html
- https://github.com/jamiepine/keytap/blob/main/Cargo.toml
