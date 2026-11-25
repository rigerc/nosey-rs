# Kitter v0.2.6 integration baseline

The supplied `kitter-main.zip` archive was unpacked and its full Git tree
reconstructed with `git add -f` and `git write-tree` (284 tracked files).
The result, `2850d10f64ec6b3e1ff0f250666e8cc90cb60b25`, matches the GitHub tree for
[commit `19711db`](https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2).
The destination at the start of this integration was `b33eb5d50cabf62b25bee3c0cf124f9f1bfb7e5d` (merged PR #6).
This establishes source identity, not build parity.

## Exact source seams

| Upstream source | Observed responsibility | Nosey integration |
| --- | --- | --- |
| `src/config.rs` | `config.json`, `recent_projects`, `project_activity`, `KITTER_HOME`, data location pointer | Port project activity ordering and explicit registration removal now; retain the existing prototype state until the full config integration. New namespace still required. |
| `src/library.rs`, `src/source/*` | Central library, local/Git acquisition, update checks | Import after resolving unsafe subprocess cancellation, source bounds and metadata identity. |
| `src/project.rs`, `src/project/install.rs`, `src/directory_link.rs` | Installation plans, links, direct discovery | Direct audit is read-only; upstream managed heuristic is not ownership proof. Do not import unmanaged removal. |
| `src/effective_skills.rs`, `src/effective_skills/*` | Per-agent effective skills, scope, visibility, token estimates | Development-only adapter for the actual exported ProjectOutput schema, with provider IDs clearly distinct from canonical IDs. Native in-process integration outstanding. |
| `src/agent_plugins/*`, `resources/agents/*` | Built-in and custom agent plugin execution, QuickJS host | Audit plugin capability boundary before activation. |
| `src/preset.rs`, `src/preset/files.rs`, `src/data_location.rs` | Presets, files, relocatable storage | Preserve behavior subject to manual change review; not yet imported. |
| `src/adoption.rs`, `src/bin/kitter.rs` | Adoption and unmanaged deletion commands | Explicitly excluded by Nosey decisions Q55/Q66; do not expose. |
| `src/ui/*`, `src/assets.rs`, `locales/*` | GPUI UI, assets and translations | Requires exact native baseline and separation of UI from headless core. |

## Build and safety findings

- Kitter's root is a single package, with optional `desktop` feature.
  Original manifest points GPUI at Zed without a revision; its locked revision is
  `6e2fae619c45ffc90e5bcf5cfbfcef8bb693fbe1`.
- GPUI component packages are pinned to
  `c27f5d5c8f70d534978c2f0739ad9e10d4e41eb4`;
  fix-path-env locks `c4c45d503ea115a839aae718d02f79e7c7f0f673`.
  The lockfile includes additional transitive Git sources; each requires review
  under the destination's deny policy. The Apache-2.0 ztracing shim must be
  preserved; do not silently import GPL tracing dependencies.
- `src/source/remote.rs::git_output` uses unsafe `libc::kill` on Unix
  to terminate a child process group. `src/platform.rs` contains native FFI,
  and `src/bin/kitter-desktop.rs` has Windows FFI. Replace or isolate safely;
  do not relax the workspace's `forbid(unsafe_code)`.
- Existing Kitter source has large files, direct unwraps and lint suppressions.
  A mechanical wholesale move into the current strict workspace is not an
  accepted integration.
- The original app's update URLs and data directory cannot be reused for Nosey.
  The baseline must stay isolated from user data and from Nosey's production namespace.
- No upstream fonts, logos, registry records or application assets are copied in
  this change. Preserve applicable license and notice files on later imports.

## Baseline execution

On Linux x86-64, with Rust 1.99, Git and the native GPUI development libraries:

```sh
git clone https://github.com/what1f/kitter ../kitter-reference
git -C ../kitter-reference checkout --detach 19711db044aabfa9846a65aaee693276d65af5c2
bash scripts/check-kitter-baseline.sh ../kitter-reference
```

The script verifies the commit, tree and clean checkout, runs upstream locked
tests, builds the CLI, checks help, verifies the tracing shim and checks the
desktop target. The corresponding GitHub workflow checks out the pin separately
from the Nosey workspace. No build result is claimed before it executes.

## Development audit bridge

For a synthetic project and isolated Kitter reference state only:

```sh
export KITTER_HOME="$(mktemp -d)"
cargo run --manifest-path ../kitter-reference/Cargo.toml --locked --bin kitter -- project /absolute/project --json > audit.json
cargo run -p nosey-cli -- --state /absolute/central/state.json register /absolute/project
cargo run -p nosey-cli -- --state /absolute/central/state.json audit-kitter /absolute/project --snapshot audit.json
cargo run -p nosey-cli -- --state /absolute/central/state.json unregister /absolute/project
```

The snapshot reader is size-bounded and checks the registered canonical project
root. It neither opens the reported skill paths nor executes programs. It
discards description and prompt text, preserves scope/visibility/agent/plugin
identity and labels the upstream managed flag as upstream-only. It does not
populate recommendation coverage, infer canonical AI Catalog identifiers, or
mutate links. File-open checking is not race-proof for an adversarial local
filesystem. Unregister forgets only prototype central state, leaving every link
unchanged; the Phase 4 approved unlink option remains pending.

This is a limited, testable extraction seam, not a full Kitter import. FND-03,
the remaining FND-04 audit, COR-01–06, and the existing Phase 2/3 gaps remain
open pending build evidence. No Beads CLI was available; reconcile issues in
a real checkout.
