#!/usr/bin/env bash
# Creates the repository that the `stash` benchmark in tak.toml runs in.
#
# 400 small tracked files plus hk.pkl, and the two changes that `prepare`
# replays before every sample as patches kept in .git:
#
#   staged.patch    edits the first line of 60 files, to be staged
#   unstaged.patch  edits a later line of 50 files, left in the worktree: 40
#                   files with no staged change and 10 of the staged ones, so a
#                   hook has partially staged files to set aside and restore
#
# hk.pkl is copied into the repository, with the path of Config.pkl made
# absolute: a config outside the repository would have its steps scoped to
# another directory.
#
# Usage: fixture.sh DIR
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
dir=$1
rm -rf "$dir"
mkdir -p "$dir"
cd "$dir"

# Edits line $2 of file $1 in place, which BSD and GNU sed spell differently
set_line() {
  sed "$2s/.*/$3/" "$1" >"$1.tmp"
  mv "$1.tmp" "$1"
}

git init -q -b main .
git config user.name bench
git config user.email bench@example.invalid
git config commit.gpgsign false

sed "s|../../pkl/Config.pkl|$root/pkl/Config.pkl|" "$root/benchmark/stash/hk.pkl" >hk.pkl
for d in $(seq 1 20); do
  mkdir -p "src/d$d"
  for f in $(seq 1 20); do
    for line in $(seq 1 20); do
      echo "d$d f$f line $line"
    done >"src/d$d/f$f.txt"
  done
done
git add -A
git commit -q -m clean
git tag clean

# Staged: files 1..60 in directory order
for i in $(seq 1 60); do
  set_line "src/d$(((i - 1) / 20 + 1))/f$(((i - 1) % 20 + 1)).txt" 1 "staged edit"
done
git diff >.git/staged.patch
git checkout -q -- .

# Unstaged: files 51..100, so files 51..60 also carry a staged change
for i in $(seq 51 100); do
  set_line "src/d$(((i - 1) / 20 + 1))/f$(((i - 1) % 20 + 1)).txt" 15 "unstaged edit"
done
git diff >.git/unstaged.patch
git checkout -q -- .
