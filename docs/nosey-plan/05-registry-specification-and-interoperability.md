[Plan index](../nosey-implementation-plan.md) · [Previous: 4. Rust workspace and architecture](04-rust-workspace-and-architecture.md) · [Next: 6. Registry consumption and trust](06-registry-consumption-and-trust.md)

## 5. Registry specification and interoperability

### 5.1 Contract

Adopt AI Catalog as the base wire specification, with two Nosey-owned extensions: catalog-level technology intelligence and entry-level skill metadata. The prior plan reviewed v0.9 RC; exact version, media types, canonical identifiers, extension field semantics, digest and signature profiles must be checked against the pinned normative revision. The application consumes conformant JSON catalogs; YAML is the editable authoring format. Standard consumers can ignore Nosey extensions. No proprietary alternative wire protocol.

- **Intelligence extension:** registry identity, extension version, technologies, detection rules, mappings, combinations, source repositories, assessment/provenance metadata as needed.
- **Skill extension:** install name, Git acquisition details or bundle metadata, declared dependencies/conflicts, technology references, source availability and assessment references.
- Extension keys use a controlled reverse-domain namespace (temporary examples `io.example.nosey.intelligence` and `io.example.nosey.skill`).
- Extensions independently use semantic versions. Accept compatible minor/patch additions; skip unsupported major extensions with warnings while preserving valid standard entries. Never activate an unknown rule operator merely because the enclosing extension version is compatible.
- Only Agent Skills are installable in v1.0. Unsupported artifact types must not be passed to the skill installer.
- Official registry and all Studio-generated registries publish a single self-contained `ai-catalog.json`; no generated modular child-catalog topology. Standard federation handling, if normative for accepted documents, is bounded and trust-gated; external nested catalog support is not a new mandatory authoring feature.

### 5.2 Authoring repository

```text
nosey-official-registry/
├── registry.yaml
├── technologies/         # one YAML per technology
├── rules/                # one YAML per rule
├── mappings/             # one YAML per mapping
├── combinations/         # one YAML per combination
├── skills/               # one YAML per skill
├── repositories/         # one YAML per source repository
├── assessments/          # one YAML per assessment
├── provenance/           # one YAML per provenance record
├── fixtures/             # sample project data and expected detections
├── schemas/              # pinned authoring/extension schemas
├── .github/workflows/publish.yaml
├── .gitignore
├── LICENSE
└── README.md
```

Source files are authoritative. Generated JSON is a build artifact, deployed to GitHub Pages rather than committed to the authoring branch. Use deterministic traversal and serialization, reject duplicate keys, custom executable tags and unsafe alias expansion, bound input sizes and nesting, and reject authoring paths outside the working tree. Validate entity types, uniqueness, reference format, required fields, licensing/provenance, source paths and supported detection operators. Distinguish unresolved external references from invalid local references.

### 5.3 Illustrative authoring schemas

The following examples express the proposed data model, not a frozen grammar or actual migrated records. Namespaces, source URLs, hashes and license examples are placeholders. Detection operator spellings and semantics must be derived from the pinned Nosey-9 audit, not invented as new detection capabilities.

`registry.yaml`:

```yaml
schema_version: '1.0.0'
ai_catalog_version: '0.9'
id: official
name: Nosey Official Registry
license: CC-BY-4.0
host:
  displayName: Nosey Official Registry
  identifier: registry.example.org
includes:
  technologies: 'technologies/**/*.yaml'
  rules: 'rules/**/*.yaml'
  mappings: 'mappings/**/*.yaml'
  combinations: 'combinations/**/*.yaml'
  skills: 'skills/**/*.yaml'
  repositories: 'repositories/**/*.yaml'
  assessments: 'assessments/**/*.yaml'
  provenance: 'provenance/**/*.yaml'
```

`technologies/react.yaml`:

```yaml
id: react
name: React
category: framework
aliases: [reactjs]
```

`rules/react-package.yaml` (illustrative existing-manifest-match shape; confirm upstream equivalent):

```yaml
id: react-package
technology: 'official:technology.react'
match:
  type: json-property
  files: [package.json]
  paths: [/dependencies/react, /devDependencies/react]
  operator: exists
```

`mappings/react.yaml`:

```yaml
id: react
technology: 'official:technology.react'
skills:
  - 'https://registry.example.org/ids/react-development'
```

`combinations/react-typescript.yaml`:

```yaml
id: react-typescript
technologies:
  - 'official:technology.react'
  - 'official:technology.typescript'
skills:
  - 'https://registry.example.org/ids/react-typescript-patterns'
```

`repositories/example.yaml`:

```yaml
id: example
kind: git
url: 'https://github.com/example/skills'
```

`skills/react-development.yaml`:

```yaml
id: react-development
identifier: 'https://registry.example.org/ids/react-development'
name: React Development
install_name: react-development
description: Example registry metadata, not a verified skill.
artifact_type: application/agent-skills+md
artifact_url: 'https://example.org/immutable/react-development/SKILL.md'
source:
  repository: 'official:repository.example'
  path: skills/react-development
  revision: '<commit-sha>'
  tree_digest: 'sha256:<tree-digest>'
license: NOASSERTION
technologies: ['official:technology.react']
tags: [react, frontend]
dependencies: []
conflicts: []
availability: available
verification_status: unreviewed
```

`assessments/react-development.yaml`:

```yaml
id: react-development-review
skill: 'https://registry.example.org/ids/react-development'
methodology: Example assessment methodology.
criteria: [clarity, relevance]
score: null
assessed_at: '2026-11-25'
reviewed_revision: '<commit-sha>'
security_review: not-performed
```

`provenance/react-development.yaml`:

```yaml
id: react-development-origin
entity: 'https://registry.example.org/ids/react-development'
origin:
  repository: 'https://github.com/rigerc/nosey-registry'
  revision: '<pinned-commit>'
  path: '<original-record-path>'
redistribution_status: pending-review
license: NOASSERTION
```

Production publication rejects placeholder identifiers, digests, invalid revisions and unresolved licensing for official records. Metadata and source acquisition can have distinct digest scopes: the digest of a `SKILL.md` representation is not the digest of the complete installable skill tree. Verify available standard artifact digests and full-content integrity when declared. A Git source pinned to a specific commit is an acquisition reference, not per-project library version pinning.

### 5.4 Compiled catalog illustration

```json
{
  "specVersion": "0.9",
  "host": {"displayName": "Nosey Official Registry", "identifier": "registry.example.org"},
  "extensions": {
    "io.example.nosey.intelligence": {
      "schemaVersion": "1.0.0",
      "registryId": "official",
      "technologies": [{"id": "official:technology.react", "name": "React", "category": "framework"}],
      "rules": [],
      "mappings": [{"id": "official:mapping.react", "technology": "official:technology.react", "skills": ["https://registry.example.org/ids/react-development"]}],
      "combinations": []
    }
  },
  "entries": [{
    "identifier": "https://registry.example.org/ids/react-development",
    "type": "application/agent-skills+md",
    "url": "https://example.org/immutable/react-development/SKILL.md",
    "displayName": "React Development",
    "tags": ["react"],
    "extensions": {
      "io.example.nosey.skill": {"schemaVersion": "1.0.0", "installName": "react-development", "dependencies": [], "conflicts": []}
    }
  }]
}
```

The JSON is an unsigned illustrative example with incomplete source information. It is not a conformance fixture or deployable catalog. The compiler emits exact standard fields required by the pinned specification and preserves unknown compatible extensions where necessary. The prior Level 2 publication/Level 3 verification target remains a design baseline, subject to confirming normative requirements; do not claim publisher authentication for operator-indexed third-party entries without evidence. Support native AI Catalog signature verification (previously reviewed JCS/JWS/ES256 profile), not an invented detached signature protocol. Explicit user approval, not signature presence, is the selected custom-registry trust policy.

### 5.5 Identity and priority

- Canonical artifact identifier is the only catalog deduplication key. Preserve eligible publisher identifiers; do not fabricate publisher-authorized identities. Operator-controlled identifiers must remain stable. Different identifiers remain different skills even if URLs, content or names coincide.
- Highest-priority enabled registry supplies the selected entry for duplicate canonical identifiers. Retain provenance from other occurrences for inspection. Never silently fall back to a different source if an approved acquisition fails integrity validation.
- Installation name/path is not canonical identity. Collision checks remain separate.
- Official registry is authoritative for canonical technologies. Custom registries may reference official identifiers, augment their rules and mappings, or define namespaced new technologies. They cannot silently redefine official technologies. Bind each namespace to an approved registry identity, not arbitrary display names.
- Cross-registry mappings can reference skills from ordinary AI Catalog registries without intelligence extensions. External unresolved references are warnings; independently valid entities remain usable. Skip affected rules or mapping associations and reevaluate when registry configuration changes. Do not auto-add dependencies or fetch unapproved registries.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 4. Rust workspace and architecture](04-rust-workspace-and-architecture.md) · [Next: 6. Registry consumption and trust](06-registry-consumption-and-trust.md)
