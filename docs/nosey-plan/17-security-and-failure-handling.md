[Plan index](../nosey-implementation-plan.md) · [Previous: 16. Storage, settings and background tasks](16-storage-settings-and-background-tasks.md) · [Next: 18. Testing, CI, packaging and release](18-testing-ci-packaging-and-release.md)

## 17. Security and failure handling

| Risk | Required behavior |
|---|---|
| Untrusted catalog or YAML | Schema and semantic checks; bounded parsing, reject executable tags; approval on addition |
| Tampered catalog/artifact | Use SDK strict JSON parsing, semantic validation, original-byte signature verification and exact-byte digest checks; reject failed assertions, distinguish unverifiable claims, retain last valid snapshot |
| False publisher claims | Distinguish registry operator assertions, publisher identity and review status |
| Malicious skill instructions | Safe preview, never execute skill content during acquisition/install, explicit install approval |
| Unsafe detection rules | Audited declarative operators only, root containment, bounded file reads/regex/parser work |
| Secret exposure | Local scan, structured allowlisted evidence, no raw credentials/content in logs or exported settings |
| Path/archive traversal | Reject escaping paths, symlink tricks, special files, oversized extraction and unsafe directory collisions |
| Link corruption | Revalidate ownership, no unmanaged mutation, safe create/unlink and targeted rollback |
| Concurrent writes | Shared locking, expected-state validation, atomic metadata updates |
| Source/registry disappears | Preserve downloaded content and installations, mark availability, optional explicit cleanup |
| Credential misuse | Existing helpers/SSH host verification, host-bound credentials, no embedded secrets |
| Compromised build/publishing | Pin action/compiler versions and verify artifact integrity; least-privilege Pages jobs; reviewed changes |
| Incomplete scan | Visible diagnostics; suppress unsafe removal proposals |
| Application update failure | Verify before replacement, preserve running version and local data, no silent privilege escalation |

Maintain clear diagnostic messages without exposing sensitive details. Do not equate user trust of a registry with authorization to install every skill. No telemetry or remote code analysis is necessary for the selected product. Security invariants are required behavior even though a comprehensive automated security suite is not selected.

The SDK does not provide Nosey's network security policy. Supply safe fetch adapters to discovery and verification, enforce HTTPS and public destinations even after DNS resolution/redirects, limit response sizes/timeouts and keep I/O off the UI thread. No automatic nested-catalog traversal. Keep catalog authorization, publisher endorsement, integrity and skill safety as separate states. Use reviewed signing primitives only for production signing; do not reimplement the SDK's cryptographic verification.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 16. Storage, settings and background tasks](16-storage-settings-and-background-tasks.md) · [Next: 18. Testing, CI, packaging and release](18-testing-ci-packaging-and-release.md)
