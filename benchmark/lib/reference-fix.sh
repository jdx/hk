#!/usr/bin/env bash
# Applies every fixer in the workload, one at a time, to every file in the
# current directory's git repository.
#
# This is the definition of "correct". The fixture's clean commit is whatever
# this produces, and a subject passes the correctness check only when fixing the
# dirty commit reproduces that tree byte for byte. Because it is strictly
# sequential it cannot race, so any difference in a subject's output comes from
# the subject.
#
# The commands match the fix commands the subjects run. The generated files are
# plain enough that the fixers agree with one another, so the order below does
# not change the result; generate-project.sh checks that it reaches a fixed
# point.
set -euo pipefail
# shellcheck disable=SC2016 # the single-quoted scripts are for `sh -c`

files() { git ls-files -z -- "$@"; }
each() { xargs -0r "$@"; }

files '*.py' | each black --quiet
files '*.py' | each ruff format --quiet --force-exclude
files '*.py' | each ruff check --quiet --force-exclude --fix
files '*.js' '*.ts' '*.css' '*.md' | each prettier --write --log-level warn
files '*.js' '*.ts' | each eslint --fix
files '*.json' | each sh -c 'for f; do t=$(mktemp) && jq -S . "$f" >"$t" && mv "$t" "$f"; done' _
# One file per yq call: `yq -i` writes every input's documents into the first file.
files '*.yml' '*.yaml' | each sh -c 'for f; do yq -iP "$f"; done' _
files '*.sh' | each shfmt -w
files | each sed -i 's/[[:space:]]*$//'
files | each sh -c 'for f; do [ -s "$f" ] && [ -n "$(tail -c 1 "$f")" ] && echo >>"$f"; done; true' _
