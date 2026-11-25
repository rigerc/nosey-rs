#!/usr/bin/env bash
# Build the exact untouched upstream reference in a separate Git checkout.
set -euo pipefail

readonly PIN="19711db044aabfa9846a65aaee693276d65af5c2"
readonly TREE="2850d10f64ec6b3e1ff0f250666e8cc90cb60b25"
readonly TOOLCHAIN="${KITTER_TOOLCHAIN:-1.99}"

if [[ $# -ne 1 ]]; then
    printf 'Usage: %s /path/to/pinned-kitter-checkout\n' "$0" >&2
    exit 2
fi
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
    printf 'Kitter baseline requires Linux x86-64\n' >&2
    exit 2
fi
cd -- "$1"
[[ "$(git rev-parse HEAD)" == "$PIN" ]]
[[ "$(git rev-parse HEAD^{tree})" == "$TREE" ]]
if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
    printf 'Upstream checkout must be clean\n' >&2
    exit 2
fi
state="$(mktemp -d)"
trap 'rm -rf -- "$state"' EXIT
export KITTER_HOME="$state"
cargo "+$TOOLCHAIN" test --locked
cargo "+$TOOLCHAIN" build --locked --bin kitter
cargo "+$TOOLCHAIN" run --locked --bin kitter -- --help
dependencies="$(cargo "+$TOOLCHAIN" tree --locked --features desktop --edges normal,build --prefix none)"
grep -F 'crates/ztracing-shim' <<<"$dependencies"
if grep -Eq '^(zlog|ztracing_macro) v|^ztracing .*github.com/zed' <<<"$dependencies"; then
    printf 'Unexpected GPL tracing dependencies\n' >&2
    exit 1
fi
cargo "+$TOOLCHAIN" check --locked --features desktop --bin kitter-desktop
