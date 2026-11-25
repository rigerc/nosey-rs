#!/usr/bin/env bash
# doctor.sh - Environment diagnostics for this Rust template.
# Checks required/optional tools, git state, symlinks, hooks, and core files.
STRICT=0
if [[ "${1:-}" == "--strict" ]]; then
  STRICT=1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ISSUES=0

pass() { printf "  ${GREEN}✓${NC} %s\n" "$1"; }
fail() { printf "  ${RED}✗${NC} %s\n" "$1"; ISSUES=$((ISSUES + 1)); }
warn() {
  if [[ $STRICT -eq 1 ]]; then
    printf "  ${RED}✗${NC} %s (strict failure)\n" "$1"
    ISSUES=$((ISSUES + 1))
  else
    printf "  ${YELLOW}!${NC} %s\n" "$1"
  fi
}
echo "=== Environment Diagnostics ==="
echo ""

# --- Required tools ---
echo "Required tools:"
for cmd in git cargo; do
  if command -v "$cmd" &>/dev/null; then
    pass "$cmd: $(command -v "$cmd")"
  else
    fail "$cmd: not found"
  fi
done
echo ""

# --- Optional quality tools ---
echo "Optional quality tools:"
install_hint() {
  case "$1" in
    cargo-*)           echo "cargo install --locked $1" ;;
    markdownlint-cli2) echo "npm install -g markdownlint-cli2" ;;
    actionlint)        echo "see https://github.com/rhysd/actionlint" ;;
    *)                 echo "install $1 with your package manager" ;;
  esac
}
for cmd in cargo-nextest cargo-audit cargo-deny cargo-machete shellcheck markdownlint-cli2 yamllint actionlint; do
  if command -v "$cmd" &>/dev/null; then
    pass "$cmd: installed"
  else
    warn "$cmd: not installed ($(install_hint "$cmd"), or run ./scripts/bootstrap.sh)"
  fi
done
echo ""

# --- Optional workflow tools ---
echo "Optional workflow tools:"
if command -v gh &>/dev/null; then
  pass "gh CLI: $(command -v gh)"
  if gh extension list 2>/dev/null | grep -qw gh-stack; then
    pass "gh-stack: installed"
  else
    warn "gh-stack: not installed (optional, for stacked PRs: gh extension install github/gh-stack)"
  fi
else
  warn "gh CLI: not installed (optional, needed for PR workflows and stacked PRs)"
fi

if command -v bd &>/dev/null; then
  if [[ -d .beads ]]; then
    pass "beads: $(bd version 2>/dev/null | head -1), initialised"
  else
    pass "beads: $(bd version 2>/dev/null | head -1) (not initialised here: ./scripts/setup-beads.sh)"
  fi
fi

if command -v gpui-mcp &>/dev/null; then
  pass "gpui-mcp: $(command -v gpui-mcp)"
  if ! grep -qs '"gpui"' .mcp.json; then
    warn "gpui-mcp installed but not registered in .mcp.json (run ./scripts/setup-gpui-mcp.sh)"
  fi
fi

if command -v do-harness &>/dev/null; then
  pass "do-harness: $(command -v do-harness)"
else
  warn "do-harness: not installed (optional agent harness workflow tool: run ./scripts/setup-harness.sh)"
fi
echo ""

# --- Linker check ---
echo "Linker configuration:"
bash "$(dirname "${BASH_SOURCE[0]}")/check-linker.sh" 2>/dev/null || warn "check-linker.sh not found or failed"
echo ""

# --- Git state ---
echo "Git state:"
BRANCH=$(git branch --show-current 2>/dev/null || echo "detached")
pass "Current branch: $BRANCH"

if git diff --quiet 2>/dev/null && git diff --cached --quiet 2>/dev/null; then
  pass "Working tree clean"
else
  warn "Working tree has uncommitted changes"
fi
echo ""

# --- Symlinks ---
echo "Skill symlinks:"
if [[ -L .claude/skills ]]; then
  pass ".claude/skills -> $(readlink .claude/skills)"
else
  warn ".claude/skills is not a symlink (run ./scripts/setup-skills.sh)"
fi
echo ""

# --- Git hooks ---
echo "Git hooks:"
HOOKS_PATH=$(git config core.hooksPath 2>/dev/null || echo "")
if [[ "$HOOKS_PATH" == ".githooks" ]]; then
  for hook in pre-commit pre-push; do
    if [[ -x ".githooks/$hook" ]]; then
      pass "$hook hook installed"
    else
      warn "core.hooksPath set but .githooks/$hook missing or not executable"
    fi
  done
else
  warn "core.hooksPath not set to .githooks (run: git config core.hooksPath .githooks)"
fi
echo ""

# --- Core files ---
echo "Core files:"
for file in AGENTS.md CHANGELOG.md Cargo.toml rust-toolchain.toml deny.toml; do
  if [[ -f "$file" ]]; then
    pass "$file"
  else
    fail "$file: missing"
  fi
done
echo ""

# --- Skills ---
echo "Skills:"
if [[ -d ".agents/skills" ]]; then
  SKILL_COUNT=$(find .agents/skills -name "SKILL.md" -type f 2>/dev/null | wc -l)
  pass ".agents/skills: $SKILL_COUNT skills"
else
  fail ".agents/skills: directory missing"
fi
echo ""

# --- Summary ---
if [[ $ISSUES -eq 0 ]]; then
  echo -e "${GREEN}All checks passed.${NC}"
  exit 0
else
  echo -e "${RED}$ISSUES issue(s) found.${NC}"
  exit 1
fi
