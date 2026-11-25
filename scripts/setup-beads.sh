#!/usr/bin/env bash
# setup-beads.sh - Optional: add beads (bd) issue tracking to this repository.
# Idempotent: safe to re-run. Also reachable via ./scripts/bootstrap.sh --with-beads.
#
# Differences from a plain `bd init`, all deliberate:
#   * git hooks are skipped: bd's hooks take over core.hooksPath, which would
#     silently disable the template's .githooks/pre-commit and pre-push.
#   * bd's generated agent files are skipped: its AGENTS.md block (~55 lines)
#     breaks the 80-line AGENTS.md limit. A short pointer is added instead, and
#     Claude Code gets context from a `bd prime` SessionStart hook.
#   * the commit `bd init` creates is undone (changes stay staged), so you can
#     review them and commit with a conventional-commit message.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DRY_RUN=0
STEALTH=0

AGENTS_BEGIN="<!-- BEGIN BEADS (scripts/setup-beads.sh) -->"
AGENTS_END="<!-- END BEADS -->"

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Install the beads CLI if missing, initialise .beads/ here, and wire it into
agent context without touching git hooks.

Options:
  --stealth    Keep beads local to this clone: nothing tracked changes
               (git exclude rules, hook in .claude/settings.local.json)
  --dry-run    Print what would change without changing anything
  -h, --help   Show this help

Remove later with: rm -rf .beads, then revert the AGENTS.md block and the
SessionStart hook this script added.
EOF
}

log()  { printf '==> %s\n' "$*"; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  \033[0;33m!\033[0m %s\n' "$*"; }
die()  { printf '\n\033[0;31m✗ %s\033[0m\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

# apply MSG CMD...: run CMD and report MSG; under --dry-run only print CMD.
apply() {
  local msg="$1"
  shift
  if (( DRY_RUN )); then
    printf '  would run: %s\n' "$*"
    return 0
  fi
  "$@" && ok "$msg"
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --stealth) STEALTH=1 ;;
      --dry-run) DRY_RUN=1 ;;
      -h|--help) usage; exit 0 ;;
      *)         printf 'Unknown option: %s\n\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
    shift
  done
}

install_bd() {
  log "Checking beads CLI"
  if have bd; then
    ok "$(bd version 2>/dev/null | head -1)"
  elif have brew; then
    apply "installed beads" brew install beads
  elif have npm; then
    apply "installed beads" npm install -g @beads/bd
  else
    die "bd not found and neither brew nor npm is available - see https://github.com/gastownhall/beads#-installation"
  fi
}

init_beads() {
  log "Initialising .beads/"
  if [[ -d .beads ]]; then
    ok ".beads/ already present"
    return 0
  fi

  local args=(init --non-interactive --skip-hooks --skip-agents --role maintainer)
  (( STEALTH )) && args+=(--stealth)
  if (( DRY_RUN )); then
    printf '  would run: bd %s\n' "${args[*]}"
    return 0
  fi

  local before after
  before="$(git rev-parse -q --verify HEAD || true)"
  bd "${args[@]}"
  after="$(git rev-parse -q --verify HEAD || true)"

  # `bd init` commits its own files with a non-conventional subject that the
  # commitlint CI job rejects. Undo that commit but keep its changes staged.
  if [[ -n "$before" && "$after" != "$before" ]] \
     && [[ "$(git log -1 --format=%s)" == "bd init:"* ]]; then
    git reset -q --soft "$before"
    ok "bd init commit undone; its changes are staged for review"
  fi
  ok ".beads/ initialised"
}

# add_session_hook FILE COMMAND: idempotently add a Claude Code SessionStart hook.
add_session_hook() {
  local file="$1" command="$2"
  if (( DRY_RUN )); then
    printf '  would add SessionStart hook "%s" to %s\n' "$command" "$file"
    return 0
  fi
  if ! have python3; then
    warn "python3 not found - add a SessionStart hook running '$command' to $file manually"
    return 0
  fi
  python3 - "$file" "$command" <<'PY'
import json
import pathlib
import sys

path, command = pathlib.Path(sys.argv[1]), sys.argv[2]
settings = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
session = settings.setdefault("hooks", {}).setdefault("SessionStart", [])
present = any(
    hook.get("command") == command
    for entry in session
    for hook in entry.get("hooks", [])
)
if not present:
    session.append({"matcher": "", "hooks": [{"type": "command", "command": command}]})
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(settings, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY
  ok "Claude Code SessionStart hook in $file"
}

add_agents_pointer() {
  if grep -qF "$AGENTS_BEGIN" AGENTS.md; then
    ok "AGENTS.md already points at beads"
    return 0
  fi
  if (( DRY_RUN )); then
    printf '  would append a beads pointer block to AGENTS.md\n'
    return 0
  fi
  cat >>AGENTS.md <<EOF

$AGENTS_BEGIN
## Issue Tracking

Track work with beads: \`bd ready\`, \`bd show <id>\`, \`bd update <id> --claim\`, \`bd close <id>\`. Run \`bd prime\` for the full workflow.
$AGENTS_END
EOF
  ok "AGENTS.md points at beads"
}

exclude_locally() {
  local pattern="$1" exclude
  exclude="$(git rev-parse --git-path info/exclude)"
  grep -qxF "$pattern" "$exclude" 2>/dev/null && return 0
  if (( DRY_RUN )); then
    printf '  would add %s to %s\n' "$pattern" "$exclude"
    return 0
  fi
  mkdir -p -- "$(dirname "$exclude")"
  printf '%s\n' "$pattern" >>"$exclude"
}

wire_agents() {
  log "Wiring agent context"
  if (( STEALTH )); then
    exclude_locally ".claude/settings.local.json"
    add_session_hook .claude/settings.local.json "bd prime --stealth --hook-json"
  else
    add_session_hook .claude/settings.json "bd prime --hook-json"
    add_agents_pointer
  fi
}

main() {
  parse_args "$@"
  cd "$SCRIPT_DIR/.."
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "not inside a git repository"

  install_bd
  init_beads
  wire_agents

  if (( DRY_RUN )); then
    printf '\nBeads dry run complete. No changes were made.\n'
  elif (( STEALTH )); then
    printf '\nBeads ready (stealth: nothing tracked changed). Try: bd ready\n'
  else
    printf '\nBeads ready. Review and commit:\n'
    printf '  git add .beads .gitignore AGENTS.md .claude/settings.json\n'
    printf '  git commit -m "chore: add beads issue tracking"\n'
  fi
}

main "$@"
