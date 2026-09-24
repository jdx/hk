# shellcheck shell=bash
# Sourced by the benchmark scripts: puts the pinned toolset from ./mise.toml,
# then the hk under test, first on PATH.
#
# HK_BIN selects the hk binary (default: this checkout's release build).
# The repository's mise.toml adds node_modules/.bin to PATH, which would let a
# project-local ESLint shadow the pinned one, so those entries are dropped
# (mise's own npm installs also live in node_modules/.bin and are kept).
BENCH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
eval "$(cd "$BENCH" && mise env -s bash)"
PATH="$(printf '%s' "$PATH" | awk -v RS=: -v ORS=: '!/\/node_modules\/\.bin$/ || /\/mise\/installs\//' | sed 's/:*$//')"
HK_BIN="$(readlink -f "${HK_BIN:-$BENCH/../target/release/hk}")"
PATH="$(dirname "$HK_BIN"):$PATH"
export PATH HK_BIN
