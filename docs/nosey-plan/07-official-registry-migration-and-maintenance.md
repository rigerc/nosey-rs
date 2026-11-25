[Plan index](../nosey-implementation-plan.md) · [Previous: 6. Registry consumption and trust](06-registry-consumption-and-trust.md) · [Next: 8. Scanner and detection](08-scanner-and-detection.md)

## 7. Official registry migration and maintenance

1. Verify data redistribution and CC BY 4.0 relicensing rights for `rigerc/nosey-registry` and pin its source commit.
2. Inventory source technology definitions, detection mechanisms and rules, mappings, combinations, skill sources, assessments, and provenance. No other seed catalogs.
3. Build a one-time Rust migration tool and transformation report. Normalize records into individual YAML entity files, resolve explicit references, assign/preserve canonical identities, retain upstream attribution and source coordinates.
4. Reject or quarantine unresolved source paths, licensing, unauthorized identities or invalid records. Reconcile migrated counts by source category; do not silently drop entries.
5. Validate registry, run detection fixtures and compile conformant `ai-catalog.json`. Deploy via the new registry's pinned Action workflow.
6. After initial migration maintain the new registry independently. No ongoing dependency on or sync from the original registry dataset.

Admission is tiered: require technical validity (`SKILL.md`, available appropriate acquisition source and metadata), acceptable licensing and provenance. Keep assessment and security-review statuses independent. A technically valid unreviewed skill may be listed; never imply certification. Registry assessments are author-maintained information only.

GitHub Actions monitors tracked original skill sources, detects revision/availability changes and opens reviewable PRs. Maintainers also curate manually and review community contributions. Approved merges trigger publication. New or removed skills are proposed, not automatically published. Registry-owned descriptions, tags, mappings, dependencies, conflicts and assessments are preserved on refresh; existing entries only synchronize source revisions and availability after review.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 6. Registry consumption and trust](06-registry-consumption-and-trust.md) · [Next: 8. Scanner and detection](08-scanner-and-detection.md)
