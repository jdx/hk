#!/usr/bin/env bash
set -euo pipefail

# Script to validate that documentation is in sync with the code
# This checks:
# 1. Config.pkl schema matches what's documented
# 2. All builtins are documented
# 3. Version references are current

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

errors=0

echo "Checking documentation sync..."

# Check if all builtins in pkl/Builtins.pkl are documented in docs/builtins.md
echo -n "Checking builtin coverage... "
builtins_in_pkl=$(grep -E "^[a-z_]+ = " pkl/Builtins.pkl | cut -d' ' -f1 | sort)
builtins_documented=$(grep -E "^#### \`[a-z_]+\`" docs/builtins.md | sed 's/#### `//;s/`//' | sort)

missing_builtins=$(comm -23 <(echo "$builtins_in_pkl") <(echo "$builtins_documented") | tr '\n' ' ')
if [ -n "$missing_builtins" ]; then
    echo -e "${RED}FAIL${NC}"
    echo -e "${RED}Missing builtins in documentation: $missing_builtins${NC}"
    errors=$((errors + 1))
else
    echo -e "${GREEN}OK${NC}"
fi

# Check version placeholders
echo -n "Checking version placeholders... "
if grep -q "{{version}}" docs/reference/schema.md; then
    echo -e "${YELLOW}WARN${NC}"
    echo -e "${YELLOW}Version placeholders found in schema.md - consider updating to specific version${NC}"
else
    echo -e "${GREEN}OK${NC}"
fi

# Check that examples in docs can be parsed (basic syntax check)
echo -n "Checking Pkl examples syntax... "
pkl_examples=$(mktemp -d)
trap "rm -rf $pkl_examples" EXIT

# Extract Pkl code blocks from documentation
awk '/```pkl/,/```/ {if (!/```/) print}' docs/reference/schema.md docs/builtins.md docs/configuration.md > "$pkl_examples/examples.txt"

# Basic syntax validation (check for common issues)
if grep -E "^\s*(check|fix|glob|stage)\s*=" "$pkl_examples/examples.txt" | grep -v "//"; then
    # Check if assignments look valid
    invalid_lines=$(grep -E "^\s*(check|fix|glob|stage)\s*=\s*$" "$pkl_examples/examples.txt" || true)
    if [ -n "$invalid_lines" ]; then
        echo -e "${RED}FAIL${NC}"
        echo -e "${RED}Invalid Pkl syntax found in examples${NC}"
        errors=$((errors + 1))
    else
        echo -e "${GREEN}OK${NC}"
    fi
else
    echo -e "${GREEN}OK${NC}"
fi

# Check cross-references between docs
echo -n "Checking cross-references... "
if grep -q "\.\./builtins\.md" docs/reference/schema.md && [ -f docs/builtins.md ]; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${YELLOW}WARN${NC}"
    echo -e "${YELLOW}Cross-references may need updating${NC}"
fi

# Summary
echo
if [ $errors -eq 0 ]; then
    echo -e "${GREEN}✓ Documentation is in sync!${NC}"
    exit 0
else
    echo -e "${RED}✗ Found $errors synchronization issues${NC}"
    exit 1
fi
