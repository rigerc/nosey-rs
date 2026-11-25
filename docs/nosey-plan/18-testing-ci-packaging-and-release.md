[Plan index](../nosey-implementation-plan.md) · [Previous: 17. Security and failure handling](17-security-and-failure-handling.md) · [Next: 19. Phased implementation backlog](19-phased-implementation-backlog.md)

## 18. Testing, CI, packaging and release

### 18.1 Essential testing strategy

User-selected requirement: core unit tests and manual desktop workflow testing. Registry detection fixtures are also required. No mandatory coverage threshold, comprehensive automated UI test system or performance benchmark suite. Establish detection parity using the pinned upstream grammar and representative fixtures; record differences due to explicit traversal/product choices. The Go reference is development-only, not a runtime dependency.

| Unit-test focus | Representative cases |
|---|---|
| Registry | YAML parse/validation, duplicate keys, extension compatibility, canonical IDs, invalid catalog retention, unresolved external references |
| Scanner | Each upstream detection mechanism, nested aggregation, exclusions and incomplete results, evidence privacy |
| Recommendation | Additive mappings, combination-first, registry ordering, duplicate canonical entries, dismissals, partial availability, no inferred matches |
| Core | Link ownership, global and project scopes, collision safety, preset reason retention, target sync, unregister choices |
| Lifecycle | Advisory dependencies/conflicts, local reuse, partial failure and retry, safe cleanup, update policy isolation |
| Compiler | Deterministic generation, fixture failures, invalid schema handling, correct wire JSON |

Manual desktop test script: first launch and skip/reopen wizard; add/approve/refresh/reorder registry; register nested project; scan and inspect evidence; install all or selected mapped skills; check global/project and unmanaged status; dismiss/restore; remove only managed links; edit and synchronize presets; change global targets and approve pending changes; fail one independent operation and retry; remove source registry with optional cleanup; offline library management; settings export/import; Registry Studio create/open/clone/form edit/import/fixture test/review/push/publish; verify Linux AppImage and update behavior. Record results and defects in release checklist. Source-level audits, manual smoke tests and fixtures are sufficient for selected test scope; targeted extra tests can be added when defects warrant them.

### 18.2 Application repository CI

Adapt the existing `.github/workflows/ci.yml`, `.github/actions/setup-rust/`, `config/ci/`, `config/xtask.json`, `crates/xtask/`, and `scripts/quality-gates.sh`; do not replace them with an unrelated pipeline. The template includes broader checks than the interview's minimum product tests. Keep existing applicable lint/security/documentation checks, reviewing template-only jobs, sample-specific assertions, heavy benchmark/evaluation jobs and path filters rather than silently disabling quality controls. Benchmark suites and automated UI coverage are not new product release requirements. Any policy changes must be explicit and reviewed. Use `.config/nextest.toml` for repository-native test execution.

On pushes and pull requests: workspace formatting (`cargo fmt --all -- --check`), Clippy, compilation and unit tests on Linux x86-64; headless compiler and GPUI build checks. Determine correct Kitter/GPUI system dependencies and compatible Rust toolchain during baseline audit. Commit Cargo.lock and use locked builds. On stable release tag, automated build and package:

| Artifact | Target |
|---|---|
| Nosey Desktop AppImage | Linux x86-64 |
| `nosey` CLI executable | Linux x86-64 |
| `nosey-registry` compiler | Linux x86-64, compatible with GitHub Actions runner |
| Checksums/authenticated release metadata | All artifacts |
| License notices and release documentation | All applicable artifacts |

Publish to `rigerc/nosey-rs` GitHub Releases. Adapt the existing tag-driven `.github/workflows/release.yml`, `dist-workspace.toml`, `release.toml`, `VERSION`, and `scripts/propagate-version.sh` as one consistent release system. Remove macOS/Windows targets from the dist configuration, replace `./target/release/sample-app --version` and any stale package or artifact names, and explicitly package the two standalone binaries plus the desktop AppImage. Add or integrate an AppImage packaging job rather than assuming cargo-dist generates AppImages. Keep version propagation and release preflight checks; ensure tag, shared workspace version and `VERSION` agree. Use an authenticated release manifest and checksums for updater verification. The updater must use `rigerc/nosey-rs` release metadata, not template or `rigerc/nosey` endpoints. Private builds may use workflow artifacts; no public pre-release tags. Test real AppImage launch on documented supported Linux environments, filesystem access, Git credentials, symlinks and updates. No `.deb`, `.rpm`, Flatpak, macOS/Windows packaging, ARM64 artifacts or public dev releases. Linux distribution baseline, graphics/Wayland/X11 compatibility and runtime dependencies must be validated; do not claim all distributions work without testing.

Repository-native validation commands for implementation (not run in this documentation-only pass):

```bash
./scripts/doctor.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --locked
cargo nextest run --workspace
cargo test --workspace --doc --all-features
./scripts/quality-gates.sh
```

After product crates exist, verify the headless build independently with `cargo build -p nosey-registry-compiler --locked`, then check the GPUI build with `cargo build -p nosey-desktop --locked`. These commands require the audited toolchain and Linux system dependencies. Release preflight should also run `bash scripts/propagate-version.sh --check`; adapt the existing release-tier gates before tagging v1.0.0.

### 18.3 Registry and Action CI

Official registry PR checks: validate YAML, local reference semantics, fixtures, licensing/provenance and deterministic compile. Reviewed merges deploy single `ai-catalog.json` to Pages. Scheduled upstream checks open PRs, not direct data publications. Action repository tests integration with the pinned compiler, input validation, failure handling and successful Pages workflow on a private/test setup. The compiler, Action and schema compatibility matrix must be explicit. Use private development artifacts or CI builds until the stable release rather than requiring public alpha/beta releases for development testing.

### 18.4 Private development and single release

All intermediate milestones are private engineering checkpoints, not public v0.x or alpha/beta releases. Coordinate the three repositories: privately validate application, compiler, Action and registry publication; establish final public URLs, keys/namespace and license rights; publish the stable compiler and compatible Action release before deploying the official catalog and releasing the app against the production endpoint. Private GitHub Pages availability depends on account/repository settings; if necessary use an authorized private/test build artifact or local fixture endpoint before public launch. Do not publish migrated data prematurely. One stable public release v1.0.0 after all gates pass, with coordinated compatible artifacts and registry availability.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 17. Security and failure handling](17-security-and-failure-handling.md) · [Next: 19. Phased implementation backlog](19-phased-implementation-backlog.md)
