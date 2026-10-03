#!/usr/bin/env bash
# Creates the repository that the `stash` benchmark in tak.toml runs in.
#
# 400 small tracked files, plus the two changes that `prepare` replays before
# every sample as patches kept in .git:
#
#   staged.patch    edits the first line of 60 files, to be staged
#   unstaged.patch  edits a later line of 50 files, left in the worktree: 40
#                   files with no staged change and 10 of the staged ones, so a
#                   hook has partially staged files to set aside and restore
#
# Usage: fixture.sh DIR
set -euo pipefail

dir=$1
rm -rf "$dir"
mkdir -p "$dir"
cd "$dir"

git init -q -b main .
git config user.name bench
git config user.email bench@example.invalid
git config commit.gpgsign false

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
  d=$(((i - 1) / 20 + 1))
  f=$(((i - 1) % 20 + 1))
  sed -i '1s/.*/staged edit/' "src/d$d/f$f.txt"
done
git diff >.git/staged.patch
git checkout -q -- .

# Unstaged: files 51..100, so files 51..60 also carry a staged change
for i in $(seq 51 100); do
  d=$(((i - 1) / 20 + 1))
  f=$(((i - 1) % 20 + 1))
  sed -i '15s/.*/unstaged edit/' "src/d$d/f$f.txt"
done
git diff >.git/unstaged.patch
git checkout -q -- .
