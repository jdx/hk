<script setup>
import { ref, onMounted } from 'vue'

const data = ref(null)
const loading = ref(true)

onMounted(async () => {
  try {
    const res = await fetch('/benchmark-data.json')
    data.value = await res.json()
  } catch (e) {
    console.warn('Could not load benchmark data:', e)
  }
  loading.value = false
})

function fmt(seconds) {
  if (!seconds) return '—'
  if (seconds < 0.1) return `${(seconds * 1000).toFixed(0)}ms`
  return `${seconds.toFixed(2)}s`
}

function speedup(scenario, tool) {
  const key = `${tool}_vs_hk`
  const s = data.value?.scenarios?.[scenario]
  if (!s || !s[key]) return ''
  const ratio = s[key]
  if (ratio >= 1.05) return `${ratio.toFixed(1)}x slower`
  if (ratio <= 0.95) return `${(1/ratio).toFixed(1)}x faster`
  return '~same'
}
</script>

# Benchmarks

These benchmarks compare hk, lefthook, pre-commit, and prek running 10 linters on a synthetic project. Since lefthook's `parallel: true` mode has race conditions when linters touch overlapping files, we run lefthook in safe (sequential) mode—the only correct option. pre-commit and prek both run hooks sequentially.

hk is the only tool that runs linters in parallel **and** safely.

See [Why hk?](/why-hk) for context on why these differences exist.

## Setup

A synthetic project with **~6,000 files** across multiple languages:

- 4000 Python, 500 JavaScript/TypeScript, 500 JSON, 500 Shell, 250 YAML, 200 CSS, 200 Markdown

Ten linters with overlapping file coverage:

| Linter | Files | How hk avoids write locks |
|--------|-------|---------------------------|
| prettier | `*.{js,ts,css,md}` | `check_list_files` — only locks files that need fixing |
| eslint | `*.{js,ts}` | Falls back to write lock (eslint has no diff/list mode) |
| black | `*.py` | `check_diff` — hk applies the diff itself |
| ruff check | `*.py` | Check only — read lock |
| ruff format | `*.py` | `check_diff` — hk applies the diff itself |
| jq | `*.json` | `check_diff` |
| yq | `*.{yml,yaml}` | `check_diff` |
| shfmt | `*.{sh,bash}` | `check_diff` |
| trailing-whitespace | `*` (all files) | `check_diff` via `hk util` (built-in Rust) |
| newlines | `*` (all files) | `check_diff` via `hk util` (built-in Rust) |

The `trailing-whitespace` and `newlines` linters use `**/*` globs—they overlap with **every other linter**. pre-commit and prek always run hooks sequentially. lefthook supports `parallel: true` but has no file-level coordination, so overlapping linters cause race conditions—we run it in safe (sequential) mode. hk is the only tool that runs everything in parallel safely, using file-level read/write locks.

## Results

![Benchmark results](/benchmark.png)

<div v-if="data">

### All Files ({{ data.total_files || '~1750' }} files, 10 linters)

| Tool | Time | |
|------|------|-|
| **hk** | **{{ fmt(data.scenarios.all_files?.hk?.mean) }}** | |
| lefthook | {{ fmt(data.scenarios.all_files?.lefthook?.mean) }} | {{ speedup('all_files', 'lefthook') }} |
| pre-commit | {{ fmt(data.scenarios.all_files?.['pre-commit']?.mean) }} | {{ speedup('all_files', 'pre-commit') }} |
| prek | {{ fmt(data.scenarios.all_files?.prek?.mean) }} | {{ speedup('all_files', 'prek') }} |

### Staged Changes ({{ data.staged_files || '~50' }} files)

| Tool | Time | |
|------|------|-|
| **hk** | **{{ fmt(data.scenarios.staged_changes?.hk?.mean) }}** | |
| lefthook | {{ fmt(data.scenarios.staged_changes?.lefthook?.mean) }} | {{ speedup('staged_changes', 'lefthook') }} |
| pre-commit | {{ fmt(data.scenarios.staged_changes?.['pre-commit']?.mean) }} | {{ speedup('staged_changes', 'pre-commit') }} |
| prek | {{ fmt(data.scenarios.staged_changes?.prek?.mean) }} | {{ speedup('staged_changes', 'prek') }} |

<p style="color: #888; font-size: 0.85em;">Last generated: {{ data.generated?.split('T')[0] }}</p>

</div>

## Reproducing

Everything is in the `benchmark/` directory.

### Prerequisites

```bash
mise use hyperfine lefthook prettier eslint shfmt jq yq
uv tool install pre-commit prek black ruff
```

### Generate and run

```bash
# Generate a synthetic project (~700 files)
benchmark/generate-project.sh /tmp/hk-bench

# Run benchmarks
benchmark/run.sh /tmp/hk-bench

# Customize
NUM_JS=500 NUM_PY=500 benchmark/generate-project.sh /tmp/hk-bench
RUNS=20 WARMUP=3 benchmark/run.sh /tmp/hk-bench
```

Results are saved as JSON in `benchmark/results/` and both a chart and data file are generated in `docs/public/`.
