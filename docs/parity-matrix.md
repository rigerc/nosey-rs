# Source audit — partial, unverified implementation

Sources: [Kitter pinned candidate](https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2), [Nosey-9 pinned candidate](https://github.com/rigerc/nosey-9/tree/dca90b0528f01bf7962c72aae27fc2ab3492c3ac), [SDK reviewed revision](https://github.com/Agent-Card/ai-catalog-rust/tree/920ef70e1e5a602e142137f9ea6a1135fedbf110).

| Upstream path | Observed mechanism | Destination / status |
|---|---|---|
| Kitter src/library.rs, src/source/* | Library and acquisition | Not imported; source inventory only |
| Kitter src/project.rs, src/project/install.rs, src/directory_link.rs | Project/link management | Not imported; prototype central project state is not Kitter parity |
| Kitter src/effective_skills/*, src/agent_plugins/* | Effective target and plugins | Not imported; injected target facts only |
| Kitter src/preset/*, src/config.rs, src/data_location.rs | Existing persistence/presets | Not imported; prototype not production storage |
| Kitter src/ui/*, src/bin/kitter-desktop.rs, src/assets.rs | GPUI | Not imported or built |
| Kitter Cargo.toml, crates/ztracing-shim | GPUI git deps, optional desktop, shim | Lockfile and policy review outstanding |
| Nosey-9 internal/detection/evaluate.go | file (glob/regex), extension, content, dotenv | Implemented subset; regex uses bounded Rust regex rather than ECMAScript regexp2; differences untested |
| Nosey-9 internal/detection/observe.go | npm, composer, cargo, python, go, ruby, hex, deno, docker | Partial providers implemented; missing JSONC, compose, exact Go/Ruby/Python parity; versions not persisted |
| Nosey-9 internal/detection/providers.go | terraform, terraform.resource, githubAction, MCP, Compose, JSONC | Not implemented; relevant rules fail closed or scans marked incomplete |
| Nosey-9 internal/detection/adapters.go | Explicit approved/hash-pinned external adapters | Unavailable, never execute registry-provided programs |
| Nosey-9 internal/detection/engine.go | Per-scope detection, coverage/unknown | Aggregate project by design; incomplete/cancelled/failed preserved |
| SDK ai-catalog, trust, validate | Standard model, strict parse, semantic validation | Integrated against inspected signatures; compile not run |
| SDK ai-catalog-discover | Discovery/fetch | Not integrated |

No parity claim until upstream fixtures, native baseline builds, quality gates and this slice pass. The original registry is not copied. All fixtures are synthetic. No production namespace or official signer selected.
