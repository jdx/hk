#!/usr/bin/env bash
set -eux

if [[ "$OSTYPE" == "darwin"* ]]; then
    if ! command -v gsed &> /dev/null; then
        echo "gsed is required on macOS. Install with: brew install gnu-sed" >&2
        exit 1
    fi
    SED="gsed"
else
    SED="sed"
fi

# Find files matching the version pattern
# Explicitly specify current directory (.) to ensure rg searches all files
files=$(rg 'package://github\.com/jdx/hk/releases/download/v[0-9.]+/hk@[0-9.]+#/' --files-with-matches . || true)

# Update each file if any were found
if [[ -n "$files" ]]; then
    echo "$files" | while IFS= read -r file; do
        "$SED" -i "s|package://github\.com/jdx/hk/releases/download/v[0-9.]\+/hk@[0-9.]\+#|package://github.com/jdx/hk/releases/download/v$VERSION/hk@$VERSION#|g" "$file"
    done
fi

# The CLI reference's pirate variant repeats the version its English page
# was generated with (docs/pirate/STYLE.md).
if [[ -f docs/pirate/cli/index.md ]]; then
    "$SED" -i "s|^\*\*Version:\*\* .*|**Version:** $VERSION|" docs/pirate/cli/index.md
fi

git add .
