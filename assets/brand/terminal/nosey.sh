#!/usr/bin/env sh
# Usage: sh nosey.sh [dark|light]. Defaults to dark. No colour on redirected output.
if [ "${NO_COLOR+x}" = x ] || [ "${TERM:-}" = dumb ] || [ ! -t 1 ]; then
  printf '%s\n' ':^) nosey'
elif [ "${1:-dark}" = light ]; then
  printf '\033[38;2;20;16;10m:\033[38;2;138;90;0m^\033[38;2;20;16;10m) nosey\033[0m\n'
else
  printf '\033[38;2;245;230;200m:\033[38;2;255;176;0m^\033[38;2;245;230;200m) nosey\033[0m\n'
fi
