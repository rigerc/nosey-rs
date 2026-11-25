#!/usr/bin/env bash
# PostToolUse hook: rustfmt the edited Rust file.
# Silent by design: no clippy run and no output, so nothing is added to the
# agent's context. Lints are checked by ./scripts/quality-gates.sh.
set -euo pipefail

input="$(cat)"
file="$(jq -r '.tool_input.file_path // empty' <<<"$input" 2>/dev/null || true)"

case "$file" in
  *.rs) [ -f "$file" ] && rustfmt --edition 2024 "$file" >/dev/null 2>&1 || true ;;
esac
exit 0
