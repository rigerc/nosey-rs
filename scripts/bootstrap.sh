#!/usr/bin/env bash
# bootstrap.sh - Single-command first-time setup for this Rust template.
# Idempotent: safe to re-run. On failure, run ./scripts/doctor.sh for diagnostics.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

DRY_RUN=0
INSTALL_TOOLS=1
RUN_GATE=1
WITH_BEADS=0
WITH_GPUI_MCP=0

# Tools the `pull-request` quality tier requires (binary name == crate name).
CARGO_TOOLS=(cargo-nextest cargo-audit cargo-deny cargo-machete)
# Tools installed through the host package manager (same package name everywhere).
SYSTEM_TOOLS=(shellcheck yamllint)
# Install command for the host package manager; empty when none is recognised.
PKG_INSTALL=()

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

One-command first-time setup. Safe to re-run.

Options:
  --dry-run     Run read-only checks; print commands instead of changing anything
  --no-install  Report missing tools instead of installing them
  --no-gate     Skip the final quality gate
  --with-beads  Also set up beads (bd) issue tracking via ./scripts/setup-beads.sh
  --with-gpui-mcp
                Also install the gpui-mcp server via ./scripts/setup-gpui-mcp.sh
  -h, --help    Show this help

Steps:
  1. Check prerequisites (git, cargo) and report linker setup
  2. Link agent skills (.claude/skills -> .agents/skills)
  3. Point git at the tracked hooks (core.hooksPath = .githooks)
  4. Validate skills
  5. Install missing quality tools (cargo, system package manager, npm)
  6. Optionally set up beads issue tracking (--with-beads) and the
     gpui-mcp server for GPUI app automation (--with-gpui-mcp)
  7. Run ./scripts/quality-gates.sh

Supported package managers: apt-get, pacman, dnf, zypper, brew.
Windows: enable Developer Mode for symlinks, or use WSL2.
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
      --dry-run)    DRY_RUN=1 ;;
      --no-install) INSTALL_TOOLS=0 ;;
      --no-gate)    RUN_GATE=0 ;;
      --with-beads) WITH_BEADS=1 ;;
      --with-gpui-mcp) WITH_GPUI_MCP=1 ;;
      -h|--help)    usage; exit 0 ;;
      *)            printf 'Unknown option: %s\n\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
    shift
  done
}

detect_pkg_manager() {
  local sudo=()
  if (( EUID != 0 )) && have sudo; then
    sudo=(sudo)
  fi
  if [[ "$(uname -s)" == Darwin* ]] && have brew; then
    PKG_INSTALL=(brew install)
  elif have apt-get; then
    PKG_INSTALL=(${sudo[@]+"${sudo[@]}"} apt-get install -y)
  elif have pacman; then
    PKG_INSTALL=(${sudo[@]+"${sudo[@]}"} pacman -S --needed --noconfirm)
  elif have dnf; then
    PKG_INSTALL=(${sudo[@]+"${sudo[@]}"} dnf install -y)
  elif have zypper; then
    PKG_INSTALL=(${sudo[@]+"${sudo[@]}"} zypper --non-interactive install)
  elif have brew; then
    PKG_INSTALL=(brew install)
  fi
}

check_prereqs() {
  log "Checking prerequisites"
  have git || die "git not found - install git first"
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "not inside a git repository"
  have cargo || die "cargo not found - install Rust via https://rustup.rs"
  ok "$(git --version), $(cargo --version)"

  # Linker setup is advisory only: .cargo/config.toml does not require mold.
  bash "$SCRIPT_DIR/check-linker.sh" | sed 's/^/  /'

  if have gh && ! gh extension list 2>/dev/null | grep -q gh-stack; then
    warn "optional: gh extension install github/gh-stack (stacked PRs)"
  fi
}

setup_skills() {
  log "Linking agent skills"
  local probe
  probe="$(mktemp -d)"
  # Git Bash on Windows silently copies instead of linking, so check -L too.
  if ln -s . "$probe/link" 2>/dev/null && [[ -L "$probe/link" ]]; then
    rm -rf -- "$probe"
    apply "skills linked" "$SCRIPT_DIR/setup-skills.sh" \
      || die "setup-skills.sh failed - run ./scripts/doctor.sh"
  else
    rm -rf -- "$probe"
    warn "symlinks unsupported here (Windows: enable Developer Mode or use WSL2); skipping"
  fi
}

setup_hooks() {
  log "Configuring git hooks"
  if [[ "$(git config --get core.hooksPath || true)" == ".githooks" ]]; then
    ok "core.hooksPath already .githooks"
  else
    apply "core.hooksPath = .githooks" git config core.hooksPath .githooks
  fi
}

validate_skills() {
  log "Validating skills"
  if "$SCRIPT_DIR/validate-skills.sh" >/dev/null 2>&1; then
    ok "skills valid"
  else
    warn "skill validation reported issues - run ./scripts/validate-skills.sh"
  fi
}

# collect_missing TOOL...: print the tools that are not on PATH, space-separated.
collect_missing() {
  local tool missing=""
  for tool in "$@"; do
    have "$tool" || missing+="${missing:+ }$tool"
  done
  printf '%s' "$missing"
}

install_cargo_tools() {
  local missing
  missing="$(collect_missing "${CARGO_TOOLS[@]}")"
  if [[ -z "$missing" ]]; then
    ok "cargo tools present"
  elif (( ! INSTALL_TOOLS )); then
    warn "missing: $missing (cargo install --locked $missing)"
  elif have cargo-binstall; then
    # shellcheck disable=SC2086  # one argument per tool
    apply "installed $missing" cargo binstall --no-confirm $missing
  else
    # shellcheck disable=SC2086  # one argument per tool
    apply "installed $missing" cargo install --locked $missing
  fi
}

install_system_tools() {
  local missing
  missing="$(collect_missing "${SYSTEM_TOOLS[@]}")"
  if [[ -z "$missing" ]]; then
    ok "system tools present"
  elif (( ! INSTALL_TOOLS )) || (( ${#PKG_INSTALL[@]} == 0 )); then
    warn "missing: $missing - install with your package manager"
  else
    # shellcheck disable=SC2086  # one argument per tool
    apply "installed $missing" "${PKG_INSTALL[@]}" $missing \
      || warn "package install failed - install $missing manually"
  fi
}

install_npm_tools() {
  if have markdownlint-cli2; then
    ok "markdownlint-cli2 present"
  elif (( ! INSTALL_TOOLS )) || ! have npm; then
    warn "missing: markdownlint-cli2 (npm install -g markdownlint-cli2)"
  else
    apply "installed markdownlint-cli2" npm install -g markdownlint-cli2 \
      || warn "npm install failed - install markdownlint-cli2 manually"
  fi
}

check_optional_tools() {
  have actionlint || warn "optional: actionlint not found (https://github.com/rhysd/actionlint)"
}

# run_optional NAME SCRIPT: run an opt-in setup script, forwarding --dry-run.
run_optional() {
  local name="$1" script="$2" args=()
  log "Setting up $name"
  (( DRY_RUN )) && args+=(--dry-run)
  "$SCRIPT_DIR/$script" ${args[@]+"${args[@]}"} | sed 's/^/  /' \
    || die "$script failed"
}

run_quality_gate() {
  log "Running quality gate"
  if (( DRY_RUN )); then
    printf '  would run: ./scripts/quality-gates.sh\n'
    return 0
  fi
  if ! "$SCRIPT_DIR/quality-gates.sh"; then
    printf '\nBootstrap finished with quality gate failures.\n' >&2
    printf 'Run ./scripts/doctor.sh for environment diagnostics.\n' >&2
    exit 1
  fi
  ok "quality gate passed"
}

main() {
  parse_args "$@"
  cd "$SCRIPT_DIR/.."
  detect_pkg_manager

  check_prereqs
  setup_skills
  setup_hooks
  validate_skills

  log "Checking quality tooling"
  install_cargo_tools
  install_system_tools
  install_npm_tools
  check_optional_tools

  if (( WITH_BEADS )); then
    run_optional beads setup-beads.sh
  fi
  if (( WITH_GPUI_MCP )); then
    run_optional gpui-mcp setup-gpui-mcp.sh
  fi
  if (( RUN_GATE )); then
    run_quality_gate
  fi

  if (( DRY_RUN )); then
    printf '\nBootstrap dry run complete. No changes were made.\n'
  else
    printf '\nBootstrap complete.\n'
  fi
}

main "$@"
