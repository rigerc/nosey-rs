[Plan index](../nosey-implementation-plan.md) · [Next: 2. Repositories, branding and licensing](02-repositories-branding-and-licensing.md)

## 1. Product scope and workflows

### 1.1 Product definition

Nosey is an intelligent Agent Skills manager for individual developers. It will be a native Rust application derived from Kitter, hosted in the existing `rigerc/nosey-rs` workspace with a GPUI desktop application and an essential CLI, augmented with a registry-driven port of Nosey-9's detection mechanisms. It discovers project technologies, recommends explicitly mapped skills, and manages a shared skill library and symbolic-link installations across coding agents.

The primary interaction is project-first. Developers manually register projects, explicitly initiate scans, inspect results and approve skill changes. No automatic project registration, automatic project scans, automatic project installation reconciliation or AI-based recommendations.

```mermaid
flowchart LR
    A[Register project] --> B[Start scan manually]
    B --> C[Registry rules detect technologies]
    C --> D[Explicit mappings and combinations]
    D --> E[Review recommendations and effective skills]
    E --> F[Approve selected changes]
    F --> G[Shared library and symbolic links]
    G --> H[Inspect installed skills]
```

### 1.2 Scope

| In v1.0 | Out of v1.0 |
|---|---|
| Linux x86-64 GPUI desktop and essential CLI | macOS, Windows, ARM64, other architectures |
| Manually registered projects; project-wide aggregate scans | Automatic project discovery; nested installation scopes |
| Registry-driven detection with Nosey-9 mechanism parity | Built-in technology-specific rules, manual technology overrides, AI or programmable detectors |
| Explicit mappings and combination-first recommendations | Inferred associations, weighted scoring, context budgets |
| Central library; symlink installations; global foundation and project additions | Copy installs, per-project revision pinning, multiple active versions |
| Advisory dependencies/conflicts; explicit approval | Automatic dependency resolution, unattended project reconciliation |
| Standard and extended AI Catalog consumption | General-purpose AI artifact installation |
| Registry Studio, shared compiler, GitHub Action, GitHub Pages | Registry TUI, general-purpose skill authoring, integrated YAML editor |
| Kitter capabilities unless superseded | Adoption of unmanaged skills, Kitter data migration |
| Global settings, centralized local state | SQLite, project-local Nosey config/lockfiles, cloud sync |
| Offline management of locally available skills | Offline scanning, recommendations and Explore |
| Configurable application and library update policies | Always-running daemon, native notifications |

### 1.3 Core journeys

**First project:** optional wizard → official registry initialization → choose global agent targets and project-relative custom targets → register directory → user starts scan → inspect inventory → review missing recommendations → approve installation → inspect effective skills.

**Existing project:** select project → view saved scan status → manually rescan if needed → inspect missing, managed, unmanaged and dismissed groups → approve additions or removal candidates.

**Personal global foundation:** find a skill in Explore or Library → select individual global installs or apply a preset → preview links for global agents → approve. Global skills satisfy matching project recommendations where available for the same agent.

**Registry author:** open or clone a local Git working tree → edit structured entity forms → validate and test → review Git changes → commit and push with existing credentials → GitHub Actions compiles and deploys to Pages → registry refresh makes new data available.

---

[Plan index](../nosey-implementation-plan.md) · [Next: 2. Repositories, branding and licensing](02-repositories-branding-and-licensing.md)
