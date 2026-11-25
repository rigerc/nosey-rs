[Plan index](../nosey-implementation-plan.md) · [Previous: 7. Official registry migration and maintenance](07-official-registry-migration-and-maintenance.md) · [Next: 9. Mapping, recommendations and exploration](09-mapping-recommendations-and-exploration.md)

## 8. Scanner and detection

### 8.1 Scan contract

A scan is explicitly started by the user and operates on one manually registered project root. Recursively inspect permitted nested applications and packages, but produce one aggregate technology inventory and one project-wide recommendation set. Preserve relative evidence paths so nested applications remain distinguishable in results. Do not define nested installation scopes or inheritance. Combination conditions are evaluated against the aggregate project inventory; two technologies in different nested directories can therefore satisfy a project-level combination.

The scanner implements every existing Nosey-9 detection mechanism in Rust, with no new operators, AI inference or technology-specific code. The registry supplies all technology definitions and detection rules. Generic parser selection and match execution are code; knowledge of which manifests, dependencies, extensions or patterns identify a technology is registry data. Use the pinned Go source and fixtures to establish exact rule semantics. No manual technology overrides. Correct errors by changing registry rules.

```text
Manual scan request
  → verify connectivity and obtain active registry snapshot set
  → compile supported, valid rule set
  → traverse root with global exclusions and safety limits
  → evaluate generic Nosey-9-equivalent match mechanisms
  → aggregate detections and structured evidence by technology ID
  → save result and completeness, fingerprint rule set and scan settings
  → evaluate explicit mappings and combinations
  → display inventory and recommendation groups
```

### 8.2 Proposed Rust result shape

```rust
pub struct ScanResult {
    pub project_id: ProjectId,
    pub rule_set_digest: String,
    pub scan_settings_digest: String,
    pub technologies: Vec<DetectedTechnology>,
    pub diagnostics: Vec<ScanDiagnostic>,
    pub completeness: ScanCompleteness,
}

pub struct DetectedTechnology {
    pub technology_id: TechnologyId,
    pub versions: Vec<VersionObservation>,
    pub evidence: Vec<Evidence>,
}

pub struct Evidence {
    pub relative_path: PathBuf,
    pub rule_id: RuleId,
    pub registry_id: RegistryId,
    pub matched_dependency: Option<String>,
    pub observed_version: Option<String>,
    pub match_kind: MatchKind,
}
```

Use typed observations, not arbitrary content snippets. Distinguish manifest version constraints from resolved installed versions. Retain multiple versions where detected; do not invent one project-wide version. No raw file contents or credential values in stored evidence or logs. A rule cannot grant permission to read outside the project root. No network transmission of project files.

### 8.3 Global scan preferences and safety

| Setting | Proposed default (benchmark before finalizing) |
|---|---|
| Maximum traversal depth | 20 levels |
| Maximum file size | 5 MiB |
| Scan timeout | 60 seconds |
| Global extra exclusion globs | Empty |

Default exclusions cover dependency trees, generated build artifacts, caches and VCS internals. Example candidates: `node_modules`, `target`, `.git`, `.venv`, `dist`, `build`; confirm practical behavior and whether to honor `.gitignore` during the audit. Users can add global exclusion globs. Exclusions are not per-project and cannot be replaced by untrusted rules. Respect root containment; reject escaping symbolic links, special files and pathological inputs. Bound regex/parser work even if the user raises normal limits. Run scanning off the UI thread with progress and cancellation.

A scan records complete, incomplete, cancelled or failed state. Hitting a configured limit or unexpected access failure is visible as an incomplete result rather than silent absence of technologies. A deliberately excluded path is not an unexpected failure; surface configured exclusions in diagnostics. Incomplete or stale scans must not generate new removal suggestions based on missing detections. Failed/cancelled scans should preserve prior results with an explicit status rather than silently marking them current.

### 8.4 Selective invalidation

| Change | Required behavior |
|---|---|
| Mapping/combination changed | Reevaluate mappings automatically using saved detections, online only |
| Registry priority changed | Reorder and deduplicate; update selected entry and metadata |
| Skill metadata changed | Update presentation; no rescan solely for metadata |
| Detection rule/technology definition changed | Invalidate affected scans and request manual rescan |
| Registry enable/disable/removal | Reevaluate resolvable references and mark scans stale when effective detection rules change |
| Scan preferences changed | Mark existing scans outdated; no automatic filesystem scan |
| Project file change | No watcher; user manually initiates scan |

Fingerprint the effective rule set and scan configuration; compare with last successful scan. Mapping reevaluation may use valid surviving observations, but label stale inputs and suppress destructive suggestions. No reevaluation or registry discovery in offline mode. No automatic link mutation under any invalidation path.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 7. Official registry migration and maintenance](07-official-registry-migration-and-maintenance.md) · [Next: 9. Mapping, recommendations and exploration](09-mapping-recommendations-and-exploration.md)
