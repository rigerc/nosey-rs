[Plan index](../nosey-implementation-plan.md) · [Previous: 8. Scanner and detection](08-scanner-and-detection.md) · [Next: 10. Library, skill lifecycle and unmanaged skills](10-library-and-skill-lifecycle.md)

## 9. Mapping, recommendations and exploration

### 9.1 Deterministic evaluation

1. Read the aggregate set of detected canonical technology identifiers from the applicable scan.
2. Evaluate every resolvable explicit individual mapping and matching combination from enabled registries.
3. Include all distinct canonical skill IDs; never infer relevance from tags, text, scores or source similarity.
4. Resolve duplicate catalog entries independently using highest-priority enabled registry. Preserve matching reasons from all contributing mappings, even when the chosen artifact representation is from another registry.
5. Order combination matches before individual-only matches, then by mapping registry priority. An artifact with both types of match occupies the combination group. Proposed final tie-break: case-insensitive display name, then canonical ID. Registry priority does not suppress distinct lower-priority recommendations.
6. Compare each candidate to effective managed/unmanaged skills for configured targets and apply persisted project dismissals. Attach informational token estimates, advisory dependency and conflict warnings, source availability and target coverage.
7. Present results. No context limits, quality weighting or automatic selection/install based on a score.

Example:

| Mapping source | Match | Skill |
|---|---|---|
| Personal (priority 1) | React | React Best Practices |
| Official (priority 2) | React + TypeScript | React TypeScript Patterns |
| Official (priority 2) | TypeScript | TypeScript Development |

All three are recommended. React TypeScript Patterns comes first due to the combination-first rule. React Best Practices then precedes TypeScript Development by registry priority. Canonical deduplication and independent highest-priority entry selection apply if the same skill occurs more than once.

### 9.2 Recommendation groups

| Group | Definition and actions |
|---|---|
| Missing | One or more configured targets lack the skill; allow individual, bulk or install-all selection |
| Managed | Nosey-managed effective availability; inspect target and provenance details, offer explicit unlink where appropriate |
| Unmanaged | Existing manually/externally installed effective skills; inspect only, no adoption or mutation |
| Dismissed | Per-project persistent dismissal; collapsed by default, individual restore |

Partially installed skills expose per-target coverage, rather than treating project-level presence as proof of availability for all agents. Exact presentation of mixed managed/unmanaged target coverage is a UI detail; use a single canonical recommendation row with target-level status. A global skill satisfies a target only when that agent exposes it to the project. Custom project-relative targets are evaluated separately.

For unmanaged skills, identify a canonical match only when supported by reliable existing provenance or equivalent authoritative metadata; do not silently equate names or content with canonical IDs. If identity cannot be established, display the unmanaged skill and warn about possible destination collisions. This is a conservative implementation default to refine in the Kitter effective-skill audit.

Dismissals are local records keyed by project ID and canonical artifact ID, survive rescans/registry updates and do not change links. Restoring a dismissed skill does not install it. Dismissed skills remain discoverable through Explore and manually installable. Install-all excludes dismissed, already-satisfied and unavailable targets; the confirmation screen makes the selection explicit.

### 9.3 Removal candidates

When a valid complete scan and applicable mapping evaluation show a formerly recommended Nosey-managed project skill is no longer mapped, present it as a candidate for unlinking. Do not remove it automatically. Do not propose removing a skill merely because it was manually installed without a mapping, global, unmanaged, required by another applied preset, or because the scan is incomplete/stale. A dismissed recommendation is not a removal instruction. Preserve independent install reasons and show them to users.

### 9.4 Explore

Read-only complete registry explorer across enabled catalogs: Skills, Technologies, Rules, Mappings, Combinations and Source Repositories, with assessment/provenance views. Search names, IDs, descriptions, aliases, tags and relevant metadata using fuzzy partial matching; combine entity type, registry, technology, category and assessment filters. Unknown/not-applicable facets must be represented correctly. Use an in-memory index rebuilt on validated catalog changes, no database or remote search service. Show origin registry and cross-reference details. Unmapped technologies appear in Scan without coverage statistics or an automated contribution workflow.

Skill detail shows metadata, source, identity, license, technologies, dependencies/conflicts, assessments, availability, installed targets and rendered `SKILL.md` preview. Preview may retrieve a remote document without installing; for locally available skills use local content. Safe Markdown rendering, no script execution, no arbitrary active embeds or automatic disclosure of local data. No complete source file browser or revision-diff viewer.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 8. Scanner and detection](08-scanner-and-detection.md) · [Next: 10. Library, skill lifecycle and unmanaged skills](10-library-and-skill-lifecycle.md)
