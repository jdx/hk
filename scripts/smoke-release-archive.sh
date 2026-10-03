#!/usr/bin/env bash
# Smoke-test an extracted hk release archive before it is published.
#
# Usage: scripts/smoke-release-archive.sh <extracted-dir> <target-triple>
#
# Runs the shipped binary, not a rebuilt one: `hk --version`, `hk validate` and
# `hk check` against a tiny hk.pkl that pins this exact version. Config
# evaluation is forced offline, so it only passes if the pkl package embedded in
# the binary (HK_REQUIRE_EMBEDDED_PKL) really made it into the archive. musl
# targets must also be statically linked.
#
# Works under bash on Linux, macOS, and Git Bash on Windows, so the Unix and
# Windows release rows share it.
set -euo pipefail

dir=${1:?usage: smoke-release-archive.sh <extracted-dir> <target-triple>}
target=${2:?usage: smoke-release-archive.sh <extracted-dir> <target-triple>}

case "$target" in
  *windows*) exe=hk.exe ;;
  *) exe=hk ;;
esac

dir=$(cd "$dir" && pwd)
bin="$dir/$exe"
if [ ! -f "$bin" ]; then
  echo "::error::$exe is missing from the archive for $target"
  ls -la "$dir"
  exit 1
fi
if [ ! -d "$dir/skills/hk-configure" ]; then
  echo "::error::skills/ is missing from the archive for $target"
  exit 1
fi

echo "== hk --version"
version_line=$("$bin" --version)
echo "$version_line"
version=$(printf '%s\n' "$version_line" | awk '{print $2}')
if [ -z "$version" ]; then
  echo "::error::could not parse a version from: $version_line"
  exit 1
fi

case "$target" in
  *-linux-musl)
    echo "== static link check"
    # A static executable has no dynamic loader (PT_INTERP) and no NEEDED
    # entries; either one means the musl build picked up shared libraries.
    # readelf reads the ELF headers, so this holds for any architecture.
    if readelf -l "$bin" | grep -q 'INTERP'; then
      echo "::error::$target binary requests a dynamic loader, so it is not static"
      readelf -l "$bin" | grep -A1 'INTERP'
      exit 1
    fi
    if readelf -d "$bin" 2>/dev/null | grep -q '(NEEDED)'; then
      echo "::error::$target binary depends on shared libraries"
      readelf -d "$bin" | grep '(NEEDED)'
      exit 1
    fi
    echo "statically linked"
    ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"
git init --quiet .
printf 'hello\n' > hello.txt
cat > hk.pkl <<PKL
amends "package://github.com/jdx/hk/releases/download/v${version}/hk@${version}#/Config.pkl"

local linters = new Mapping<String, Step> {
  ["smoke"] = new Step {
    glob = "*.txt"
    check = "git --version"
  }
}

hooks {
  ["check"] {
    steps = linters
  }
}
PKL
git add hello.txt hk.pkl

# Never reach the network or a developer's cache: the embedded package has to
# satisfy the amends line on its own.
export HK_PKL_OFFLINE=1
export HK_PKL_CACHE_DIR="$work/pkl-cache"

echo "== hk validate"
"$bin" validate
echo "== hk check --all"
"$bin" check --all
echo "smoke ok: $target (hk $version)"
