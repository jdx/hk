#!/usr/bin/env bash
# Breaks the formatting of a deterministic subset of the fixture's files.
#
# Every defect is one the workload's fixers undo exactly, so fixing the dirty
# commit must reproduce the clean commit. Each touched file gets a
# language-specific defect *and* a whitespace defect, which means two or three
# fixers must write the same file in the same run. That overlap is what the
# benchmark is about: running those fixers concurrently without coordination
# lets one fixer overwrite another's work.
#
# Usage: inject-defects.sh [EVERY]   (default: every 4th file of each type)
set -euo pipefail

EVERY="${1:-4}"

pick() { git ls-files -- "$@" | sort | awk -v n="$EVERY" 'NR % n == 1'; }

# Trailing whitespace on the first five lines, and no final newline.
whitespace() {
    sed -i '1,5s/$/   /' "$1"
    printf '%s' "$(cat "$1")" >"$1"
}

# The unused import is for ruff check, which removes it; black and ruff format
# leave it alone, so ruff's fix is the only one that changes that line.
for f in $(pick '*.py'); do
    sed -i -e 's/sorted(result, key=lambda r: r.value)/sorted( result,key=lambda r:r.value )/' \
        -e 's/batch_size: int = 100/batch_size:int=100/' \
        -e 's/^from dataclasses import dataclass$/&\nimport os/' "$f"
    whitespace "$f"
done

for f in $(pick '*.js' '*.ts'); do
    sed -i -e 's/from "react"/from '"'"'react'"'"'/' \
        -e 's/return this.cache.get(id)$/return this.cache.get(id);/' \
        -e 's/const item = this.items.find/let item = this.items.find/' "$f"
    whitespace "$f"
done

for f in $(pick '*.css'); do
    sed -i 's/display: flex;/display:flex;/' "$f"
    whitespace "$f"
done

for f in $(pick '*.md'); do
    sed -i '1a\\' "$f" # shellcheck disable=SC1003
    whitespace "$f"
done

for f in $(pick '*.json'); do
    t=$(mktemp) && jq -c . "$f" >"$t" && mv "$t" "$f"
    whitespace "$f"
done

for f in $(pick '*.yml' '*.yaml'); do
    sed -i 's/^name: /name:    /' "$f"
    whitespace "$f"
done

for f in $(pick '*.sh'); do
    sed -i 's/^\t/    /' "$f"
    whitespace "$f"
done
