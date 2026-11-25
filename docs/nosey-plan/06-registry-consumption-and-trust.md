[Plan index](../nosey-implementation-plan.md) · [Previous: 5. Registry specification and interoperability](05-registry-specification-and-interoperability.md) · [Next: 7. Official registry migration and maintenance](07-official-registry-migration-and-maintenance.md)

## 6. Registry consumption and trust

### 6.1 Global registry management

The built-in official endpoint is preconfigured and enabled by default; users may disable or reorder it. Custom registries are added by URL and explicitly approved after reviewing identity, source and validation status. Registry settings and priorities are global only. No project-provided registry activation or overrides.

The Registries screen supports add, inspect, trust, enable/disable, reorder, refresh, view last successful refresh, diagnose errors, and remove. Removing a registry preserves installed/downloaded skills by default and offers optional, approved cleanup. Removing or disabling the official registry does not let another registry claim its reserved namespace.

### 6.2 SDK discovery and refresh

Use `ai_catalog_discover::discover_with` for explicitly requested URL/domain discovery (direct, HTTP Link, HTML link, well-known URI). The SDK default fetcher does not block private addresses. Supply a Nosey-owned `Fetcher` using the same bounded HTTPS transport as verification and acquisition: validate DNS results and actual connection destinations including redirects; reject private, loopback, link-local and other non-public addresses, downgrade redirects and credential leakage; enforce response/redirect/time limits. Discovery is not approval or semantic validation. Show the resolved URL and identity before explicit approval; store the resolved URL, not an implicit auto-discovery subscription. Retain original served bytes and base URL for relative references, signatures and atomic cache storage. Do not follow external nested catalogs automatically.

### 6.3 Refresh and availability state

Use complete-document fetch, validation and atomic cache replacement. Evaluate configurable TTL while desktop runs and when registry-dependent CLI functionality executes. Manual refresh bypasses TTL. Proposed initial TTL: 24 hours. Preserve the last valid snapshot on failed refresh; with connectivity available, use it with explicit stale-data age and warning. No snapshot means registry unavailable. Never activate invalid documents.

```text
unconfigured → added/pending approval → approved/enabled → fetching
  → valid/current → expired → fetching
  → stale (last valid retained) | unavailable (no valid snapshot)
  → disabled or removed
```

Offline policy is deliberately restricted: Library and installation management using already-local content continue to work; scanning, recommendation evaluation and Explore are unavailable without connectivity even if a cache exists. Preserve cached results and state, but do not expose them as fresh online discovery. Define a connectivity predicate that distinguishes general network reachability from one failed registry; do not equate a registry timeout with global offline status. Avoid requiring an undisclosed third-party telemetry endpoint. Exact connectivity probe and UI treatment of saved historical scans are implementation decisions to settle in Phase 0. No project file content is sent to any server.

### 6.4 Trust

Parse original bounded bytes using `ai_catalog_trust::parse_json` to reject duplicate members, then `ai_catalog::parse_slice` and `ai_catalog_validate::validate_with`. Require `is_valid` in addition to any reported conformance level; map stable diagnostic codes, paths and severity into Nosey errors. Apply Nosey extension, namespace, source and safety validation separately. For signed documents, call `ai_catalog_validate::verify` on the original served bytes, with `Options::with_base_url`, explicitly configured `with_authorized_operators` and a bounded safe `Fetch`. Never reserialize a signed document for verification. Distinguish SDK `Verified`, `Failed` and `Unverifiable`; failed asserted signatures or digests block activation, unverifiable assertions remain visibly unverified and must not satisfy any configured trust requirement. Check the actual fetched artifact with `ai_catalog_trust::verify_digest` independently at installation. Do not infer trusted status from signature presence, structural validity or operator endorsement. Retain last valid snapshot on failure. No SDK network calls on the GPUI thread.

Explicit approval on addition is required for custom registries; imported settings do not transfer approvals. Validate base catalog and extensions, check available digests, and display verified/unverified claims separately. An invalid asserted signature is a validation failure, not equivalent to an unsigned catalog. The previously agreed official signing baseline is retained pending key/domain selection; mandatory signatures for all custom registries are excluded. Never equate a signature or quality score with skill safety. A changed source identity requires fresh approval. Registry trust does not grant executable code access or authorize project modifications.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 5. Registry specification and interoperability](05-registry-specification-and-interoperability.md) · [Next: 7. Official registry migration and maintenance](07-official-registry-migration-and-maintenance.md)
