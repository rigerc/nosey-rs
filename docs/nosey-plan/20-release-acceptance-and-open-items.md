[Plan index](../nosey-implementation-plan.md) · [Previous: 19. Phased implementation backlog](19-phased-implementation-backlog.md) · [Next: 21. Interview decision register](21-interview-decision-register.md)

## 20. Release acceptance and open items

### 20.1 Definition of done

- [ ] `rigerc/nosey-rs` existing workspace with seven product crates plus `xtask`, Linux x86-64 build; clean Nosey application namespace; Kitter functionality imported and audited, upstream attribution retained, template sample references retired, deviations documented.
- [ ] Three independent repositories, correct licensing/attribution, registry seed exclusively authorized data from `rigerc/nosey-registry`.
- [ ] Pinned AI Catalog version, versioned extensions, individual YAML entities and single JSON distribution; standard catalogs usable and cross-registry references gracefully handled.
- [ ] Built-in configurable official registry; global priority, explicit custom trust, TTL/manual refresh, last-valid stale fallback with online connectivity gate.
- [ ] Manual project registration and scanning; registry-only Nosey-9 detection mechanisms; aggregate project inventory, exclusions, limits, structured evidence and incomplete-scan protection.
- [ ] Explicit additive mapping and combination-first order, canonical identity deduplication, unmapped technologies displayed, recommendation groups and persistent dismissals, selective invalidation.
- [ ] Central one-copy library, Git and bundle acquisition, Linux symlinks, global foundation, per-project additions, advisory warnings, informational token estimates, unmanaged detection without adoption.
- [ ] Manual individual/bulk/install-all confirmation, pending review, linked preset and global-target synchronization, safe cleanup, independent failure handling and retry.
- [ ] Offline library management; no offline discovery, no project-local Nosey files, no new database, no automatic project mutations.
- [ ] Project-centric desktop, Explore full browser and filters, skill preview, Registry Studio structured forms, optional wizard, settings YAML exchange, in-app notifications and bounded recent activity.
- [ ] Studio source imports, tracked repository proposals, Git working tree drafts, GitHub publishing using existing credentials, shared compiler and reusable Action, fixture tests, successful Pages publication with prior catalog preserved on failure.
- [ ] Essential CLI and separate headless compiler, independent application/skill update policies, background work only while desktop runs, verified safe application updater.
- [ ] Core unit tests and manual desktop workflows recorded, standard Rust CI, stable tagged Linux x86-64 AppImage and standalone CLI/compiler release assets.
- [ ] No public prerelease milestones; official registry endpoint available and verified for v1.0.0.

### 20.2 Blockers and risks

| Item | Status and next action |
|---|---|
| Registry redistribution/relicensing | External release blocker; establish permission for CC BY 4.0 seed publication. |
| Exact upstream source audit | Not complete; pin commits, compile, inspect actual rule grammar and persistence/GPUI interfaces. |
| Existing template baseline | Archive inspected only; no compilation or quality gates run. Confirm actual Git checkout, Rust 1.99 availability/compatibility, GPUI system dependencies, sample consumers and current CI status. |
| Template release pipeline | Existing cargo-dist targets include macOS/Windows and smoke test names sample-app. Adapt and test Linux-only product artifacts, version propagation and AppImage packaging. |
| AI Catalog RC stability | Pin normative revision; verify schema and signature examples against conformance requirements. |
| Controlled namespace and signing identity | App repository is confirmed as rigerc/nosey-rs. Confirm companion owners/domain and official production endpoints; settle key management and release authentication. |
| GitHub SSH-only authentication | Clone/push works with existing credentials; API creation/Pages setup may require manual browser steps. |
| Private development and Pages hosting | Verify repository/account features; use authorized test builds and endpoints until public launch. |
| AppImage and CLI automatic updates | Design authenticated artifact verification, writable paths and failure-safe replacement; validate target platforms. |
| Unmanaged identity | Audit what reliable provenance Kitter exposes; canonical ID only for deduplication, no name-based equivalence. |
| Retained Kitter features vs essential CLI | Preserve compatible existing functions, remove only explicitly superseded features; reconcile exact command tree in audit. |
| Scope estimate | Re-estimate after actual audit; previous effort range superseded. |

### 20.3 Implementation defaults requiring validation, not confirmed interview requirements

24-hour registry TTL; notifications as default application/skill update mode; 20-level scan depth, 5 MiB file limit and 60-second timeout; 100-entry recent activity history; alphabetical plus canonical-ID tie-break; safe offline reachability predicate; exact source-acquisition/digest and signing details; Linux distribution baseline and release artifact naming; compiler flag spellings; exact default exclusions and periodic check intervals. All remain adjustable where the interview specified configuration. The prior Level 2 publication and Level 3 verification target and official signing baseline are carried forward from the AI Catalog plan, not an interview mandate to require signatures on custom registries.

### 20.4 Normative references

- [Nosey application repository](https://github.com/rigerc/nosey-rs) — confirmed destination; supplied archive is the inspected baseline.
- Repository-local sources: `../Cargo.toml`, `../AGENTS.md`, `../rust-toolchain.toml`, `../dist-workspace.toml`, `../.github/workflows/ci.yml`, `../.github/workflows/release.yml`, `../scripts/quality-gates.sh` and `architecture.md`.
- [Kitter](https://github.com/what1f/kitter) — pinned foundation; retain license and notices.
- [Nosey-9](https://github.com/rigerc/nosey-9) — pinned behavioral reference for detection.
- [Original Nosey registry](https://github.com/rigerc/nosey-registry) — exclusive initial data source; licensing gate.
- [AI Catalog specification](https://ai-catalog.io/spec/) — previously reviewed v0.9 RC; normative pinned revision takes precedence over illustrative schemas.
- [JSON Canonicalization Scheme, RFC 8785](https://www.rfc-editor.org/rfc/rfc8785) and [JSON Web Signature, RFC 7515](https://www.rfc-editor.org/rfc/rfc7515) — use where required by the pinned signing profile.
- [GitHub Pages custom workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages) — confirm current job permissions and deployment setup in implementation.

This adaptation inspects the supplied `nosey-rs-master.zip` only. It does not modify Cargo manifests, application source, CI, Beads, Git history or remote repositories. Neither the template nor upstream applications have been built in this documentation pass. The application repo is confirmed by the user; companion names are retained but owners/endpoints are not independently verified. The archive contains no Git metadata, so the import/history strategy remains an implementation decision. The only archive change is the addition of this Markdown plan. This is a requirements and implementation specification, not a report of completed implementation.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 19. Phased implementation backlog](19-phased-implementation-backlog.md) · [Next: 21. Interview decision register](21-interview-decision-register.md)
