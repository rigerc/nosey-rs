# Keytap Rust — portable Agent Skill

Coding-agent skill for **jamiepine/keytap v0.4**: global background key events, left/right modifiers, held-key state, momentary/toggle shortcuts, and permission handling.

## Install

Copy the `keytap-rust/` folder into the skills directory used by your coding agent. Common locations include:

- **Codex:** `~/.agents/skills/keytap-rust/`
- **Claude Code:** `~/.claude/skills/keytap-rust/`

Consult the specific agent's skill setup if its path differs. Restart/reload the agent after installing if it does not discover new skills automatically.

## Use

Ask an agent: "Use the keytap-rust skill to implement global push-to-talk in my Rust desktop app." The agent will use `SKILL.md` and consult the bundled reference and examples as needed.

## Examples

```sh
cd examples
cargo run --bin held_keys
cargo run --bin push_to_talk
cargo run --bin toggle_hotkey
cargo run --bin timeout_loop
```

The examples depend on OS permissions and an actual keyboard device. They are not guaranteed to run in CI, containers, or sandboxes.

## Notes

- Targets v0.4.0 as of 2026-11-10; the upstream README may include earlier-version examples.
- Requires Rust 1.85+ for v0.4.0.
- `keytap` is observe-only and reports physical keys rather than characters.
- Source: https://github.com/jamiepine/keytap
- Rustdoc: https://docs.rs/keytap/0.4.0/keytap/
