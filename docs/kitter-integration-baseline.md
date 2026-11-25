# Selective Kitter source reuse

Kitter v0.2.6 commit [19711db](https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2)
is the verified source reference (archive tree 2850d10f64ec6b3e1ff0f250666e8cc90cb60b25).
Nosey is its own application. Do not vendor, fork, or require a separate Kitter
build or executable. Reuse useful code inside Nosey's existing crate boundaries.

| Reference | Disposition |
| --- | --- |
| src/config.rs | Adapt project activity ordering; Nosey owns state and storage. |
| src/effective_skills/scanner.rs | Direct-child manifest discovery adapted in nosey-core; later provider profiles only as needed. |
| src/directory_link.rs | Read-only symlink inspection adapted; no link following or mutation. |
| src/library.rs, src/project/install.rs | Consider acquisition, collision and symlink planning primitives when implementing the Nosey library. |
| src/agent_plugins/*, src/effective_skills/* | Reuse useful target and visibility logic behind Nosey policy; no external CLI bridge. |
| src/ui/* | Select useful GPUI components for the project-centric Nosey UI; no requirement to retain upstream navigation. |
| src/adoption.rs | Excluded. |
| src/source/remote.rs, src/platform.rs | Do not copy unsafe process termination or platform FFI into Nosey. Implement only needed behavior under existing safety rules. |

The actual upstream source and license remain the provenance for adapted
pieces. The Apache-2.0 notice is retained. The old standalone baseline
workflow and audit JSON bridge are removed from this PR. Existing Nosey
CI now includes master and remains the validation mechanism.

## Native development commands

```sh
cargo run -p nosey-cli -- --state /absolute/central/state.json register /absolute/project
cargo run -p nosey-cli -- --state /absolute/central/state.json inspect-target /absolute/project --target .agents/skills
cargo run -p nosey-cli -- --state /absolute/central/state.json unregister /absolute/project
cargo test -p nosey-core
```

This development command lists direct regular SKILL.md candidates and reports
symbolic links without following them. It requires explicit project registration
and a relative target; never reads manifest contents or mutates the tree.
Candidate paths are not canonical catalog IDs, verified ownership, or full
agent-effective coverage. Unregister changes only central state. Directory
enumeration is bounded; metadata checks are not race-proof against hostile
concurrent filesystem mutation. Native tests and full quality gates have not
run here. The GPUI, registry and scanner/recommendation work remains subject
to Nosey's own acceptance criteria, not Kitter's behavior.
