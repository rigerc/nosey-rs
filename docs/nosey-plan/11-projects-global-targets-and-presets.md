[Plan index](../nosey-implementation-plan.md) · [Previous: 10. Library, skill lifecycle and unmanaged skills](10-library-and-skill-lifecycle.md) · [Next: 12. Pending changes and operations](12-pending-changes-and-operations.md)

## 11. Projects, global targets and presets

### 11.1 Project management

Manual registration via directory picker or CLI; canonicalize and deduplicate root paths, report inaccessible/missing directories. A registered project has a local stable ID, path, display metadata, most recent scan, installation/selection state, dismissals and pending changes. No Nosey-specific config, lockfile or metadata files are written to project roots. Agent files, skill links and retained Kitter instruction features may write only their explicitly requested managed content.

Unregister dialog offers keep existing links or unlink all Nosey-managed project links. Neither option affects global installs, unmanaged skills or library content. If unlinking partially fails, retain actionable ownership/retry state rather than silently forgetting failed targets.

### 11.2 Global targets and global skill foundation

Configure agent targets globally using Kitter's agent plugins. All projects inherit the same enabled target set. Preserve plugin extensibility, but new simple custom locations are reusable project-relative paths only (e.g. `.custom/skills`). Validate they cannot escape through absolute paths, `..` or symlink redirection. Existing built-in agent global paths continue to follow their supported scope; custom relative targets do not imply global absolute targets. Global installations use supported agent-global locations; project installations use each registered root and its supported agent/custom relative locations.

Enable/disable/modify target settings → derive affected global and project changes → show pending changes → explicit synchronization. Old links remain unchanged until approved. Do not remove a link that has unmanaged ownership or another retained installation reason. Settings change desired target configuration, not actual link state.

### 11.3 Linked presets

Reuse Kitter preset UI and storage, extending it with a relationship between preset applications and target scope. Apply a preset globally or to a project wherever supported by retained functionality; initial application requires approval. Editing preset membership creates pending additions/removals; no automatic propagation. Track independent reasons (manual selection, preset A, preset B, accepted recommendation) to avoid unlinking content still needed by another reason. Synchronization removes a link only when it is managed and no retained reason requires it. Project recommendation mapping is separate from preset selection.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 10. Library, skill lifecycle and unmanaged skills](10-library-and-skill-lifecycle.md) · [Next: 12. Pending changes and operations](12-pending-changes-and-operations.md)
