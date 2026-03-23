#!/usr/bin/env bash
# Generates a synthetic project for benchmarking hk vs other hook managers.
# The generated project has many files across multiple languages to demonstrate
# hk's parallel execution advantage.
#
# Usage: benchmark/generate-project.sh <output-dir>
set -euo pipefail

DIR="${1:?Usage: generate-project.sh <output-dir>}"
mkdir -p "$DIR"
cd "$DIR"

# Initialize git repo if needed
if [ ! -d .git ]; then
    git init -q
    git commit --allow-empty -m "initial" -q
fi

NUM_JS=${NUM_JS:-500}
NUM_PY=${NUM_PY:-4000}
NUM_JSON=${NUM_JSON:-500}
NUM_YAML=${NUM_YAML:-250}
NUM_CSS=${NUM_CSS:-200}
NUM_MD=${NUM_MD:-200}
NUM_SH=${NUM_SH:-500}

echo "Generating synthetic project in $DIR..."
echo "  JS files:   $NUM_JS"
echo "  PY files:   $NUM_PY"
echo "  JSON files: $NUM_JSON"
echo "  YAML files: $NUM_YAML"
echo "  CSS files:  $NUM_CSS"
echo "  MD files:   $NUM_MD"
echo "  SH files:   $NUM_SH"

# --- JavaScript/TypeScript files ---
mkdir -p src/components src/utils src/hooks src/api
for i in $(seq 1 "$NUM_JS"); do
    dir="src/components"
    if (( i % 4 == 1 )); then dir="src/utils"; fi
    if (( i % 4 == 2 )); then dir="src/hooks"; fi
    if (( i % 4 == 3 )); then dir="src/api"; fi
    ext="js"
    if (( i % 3 == 0 )); then ext="ts"; fi

    cat > "$dir/module_${i}.${ext}" << JSEOF
import { useState, useEffect } from "react"

const ITEMS = [
  { id: ${i}, name: "item_${i}", value: $((i * 17 % 100)) },
  { id: $((i + 100)), name: "item_$((i + 100))", value: $((i * 31 % 100)) },
]

export function processItems_${i}(items) {
  return items
    .filter((item) => item.value > 10)
    .map((item) => ({
      ...item,
      label: \`\${item.name} (\${item.value})\`,
      processed: true,
    }))
    .sort((a, b) => a.value - b.value)
}

export function calculateTotal_${i}(items) {
  let total = 0
  for (const item of items) {
    total += item.value
  }
  return total
}

export class DataManager_${i} {
  constructor() {
    this.items = [...ITEMS]
    this.cache = new Map()
  }

  getItem(id) {
    if (this.cache.has(id)) {
      return this.cache.get(id)
    }
    const item = this.items.find((i) => i.id === id)
    if (item) {
      this.cache.set(id, item)
    }
    return item
  }

  addItem(item) {
    this.items.push(item)
    this.cache.delete(item.id)
  }
}

export default { processItems_${i}, calculateTotal_${i}, DataManager_${i} }
JSEOF
done

# --- Python files ---
mkdir -p lib/models lib/services lib/utils
for i in $(seq 1 "$NUM_PY"); do
    dir="lib/models"
    if (( i % 3 == 1 )); then dir="lib/services"; fi
    if (( i % 3 == 2 )); then dir="lib/utils"; fi

    cat > "$dir/module_${i}.py" << PYEOF
"""Module ${i} - data processing utilities."""
from dataclasses import dataclass
from typing import List, Optional, Dict


@dataclass
class Record_${i}:
    """A data record for module ${i}."""
    id: int
    name: str
    value: float
    tags: List[str]
    metadata: Optional[Dict[str, str]] = None


def process_records_${i}(records: List[Record_${i}]) -> List[Record_${i}]:
    """Filter and transform records."""
    result = []
    for record in records:
        if record.value > $((i % 50)):
            record.name = record.name.strip().lower()
            record.tags = sorted(set(record.tags))
            result.append(record)
    return sorted(result, key=lambda r: r.value)


def aggregate_${i}(records: List[Record_${i}]) -> Dict[str, float]:
    """Aggregate record values by first tag."""
    aggregation: Dict[str, float] = {}
    for record in records:
        if record.tags:
            key = record.tags[0]
            aggregation[key] = aggregation.get(key, 0.0) + record.value
    return aggregation


class DataPipeline_${i}:
    """Pipeline for processing module ${i} data."""

    def __init__(self, batch_size: int = 100):
        self.batch_size = batch_size
        self._cache: Dict[int, Record_${i}] = {}

    def load(self, records: List[Record_${i}]) -> None:
        for record in records:
            self._cache[record.id] = record

    def transform(self) -> List[Record_${i}]:
        return process_records_${i}(list(self._cache.values()))

    def summary(self) -> Dict[str, float]:
        return aggregate_${i}(self.transform())
PYEOF
done

# Create __init__.py files
touch lib/__init__.py lib/models/__init__.py lib/services/__init__.py lib/utils/__init__.py

# --- JSON files ---
mkdir -p config data
for i in $(seq 1 "$NUM_JSON"); do
    dir="config"
    if (( i % 2 == 0 )); then dir="data"; fi

    cat > "$dir/settings_${i}.json" << JSONEOF
{
  "id": ${i},
  "name": "config_${i}",
  "version": "1.${i}.0",
  "enabled": $([ $((i % 2)) -eq 0 ] && echo "true" || echo "false"),
  "settings": {
    "timeout": $((i * 100)),
    "retries": $((i % 5)),
    "batch_size": $((i * 10)),
    "features": {
      "feature_a": true,
      "feature_b": false,
      "feature_c": $([ $((i % 3)) -eq 0 ] && echo "true" || echo "false")
    },
    "endpoints": [
      "https://api.example.com/v${i}/resource",
      "https://api.example.com/v${i}/health"
    ]
  },
  "metadata": {
    "created_by": "generator",
    "module": "benchmark_${i}"
  }
}
JSONEOF
done

# --- YAML files ---
mkdir -p config/workflows
for i in $(seq 1 "$NUM_YAML"); do
    cat > "config/workflows/pipeline_${i}.yml" << YAMLEOF
name: Pipeline ${i}
version: "1.${i}"

stages:
  - name: build_${i}
    timeout: $((i * 60))
    steps:
      - run: echo "Building stage ${i}"
      - run: echo "Testing stage ${i}"
    environment:
      NODE_ENV: production
      LOG_LEVEL: info

  - name: deploy_${i}
    timeout: $((i * 120))
    depends_on:
      - build_${i}
    steps:
      - run: echo "Deploying ${i}"
    environment:
      DEPLOY_TARGET: staging

settings:
  parallel: true
  max_retries: $((i % 3 + 1))
  notifications:
    slack: true
    email: false
YAMLEOF
done

# --- CSS files ---
mkdir -p src/styles
for i in $(seq 1 "$NUM_CSS"); do
    cat > "src/styles/component_${i}.css" << CSSEOF
.component-${i} {
  display: flex;
  flex-direction: column;
  padding: $((i * 2))px;
  margin: $((i))px;
  background-color: #$( printf '%02x%02x%02x' $((i*5%256)) $((i*7%256)) $((i*11%256)) );
  border-radius: $((i % 8 + 2))px;
}

.component-${i} .header {
  font-size: $((14 + i % 10))px;
  font-weight: bold;
  color: #333333;
  margin-bottom: 8px;
}

.component-${i} .content {
  flex: 1;
  padding: 16px;
  line-height: 1.5;
}

.component-${i} .footer {
  display: flex;
  justify-content: space-between;
  padding-top: 8px;
  border-top: 1px solid #eeeeee;
}

@media (max-width: 768px) {
  .component-${i} {
    padding: $((i))px;
    margin: $((i / 2))px;
  }
}
CSSEOF
done

# --- Markdown files ---
mkdir -p docs
for i in $(seq 1 "$NUM_MD"); do
    cat > "docs/page_${i}.md" << MDEOF
# Module ${i} Documentation

## Overview

This module provides data processing utilities for component ${i}.

## Usage

\`\`\`javascript
import { processItems_${i} } from "./module_${i}"

const result = processItems_${i}(data)
\`\`\`

## Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| timeout | number | $((i * 100)) | Request timeout in ms |
| retries | number | $((i % 5)) | Number of retry attempts |
| batchSize | number | $((i * 10)) | Items per batch |

## API Reference

### \`processItems_${i}(items)\`

Filters and transforms the input items.

**Parameters:**
- \`items\` - Array of item objects

**Returns:** Processed array sorted by value

### \`calculateTotal_${i}(items)\`

Calculates the sum of all item values.

**Parameters:**
- \`items\` - Array of item objects

**Returns:** Total sum as a number
MDEOF
done

# --- Shell scripts ---
mkdir -p scripts
for i in $(seq 1 "$NUM_SH"); do
    cat > "scripts/task_${i}.sh" << 'SHEOF'
#!/usr/bin/env bash
set -euo pipefail

SHEOF

    cat >> "scripts/task_${i}.sh" << SHEOF
LOG_PREFIX="[task_${i}]"

log() {
    echo "\$LOG_PREFIX \$(date '+%Y-%m-%d %H:%M:%S') \$*"
}

process_batch() {
    local batch_size=\${1:-10}
    local input_dir=\${2:-.}
    local output_dir=\${3:-./output}

    mkdir -p "\$output_dir"
    log "Processing batch of \$batch_size from \$input_dir"

    local count=0
    for file in "\$input_dir"/*; do
        if [ -f "\$file" ]; then
            cp "\$file" "\$output_dir/"
            count=\$((count + 1))
            if [ \$count -ge \$batch_size ]; then
                break
            fi
        fi
    done

    log "Processed \$count files"
}

cleanup() {
    log "Cleaning up temporary files"
    rm -rf "/tmp/task_${i}_*"
}

trap cleanup EXIT

main() {
    log "Starting task ${i}"
    process_batch "\$@"
    log "Task ${i} complete"
}

main "\$@"
SHEOF
    chmod +x "scripts/task_${i}.sh"
done

# --- Config files for linters ---
cat > eslint.config.js << 'ESLINTEOF'
export default [
  {
    rules: {
      "no-unused-vars": "warn",
    },
  },
]
ESLINTEOF

cat > .prettierrc << 'PRETTIEREOF'
{
  "semi": false,
  "singleQuote": false,
  "trailingComma": "all"
}
PRETTIEREOF

# Stage all files
git add -A
git commit -q -m "generated benchmark project"

TOTAL=$(find . -type f -not -path './.git/*' | wc -l)
echo "Done! Generated $TOTAL files."
