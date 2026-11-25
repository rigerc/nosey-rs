[Plan index](../nosey-implementation-plan.md) · [Previous: 14. Registry Studio and publishing](14-registry-studio-and-publishing.md) · [Next: 16. Storage, settings and background tasks](16-storage-settings-and-background-tasks.md)

## 15. CLI and standalone compiler

### 15.1 Essential user CLI

The desktop is the primary management interface. Add only essential discovery and lifecycle commands; retain compatible existing Kitter CLI capabilities unless explicitly superseded. Actual command names and collision handling must be verified against the pinned Kitter command tree. The following is a proposed user-facing contract:

```bash
nosey project add /path/to/project
nosey project list
nosey project remove /path/to/project  # prompt to keep or unlink managed links
nosey scan --project /path/to/project
nosey recommend --project /path/to/project
nosey install <canonical-artifact-id> --project /path/to/project
nosey install --recommended --project /path/to/project
nosey install <canonical-artifact-id> --global
nosey uninstall <canonical-artifact-id> --project /path/to/project
nosey update <canonical-artifact-id>
nosey update --all
```

No new full registry administration/authoring, publishing, global settings or automation-oriented CLI suite. Use shared core services and the same connectivity, trust, warning and safety rules as desktop. Installation/removal commands preview changes and obtain explicit approval. A noninteractive mode, if retained from Kitter, must require an explicit approval flag rather than implicitly mutating state. Structured YAML/JSON output is compatible with the prior plan where practical; it is not a requirement to create an independently stable automation API. Preserve upstream exit behavior where compatible; return nonzero for failed/partial operations and useful diagnostics. No TUI.

### 15.2 Standalone registry compiler

Separate from the user-facing CLI and packaged without GPUI. Proposed executable contract:

```bash
nosey-registry validate --registry .
nosey-registry test --registry .
nosey-registry compile --registry . --output dist/ai-catalog.json
```

Validation includes schema, semantic, licensing/provenance and supported rule checks. Fixture tests invoke the same scanner as Studio. Compile deterministically to the pinned AI Catalog version; output only valid artifacts or fail with a nonzero exit code. External references cannot always be resolved during independent build: distinguish invalid local references from valid external identities; warn on unresolved optional external references and prevent invalid emitted records. Require the compiler to produce consistent output for identical inputs and version pins. The Action downloads a specific released compiler; Studio calls the same library code directly. Signing and verification follow the pinned standard if configured, with secrets injected through a secure build environment rather than committed.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 14. Registry Studio and publishing](14-registry-studio-and-publishing.md) · [Next: 16. Storage, settings and background tasks](16-storage-settings-and-background-tasks.md)
