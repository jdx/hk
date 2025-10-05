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

# Debug: show current directory and ripgrep version
echo "Current directory: $(pwd)"
echo "Ripgrep version: $(rg --version | head -1)"

# Find files matching the pattern - using newline-separated list for simplicity
files=$(rg 'package://github\.com/jdx/hk/releases/download/v[\d\.]+/hk@[\d\.]+#/' --files-with-matches || true)

# Debug: show what we found
echo "Found files:"
echo "$files"
echo "File count: $(echo "$files" | grep -c . || echo 0)"

# Update each file if any were found
if [[ -n "$files" ]]; then
    echo "$files" | while IFS= read -r file; do
        echo "Updating $file"
        "$SED" -i "s|package://github\.com/jdx/hk/releases/download/v[0-9.]\+/hk@[0-9.]\+#|package://github.com/jdx/hk/releases/download/v$VERSION/hk@$VERSION#|g" "$file"
    done
else
    echo "WARNING: No files found matching the pattern"
fi

git add .
