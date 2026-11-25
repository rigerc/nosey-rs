# Tool Adapters

Supported agents: **Claude Code, Codex, OpenCode, Pi**.

1. **Canonical contract** (`AGENTS.md`) — the single source of truth, max 80 LOC.
   Codex, OpenCode and Pi load it natively.
2. **Claude adapter** (`CLAUDE.md`) — imports `@AGENTS.md` and holds only Claude-specific deltas.
3. **Skills** (`.agents/skills/`) — read natively by Codex, OpenCode and Pi; Claude reads them
   through the `.claude/skills` symlink (`./scripts/setup-skills.sh`). Opt-in skills live in
   `.agents/skills-optional/` and are not auto-discovered.

## Adapter Manifest

Adapters are declared in `.agents/agent-adapters.toml` (contract, validation rules, adapters).
Only tools that need a file of their own get an `[[adapters]]` entry.

## Validation

```bash
cargo xtask agents validate                     # full validation pass
cargo xtask agents inventory --format markdown  # list registered adapters
cargo xtask agents check-context                # check context files exist
./scripts/validate-agent-entrypoints.sh         # CLAUDE.md + AGENTS.md length
```

CI runs these in the `validate-agents` job on every PR that touches agent files.

## Adding a New Adapter

Only needed if the tool cannot read `AGENTS.md` / `.agents/skills/` natively:

1. Create the entrypoint with `@AGENTS.md` as the first content line.
2. Add a `[[adapters]]` entry to `.agents/agent-adapters.toml` and the file to
   `scripts/validate-agent-entrypoints.sh`.
3. If it needs a skills symlink, add it to `scripts/setup-skills.sh` and `scripts/validate-skills.sh`.
4. Run `cargo xtask agents validate`.
