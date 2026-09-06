#!/usr/bin/env bash
# Validate examples; VitePress includes their source directly in the guide pages.
set -euo pipefail

cd "$(dirname "$0")/.."

for pkl_file in docs/public/*.pkl; do
    [ -f "$pkl_file" ] || continue
    example_name=$(basename "$pkl_file" .pkl)
    guide="docs/reference/examples/$example_name.md"
    if ! rg -Fq "<<< @/public/$example_name.pkl" "$guide"; then
        echo "Missing source include in $guide" >&2
        exit 1
    fi
    pkl eval --format json "$pkl_file" >/dev/null
    echo "Validated $pkl_file and its documentation include"
done
