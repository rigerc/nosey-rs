[Plan index](../nosey-implementation-plan.md) · [Previous: 9. Mapping, recommendations and exploration](09-mapping-recommendations-and-exploration.md) · [Next: 11. Projects, global targets and presets](11-projects-global-targets-and-presets.md)

## 10. Library, skill lifecycle and unmanaged skills

### 10.1 Central library

Preserve Kitter's centralized library model. One active downloaded copy per canonical artifact identifier; links from project and agent-global targets point to that shared copy. Use Kitter's storage layout and safe unique paths, extending metadata with canonical ID, source registry/acquisition information, revision/digest, availability and ownership. A central skill can have many managed link targets. No per-project version pinning, historical revision library, independent copy installations or Windows junctions.

A skill's source revision may be pinned in the catalog to retrieve verified content, and the library records the currently downloaded revision. This is not a requirement to maintain multiple selectable local versions. A temporary staging/rollback directory for a safe update is allowed; it is not user-visible version storage.

### 10.2 Acquisition

Support both Git-based retrieval and AI Catalog installable artifact bundles. Preserve compatible Kitter acquisition mechanisms. Resolve the selected canonical entry, validate the source and fetch into a temporary staging directory. For Git, use approved repository, path and revision, credential helpers and host verification. For bundles, validate content type, declared digest where present, archive path safety, extraction size and required Agent Skill structure. Never treat arbitrary non-skill AI Catalog artifacts as installable skills. Verify integrity against the representation actually fetched; verify full-tree digest separately if defined. Never execute `SKILL.md` instructions, scripts, hooks or build commands as part of fetching, previewing or installation.

If content is already local, reuse it instead of downloading again. Source disappearance or registry removal does not delete local content. Offline, permit installations using existing library content; online acquisition is required for uncached skills. A missing source prevents fetching, not linking an existing intact copy.

### 10.3 Install/unlink flow

```text
Select canonical skill(s) and global/project scope
 → enumerate configured compatible targets
 → inspect existing paths and ownership
 → display dependencies/conflicts as warnings and token estimates
 → display source and target plan, request explicit approval
 → fetch/validate/stage missing library content
 → create safe managed symbolic links
 → persist ownership and update effective inventory
 → report individual results
```

Dependencies and conflicts are advisory only. A missing dependency does not get auto-added, and a declared conflict does not automatically prevent an informed user from proceeding. Integrity failure, unsupported artifact type, unsafe path, external ownership or a filesystem collision remains a blocking operational error for that change. Show other independent valid changes when possible.

Unlink only Nosey-managed symbolic links, after verifying the actual link and recorded ownership. Never overwrite a real directory, file or unmanaged link; do not follow a link when deleting it. Do not delete shared library content as part of project unlink. Use path containment checks immediately before mutation, handle symlink races and concurrent CLI/desktop actions, and make operations idempotent.

### 10.4 Central skill updates

Global policy modes: manual, notifications, automatic. Proposed default: notifications. While desktop is running, check according to the selected policy and background cadence. An approved automatic update affects all links to the shared copy, including every project and global target; communicate this when enabling the policy. No project link additions/removals occur through this update policy.

Fetch and verify replacement content before switching the stable library path. Preserve existing content and usable links if retrieval/validation fails. Use a staged safe replacement with targeted recovery; on Linux ensure concurrent operations cannot read or install from a half-updated directory. Avoid auto-switching to a different source solely because registry priority changed; surface acquisition identity/provenance changes for user review. This is a proposed trust safeguard, not per-skill update policy functionality.

### 10.5 Unmanaged detection and cleanup

Retain effective-skill auditing, but remove Kitter adoption actions. Existing unmanaged files or links are read-only to Nosey. Detect broken or orphaned managed links, disconnected source records, unavailable sources and unused central library entries. Library Cleanup previews dependencies on registered projects, global installs, applied presets and retained links from unregistered projects. Require explicit approval for removal; refuse to delete a shared copy if any remaining managed reference requires it. If a project was unregistered without unlinking, retain enough minimal detached-link ownership metadata to avoid deleting content still in use; the project is not treated as registered. Unknown external references should be flagged rather than assumed safe to delete.

When a source registry is removed, offer optional cleanup and preserve existing content by default. Unavailable upstream skills retain their entries/status and existing local installations; optional registry-entry removal and library cleanup are separate, explicit operations.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 9. Mapping, recommendations and exploration](09-mapping-recommendations-and-exploration.md) · [Next: 11. Projects, global targets and presets](11-projects-global-targets-and-presets.md)
