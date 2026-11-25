#!/usr/bin/env bash
# PreToolUse hook: block agent edits to lint/quality-gate config files.
# Agents hitting a Clippy/fmt error should fix the code, not silence the tool.
# If a lint config change is genuinely needed, a human must make it manually.
# Cargo.toml is not protected: agents must be able to add workspace dependencies.
set -euo pipefail

input="$(cat)"
file="$(jq -r '.tool_input.file_path // empty' <<<"$input" 2>/dev/null || true)"
[ -n "$file" ] || exit 0

PROTECTED=(
  ".clippy.toml"
  "rustfmt.toml"
  ".pre-commit-config.yaml"
  "deny.toml"
  ".gitleaks.toml"
  ".yamllint.yml"
  ".shellcheckrc"
)

name="$(basename "$file")"
for protected in "${PROTECTED[@]}"; do
  if [ "$name" = "$protected" ]; then
    jq -n --arg f "$file" '{
      hookSpecificOutput: {
        hookEventName: "PreToolUse",
        permissionDecision: "deny",
        permissionDecisionReason: ("Lint/quality-gate config is human-only. Fix the code instead of silencing the linter. File: " + $f)
      }
    }'
    exit 0
  fi
done
exit 0
