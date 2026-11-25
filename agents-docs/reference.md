# Agent Reference (read on demand)

Details moved out of `AGENTS.md` to keep the always-loaded contract small.
`AGENTS.md` wins on any conflict.

## Supported agents

| Agent | Instructions | Skills |
|-------|--------------|--------|
| Codex | `AGENTS.md` (native) | `.agents/skills/` (native) |
| OpenCode | `AGENTS.md` (native) | `.agents/skills/` (native); commands in `.opencode/commands/` |
| Pi | `AGENTS.md` (native) | `.agents/skills/` (native) |
| Claude Code | `CLAUDE.md` → `@AGENTS.md` | `.claude/skills` → `../.agents/skills` (`./scripts/setup-skills.sh`) |

Opt-in skills live in `.agents/skills-optional/` and are not auto-discovered.
Copy or symlink one into `.agents/skills/` to enable it, then run
`bash scripts/generate-skills-md.sh`.

Claude hooks (`.claude/settings.json`): a PreToolUse guard that blocks edits to
lint/quality-gate config, and a silent PostToolUse `rustfmt` on edited `.rs` files.

## Setup and diagnostics

```bash
./scripts/bootstrap.sh    # one-command setup (includes skill symlinks)
./scripts/doctor.sh       # environment diagnostics
./scripts/setup-beads.sh  # optional: beads (bd) issue tracking; --stealth keeps it local
./scripts/setup-gpui-mcp.sh  # optional: gpui-mcp server for driving a running GPUI app
```

`bootstrap.sh --help` lists its options (`--dry-run`, `--no-install`, `--no-gate`, `--with-beads`, `--with-gpui-mcp`).
`setup-gpui-mcp.sh` installs a pinned commit of [gpui-mcp](https://github.com/themixednuts/gpui-mcp) with `cargo install` and registers it as `gpui` in `.mcp.json`, `.codex/config.toml` and `opencode.json`. The app still needs the `gpui-mcp` bridge crate and its GPUI patches (see the upstream README). Use it only in development or another trusted environment.
`setup-beads.sh` skips bd's git hooks so `core.hooksPath` stays on `.githooks`.

## Quality tooling

- **Quality SSOT:** `./scripts/quality-gates.sh` (the `xtask` engine) is the single source of truth for all project quality tiers, Cargo compilation, lints, tests, and security checks.
- **Harness & tool ownership:** `xtask` owns project quality checks and telemetry (`.agents/ci/quality-run.json`). `do-harness` is an optional agent tool (`scripts/setup-harness.sh`) used for agent diagnostics (`do-harness doctor`) and PR readiness (`do-harness pr review`, `do-harness pr ready`). `do-harness.toml` delegates quality verification to `./scripts/quality-gates.sh` without running duplicate Cargo/clippy/test passes. Never treat one tool's green status as proof that the other tool's checks ran.
- **Diagnostics & PR context:** When `do-harness` is absent, use `./scripts/doctor.sh` and `./scripts/quality-gates.sh`. If no PR context or `gh` CLI is available, `do-harness pr review` / `do-harness pr ready` report missing PR context gracefully. Uncommitted working tree changes or stale evidence require a fresh `./scripts/quality-gates.sh` run.
- **Verification tiers:** Which checks run for each lifecycle trigger (pull request / protected branch / scheduled / release) is configured in `config/xtask.json` (`tiers` map: `pull-request`, `protected-branch`, `scheduled`, `release`) — not in workflow YAML. Override per run via `xtask quality run --tier <name>` or `$XTASK_TIER`. Legacy names `fast-pr` and `full-gate`/`all` are aliases for `pull-request` and `protected-branch`.
- **When-changed check selection:** `xtask quality run --changed-from <ref>` uses declarative `when_changed` glob mappings in `config/xtask.json` (e.g. `crates/**/*.rs` -> Rust checks, `scripts/**/*.sh` -> ShellCheck). Use `xtask quality explain --changed-from <ref>` to inspect selected vs skipped checks and matched patterns. Unreadable git state fails closed by selecting all checks and recording `fallback_used: true`.
- **Context files:** Run `bash scripts/generate-llms-txt.sh` after significant architecture changes.

## Versions and toolchain

- **Edition:** Rust 2024 (MSRV 1.99, the latest stable at the last bump). The toolchain pin and `rust-version` move together (ADR 0001); bump both when adopting a new stable release.
- **Versions:** `VERSION` / workspace `0.0.0` is the **adopter app** version. Template meta-releases (e.g. v0.3.x) live only in `.template/CHANGELOG-TEMPLATE.md` — do not sync them into `VERSION`.
- **Tracing:** Minimize CLI tracing metadata (thread IDs/names) unless high-concurrency.
- **Dependencies:** Library/template crates should ignore `Cargo.lock`; binary applications should commit it. Audit with `cargo tree` and `deny.toml`. Prefer lockfile pins over exact `=` requirements in manifests.

## Cross-repo context

Derived repositories should check `.agents/context/` for shared conventions and related repository links:

- `.agents/context/external-repos.json`: related repos and their agent context URLs.
- `.agents/context/shared-conventions.md`: cross-repo conventions (commit format, branch naming, PR requirements).

Merge precedence: local repo instructions > imported context > template defaults.

## Stacked PRs (optional)

For large features with 3+ distinct concerns, stacked PRs are available via `gh stack`
(`gh extension install github/gh-stack`).

## Release failures (priority 1)

If a `release-failure` issue is open:

1. Create a `hotfix/` branch and apply the minimal fix.
2. Open a PR with the `hotfix` label.
3. Close the issue with: `Recovered at: <TIMESTAMP>. FDRT: <HOURS>`.
