[Plan index](../nosey-implementation-plan.md) · [Previous: 11. Projects, global targets and presets](11-projects-global-targets-and-presets.md) · [Next: 13. Desktop and onboarding](13-desktop-and-onboarding.md)

## 12. Pending changes and operations

### 12.1 Shared change-preview model

Use one change-planning service for recommendations, direct selections, cleanup, preset synchronization and target synchronization. The plan is an actionable preview, not an automatic dependency solver or reconciliation daemon.

```rust
pub struct ChangePreview {
    pub id: ChangeId,
    pub cause: ChangeCause,
    pub scope: ChangeScope,
    pub artifact: ArtifactId,
    pub target: PathBuf,
    pub action: ChangeAction,
    pub advisory_warnings: Vec<Warning>,
    pub blockers: Vec<Blocker>,
    pub expected_state_fingerprint: String,
}

pub enum ChangeAction {
    InstallLink,
    UnlinkManaged,
    UpdateLibrarySkill,
    RemoveUnusedLibrarySkill,
}
```

The exact types and persistence fields are implementation proposals. Use stable IDs and revalidate actual filesystem state, ownership, target settings and source integrity immediately before apply. Do not execute a stale approved plan against changed targets; regenerate preview and obtain approval where scope changed. Group compatible actions sharing a download so the same artifact is acquired once. Distinguish advisory warnings from blocking errors. For global/library automatic skill updates, the user-approved global policy supplies authorization only within its declared scope; all project link changes still require direct user approval.

### 12.2 Contextual and centralized review

The project Recommendations/Installed Skills screens, Presets and Settings expose relevant pending changes. Global Pending Changes aggregates additions, removals and synchronization changes, with affected project, agent, path, cause, source, warning and status. Support selecting one, multiple or all eligible changes with a shared confirmation. Show stale or blocked entries but do not execute them. Never queue a hidden automatic apply job.

### 12.3 Partial failure behavior

Continue independent operations after a failure. Roll back incomplete individual changes where possible, preserve successful operations and skip dependent changes that cannot proceed. Stage downloads and metadata safely; remove partial downloads/links if failed; record accurate ownership only after verification. Report individual status and enable retry of failed items using a fresh validation/preview. No global all-or-nothing rollback, persistent undo engine or comprehensive transaction audit system. Retain bounded recent activity and minimal recovery metadata required for crash-safe writes.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 11. Projects, global targets and presets](11-projects-global-targets-and-presets.md) · [Next: 13. Desktop and onboarding](13-desktop-and-onboarding.md)
