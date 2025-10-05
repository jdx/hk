#!/usr/bin/env bash
set -eux

if [[ "$OSTYPE" == "darwin"* ]]; then
    # Ensure gsed is installed
    if ! command -v gsed &> /dev/null; then
        echo "gsed is required on macOS. Install with: brew install gnu-sed" >&2
        exit 1
    fi
    SED="gsed"
else
    SED="sed"
fi

# Find files matching the pattern - using newline-separated list for simplicity
files=$(rg 'package://github\.com/jdx/hk/releases/download/v[\d\.]+/hk@[\d\.]+#/' --files-with-matches || true)

# Update each file if any were found
if [[ -n "$files" ]]; then
    echo "$files" | while IFS= read -r file; do
        "$SED" -i "s|package://github\.com/jdx/hk/releases/download/v[0-9.]\+/hk@[0-9.]\+#|package://github.com/jdx/hk/releases/download/v$VERSION/hk@$VERSION#|g" "$file"
    done
fi

git add .
