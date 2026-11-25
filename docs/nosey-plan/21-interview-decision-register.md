[Plan index](../nosey-implementation-plan.md) · [Previous: 20. Release acceptance and open items](20-release-acceptance-and-open-items.md)

## 21. Interview decision register

All 100 decisions are listed in interview order. Later answers supersede earlier decisions when they narrow or clarify scope. Q41 records the revised selection. Q58 clarifies Q10 rather than contradicting additive recommendations. Explicitly superseded original plan features are not release requirements.

| Q | Topic | Final decision |
|---:|---|---|
| 1 | Positioning | A — Intelligent Agent Skills manager |
| 2 | Audience | A — Individual developers |
| 3 | Experience | A — Project-first |
| 4 | Registration | A — Manual directories only |
| 5 | Monorepos | A — Project-wide aggregate inventory and single collection |
| 6 | Detection | A — All technology rules registry-driven |
| 7 | Rule capabilities | E — Nosey-9 mechanism parity, no new operators |
| 8 | Technology identity | C — Official registry canonical authority |
| 9 | Recommendations | A — Explicit registry mappings only |
| 10 | Competing recommendations | B — Registry priority; clarified in Q58 |
| 11 | Registry scope | A — Global only |
| 12 | Official registry | B — Built-in, enabled by default, configurable |
| 13 | Cache refresh | C — Configurable TTL and manual refresh |
| 14 | Library | A — Central shared copy and symbolic links |
| 15 | Skill updates | E — Global manual, notify, automatic policy |
| 16 | Project skill changes | A — Explicit manual approval |
| 17 | Agent targets | C — Global, plus custom locations |
| 18 | Custom locations | A — Reusable project-relative paths |
| 19 | Project persistence | A — Central state, no Nosey files in projects |
| 20 | Storage | A — Retain Kitter, no database |
| 21 | Registry authoring | C — Integrated desktop Registry Studio |
| 22 | Publishing | C — Integrated GitHub publishing |
| 23 | Authentication | C — Existing Git credentials, SSH, helpers |
| 24 | Trust | A — Explicit approval on addition |
| 25 | Dependencies/conflicts | B — Advisory only |
| 26 | Install selection | E — Individual, multiple and install-all, shared confirmation |
| 27 | Removal | E — Removal recommendations plus library cleanup |
| 28 | Scan triggers | A — Manual only |
| 29 | Scan presentation | B — Detailed inventory and evidence |
| 30 | Explore | C — Complete registry explorer |
| 31 | Acquisition | C — Git repositories and AI Catalog bundles |
| 32 | Offline | A — Locally available skills management only |
| 33 | CLI | A — Essential commands |
| 34 | Navigation | B — Project-centric tabs |
| 35 | Kitter features | A — Preserve except explicitly superseded |
| 36 | Assessments | B — Registry-maintained, informational |
| 37 | Authoring files | A — Individual YAML entity files |
| 38 | Distribution | A — Single ai-catalog.json |
| 39 | Publishing workflow | C — GitHub Actions builds and publishes |
| 40 | Hosting | A — GitHub Pages |
| 41 | Compiler delivery | E (revised) — Shared Rust library, lightweight compiler and reusable Action |
| 42 | Repositories | B — Three repos: app, official registry, Action |
| 43 | Rust organization | C — Seven domain-oriented workspace crates |
| 44 | Platforms | Linux only |
| 45 | Packaging | B — Desktop AppImage and standalone CLI |
| 46 | Architecture | A — x86-64 only |
| 47 | App updates | E — Manual, notify or automatic |
| 48 | Onboarding | D — Optional wizard, skip and reopen |
| 49 | Kitter migration | A — Clean installation, no migration |
| 50 | Portability | B — Global YAML settings import/export |
| 51 | Official maintenance | E — Automated monitoring proposals and reviewed community contributions |
| 52 | Admission | E — Technical/licensing validity, tiered assessment/verification |
| 53 | Unmapped tech | A — Display only |
| 54 | Preview | B — Metadata and rendered SKILL.md |
| 55 | Unmanaged skills | B — Detect/display, no adoption |
| 56 | Unmanaged recommendations | D — Separate missing, managed and unmanaged groups |
| 57 | Identity | A — Canonical AI Catalog ID only |
| 58 | Mapping priority | B — Additive distinct skills with priority ordering |
| 59 | Combinations | C — Include all, combinations first |
| 60 | Token estimates | B — Informational only |
| 61 | Installation scopes | C — Global foundation plus project additions |
| 62 | Global foundation | D — Manual selection and presets |
| 63 | Presets | C — Linked, manual synchronization |
| 64 | Target changes | C — Preview and manual synchronization |
| 65 | Change review | C — Contextual and centralized Pending Changes |
| 66 | Failure | D — Continue independent operations, targeted rollback and retry |
| 67 | History | B — Bounded recent activity |
| 68 | Notifications | A — In-app only |
| 69 | Background tasks | C — Startup/periodic while desktop open |
| 70 | Catalog compatibility | C — Standard catalogs and cross-registry mappings |
| 71 | External references | B — Graceful degradation with diagnostics |
| 72 | Schema versions | B — SemVer compatibility, preserve standard entries |
| 73 | Refresh failures | B — Last valid snapshot with stale warning when online |
| 74 | Freshness | C — Selective invalidation, manual rescan for changed rules |
| 75 | Unregister | C — User choice keep links or unlink managed project links |
| 76 | Registry removal | C — Preserve skills; optional approved cleanup |
| 77 | Exclusions | C — Global defaults and configurable extra globs |
| 78 | Scan limits | C — Global configurable depth, size and timeout |
| 79 | Evidence | B — Structured relative paths/rules/dependency/version observations |
| 80 | Overrides | A — No manual technology overrides |
| 81 | Dismissals | B — Persistent per-project skill dismissal |
| 82 | Restore dismissed | A — Collapsible Dismissed group |
| 83 | Search | D — Fuzzy search and faceted filters |
| 84 | Studio editing | A — Structured entity forms |
| 85 | Rule testing | D — Local projects and reusable fixtures |
| 86 | Skill authoring | E — Metadata and existing repository/frontmatter import |
| 87 | Bulk import | E — Tracked repositories with selective/bulk import |
| 88 | Source sync | B — Automatic detection, manual approval |
| 89 | Metadata ownership | B — Registry wins; sync revisions and availability only |
| 90 | Unavailable skills | D — Retain installations, mark unavailable, optional cleanup |
| 91 | Draft management | D — Git working tree, review/discard/commit/push |
| 92 | Authoring repos | C — Local open or remote clone to selected directory |
| 93 | Initialization | B — Complete publishing template |
| 94 | Licensing | D — Separate registry and original skill licensing |
| 95 | Application license | B — MIT new code, upstream notices preserved |
| 96 | Registry license | B — CC BY 4.0 subject to redistribution rights |
| 97 | Action license | A — MIT |
| 98 | Testing | A — Core unit tests and manual desktop workflows; fixtures retained |
| 99 | CI/CD | C — Standard Rust CI and tagged automated releases |
| 100 | Release | A — Private development, single stable public v1.0 release |

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 20. Release acceptance and open items](20-release-acceptance-and-open-items.md)
