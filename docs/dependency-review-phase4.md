# Dependency review — Phase 4 (2026-11-25)

Scope: PR #9 base/head 095f0709ec99327ba52078cd1386093fa179a538, six product crates and the workspace declarations they use. The supplied rust-crate-discovery skill was checked against its source, and its evaluator was attempted with `eval rustix --json`; outbound DNS failed. No fabricated numerical scores, download counts, grades or complete advisory verdicts. Manual review uses the skill's six health dimensions and separates API fit from health; public docs and named RustSec records below are selective evidence. No local Cargo or Beads CLI was available.

## Lockfile finding

The committed Cargo.lock contains none of nosey-core, nosey-scanner, nosey-registry or the other product packages; it omits declared ai-catalog SDK, reqwest, fancy-regex, hcl-rs, serde_yaml, sha2, semver, globset, walkdir, url and tokio. It is a template-era lockfile, not the product's resolved graph. Existing overlapping entries include rustix 1.1.6, regex 1.13.1, tempfile 3.27.0, serde 1.0.229, serde_json 1.0.154, toml 1.2.1+spec-1.1.0, thiserror 2.0.21, anyhow 1.0.104 and clap 4.6.7. None proves final product resolution. Regenerate against the reviewed toolchain; check all runtime and dev graphs. The current deny policy has no advisory ignores and restricts Git sources to the SDK; do not weaken it.

## Direct dependency decisions

| Dependency / declared range | Fit and decision | Review concern |
|---|---|---|
| serde 1, serde_json 1 | Retain standard models/JSON | Strict original-byte parsing remains SDK-owned; no hand-written JSON replacement |
| thiserror 2, anyhow 1 | Retain typed library and CLI errors | Do not leak paths, credentials or raw content in diagnostics |
| clap 4 | Retain existing CLI | Confirm argument parser and generated help |
| sha2 0.10, semver 1 | Retain hashing/version primitives | Local tree digest is not SDK digest; ecosystem constraints are not all SemVer |
| tempfile 3 | Retain staging | Same-filesystem publication, permissions, crash cleanup, no-clobber |
| rustix 1 | Retain descriptor-relative filesystem APIs | RUSTSEC-2026-0486 affects 1.0.0 through 1.1.5 xattr APIs; fixed in 1.1.6. Raise workspace minimum to 1.1.6 (caret range); current source does not use xattr APIs. No claim all transitives are clear. |
| walkdir 2, globset 0.4 | Retain single walk and rule globs | Test bounded iteration and exclusion semantics; path-based enumeration is not a security boundary |
| regex 1, fancy-regex =0.19.2 | Retain linear first and bounded fallback | [fancy-regex 0.19.2](https://docs.rs/crate/fancy-regex/0.19.2) exists; ECMAScript parity and operation/time budgets unverified |
| hcl-rs =0.19.8 | Retain parsed HCL AST | [0.19.8](https://docs.rs/crate/hcl-rs/0.19.8) exists, MIT OR Apache-2.0; pre-1.0 API stability and malicious nesting/resource budgets need tests |
| toml 1.1 | Retain typed TOML | Bound input; test TOML 1.1 semantics against legacy manifests |
| serde_yaml 0.9 | Replace before production | Deprecated; uses unmaintained unsafe-libyaml ([RUSTSEC-2024-0449](https://rustsec.org/advisories/RUSTSEC-2024-0449)); parser handles untrusted authoring, Compose and skill frontmatter |
| reqwest 0.13, url 2, tokio 1 | Retain restricted HTTP boundary | Verify rustls feature/transitive graph and current advisory fixes, including [RUSTSEC-2026-0348](https://rustsec.org/advisories/RUSTSEC-2026-0348); no TLS downgrade or proxy/redirect weakening |
| ai-catalog, ai-catalog-trust, ai-catalog-validate, ai-catalog-discover (Git rev 920ef70e1e5a602e142137f9ea6a1135fedbf110) | Retain required standard SDK | Git-only manual audit, pin and provenance, exact original-byte verification, normative spec revision, unknown transitives/lock |
| tracing, tracing-subscriber, proptest, insta, criterion | Existing workspace/template and testing facilities; retain as needed | Product vs dev/template scope must be established by resolved metadata; do not use their presence as justification to add runtime features |

Rustix advisory: https://rustsec.org/advisories/RUSTSEC-2026-0486.html. Fixed minimum is adopted in this PR, not a complete dependency update. Full direct/transitive advisories and licenses remain unresolved pending lockfile regeneration. Exact pins for HCL/fancy-regex diverge from the workspace caret-range policy; evaluate and normalize with parity tests rather than silently upgrading.

## Alternatives for Phase 4 and related gaps

| Problem | Candidate comparison | Recommendation |
|---|---|---|
| YAML parsing | [serde-saphyr 2.2.0](https://docs.rs/crate/serde-saphyr/2.2.0) supports typed Serde, duplicate-key policy and [budgets](https://docs.rs/serde-saphyr/latest/serde_saphyr/struct.Budget.html); [serde_norway](https://docs.rs/serde_norway/latest/serde_norway/) is closer to existing Value API but a libyaml fork; serde_yaml_ng retains unmaintained unsafe-libyaml. | Prioritize serde-saphyr proof of fit; typed DTOs for frontmatter/Compose/Actions and strict authoring, explicitly configure resource/alias limits and duplicate keys; compare fixtures before migration. Do not substitute serde_yml blindly ([RUSTSEC-2025-0068](https://rustsec.org/advisories/RUSTSEC-2025-0068), recheck current affected versions). |
| Git acquisition | Existing Git CLI/Kitter reference vs [gix](https://docs.rs/gix/latest/gix/) (Rust-native, credential helpers, fetch interruption, substantial pre-1.0 graph) vs [git2](https://docs.rs/git2/latest/git2/) (libgit2 bindings, native packaging and credential callbacks). | Reuse a constrained Git CLI for initial Linux fetch; no shell, isolated config, approved HTTPS/SSH host and explicit immutable revision, no checkout filters/hooks/submodule execution, safe tree-object export; allow trusted credential helpers by explicit policy. Prototype gix only if avoiding external Git is a product requirement; verify object integrity and transport policy. |
| Artifact bundles | [tar](https://docs.rs/tar/latest/tar/enum.EntryType.html) + flate2 vs zip vs general decompress helper. | Select exact SDK-supported representation first. Use a reviewed patched parser to read entries, not convenience extraction. tar advisory [RUSTSEC-2026-0067](https://rustsec.org/advisories/RUSTSEC-2026-0067.html) fixed in 0.4.45; zip advisory [RUSTSEC-2026-0546](https://rustsec.org/advisories/RUSTSEC-2026-0546.html) fixed in 9.0.0, unaffected below 7.2.0 for this advisory only. Other advisories still require review. Reject links, devices, absolute/parent paths, collisions, duplicate entries and resource expansion; verify SDK declared digest before extraction. Avoid decompress ([RUSTSEC-2026-0328](https://rustsec.org/advisories/RUSTSEC-2026-0328.html)). |
| Managed filesystem | Existing [rustix](https://docs.rs/rustix/latest/rustix/fs/index.html) vs [cap-std](https://docs.rs/cap-std/latest/cap_std/fs/struct.Dir.html). | Retain rustix for Linux no-follow opens, no-replace publication and recovery primitive tests. Cap-std is an alternative for later portable capability filesystem design, not a second abstraction now. No crate guarantees safe ownership-based unlink against arbitrary concurrent same-user actors. |
| State/locking | std File::try_lock, tempfile, existing JSON vs fs2 or database. | Keep std and JSON, one shared state/library lock and journal protocol; no extra lock crate or DB absent demonstrated need. |
| Token estimation | Kitter's effective-skill estimation versus [tiktoken-rs](https://docs.rs/tiktoken-rs/latest/tiktoken_rs/) or tokenizers. | Reuse Kitter's bounded estimate first, label approximation and visibility; evaluate tokenizer only for identified model/encoding and explicit data/feature costs. |
| ECMAScript parity | Existing regex/fancy-regex vs [regress](https://docs.rs/regress/latest/regress/), a JS syntax backtracking engine. | Keep bounded existing engine; compare regress against upstream fixtures, require enforceable operation/time budget before adoption. Do not confuse syntax fidelity with resource safety. |
| GPUI | Kitter's pinned GPUI vs existing workspace GPUI Kit conventions. | Separate desktop dependency audit when introduced; don't introduce a GUI stack to headless core. |
| Presets and pending changes | Existing Serde/state model vs a generic workflow framework. | Reuse typed reason sets and journal primitives; no separate workflow engine. |

No alternative dependency except the rustix minimum adjustment is adopted in this change. Candidate release, advisory, license, feature and benchmark data require live evaluation; the above is fit/security triage, not scored approval.

## Reproducible review commands

Run after network and the required Rust toolchain are available:

```sh
python3 .agents/skills/rust-crate-discovery/scripts/crate_eval.py compare serde-saphyr serde_norway serde_yaml_ng
python3 .agents/skills/rust-crate-discovery/scripts/crate_eval.py compare gix git2
python3 .agents/skills/rust-crate-discovery/scripts/crate_eval.py compare tar zip
python3 .agents/skills/rust-crate-discovery/scripts/crate_eval.py compare rustix cap-std
python3 .agents/skills/rust-crate-discovery/scripts/crate_eval.py compare fancy-regex regress
cargo generate-lockfile
cargo metadata --locked --format-version 1
cargo tree --locked -e features
cargo deny check
cargo audit
```

Record raw output and exact chosen versions. No score can override an applicable advisory or license failure. Preserve the approved development/build-remediation separation; do not claim release readiness.

## YAML migration decision

Adopt [serde-saphyr 2.2.0](https://docs.rs/crate/serde-saphyr/2.2.0), the stable release reviewed
against Rust 1.99. It declares Rust 1.89 and MIT OR Apache-2.0. The
[upstream repository](https://github.com/bourumir-wyngs/serde-saphyr) is active and unarchived
(GitHub reported a push on 2026-11-25). The standard library has no YAML parser, JSON does not
cover the YAML fixtures, and serde_norway retains the libyaml-derived parser design without
the same resource-budget API. The discovery evaluator ranked serde-saphyr 90/100 and
serde_norway 64/100. Its RustSec lookup was inconclusive, so the version-aware lockfile
audit and Cargo Deny advisory check were used for the resolved versions.

Only the `deserialize` feature is enabled. The direct graph adds granit-parser 2.2.0,
annotate-snippets 0.12.18, encoding_rs_io 0.1.8, num-traits 0.2.19 and smallvec 1.16.5,
alongside Serde. Serialization, comment capture, file inclusion and property
interpolation are disabled. All readers reject duplicate keys and anchor/alias events,
limit depth and scalar bytes, and treat merge keys as ordinary keys so providers
can retain their existing merge checks. Registry authoring decodes directly to its
existing typed models. Provider lookups retain a value tree to inspect heterogeneous
mappings and preserve their field-specific diagnostics. The provider tree uses JSON
values, so YAML collection keys and non-finite numbers are outside its supported model.
