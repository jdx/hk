#!/usr/bin/env bash
# Prepares the working directory tak.toml measures in: one clone of the fixture
# per subject, each with that subject's configuration committed on top of both
# the `clean` and `dirty` states.
#
# Separate clones keep the tools from seeing one another's configuration and
# caches. Committing the configuration keeps it out of every tool's view of
# "changed files" and stops the pre-commit stash from refusing to run.
#
# The work directory lives outside the repository (JS tooling resolves config
# by walking up from the working directory and would otherwise find hk's own
# package.json); benchmark/.work is a symlink to it.
#
# Usage: benchmark/setup.sh
# Environment:
#   HK_BENCH_DIR   where to put the work directory
#                  (default: ${XDG_CACHE_HOME:-~/.cache}/hk-bench)
#   REGENERATE=1   rebuild the fixture even if its inputs have not changed
#   HK_PKL         base URI of the Config.pkl/Builtins.pkl the hk subject's
#                  configuration imports (default: this checkout's pkl/). Set
#                  it to a release's package URI when measuring that release.
set -euo pipefail

BENCH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$BENCH/.." && pwd)"
WORK="${HK_BENCH_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/hk-bench}"
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1

# subject name -> directory under subjects/ holding its configuration
declare -A CONFIG=(
    [hk]=hk
    [lefthook]=lefthook
    [pre-commit]=pre-commit
    [prek]=prek
)

# prek's configuration must be pre-commit's plus `priority` keys, so the two
# run the same hooks and differ only in scheduling.
strip_priority() { yq -o json 'del(.. | select(tag == "!!map") | .priority)' "$1"; }
if ! diff -u <(strip_priority "$BENCH/subjects/pre-commit/.pre-commit-config.yaml") \
    <(strip_priority "$BENCH/subjects/prek/.pre-commit-config.yaml") >&2; then
    echo "error: subjects/prek differs from subjects/pre-commit in more than priority" >&2
    exit 1
fi

if [ -z "${HK_PKL:-}" ]; then
    HK_PKL="$REPO/pkl"
    [ -f "$HK_PKL/Builtins.pkl" ] || (cd "$REPO" && mise run pkl:gen >/dev/null)
fi

mkdir -p "$WORK"
[ -L "$BENCH/.work" ] || rm -rf "$BENCH/.work"
ln -sfn "$WORK" "$BENCH/.work"

# The fixture takes a couple of minutes to normalise, so reuse it unless the
# scripts that build it, the tools that normalise it or the requested file
# counts (NUM_* in generate-project.sh) have changed.
inputs=$({
    cat "$BENCH/generate-project.sh" "$BENCH"/lib/*.sh "$BENCH/mise.toml"
    env | grep '^NUM_' | sort || true # none set is the default
} | sha256sum | cut -c1-16)
if [ "${REGENERATE:-0}" != "0" ] || [ "$(cat "$WORK/fixture.inputs" 2>/dev/null)" != "$inputs" ]; then
    "$BENCH/generate-project.sh" "$WORK/fixture"
    echo "$inputs" >"$WORK/fixture.inputs"
fi

dirty_files() {
    git diff --name-only clean dirty -- . ':!hk.pkl' ':!lefthook.yml' ':!.pre-commit-config.yaml'
}
# A commit-sized change for the staged scenario: every 25th dirty file, about
# 60, in the same language mix as the project.
staged_files() { dirty_files | awk 'NR % 25 == 1'; }
# A small commit: the first dirty file of each type, so each language's hooks
# have one file to fix. The linters' own configuration files are skipped.
small_commit_files() {
    dirty_files | grep -vxE 'eslint\.config\.js|tsconfig\.json' | awk -F. '!seen[$NF]++'
}

for subject in "${!CONFIG[@]}"; do
    dir="$WORK/$subject"
    rm -rf "$dir"
    git clone -q "$WORK/fixture" "$dir"
    cd "$dir"
    git config user.name benchmark
    git config user.email benchmark@example.invalid
    git config commit.gpgsign false
    git fetch -q --tags

    git checkout -q clean
    cp -R "$BENCH/subjects/${CONFIG[$subject]}/." .
    if [ -f hk.pkl ]; then sed -i "s|@PKL@|$HK_PKL|g" hk.pkl; fi
    # The workload's yq fixer formats every YAML file, the tool's own
    # configuration included; commit it already formatted so it is not a diff.
    for f in lefthook.yml .pre-commit-config.yaml; do
        if [ -f "$f" ]; then yq -iP "$f"; fi
    done
    git add -A
    git commit -q -m "configure $subject"
    git tag -f clean
    git cherry-pick "$(git rev-parse dirty)" >/dev/null
    git tag -f dirty
    git checkout -q -B main dirty
    staged_files >.git/staged-files
    small_commit_files >.git/small-commit-files

    mkdir -p .git/cache .git/state
done

# Install pre-commit-hooks for pre-commit and prek outside the timed runs, as a
# developer would have before their first commit.
for tool in pre-commit prek; do
    cd "$WORK/$tool"
    if [ "$tool" = prek ]; then cmd=prepare-hooks; else cmd=install-hooks; fi
    PRE_COMMIT_HOME=.git/cache PREK_HOME=.git/cache "$tool" "$cmd" >/dev/null
done

# Prime hk's configuration cache so no sample pays for Pkl evaluation, which
# is what a developer's second commit looks like.
cd "$WORK/hk"
HK_CACHE_DIR=.git/cache HK_STATE_DIR=.git/state hk validate --quiet

echo "Prepared ${#CONFIG[@]} subjects in $WORK"
