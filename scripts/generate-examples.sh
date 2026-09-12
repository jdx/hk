#!/usr/bin/env bash
# Validate examples; VitePress includes their source directly in the guide pages.
set -euo pipefail

cd "$(dirname "$0")/.."

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

for pkl_file in docs/public/*.pkl; do
    [ -f "$pkl_file" ] || continue
    example_name=$(basename "$pkl_file" .pkl)
    guide="docs/reference/examples/$example_name.md"
    if ! rg -Fq "<<< @/public/$example_name.pkl" "$guide"; then
        echo "Missing source include in $guide" >&2
        exit 1
    fi
    local_example="$tmp_dir/$example_name.pkl"
    sed \
        -e "s|package://github.com/jdx/hk/releases/download/[^\"]*#/Config.pkl|$PWD/pkl/Config.pkl|" \
        -e "s|package://github.com/jdx/hk/releases/download/[^\"]*#/Builtins.pkl|$PWD/pkl/Builtins.pkl|" \
        "$pkl_file" > "$local_example"
    pkl eval --format json "$local_example" >/dev/null
    echo "Validated $pkl_file and its documentation include"
done
