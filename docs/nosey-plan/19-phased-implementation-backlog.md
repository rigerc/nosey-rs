[Plan index](../nosey-implementation-plan.md) · [Previous: 18. Testing, CI, packaging and release](18-testing-ci-packaging-and-release.md) · [Next: 20. Release acceptance and open items](20-release-acceptance-and-open-items.md)

## 19. Phased implementation backlog

> **Maintainer clarification (2026-11-25):** Kitter is a starting point for selective source reuse, not an application to fork, rebrand wholesale, or reproduce. Reuse, adapt, or replace only code that serves Nosey's specified behavior, with attribution and Nosey-native tests. Building the untouched Kitter application, retaining its storage format, reproducing its CLI/UI, and full Kitter parity are not prerequisites. This clarification supersedes conflicting Kitter-specific import/parity gates in the earlier plan; Nosey product requirements, Nosey-9 detection parity, SDK interoperability and repository quality gates remain in force.

The earlier 20–34 developer-week estimate no longer reflects the approved scope. Removed features reduce complexity, while Registry Studio, linked presets, shared change review and the updater add work. Re-estimate after Phase 0 from the actual pinned source and existing reusable GPUI functionality. No delivery date is committed. The following private milestones define dependencies and exit gates, not public releases.

### Phase 0 — Existing workspace adoption and upstream audit

| ID | Work item |
|---|---|
| FND-01 | Confirm existing private `rigerc/nosey-rs` checkout and history; capture template baseline; plan separate official registry and Action repos; no replacement app repo |
| FND-02 | Pin Kitter, Nosey-9, source registry, AI Catalog 1.0 normative revision and reviewed `ai-catalog-rust` SDK; validate toolchain and dependency policy |
| FND-03 | Run template-native bootstrap/doctor/quality checks and separately establish pinned Kitter Linux x86-64 CLI/GPUI baseline builds and tests |
| FND-04 | Audit Kitter storage, library, installer, agent plugins, presets, preview and CLI |
| FND-05 | Audit Nosey-9 rule grammar and fixtures; document exact parity and intentional deviations |
| FND-06 | Resolve upstream notices and initiate registry redistribution permission process |
| FND-07 | Confirm companion repo owners, extension namespace, official URL, compiler naming and release identity; app owner/repo already confirmed |
| FND-08 | Inventory placeholder references, xtask/config, Beads, toolchain, docs and release integration; plan auditable upstream import |

**Gate:** baseline builds and source-level integration map complete. Licensing status documented; unresolved imported data remains quarantined.

### Phase 1 — Kitter import, rebrand and workspace extraction

| ID | Work item |
|---|---|
| COR-01 | Import audited Kitter source/resources/tests; rename product binaries, paths, docs and assets to Nosey; set repo metadata to rigerc/nosey-rs |
| COR-02 | Introduce seven product crates alongside xtask; update root workspace membership/default members, Cargo.lock, docs/architecture.md and dependency rules; retire sample crates after replacement smoke tests |
| COR-03 | Retain existing file persistence in new application namespace; add typed domain IDs and errors |
| COR-04 | Shared core services, locking and safe writes; disable/remove adoption UI and commands |
| COR-05 | Manual project registration/unregistration; effective-skill audit adapter |
| COR-06 | Adapt existing Rust CI, xtask quality gates, toolchain, Linux GPUI prerequisites and sample-specific tests |

**Gate:** rebranded app and CLI build privately; compatible Kitter functions pass smoke tests; no new database, no migration.

### Phase 2 — AI Catalog registry platform and compiler

| ID | Work item |
|---|---|
| REG-01 | Confirm pinned SDK/spec compatibility; define versioned Nosey intelligence/skill extensions and YAML authoring schemas (no duplicate standard wire schema) |
| REG-02 | Integrate pinned `ai-catalog`, `ai-catalog-validate`, `ai-catalog-trust`, `ai-catalog-discover` behind registry adapter; strict JSON, SDK diagnostics, original-byte trust, safe fetch and unknown extension preservation |
| REG-03 | Namespace authority, semantic version compatibility and cross-registry reference resolution |
| REG-04 | Global registry settings, approval, enable/priority, remove/refresh and official defaults |
| REG-05 | Atomic original-byte snapshot cache with resolved base URL, TTL, stale fallback and explicit connectivity policy |
| REG-06 | YAML entity loader and deterministic SDK-backed compiler with standalone binary; Nosey source schemas under schema/nosey-registry/ |
| REG-07 | Private migration utility (xtask or isolated script/tool), attribution/provenance report and authorized sole-source import |
| REG-08 | In-memory unified catalog index with priority and explicit within-registry release selection for duplicate canonical IDs |

**Gate:** synthetic/authorized registry loads, compiles, validates and supports standard skills and custom intelligence. Failed refresh preserves the old snapshot; no unauthorized seed data published.

### Phase 3 — Scanner and recommendations

| ID | Work item |
|---|---|
| DET-01 | Implement generic audited rule mechanisms and project-root traversal |
| DET-02 | Global exclusions, configurable limits, cancellation and structured evidence |
| DET-03 | Aggregate project inventory and complete/incomplete status; central persistence |
| DET-04 | Port representative upstream detection fixtures; implement reusable registry fixture runner |
| REC-01 | Explicit mapping and combination evaluation; combination-first + registry ordering |
| REC-02 | Canonical deduplication, cross-registry mappings, unresolved warnings and reasons |
| REC-03 | Selective invalidation, persistent dismissals, effective target coverage |
| REC-04 | Removal candidate logic, no destructive recommendations from stale/incomplete scans |
| REC-05 | Fuzzy search and faceted registry index |

**Gate:** manual project scan and deterministic recommendation flow work from core/CLI using registry data; no automatic scan or installation.

### Phase 4 — Shared library, installation and change management

| ID | Work item |
|---|---|
| LIB-01 | Extend central library metadata with canonical ID and source revision, one active copy |
| LIB-02 | Git and AI Catalog artifact bundle acquisition, SDK exact-byte digest verification and safe staging |
| LIB-03 | Managed symlinks, global/project scopes and custom relative target support |
| LIB-04 | Effective-skill groups, unmanaged read-only, partial target coverage |
| LIB-05 | Advisory dependency/conflict warnings and token estimate integration |
| CHG-01 | Shared change-preview and confirmation model; individual/bulk/install-all |
| CHG-02 | Independent operation continuation, targeted rollback and retry |
| CHG-03 | Global target manual synchronization and linked preset reason tracking |
| CHG-04 | Library cleanup, detached link ownership, unavailable/removed-registry handling |
| LIB-06 | Global manual/notify/automatic skill update modes and safe library replacement |

**Gate:** the full project-to-symlink vertical slice succeeds; ownership and partial failure cases pass unit/manual tests; global skills satisfy appropriate project targets.

### Phase 5 — Project-centric GPUI and essential CLI

| ID | Work item |
|---|---|
| UI-01 | Project Overview, Scan, Recommendations and Installed Skills tabs |
| UI-02 | Detailed evidence and status UI; dismissed group and removal candidate actions |
| UI-03 | Complete Explore entity browser, fuzzy filters and safe SKILL.md preview |
| UI-04 | Library and cleanup UI; global foundation and linked preset synchronization |
| UI-05 | Registries global management, stale/trust warnings and priority controls |
| UI-06 | Contextual and centralized Pending Changes screens |
| UI-07 | Global Settings, YAML settings exchange, optional onboarding wizard |
| UI-08 | In-app notifications, bounded recent history and background checks |
| CLI-01 | Essential project, scan, recommendation, install, uninstall, update commands |
| CLI-02 | Retained compatible Kitter command integration and error/approval behavior |

**Gate:** all selected personal developer workflows available through desktop; essential CLI shares identical core behavior.

### Phase 6 — Registry Studio, Action and official registry

| ID | Work item |
|---|---|
| STD-01 | Local working-tree open/clone/create template and Git credentials |
| STD-02 | Structured entity forms, inline validation and reference selectors |
| STD-03 | Draft rule tests against registered projects and reusable fixture authoring |
| STD-04 | Tracked repository discovery, individual/bulk imports and frontmatter extraction |
| STD-05 | Upstream change proposals and registry-owned metadata synchronization |
| STD-06 | Git diff/discard/review/commit/push and publication status |
| ACT-01 | Separate reusable GitHub Action with pinned verified compiler |
| ACT-02 | Generated Pages build/deploy workflow and failure isolation |
| OFF-01 | Authorized registry migration validation, tiered admission and license notices |
| OFF-02 | Reviewed community contribution and automated upstream PR workflows |

**Gate:** authors can create/edit/test/push a registry; Action validates, compiles and deploys a single catalog in a test environment. The same library code powers Studio/compiler. Licensing gate remains mandatory for official production data.

### Phase 7 — Release integration and stabilization

| ID | Work item |
|---|---|
| REL-01 | Implement global application update modes and authenticated artifact replacement |
| REL-02 | Adapt dist-workspace.toml, release.yml, version propagation and smoke tests; package AppImage and standalone CLI/compiler artifacts on Linux x86-64 |
| REL-03 | Core unit tests, upstream detection parity evidence and manual desktop checklist |
| REL-04 | Documentation: setup, projects, skill ownership, registries, Studio, compiler, update policies |
| REL-05 | Verify data rights, official namespace/URL, signing/trust and compatible release pins |
| REL-06 | Complete acceptance gate; coordinate stable compiler/Action, Pages and app release |

**Gate:** all requirements and blockers resolved; one public v1.0.0 stable release.

### First implementation sprint: recommended order

1. Use the existing `rigerc/nosey-rs` checkout; confirm baseline, remotes, branch and private visibility. Pin upstream references and normative specification; retain the destination history and upstream attribution. Establish companion repositories when authorized. Seed Beads issues from the work-item IDs.
2. Build and run the original Kitter desktop and CLI on Linux x86-64; inspect persistence, symlink engine, GPUI components and current adoption behavior.
3. Audit Nosey-9 detector grammar and reference fixtures; define exact porting boundaries before adding rule types. Start rights clearance for the registry data.
4. Import and rebrand Kitter in the existing template workspace; retain `xtask`, adapt root Cargo metadata, quality gates and sample references without changing retained behavior; implement a synthetic registry fixture with one technology, existing detection mechanism, explicit mapping and safe skill source.
5. Build one vertical slice: manual project registration → registry validation → manual scan → explicit recommendation → approved link installation → effective-skill inspection. Expose it through a minimal desktop project tab and CLI. Use this slice to verify architectural boundaries before scaling rules or building the complete Studio.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 18. Testing, CI, packaging and release](18-testing-ci-packaging-and-release.md) · [Next: 20. Release acceptance and open items](20-release-acceptance-and-open-items.md)
