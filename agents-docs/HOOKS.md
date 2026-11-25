# Agent Hooks

## Overview

Hooks are scripts that run at specific points in the agent lifecycle. They enable automated context injection, validation, and cleanup.

## Available Hooks

### Claude Code tool hooks

Registered in `.claude/settings.json`:

| Hook | File | Purpose |
|------|------|---------|
| PreToolUse (`Write\|Edit\|MultiEdit`) | `.claude/hooks/guard-lint-config.sh` | Deny edits to lint/quality-gate config (`.clippy.toml`, `rustfmt.toml`, `deny.toml`, ...) |
| PostToolUse (`Write\|Edit\|MultiEdit`) | `.claude/hooks/post-rust-fmt.sh` | Silently `rustfmt` the edited `.rs` file |

Hooks deliberately inject no context and run no workspace-wide cargo commands;
lints and tests run in `./scripts/quality-gates.sh`.

### Pre-Commit Hook

**File**: `.githooks/pre-commit`
**Trigger**: Before each git commit
**Purpose**: Enforce quality gates before code enters the repository

Checks performed:
- LOC limit enforcement (500 lines per .rs file)
- TOML/YAML syntax validation
- Rust formatting (rustfmt)
- Rust linting (clippy)
- Privacy scan (email addresses)
- Secret scan (API keys, tokens)

### Pre-Push Hook

**File**: `.githooks/pre-push`
**Trigger**: Before pushing to remote
**Purpose**: Run the checks too slow for every commit before sharing code

Checks performed:
- Rust linting across all targets and features (`scripts/harness-check.sh clippy`)
- Dependency policy (`scripts/harness-check.sh deny`, skipped with a warning if `cargo-deny` is missing)

Bypass once with `git push --no-verify`.

## Creating Custom Hooks

### Hook Script Location

Place agent hooks in `.claude/hooks/` and git hooks in `.githooks/`:

```bash
#!/usr/bin/env bash
# .claude/hooks/my-hook.sh
# Description of what this hook does
set -euo pipefail

# Your hook logic here
```

### Hook Configuration

Register hooks in the appropriate config:

- **Session hooks**: Register in `.claude/settings.json`
- **Git hooks**: Place in `.githooks/` and run `git config core.hooksPath .githooks`
- **Pre-commit framework**: Add to `.pre-commit-config.yaml`

### Hook Best Practices

1. **Idempotent**: Hooks should produce the same result when run multiple times
2. **Fast**: Keep hooks under 10 seconds to avoid blocking workflow
3. **Informative**: Use colors and clear messages for pass/fail/warn
4. **Optional**: Allow skipping with environment variables (e.g., `SKIP_HOOKS=1`)
5. **Logged**: Write hook results to event files for observability

## Hook Integration Points

```
Agent Working... (.claude/hooks: lint-config guard, rustfmt)
    |
    v
Pre-Commit (.githooks/pre-commit)
    |
    v
CI Pipeline (.github/workflows/ci.yml)
    |
    v
Post-Release (release-drafter, DORA metrics)
```

## Debugging Hooks

To debug a hook:
1. Run it manually: `echo '{"tool_input":{"file_path":"deny.toml"}}' | bash .claude/hooks/guard-lint-config.sh`
2. Add `set -x` at the top for trace output
3. Check the hook's exit code: `echo $?`
4. Review hook output for error messages
