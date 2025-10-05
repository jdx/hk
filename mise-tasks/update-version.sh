#!/usr/bin/env bash
set -euxo pipefail

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

rg 'package://github\.com/jdx/hk/releases/download/v[\d\.]+/hk@[\d\.]+#/' --files-with-matches -0 | xargs -0 -r "$SED" -i "s|package://github\.com/jdx/hk/releases/download/v[0-9.]\+/hk@[0-9.]\+#|package://github.com/jdx/hk/releases/download/v$VERSION/hk@$VERSION#|g"

git add .
