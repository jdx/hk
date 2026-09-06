---
description: Recorded hk benchmark results, workload configuration, limitations, and reproduction commands.
---

<script setup>
import data from './public/benchmark-data.json'

function fmt(seconds) {
  return Number.isFinite(seconds) ? seconds.toFixed(2) + ' s' : '—'
}
</script>

# Benchmarks

These results measure one synthetic workload with overlapping linter file patterns. They illustrate how orchestration affects this setup; they are not a prediction for every repository or a survey of every tool’s available configuration.

The recorded run was generated on **{{ data.generated.split('T')[0] }}**. No new benchmark run is implied by this page’s last-updated date.

## Recorded results

Mean wall time, in seconds. Lower is faster.

| Tool       | All files ({{ data.total_files }})                     | Staged changes ({{ data.staged_files }})                    |
| ---------- | ------------------------------------------------------ | ----------------------------------------------------------- |
| hk         | {{ fmt(data.scenarios.all_files.hk.mean) }}            | {{ fmt(data.scenarios.staged_changes.hk.mean) }}            |
| lefthook   | {{ fmt(data.scenarios.all_files.lefthook.mean) }}      | {{ fmt(data.scenarios.staged_changes.lefthook.mean) }}      |
| pre-commit | {{ fmt(data.scenarios.all_files['pre-commit'].mean) }} | {{ fmt(data.scenarios.staged_changes['pre-commit'].mean) }} |
| prek       | {{ fmt(data.scenarios.all_files.prek.mean) }}          | {{ fmt(data.scenarios.staged_changes.prek.mean) }}          |

![Recorded mean runtimes for all-files and staged-change scenarios; values are in the table above](/benchmark.png)

[Download the recorded data](/benchmark-data.json), including standard deviations, minimums, and maximums.

## Workload

The generator defaults to roughly 6,000 files: 4,000 Python, 500 JavaScript/TypeScript, 500 JSON, 500 shell, 250 YAML, 200 CSS, and 200 Markdown files, plus project configuration.

Ten configured steps include ESLint, Prettier, Black, Ruff linting, Ruff formatting, jq, yq, shfmt, trailing whitespace, and final newlines. The whitespace steps overlap with the language-specific steps.

The committed runner:

- Invokes hk’s pre-commit hook in fix mode, with stashing disabled through `HK_STASH=false`.
- Configures lefthook with sequential execution to avoid concurrent writes from overlapping formatters in this workload.
- Runs pre-commit and prek using the provided hook definitions.
- Resets the fixture between runs, primes hk’s configuration cache, and uses Hyperfine warmups and repeated measurements.

The [runner](https://github.com/jdx/hk/blob/main/benchmark/run.sh) and [tool configurations](https://github.com/jdx/hk/tree/main/benchmark/parallel) define the comparison. These choices matter as much as the timing values.

## Limitations

This workload favors concurrent work across languages while also exercising overlapping formatters. A small project, a single linter, or tools that already parallelize internally may see different results.

Stashing is disabled in the runner, so the results do not measure partial-commit restoration. Hyperfine is configured to tolerate nonzero exits from lint commands; timings alone do not establish equivalent fixes or successful checks.

The recorded JSON does not include machine specifications or exact tool versions. Treat it as a historical example and rerun the workload with those details recorded before using the numbers for a tool-selection decision. Current scripts may also differ from the ones used for the recorded result.

## Reproduce

Use a disposable directory. The benchmark runner resets its fixture repository and overwrites generated results.

Install `hk`, `hyperfine`, `lefthook`, `pre-commit`, `prek`, `prettier`, `eslint`, `black`, `ruff`, `shfmt`, `jq`, `yq`, and `uv`, and record their versions. The shell scripts expect a Unix-like environment and compatible command-line utilities.

From the repository root:

```sh
benchmark/generate-project.sh /tmp/hk-bench
benchmark/run.sh /tmp/hk-bench
```

To change the workload or number of repetitions:

```sh
NUM_JS=500 NUM_PY=500 benchmark/generate-project.sh /tmp/hk-bench
RUNS=20 WARMUP=3 benchmark/run.sh /tmp/hk-bench
```

Results are written to `benchmark/results/`; the runner also updates `docs/public/benchmark.png` and `docs/public/benchmark-data.json`.

For your own project, start with [hk timing reports](/logging#a-run-is-slow). See [Why hk?](/why-hk) for the execution model.
