[Plan index](../nosey-implementation-plan.md) · [Previous: 15. CLI and standalone compiler](15-cli-and-standalone-compiler.md) · [Next: 17. Security and failure handling](17-security-and-failure-handling.md)

## 16. Storage, settings and background tasks

### 16.1 Persistence

Retain Kitter's existing file-based persistence mechanisms and library layout, renamed to Nosey's own application-data namespace. Do not introduce SQLite or another database. Phase 0 audits actual persisted files, serializers, update atomicity and concurrent access behavior; use those mechanisms rather than inventing a second storage subsystem. YAML is the preferred format for new human-facing configuration and settings interchange; do not convert working Kitter-native files solely for cosmetic consistency. Published AI Catalog cache remains original JSON so standard signature verification is reproducible. No Nosey-owned project-local settings or lockfiles.

| Record | Ownership / lifecycle |
|---|---|
| Global settings | Central Nosey state, editable through Settings |
| Registry config and priority | Central state, explicit enabled flag and approved binding |
| Registry trust approvals | Local user-owned trust store; never portable through import |
| Catalog snapshot | Original validated JSON, fetch/validation metadata and age |
| Project registrations and scan results | Central per-project state, structured evidence and completeness |
| Recommendation dismissals | Central per-project + canonical ID keys |
| Library entries | Existing Kitter layout, extended canonical identity/source metadata |
| Link ownership and install reasons | Central managed-link records, rechecked against filesystem |
| Preset relationships | Central state, independently tracked install reasons |
| Pending changes | Computed from saved desired/actual state; persist enough for retry |
| Recent activity | Bounded central history |
| Registry authoring files | Author-selected local Git working tree outside managed application state |

Guard state against concurrent CLI/desktop writes. Use existing locking primitives where reliable, atomic file replacement, bounded temporary staging and targeted rollback. Define lock ordering to avoid deadlocks between library and project operations. Recover abandoned temporary directories conservatively; never infer ownership from directory names alone. In-memory indexes may be rebuilt from validated snapshots rather than persisted to a database.

### 16.2 Settings exchange

Example portable settings schema (illustrative; preserve existing Kitter settings internals):

```yaml
schema_version: 1
registries:
  cache_lifetime_hours: 24
  sources:
    - id: official
      url: 'https://example.github.io/nosey-official-registry/ai-catalog.json'
      enabled: true
      priority: 1
agents:
  enabled: [codex, claude-code]
  custom_project_paths: [.custom/skills]
scan:
  additional_exclusions: []
  max_depth: 20
  max_file_size_mib: 5
  timeout_seconds: 60
updates:
  application: notify
  skills: notify
```

Export/import global preferences, registries and order, agent targets, custom paths and update policies as YAML. Validate import and preview changes; registry trust is not imported, and newly introduced registries require fresh approval. Target settings changes generate pending synchronization rather than immediately rewriting links. Exclude credentials, approvals, projects, scan results and skill contents. Do not silently apply an imported automatic-update setting without making its scope clear to the user. No full backup/restore or sync service.

### 16.3 Background processing

Run startup and periodic asynchronous checks only while Nosey Desktop is running. Registry refresh observes the configurable cache TTL; application and skill update checks obey independent global policies; tracked upstream repositories are checked periodically and proposed changes await approval. Proposed cadence: evaluate due tasks on startup and at a bounded periodic tick, with retry/backoff; tune exact intervals during implementation. Manual mode does not perform unauthorized background update application. Do not automatically scan projects or apply pending link changes. No daemon, tray process or systemd service. CLI evaluates registry freshness on demand for registry-dependent workflows and exits when done.

### 16.4 Application updater

Three independent application update modes: manual, notification-only and automatic. Proposed default: notification-only. Check stable GitHub Releases while running according to the policy. Download compatible Linux x86-64 artifacts into a temporary location, verify release provenance/integrity before replacement, check write permissions, and safely replace on restart or another tested safe transition. Preserve user data and a recoverable executable if replacement fails. Do not invoke unreviewed scripts or escalate privileges automatically. AppImage self-update is supported only for a user-writable installation path; report unsupported/managed locations and offer a manual update. Define how a standalone CLI binary is updated (proposal: manage only a verified user-writable binary installed by Nosey; otherwise notify). Artifact authentication, signing key/release metadata distribution and replacement mechanics are release-blocking implementation details, not already verified features.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 15. CLI and standalone compiler](15-cli-and-standalone-compiler.md) · [Next: 17. Security and failure handling](17-security-and-failure-handling.md)
