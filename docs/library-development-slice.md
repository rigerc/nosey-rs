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

Local fixtures use a reserved development identifier prefix; a local tree hash is not an SDK digest or publisher endorsement. Acquisition copies only bounded regular files and directories, validates a restricted UTF-8 Agent Skill frontmatter, stages privately, verifies staged content and publishes one active copy atomically without replacement. No scripts are executed. The user supplies the existing target directory; no agent target configuration is inferred. Install revalidates the stored content and reviewed destination, uses no-follow descriptor-relative parent opens and atomic non-replacing symlink creation. A pending journal precedes creation; interrupted operations remain visible and cannot be silently adopted. Independent install reasons are retained. Unregister keeps link journals and library content; audits show detached ownership. Global installation, external acquisition, preset synchronization and automatic installation remain outside this slice.

Use these subcommands with the same `--state` argument. Review each preview and pass its fingerprint to the matching apply command.

```text
preview-recover <library> <absolute-destination>
apply-recover <library> <absolute-destination> --approve <fingerprint>
preview-unlink <library> <absolute-destination> [--reason <reason>]
apply-unlink <library> <absolute-destination> [--reason <reason>] --approve <fingerprint>
preview-update <library> <fixture-id> <fixture-source>
apply-update <library> <fixture-id> <fixture-source> --approve <fingerprint>
```

Recovery can drop an unresolved record when the destination is absent under its original parent, or confirm an exact pending symlink after checking the artifact digest and parent identity. It changes only the journal. Pending journals created before digest tracking can be cleared if the destination is absent, but cannot be confirmed from target bytes alone.

Unlink removes one reason. The CLI defaults to `manual` and also accepts `recommendation` and `preset:<id>`. When the last reason is removed, Nosey verifies the original parent, symlink inode and target bytes immediately before descriptor-relative unlink. A crash after unlink can leave a journal for the now-missing symlink.

Update stages and verifies the replacement and atomically exchanges it with the existing library object. The fixture ID and manifest name stay the same, and managed links continue to reference the same path. Updates require a filesystem with Linux rename-exchange support.

The library lock coordinates cooperating operations within one private library. Keep project targets stable while applying changes. Linux does not offer a conditional unlink by inode, so an external writer can still race the final check. Private central storage must be protected from concurrent external writes. Frontmatter validation is intentionally conservative, and power-loss recovery has not been tested.
