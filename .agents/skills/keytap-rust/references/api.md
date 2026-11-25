# Keytap 0.4 API quick reference

**Crate:** `keytap = "0.4"` (default feature: `chord`). Rust >= 1.85.

## Raw tap

```rust
use keytap::{Tap, Event, EventKind, Key};
let tap = Tap::new()?;     // create an OS listener
let ev = tap.recv()?;      // blocking Result<Event, RecvError>
let maybe = tap.try_recv();
let timed = tap.recv_timeout(std::time::Duration::from_millis(100));
for event in tap.iter() { /* blocking iterator */ }
```

`Event` includes `kind` and an event timestamp. `EventKind::{KeyDown, KeyUp, KeyRepeat}` all hold a `Key`. `Key` uses physical keys; left/right Shift/Control/Alt/Meta are separate; unknown codes are still represented with `Key::Unknown(RawCode)`.

`Tap::builder()` returns `TapBuilder`, with these verified methods:

- `.capacity(usize)` — set bounded event queue capacity
- `.unbounded()` — use an unbounded event queue (watch memory under load)
- `.linux_hotplug_interval(Duration)`
- `.macos_no_repeat_detection()`
- `.shutdown_timeout(Duration)`
- `.build()`

`Tap::new()` uses defaults. `Tap` is dropped for teardown; no `close()` API.

## Chord matcher

```rust
use keytap::{Key, chord::{Chord, ChordEvent, ChordMatcher}};
let matcher = ChordMatcher::builder()
    .add("momentary", Chord::of([Key::MetaRight, Key::Space]))
    .add_toggle("sticky", Chord::of([Key::MetaRight, Key::AltRight, Key::Space]))
    .build()?;
while let Ok(event) = matcher.recv() {
    match event {
        ChordEvent::Start { id, .. } => println!("start {id}"),
        ChordEvent::End { id, .. } => println!("end {id}"),
    }
}
```

- `ChordMatcherBuilder`: `.add`, `.add_toggle`, `.add_with_mode`, `.with_tap`, `.build`.
- `ChordMatcher`: `.recv`, `.try_recv`, `.recv_timeout`, and Drop shutdown.
- `Chord`: `.of(iterable_of_Key)`, `.keys`, `.len`, `.is_empty`.
- Chords are unordered physical-key sets; longest match wins; in transitions `End(previous)` comes before `Start(next)`; repeats are ignored. Momentary `End` on release. Toggle `End` on next full press.
- A tap injected via `.with_tap(tap)` is consumed by the matcher. It cannot simultaneously be read via raw `Tap` methods; shared channel receivers compete for events, not broadcast them.

## Features

```toml
keytap = { version = "0.4", features = ["serde", "tracing"] }
```

- `chord` (default): exposes `keytap::chord`.
- `serde` (opt-in): key/chord serialization support.
- `tracing` (opt-in): diagnostic tracing.

## Error handling

`keytap::Error` is `#[non_exhaustive]`; include wildcard if matching. As of 0.4.0, variants include `PermissionDenied`, `NoDevices`, `TapFailed(String)`, and `Io(std::io::Error)`.

```rust
match Tap::new() {
    Ok(tap) => { /* keep tap alive */ }
    Err(keytap::Error::PermissionDenied) => eprintln!("Input access denied"),
    Err(keytap::Error::NoDevices) => eprintln!("No keyboard devices available"),
    Err(e) => eprintln!("Could not start keytap: {e}"),
}
```

## Verified links

- https://docs.rs/keytap/0.4.0/keytap/struct.Tap.html
- https://docs.rs/keytap/0.4.0/keytap/struct.TapBuilder.html
- https://docs.rs/keytap/0.4.0/keytap/enum.EventKind.html
- https://docs.rs/keytap/0.4.0/keytap/enum.Key.html
- https://docs.rs/keytap/0.4.0/keytap/chord/struct.ChordMatcher.html
- https://docs.rs/keytap/0.4.0/keytap/chord/struct.ChordMatcherBuilder.html
- https://docs.rs/keytap/0.4.0/keytap/enum.Error.html
