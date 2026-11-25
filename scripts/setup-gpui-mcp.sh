#!/usr/bin/env bash
# setup-gpui-mcp.sh - Optional: install the gpui-mcp server and register it with
# this project's MCP clients (Claude Code, Codex, OpenCode).
# Idempotent: safe to re-run. Also reachable via ./scripts/bootstrap.sh --with-gpui-mcp.
#
# gpui-mcp lets agents read and drive a running GPUI app (UI tree, clicks,
# typing, screenshots). The app side - the `gpui-mcp` bridge crate and its GPUI
# patches - is per-app code; see https://github.com/themixednuts/gpui-mcp#setup.
# Only enable automation in development or another trusted environment.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

GPUI_MCP_REPO="https://github.com/themixednuts/gpui-mcp"
# Upstream publishes no release tags, so installs are pinned to a reviewed commit.
DEFAULT_REV="eb248ccf43847797b9067c47eeef1dd188dc03a6"
GPUI_MCP_REV="${GPUI_MCP_REV:-$DEFAULT_REV}"
SERVER_PACKAGE="gpui-mcp-server"
SERVER_BIN="gpui-mcp"
SERVER_NAME="gpui"

DRY_RUN=0
REGISTER=1
REINSTALL=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Install the gpui-mcp MCP server (cargo install, pinned commit) and register it
as "$SERVER_NAME" in .mcp.json (Claude Code), .codex/config.toml (Codex) and
opencode.json (OpenCode).

Options:
  --rev REV       Commit to install (default: $DEFAULT_REV, or \$GPUI_MCP_REV)
  --reinstall     Reinstall even if $SERVER_BIN is already on PATH
  --no-register   Install only; leave the MCP client configs untouched
  --dry-run       Print what would change without changing anything
  -h, --help      Show this help

Linux note: exact-window screenshots currently require X11.
EOF
}

log()  { printf '==> %s\n' "$*"; }
ok()   { printf '  \033[0;32m✓\033[0m %s\n' "$*"; }
warn() { printf '  \033[0;33m!\033[0m %s\n' "$*"; }
die()  { printf '\n\033[0;31m✗ %s\033[0m\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --rev)
        [[ $# -ge 2 && -n "$2" ]] || die "--rev needs a commit"
        GPUI_MCP_REV="$2"
        shift
        ;;
      --reinstall)   REINSTALL=1 ;;
      --no-register) REGISTER=0 ;;
      --dry-run)     DRY_RUN=1 ;;
      -h|--help)     usage; exit 0 ;;
      *)             printf 'Unknown option: %s\n\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
    shift
  done
  [[ "$GPUI_MCP_REV" =~ ^[0-9a-f]{7,40}$ ]] || die "--rev must be a commit hash, got: $GPUI_MCP_REV"
}

install_server() {
  log "Installing $SERVER_PACKAGE ($GPUI_MCP_REV)"
  if have "$SERVER_BIN" && (( ! REINSTALL )); then
    ok "$SERVER_BIN already installed: $(command -v "$SERVER_BIN") (use --reinstall to update)"
    return 0
  fi
  have cargo || die "cargo not found - install Rust via https://rustup.rs"
  local args=(install --locked --git "$GPUI_MCP_REPO" --rev "$GPUI_MCP_REV" "$SERVER_PACKAGE")
  (( REINSTALL )) && args+=(--force)
  if (( DRY_RUN )); then
    printf '  would run: cargo %s\n' "${args[*]}"
    return 0
  fi
  cargo "${args[@]}"
  if have "$SERVER_BIN"; then
    ok "installed $(command -v "$SERVER_BIN")"
  else
    warn "installed, but $SERVER_BIN is not on PATH (add \$CARGO_HOME/bin, usually ~/.cargo/bin)"
  fi
}

# merge_json FILE KIND: idempotently add the server entry to a JSON client config.
merge_json() {
  local file="$1" kind="$2"
  if (( DRY_RUN )); then
    printf '  would register "%s" in %s\n' "$SERVER_NAME" "$file"
    return 0
  fi
  have python3 || { warn "python3 not found - add \"$SERVER_NAME\" to $file manually"; return 0; }
  python3 - "$file" "$kind" "$SERVER_NAME" "$SERVER_BIN" <<'PY'
import json
import pathlib
import sys

path, kind, name, command = pathlib.Path(sys.argv[1]), *sys.argv[2:]
config = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
if kind == "claude":
    servers, entry = config.setdefault("mcpServers", {}), {"command": command, "args": []}
else:  # opencode
    servers, entry = config.setdefault("mcp", {}), {"type": "local", "command": [command], "enabled": True}
if name not in servers:
    servers[name] = entry
    path.write_text(json.dumps(config, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY
  ok "\"$SERVER_NAME\" registered in $file"
}

register_codex() {
  local file=".codex/config.toml"
  if [[ -f "$file" ]] && grep -qF "[mcp_servers.$SERVER_NAME]" "$file"; then
    ok "\"$SERVER_NAME\" already registered in $file"
    return 0
  fi
  if (( DRY_RUN )); then
    printf '  would register "%s" in %s\n' "$SERVER_NAME" "$file"
    return 0
  fi
  mkdir -p .codex
  if [[ -s "$file" ]]; then
    printf '\n' >>"$file"
  fi
  printf '[mcp_servers.%s]\ncommand = "%s"\n' "$SERVER_NAME" "$SERVER_BIN" >>"$file"
  ok "\"$SERVER_NAME\" registered in $file (Codex reads it once the project is trusted)"
}

register_clients() {
  log "Registering MCP server \"$SERVER_NAME\""
  merge_json .mcp.json claude
  register_codex
  merge_json opencode.json opencode
}

main() {
  parse_args "$@"
  cd "$SCRIPT_DIR/.."

  install_server
  if (( REGISTER )); then
    register_clients
  fi

  if (( DRY_RUN )); then
    printf '\ngpui-mcp dry run complete. No changes were made.\n'
  else
    printf '\ngpui-mcp ready. Restart your MCP client, then add the bridge to your app:\n'
    printf '  %s#setup\n' "$GPUI_MCP_REPO"
  fi
}

main "$@"
