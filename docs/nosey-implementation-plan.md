# Nosey — v1.0 implementation plan for `rigerc/nosey-rs`

**Status:** Consolidated 100-decision specification, adapted to the supplied `nosey-rs` Rust workspace archive  
**Application repository:** [rigerc/nosey-rs](https://github.com/rigerc/nosey-rs) (confirmed; product name remains Nosey)  
**Repository baseline:** `nosey-rs-master.zip`, archive comment `b56255cc65062ca0b4faa0f80321da5441a044ff`; branch and remote state not independently verified  
**Date:** 2026-11-25  
**Release:** One stable public v1.0.0 release; development and validation remain private until all release gates pass.  
**Application functionality donor:** [what1f/kitter](https://github.com/what1f/kitter) (to be imported into the existing Rust template, not yet present in the archive)  
**Detection behavior reference:** [rigerc/nosey-9](https://github.com/rigerc/nosey-9)  
**Exclusive initial registry data source:** [rigerc/nosey-registry](https://github.com/rigerc/nosey-registry)  
**Interoperability:** [AI Catalog](https://ai-catalog.io/spec/) 1.0 target, implemented through [Agent-Card/ai-catalog-rust](https://github.com/Agent-Card/ai-catalog-rust/tree/920ef70e1e5a602e142137f9ea6a1135fedbf110) (reviewed workspace version 0.4.0; pin normative specification revision in Phase 0).

This linked implementation plan supersedes the earlier implementation plan, including its full non-TUI parity, automatic project reconciliation, immutable multi-version library, SQLite, cross-platform release and offline scanning proposals. Later interview decisions are authoritative. The repository-specific integration instructions in sections 2–4, 18–20 also supersede earlier assumptions about starting from a fresh GitHub fork. This plan is a specification, not an active task tracker; the work-item IDs in section 19 are stable planning references to translate into Beads issues. Proposed defaults and implementation details are identified separately from confirmed requirements.

**SDK integration amendment:** Reuse upstream Rust crates for standard wire models, parsing, semantic validation, canonicalization, verification and discovery; Nosey owns intelligence extensions, policy, acquisition and YAML compilation. See chapters 4–6. All 100 interview decisions remain unchanged.

## Contents

The full plan is split into 21 numbered chapters under `docs/nosey-plan/`. Section numbers and work-item IDs are unchanged. Read sequentially or jump to a topic below; each chapter links back to this index and adjacent chapters.

### Product and foundation

- [1. Product scope and workflows](nosey-plan/01-product-scope-and-workflows.md)
- [2. Repositories, branding and licensing](nosey-plan/02-repositories-branding-and-licensing.md)
- [3. Upstream audit and parity boundaries](nosey-plan/03-upstream-audit-and-parity.md)
- [4. Rust workspace and architecture](nosey-plan/04-rust-workspace-and-architecture.md)

### Registry and intelligence

- [5. Registry specification and interoperability](nosey-plan/05-registry-specification-and-interoperability.md)
- [6. Registry consumption and trust](nosey-plan/06-registry-consumption-and-trust.md)
- [7. Official registry migration and maintenance](nosey-plan/07-official-registry-migration-and-maintenance.md)
- [8. Scanner and detection](nosey-plan/08-scanner-and-detection.md)
- [9. Mapping, recommendations and exploration](nosey-plan/09-mapping-recommendations-and-exploration.md)

### Skill management and interfaces

- [10. Library, skill lifecycle and unmanaged skills](nosey-plan/10-library-and-skill-lifecycle.md)
- [11. Projects, global targets and presets](nosey-plan/11-projects-global-targets-and-presets.md)
- [12. Pending changes and operations](nosey-plan/12-pending-changes-and-operations.md)
- [13. Desktop and onboarding](nosey-plan/13-desktop-and-onboarding.md)
- [14. Registry Studio and publishing](nosey-plan/14-registry-studio-and-publishing.md)
- [15. CLI and standalone compiler](nosey-plan/15-cli-and-standalone-compiler.md)

### Application infrastructure and release

- [16. Storage, settings and background tasks](nosey-plan/16-storage-settings-and-background-tasks.md)
- [17. Security and failure handling](nosey-plan/17-security-and-failure-handling.md)
- [18. Testing, CI, packaging and release](nosey-plan/18-testing-ci-packaging-and-release.md)

### Execution and requirements traceability

- [19. Phased implementation backlog](nosey-plan/19-phased-implementation-backlog.md)
- [20. Release acceptance and open items](nosey-plan/20-release-acceptance-and-open-items.md)
- [21. Interview decision register](nosey-plan/21-interview-decision-register.md)

## Implementation entry points

- Start implementation with the [upstream audit](nosey-plan/03-upstream-audit-and-parity.md) and [phased backlog](nosey-plan/19-phased-implementation-backlog.md).
- Verify the scope against the [100-decision register](nosey-plan/21-interview-decision-register.md).
- Review the [release acceptance gates and open items](nosey-plan/20-release-acceptance-and-open-items.md) before publishing.
