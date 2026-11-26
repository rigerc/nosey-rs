# Source audit — partial, unverified implementation

Sources: [Kitter pinned candidate](https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2), [Nosey-9 pinned candidate](https://github.com/rigerc/nosey-9/tree/dca90b0528f01bf7962c72aae27fc2ab3492c3ac), [SDK reviewed revision](https://github.com/Agent-Card/ai-catalog-rust/tree/920ef70e1e5a602e142137f9ea6a1135fedbf110).

| Upstream path | Observed mechanism | Destination / status |
|---|---|---|
| Kitter src/library.rs, src/source/* | Library and acquisition | Fixtures, digest-verified HTTPS Agent Skill ZIPs and commit-pinned HTTPS Git subtrees share staged active-copy publication and provenance. Matching remote acquisitions support offline reuse. |
| Kitter src/project.rs, src/project/install.rs, src/directory_link.rs | Project/link management | Project activity and non-destructive unregister ported; native direct-target discovery added; explicit install preview, no-replace linking and recovery journal added |
| Kitter src/effective_skills/*, src/agent_plugins/* | Effective target and plugins | Native configured project and global targets, read-only effective inventory and canonical coverage implemented for shared agent and Claude locations; broader plugin coverage outstanding |
| Kitter src/preset/*, src/config.rs, src/data_location.rs | Existing persistence/presets | Nosey-owned project and preset state. Applied preset membership retains library copies during cleanup, including before link synchronization. |
| Kitter src/ui/*, src/bin/kitter-desktop.rs, src/assets.rs | GPUI | Reference only; reuse components selectively for Nosey |
| Kitter Cargo.toml, crates/ztracing-shim | GPUI git deps, optional desktop, shim | Lockfile and policy review outstanding |
| Nosey-9 internal/detection/evaluate.go | file (glob/regex), extension, content, dotenv | Implemented subset; bounded Rust regex with lookaround/backreference fallback; exact ECMAScript parity unverified |
| Nosey-9 internal/detection/observe.go | npm, composer, cargo, python, go, ruby, hex, deno, docker | Partial providers implemented; JSONC and Compose added; typed npm/Composer/Cargo/Python/Go manifest constraints added; resolved lockfile/installed versions and exact fixture parity outstanding |
| Nosey-9 internal/detection/providers.go | terraform, terraform.resource, githubAction, MCP, Compose, JSONC | Passive MCP, Compose, JSONC, GitHub Actions and HCL Terraform provider/resource detection added; exact fixture parity outstanding. YAML aliases/tags and malformed inputs mark incomplete. |
| Nosey-9 internal/detection/adapters.go | Explicit approved/hash-pinned external adapters | Unavailable, never execute registry-provided programs |
| Nosey-9 internal/detection/engine.go | Per-scope detection, coverage/unknown | Aggregate project by design; incomplete/cancelled/failed preserved |
| SDK ai-catalog, trust, validate | Standard model, strict parse, semantic validation | Integrated against inspected signatures; original-byte verify and failed-assertion rejection added; compile not run |
| SDK ai-catalog-discover | Discovery/fetch | SDK discover_with, restricted direct HTTPS, reviewed identity and explicit approval/refresh added; native network tests and global settings wiring outstanding |
| what-stack `aa79257` src/config/files.rs | bounded regular-file reads and nonblocking Unix opens | Adapted to rooted descriptor-relative opens, no symlink traversal, hard byte cap and explicit failure. |
| what-stack src/config/rules.rs, src/config/python.rs | filename, extension and dependency heuristics | Translated supported positive predicates to a candidate fixture registry; no runtime dependency or first-match precedence; compound/source/import and runtime-only conditions deferred. |
| projectdetect `6b75606` src/builtins.rs | project markers and build/cache names | 18 distinct additional candidate technologies, 42 rules and 5 nonduplicate .NET rules; reviewed directory exclusions only. |
| projectdetect src/find.rs, src/indicator.rs | nested discovery, cancellation and indicator results | Regression scenarios adapted to existing Nosey traversal; no second walker, CEL or alternate registry. |

Kitter is a selective source reference, not a parity target. Validate adapted behavior in Nosey; Nosey-9 detection parity and SDK requirements are unchanged. No successful build is claimed. The original registry is not copied. All fixtures are synthetic. No production namespace or official signer selected.

See [Kitter integration baseline](kitter-integration-baseline.md) for the archive tree verification, exact safety blockers and baseline commands.

Saved-scan freshness fingerprints are independent of mapping and priority-only changes; removal suggestions require a current complete scan. Explore search covers five implemented entity kinds (skills, technologies, rules, mappings, combinations). No production connectivity or global configuration UI is wired yet. External executable adapters remain fail-closed.

See [what-stack selective reuse](what-stack-source-reuse.md).

See [projectdetect selective reuse](projectdetect-source-reuse.md).
