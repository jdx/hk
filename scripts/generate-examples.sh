#!/usr/bin/env bash
set -euo pipefail

# Script to extract and validate examples from docs/public/*.pkl files
# These examples can be embedded in the documentation

PUBLIC_DIR="docs/public"
OUTPUT_DIR="docs/reference/examples"

if [ ! -d "$PUBLIC_DIR" ]; then
    echo "No public examples directory found at $PUBLIC_DIR"
    exit 0
fi

echo "Extracting examples from $PUBLIC_DIR..."

mkdir -p "$OUTPUT_DIR"

# Process each .pkl file in the public directory
for pkl_file in "$PUBLIC_DIR"/*.pkl; do
    if [ ! -f "$pkl_file" ]; then
        continue
    fi

    basename=$(basename "$pkl_file" .pkl)
    output_file="$OUTPUT_DIR/${basename}.md"

    echo "Processing $pkl_file -> $output_file"

    cat > "$output_file" << EOF
# Example: ${basename}

\`\`\`pkl
$(cat "$pkl_file")
\`\`\`

## Description

$(grep -E "^///" "$pkl_file" 2>/dev/null | sed 's|^///[ ]*||' || echo "No description available.")

## Key Features

$(grep -E "^//\s+\*" "$pkl_file" 2>/dev/null | sed 's|^//[ ]*||' || echo "- Standard configuration")

EOF
done

# Generate index file
cat > "$OUTPUT_DIR/index.md" << EOF
# Configuration Examples

This directory contains runnable examples extracted from the public Pkl configurations.

## Available Examples

EOF

for pkl_file in "$PUBLIC_DIR"/*.pkl; do
    if [ ! -f "$pkl_file" ]; then
        continue
    fi
    basename=$(basename "$pkl_file" .pkl)
    echo "- [${basename}](./${basename}.md)" >> "$OUTPUT_DIR/index.md"
done

echo "Examples generated in $OUTPUT_DIR"
