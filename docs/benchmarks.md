---
description: How hk compares with lefthook, pre-commit, and prek on wall time and on whether each tool produces the right files, with the method used and how to reproduce it.
---

# Benchmarks

hk runs fixers in parallel and uses file locks so that two of them never write the same file at once. This page compares hk with lefthook, pre-commit, and prek on how long each run takes and whether it leaves the right files behind.

<BenchmarkResults />

## What the results show

hk is the fastest of the four in every scenario, and every tool produces the right files.

- **Fixing every file**, the gap is smallest. pre-commit and prek already use every core by splitting each hook's files into batches, and hk also runs different fixers at the same time. lefthook runs one fixer at a time over every file and takes more than twice as long as hk.
- **Checking every file**, hk stays ahead of lefthook even though lefthook starts every job at once here.
- **Committing** a few dozen files, the gap is largest. There is little work to split into batches, so most of each run is fixers starting up and finishing. hk runs the fixers at the same time and stages each one's files as soon as it finishes. The others run them one after another.

## The workload

A generated repository of about 6,000 files: 4,000 Python, 500 JavaScript and TypeScript, 500 JSON, 500 shell, 250 YAML, 200 CSS, and 200 Markdown. Every tool runs the same ten fixers over it: black, ruff format, ruff check, Prettier, ESLint, jq, yq, shfmt, trailing whitespace, and final newline.

The generator commits a `clean` state, the result of running the ten fixers one after another, and a `dirty` state in which a quarter of the files have defects. Each defective file needs both a language fixer and a whitespace fixer. After fixing `dirty`, a run is correct only if the tree is byte-for-byte identical to `clean`. tak checks this after every timed sample, and the page publishes a run only if every sample of every tool was correct.

- **Fix every file**: `hk fix --all` and equivalents, starting from `dirty`.
- **Check every file**: a read-only check of the `clean` tree, as in CI.
- **Commit**: about 60 files with defects are staged and each tool's pre-commit hook fixes them. hk and lefthook stage their fixes. pre-commit and prek leave them unstaged and fail the commit, which is how they are designed to work.

## How each tool runs

Each tool uses its fastest configuration in which two fixers can never write the same file at once:

| Tool       | Fixing                                                              | Checking                               |
| ---------- | ------------------------------------------------------------------- | -------------------------------------- |
| hk         | Steps in parallel; file locks keep two fixers off the same file.    | The same.                              |
| lefthook   | One job at a time, its default.                                     | `parallel: true`: every job at once.   |
| pre-commit | One hook at a time; each hook's files split into batches per CPU.   | The same.                              |
| prek       | Same as pre-commit.                                                 | The same.                              |

lefthook's `parallel: true` and prek's hooks that share a `priority` start fixers at once with nothing to stop two of them from writing the same file, and prek's documentation warns that this gives undefined results. Whether a race corrupts a run depends on timing, so it can pass on one machine and fail on another. Neither mode is used for fixing. lefthook's `parallel: true` is still used for checking, where nothing writes.

## Keeping it fair

- **The same work.** hk uses its builtins as shipped, including their check-before-fix behavior and the `hk util` whitespace fixers. The others run each linter's fix command directly, and [pre-commit-hooks](https://github.com/pre-commit/pre-commit-hooks)' `trailing-whitespace` and `end-of-file-fixer` for whitespace. prek swaps those for its bundled Rust versions, and lefthook calls the package's console scripts. See [`benchmark/subjects`](https://github.com/jdx/hk/tree/main/benchmark/subjects).
- **Pinned versions.** Every tool and runtime is pinned in [`benchmark/mise.toml`](https://github.com/jdx/hk/blob/main/benchmark/mise.toml).
- **Interleaved sampling.** [tak](https://github.com/jdx/tak) takes one sample of every tool per round, in a new random order each round, so thermal throttling or a noisy neighbour is shared across tools.
- **Same starting state.** Each tool has its own clone, reset before every sample. Caches such as ruff's and black's stay warm, as on a developer's machine, and each clone keeps its own.
- **Hermetic Git.** Global and system Git configuration is disabled, so the host's hooks, signing, and filesystem monitor stay out of the measurements.
- **Wins must beat the noise.** A tool is called faster only when the gap between medians is larger than both tools' ranges across samples.

## What this does not measure

- **Your repository.** The generated files are uniform. Real results depend on your linters, how much their file patterns overlap, how many files change, and how many cores you have. [hk timing reports](/logging#a-run-is-slow) show where your own runs spend their time.
- **CPU-bound linters.** Tools that already use every core, such as black, gain less from running steps concurrently.
- **Stashing.** In the commit scenario no file has unstaged changes, so saving and restoring unstaged work costs almost nothing.
- **Setup.** Install time, first runs with cold caches, and features beyond running fixers.

## Reproduce

The benchmark runs on Linux (its scripts use GNU sed) and needs [mise](https://mise.jdx.dev) and a checkout of hk:

```sh
mise run benchmark
```

This builds hk, generates the project in `~/.cache/hk-bench`, times every scenario, and writes `benchmark/results.json`, which this page renders. Pass tak flags to narrow a run, for example `mise run benchmark -- --bench fix-staged --runs 5`; such a run is never marked publishable. Set `HK_BIN` to measure a released hk. [`benchmark/README.md`](https://github.com/jdx/hk/blob/main/benchmark/README.md) explains how the pieces fit together.
