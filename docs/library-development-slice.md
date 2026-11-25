# Phase 4 local library and change-review slice

Kitter v0.2.6 ([19711db](https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2)) is a source reference for central library metadata and install-plan revalidation, not a runtime dependency. This slice adapts those patterns, rejects replacement of occupied paths, and retains Apache-2.0 attribution.

On Linux, choose separate existing central and project directories. The library directory must be private (0700) and outside the registered project tree. For an explicitly chosen synthetic skill with valid SKILL.md frontmatter, use:

```sh
mkdir -m 700 /absolute/central/library
mkdir -p /absolute/project/.agents/skills
cargo run -p nosey-cli -- --state /absolute/central/state.json register /absolute/project
cargo run -p nosey-cli -- --state /absolute/central/state.json import-fixture /absolute/central/library urn:nosey:fixture:sample /absolute/fixture-skill --approve-fixture
cargo run -p nosey-cli -- --state /absolute/central/state.json preview-install /absolute/central/library /absolute/project .agents/skills urn:nosey:fixture:sample
# Inspect the exact destination/source/status and copy the displayed fingerprint.
cargo run -p nosey-cli -- --state /absolute/central/state.json apply-install /absolute/central/library /absolute/project .agents/skills urn:nosey:fixture:sample --approve <fingerprint>
cargo run -p nosey-cli -- --state /absolute/central/state.json audit-links /absolute/central/library
```

Local fixtures use a reserved development identifier prefix; a local tree hash is not an SDK digest or publisher endorsement. Acquisition copies only bounded regular files and directories, validates a restricted UTF-8 Agent Skill frontmatter, stages privately, verifies staged content and publishes one active copy atomically without replacement. No scripts are executed. The user supplies the existing target directory; no agent target configuration is inferred. Install revalidates the stored content and reviewed destination, uses no-follow descriptor-relative parent opens and atomic non-replacing symlink creation. A pending journal precedes creation; interrupted operations remain visible and cannot be silently adopted. Independent install reasons are retained. Unregister keeps link journals and library content; audits show detached ownership. No unlink, update, cleanup, global installation, external acquisition, preset synchronization or automatic installation is enabled.

Limitations: cooperative exclusive locking is scoped to the selected private library, not the existing project-state file or multiple different libraries. Private central storage is trusted and must not be concurrently modified outside Nosey. Descriptor-relative access reduces symlink races but concurrent directory renames, mounts, hardlinks, inode reuse or a hostile same-user actor can still undermine provenance or observation; no destructive mutation is implemented. A pending record after a failed create requires explicit recovery tooling not yet supplied. Frontmatter is intentionally conservative, and staged bytes are not a complete production Agent Skill compliance validator. File and metadata sync do not constitute tested power-loss recovery. No build, test execution, quality gate or Beads reconciliation is claimed; continue feature development separately from build remediation per maintainer instruction.
