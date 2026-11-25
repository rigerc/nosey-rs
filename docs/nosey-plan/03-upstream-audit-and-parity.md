[Plan index](../nosey-implementation-plan.md) · [Previous: 2. Repositories, branding and licensing](02-repositories-branding-and-licensing.md) · [Next: 4. Rust workspace and architecture](04-rust-workspace-and-architecture.md)

## 3. Upstream audit and parity boundaries

### 3.1 Audit deliverables

Before restructuring, establish the current template baseline and build and test the pinned Kitter Linux CLI/desktop in a separate reference checkout; build Nosey-9 as a development reference where feasible. Inventory actual source modules, persistent formats, commands, GPUI views, agent capability model, source retrieval, symlink handling, presets, and detection rule grammar/fixtures. Document each feature in `docs/parity-matrix.md` with upstream source reference, expected behavior, Rust destination, disposition and test evidence. Do not claim parity from README descriptions alone.

| Capability | Disposition |
|---|---|
| Kitter library, project registration, plugins, auditing, token estimates, presets, instructions, themes, configuration | Retain except where explicit interview decisions override behavior |
| Kitter adoption | Remove/disable adoption; retain unmanaged discovery and effective-skill inspection |
| Kitter existing source handling | Preserve compatible source features; add AI Catalog bundle support |
| Nosey-9 detection operators, parsers and matching semantics | Port all existing mechanisms; no additional detection operators |
| Nosey-9 scan traversal | Adapt to global exclusions, depth/file/timeout limits and project-wide aggregation |
| Nosey-9 mapping and combinations | Adapt to explicit mappings, combination-first and priority semantics |
| Registry schemas, authoring and publishing | Redesign on AI Catalog plus YAML, GPUI Studio, compiler and GitHub Action |
| Dependency and conflict solver | Advisory warnings only |
| Copy installation, pinning, separate versions | Intentionally excluded |
| Registry offline discovery and local project lock/config files | Intentionally excluded |
| Full CLI/TUI | Essential new CLI commands only; no TUI; existing compatible Kitter CLI preserved unless superseded |
| Automatic skill reconciliation and source adoption | Explicitly excluded |

### 3.2 Observed repository baseline and adoption map

The uploaded archive contains a virtual Rust workspace template, not the Kitter application. These findings are from the supplied files, not a remote repository inspection or executed build.

| Existing location | Observed baseline | Planned adaptation |
|---|---|---|
| `Cargo.toml`, `Cargo.lock`, `VERSION` | Workspace globs `crates/*`, `examples/*`; default `crates/sample-app`; shared version `0.0.0`, edition 2024, declared Rust 1.99, placeholder `rigerc/rs-template` URL | Preserve workspace infrastructure; add seven product crates; explicitly enumerate or review members, select product default members, update metadata and regenerate lockfile after imports |
| `crates/sample-app/`, `crates/example-crate/` | Sample executable and placeholder library | Keep until the imported product builds; remove or replace with actual product crates after checking all consumers |
| `crates/xtask/`, `config/xtask.json`, `config/profiles/` | Existing development automation and quality policy | Retain as an eighth, non-product workspace member; extend existing quality tooling instead of introducing another task runner |
| `examples/`, `tests/`, `tests/fixtures/` | Template examples, test and CI regression harnesses | Audit consumers; retain relevant tooling/tests, relocate or remove sample-specific examples with their references; isolate new detector/registry fixtures under named subdirectories |
| `rust-toolchain.toml`, `.cargo/config.toml`, `.config/nextest.toml` | Pinned toolchain, aliases and nextest profiles | Check actual availability and GPUI compatibility; align MSRV, CI setup and target; use `x86_64-unknown-linux-gnu` |
| `AGENTS.md`, `agents-docs/reference.md`, `.agents/` | Existing agent contract, skills and historical CI artifacts | Follow safety, lint, error, test-first and task-tracking rules; update external references and regenerate derived context only when relevant |
| `.beads/`, `plans/`, `plans/adr/`, `docs/adr/` | Beads and design documentation conventions | Use Beads for execution tracking, maintain ADRs in the existing documentation tree, retain historical decisions |
| `schema/` | Existing template schemas | Preserve existing directory and consumers; place Nosey authoring/extension schemas in `schema/nosey-registry/`; generate registry template `schemas/` from this source |
| `.github/workflows/ci.yml`, `.github/actions/setup-rust/`, `scripts/quality-gates.sh` | Existing Rust CI and quality gate machinery | Adapt path filters, system dependencies and selected test/build packages; retain applicable repository checks |
| `dist-workspace.toml`, `.github/workflows/release.yml`, `release.toml`, `scripts/propagate-version.sh` | cargo-dist, version propagation and release preflight; dist currently lists Linux, macOS and Windows; release smoke test invokes `sample-app` | Restrict product targets to Linux x86-64; replace sample smoke test; integrate AppImage, compiler and CLI assets; preserve version/checksum contracts |
| `docs/`, `docs/src/`, `docs/book.toml` | Root docs plus mdBook source tree | The plan index lives in `docs/nosey-implementation-plan.md`; chapters live in `docs/nosey-plan/`; later refresh product architecture, CI and book navigation with actual implementation |
| `LICENSE`, `NOTICE` | MIT template with d-oit attribution and peer-template notice | Retain notices; add Apache-2.0 Kitter and MIT Nosey-9 source notices before redistribution |

No GPUI dependency or Nosey product crates are present in the inspected root workspace manifest. A previously stored `.agents/ci/ci-summary.md` is historical template evidence, not proof of a successful build of the archive. The declared toolchain, dependencies and quality gates must be run in a suitable environment before accepting the baseline.

Add `Agent-Card/ai-catalog-rust` to `docs/upstream-baseline.yaml`: reviewed commit `920ef70e1e5a602e142137f9ea6a1135fedbf110`, workspace version 0.4.0, edition 2024, declared MSRV 1.95, Apache-2.0. Audit the matching normative specification revision, crate features, compatibility with the actual Nosey/GPUI toolchain and dependency/license policy; record test evidence for the pinned SDK before product integration. Do not infer successful compilation from the manifests or assume the repository README's older version examples are current. Prefer consuming the upstream crates over importing their source; retain their applicable notices.

The audit resolves the exact detection grammar and existing Kitter platform/dependency assumptions. Any newly discovered feature that conflicts with the interview is an intentional deviation, not an automatic expansion of scope.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 2. Repositories, branding and licensing](02-repositories-branding-and-licensing.md) · [Next: 4. Rust workspace and architecture](04-rust-workspace-and-architecture.md)
