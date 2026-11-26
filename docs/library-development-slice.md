# Phase 4 library and change-review slice

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

Local fixtures use a reserved development identifier prefix; a local tree hash is not an SDK digest or publisher endorsement. Acquisition copies only bounded regular files and directories, validates a restricted UTF-8 Agent Skill frontmatter, stages privately, verifies staged content and publishes one active copy atomically without replacement. No scripts are executed. Install revalidates the stored content and reviewed destination, uses no-follow descriptor-relative parent opens and atomic non-replacing symlink creation. A pending journal precedes creation; interrupted operations remain visible and cannot be silently adopted. Independent install reasons are retained. Unregister keeps link journals and library content; audits show detached ownership. Automatic installation remains outside this slice.

Use these subcommands with the same `--state` argument. Review each preview and pass its fingerprint to the matching apply command.

```text
preview-recover <library> <absolute-destination>
apply-recover <library> <absolute-destination> --approve <fingerprint>
preview-unlink <library> <absolute-destination> [--reason <reason>]
apply-unlink <library> <absolute-destination> [--reason <reason>] --approve <fingerprint>
preview-update <library> <fixture-id> <fixture-source>
apply-update <library> <fixture-id> <fixture-source> --approve <fingerprint>
```

Recovery can drop an unresolved record when the destination is absent under its original parent, or confirm an exact pending symlink after checking the artifact digest and parent identity. It changes only the journal. Dropping a journal is the only recovery action for pending records created before digest tracking. The destination must be absent under the original parent.

Unlink removes one reason. The CLI defaults to `manual` and also accepts `recommendation` and `preset:<id>`. When the last reason is removed, Nosey verifies the original parent, symlink inode and target bytes immediately before descriptor-relative unlink. This also works when the library content is missing or damaged. The final `unlinkat` cannot be conditional on inode, so another process writing the same directory can race the last check. A crash after unlink can leave a journal for the now-missing symlink.

Update stages and verifies the replacement and atomically exchanges it with the existing library object. The fixture ID and manifest name stay the same, and managed links continue to reference the same path. Updates require kernel and filesystem support for `RENAME_EXCHANGE`. An unsupported exchange returns a capability error and preserves the active copy.

## Batch change plans

One shared planning service reviews installs, unlinks, bundle acquisitions and library updates together. Write the requests to a JSON file (at most 256 changes, 1 MiB):

```json
{"changes": [
  {"action": "install_link", "project": "/absolute/project", "target": ".agents/skills",
   "artifact": "urn:nosey:fixture:sample", "reason": {"kind": "manual"}},
  {"action": "unlink_managed", "destination": "/absolute/project/.agents/skills/old",
   "reason": {"kind": "preset", "id": "web"}},
  {"action": "update_library_skill", "artifact": "urn:nosey:fixture:sample",
   "source": "/absolute/fixture-skill-v2"}
]}
```

```text
plan <library> <requests.json> [--only <change-id>]... [--catalog <snapshot> --registry-id <id>]
apply-plan <library> <requests.json> [--only <change-id>]... [--catalog <snapshot> --registry-id <id>] --approve <plan-fingerprint>
```

`plan` previews every request without changing anything. Duplicate requests collapse to one change, and each change gets a stable ID. A request that cannot be previewed, or whose destination is occupied, unowned or needs recovery, is listed with a `blocker` and is never executed. The plan fingerprint covers every listed change, so approval applies to exactly the reviewed set; use `--only` to plan and approve a subset. `apply-plan` refuses the whole batch before any mutation if the fresh plan no longer matches the approval.

Approved changes run in order: unlinks, bundle acquisitions, installs and library updates, so replacing content never invalidates a reviewed link. Each change is revalidated immediately before it runs. Results are reported per change in plan order as `applied`, `blocked`, `stale` (state changed after review, possibly by an earlier change in the same batch) or `failed`. A failure does not stop later changes, and successful changes are kept. `apply-plan` exits non-zero when any change is stale or failed; retry by planning again.

Supply the same local snapshot and registry ID when reviewing and applying a
catalog-aware plan. The skill extension supplies canonical dependency and conflict
references. The plan compares them with eligible installs and managed links in the
same scope, plus unmanaged symlinks that point directly to a verified
local library artifact. Missing dependencies, declared conflicts and unresolved
references appear in `warnings` and are covered by the approval fingerprint.
Dependencies are never queued automatically, and advisory warnings leave
eligibility unchanged.

For a skill already in the central library, `estimated_tokens` is the UTF-8 byte
count of the `SKILL.md` body after frontmatter, divided by four and rounded up.
The estimate is informational and uses local content only. Recommendation callers
can supply `Library::estimate_skill_tokens` through
`evaluate_with_token_estimates`, returning no estimate for unavailable content.

## Configured targets and inventory

Central state defaults to the shared `.agents/skills` target. Use
`set-targets --agents --claude --custom .custom/skills` to replace the
desired configuration. This command leaves existing links in place. Every
project inherits the same project-relative targets. Global scope uses the
enabled built-in locations under the explicitly supplied home.

A plan request with `"action": "install_scoped"` accepts
`"scope": {"kind": "project", "root": "/absolute/project"}` or
`"scope": {"kind": "global", "home": "/absolute/home"}`. It expands into one
preview per enabled compatible target. Each target directory must already
exist. The explicit `install_link` request remains available.

Run `inventory LIBRARY --project ROOT --home HOME` to include global links
in project coverage, or `inventory LIBRARY --home HOME` for global inventory.
The JSON groups managed links by artifact ID and readable unmanaged
manifests by name. It lists other unmanaged entries, partial coverage,
journal entries needing recovery, and journals outside the enabled targets.
Inventory never follows unmanaged symlinks.

## Target synchronization and linked presets

`sync-targets LIBRARY` previews changes for registered projects and any homes
recorded in managed journals. It carries each installation reason to enabled
targets and queues removal of reasons from disabled targets. If a replacement
is blocked or fails, the old target keeps the affected reason.

```text
set-preset ID NAME [--member ARTIFACT]...
edit-preset ID [--add ARTIFACT]... [--remove ARTIFACT]...
apply-preset LIBRARY ID (--project ROOT | --home HOME)
unapply-preset LIBRARY ID (--project ROOT | --home HOME)
sync-presets LIBRARY [--approve FINGERPRINT]
sync-targets LIBRARY [--approve FINGERPRINT]
```

Preset commands record desired definitions and applications in central state.
Application and removal commands also show the pending preset plan. Review
`sync-presets` after editing membership. A removed member or application
releases only its preset reason; manual, recommendation and other preset
reasons retain their links. Legacy state loads with empty preset fields.
Both synchronization commands print a JSON plan by default. Pass its fingerprint
with `--approve` to apply the reviewed changes. Both use the shared plan size limit.

## Catalog bundle acquisition

Select a validated local AI Catalog snapshot and its registry identity. Preview
shows the source URL, media type, declared digest, release and any intact cached
copy. Apply requires the fingerprint printed by preview.

```text
preview-acquire <library> <catalog.json> --registry-id <registry> <artifact>
apply-acquire <library> <catalog.json> --registry-id <registry> <artifact> --approve <fingerprint>
```

A batch can include the corresponding request and one or more `install_link` or
`install_scoped` requests using the same artifact identifier. Use `install_name` to review the
exact link destinations before downloading. Acquisition checks that the
validated manifest name matches before publishing.

```json
{"action": "acquire_library_skill", "install_name": "sample",
 "source": {"artifact": "example-registry:skill.sample", "registry": "example-registry",
 "url": "https://registry.example.org/sample.zip", "media_type": "application/agent-skills+zip",
 "digest": "sha256:<hex>", "revision": "1.0"}}
```

The library preserves the catalog identifier and registry provenance. An intact
matching active copy is reused without fetching and stays installable offline.
An existing copy with different provenance or content requires an explicit
update. Registry bundle replacement and source availability refresh are
future work.

HTTPS Agent Skill ZIP bundles must contain a root `SKILL.md`. The transport
caps response bytes at 10 MiB. After verifying the SDK digest over the original
bytes, extraction caps the tree at 4,096 paths and 64 MiB, with 8 MiB per file
and 32 path components. Extraction rejects traversal, links and special
entries and executes no content. Git acquisition is unavailable pending an
implementation that safely isolates Git configuration and bounds checkout
resources. Other bundle media types and inline data are unsupported.

## Limitations

The library lock coordinates cooperating operations within one private library. Keep project targets stable while applying changes. Linux does not offer a conditional unlink by inode, so an external writer can still race the final check. Private central storage must be protected from concurrent external writes. Frontmatter validation is intentionally conservative, and power-loss recovery has not been tested.

## Library cleanup

```text
cleanup-report <library>
preview-removed-source <library> <stored-source-path>
preview-remove-unused <library> <fixture-id> [--external-references-checked]
apply-remove-unused <library> <fixture-id> --external-references-checked --approve <fingerprint>
```

Source removal review preserves the copies it lists. Every recorded link or journal, including those retained after unregistering a project, blocks removal. Use `--external-references-checked` only after ruling out unmanaged references to the exact copy. The library cannot enumerate arbitrary external paths.

In a plan, use `{"action":"remove_unused_library_skill","artifact":"urn:nosey:fixture:sample","external_references_checked":true}`. Removal runs after link changes. If an unlink clears the final reference, preview removal again so the approval reflects the new inventory.
