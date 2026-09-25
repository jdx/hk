---
description: How hk compares with lefthook, pre-commit, and prek on wall time and on whether each tool produces the right files, with the method used and how to reproduce it.
---

# Benchmarks

Running linters in parallel is easy. Running them in parallel without two fixers overwriting each other's work is the hard part, and it is what hk is built around. This benchmark therefore measures two things for every tool: how long a run takes, and whether the files it leaves behind are right.

<BenchmarkResults />

## What is measured

**The project.** A generated repository of about 6,000 files: 4,000 Python, 500 JavaScript and TypeScript, 500 JSON, 500 shell, 250 YAML, 200 CSS, and 200 Markdown. Every tool runs the same ten fixers over it: black, ruff format, ruff check, Prettier, ESLint, jq, yq, shfmt, trailing whitespace, and final newline. The whitespace fixers apply to every file, so every file is written by at least two of them.

**Correct output.** The generator commits a `clean` state, which is the result of running the ten fixers one after another, then a `dirty` state in which a quarter of the files have formatting defects. Each defective file needs a language fixer _and_ a whitespace fixer to write it. After fixing `dirty`, a tool is correct only if the tree is byte-for-byte identical to `clean`. tak checks this after every timed sample, so the page reports, for each tool, how many of the very runs it timed were correct.

**Scenarios.**

- **Fix every file**: `hk fix --all` and equivalents, starting from `dirty`.
- **Check every file**: a read-only check of the whole `clean` tree, as in CI. Nothing writes, so running in parallel is safe for every tool here.
- **Commit**: about 60 files with defects are staged and each tool's pre-commit hook fixes them. hk and lefthook stage their fixes. pre-commit and prek leave them unstaged and fail the commit, which is how they are designed to work.

**Tools and modes.** Each tool has one configuration: the fastest it offers that still produces the right files in every scenario.

| Tool       | Fixing (fix every file, commit)                                                                 | Checking (check every file)                                            |
| ---------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| hk         | Builtins with defaults. Steps run in parallel; per-file read and write locks keep two fixers from writing one file at once. | The same.                                                              |
| lefthook   | One job at a time, its default.                                                                  | `parallel: true`: every job starts at once.                            |
| pre-commit | One hook at a time. Each hook's files are split into batches that run across CPUs.              | The same.                                                              |
| prek       | Same model as pre-commit.                                                                        | `priority: 0` on every hook, so the hooks run concurrently.            |

lefthook's `parallel: true` and prek's `priority: 0` start fixers at once with nothing to stop two of them writing the same file, and here every file is written by two or three fixers. prek's documentation warns that hooks in one priority group that modify the same files give undefined results. When both settings were measured on the fixing scenarios, every run left wrong files ([#1463](https://github.com/jdx/hk/pull/1463)), so they are used only for checking, where nothing writes.

## Keeping it fair

- **The same work.** Every configuration runs the same ten fixers and must produce the same bytes. hk uses its builtins as shipped, including their check-before-fix behavior and the `hk util` whitespace fixers. The competitors run each linter's fix command directly. For whitespace they use [pre-commit-hooks](https://github.com/pre-commit/pre-commit-hooks)' `trailing-whitespace` and `end-of-file-fixer`, the fixers pre-commit's documentation points to. prek replaces those two with its own bundled Rust implementation, and lefthook, which has no built-in fixers, calls the same package's console scripts. The configurations are in [`benchmark/subjects`](https://github.com/jdx/hk/tree/main/benchmark/subjects).
- **Pinned versions.** Every hook manager, linter, and runtime is pinned in [`benchmark/mise.toml`](https://github.com/jdx/hk/blob/main/benchmark/mise.toml). A refresh changes one variable at a time, and the page lists the versions that were measured.
- **Interleaved sampling.** Timing uses [tak](https://github.com/jdx/tak). tak takes one sample of every tool per round, in a new random order each round, so thermal throttling or a noisy neighbour is shared across tools instead of landing on whichever tool happened to be running.
- **Same starting state.** Each tool has its own clone of the project. Before every sample, an untimed step resets the clone to the scenario's starting state. Tool caches such as ruff's and black's are left warm, as they would be on a developer's machine, and each clone keeps its own so that one tool's runs cannot evict another's entries.
- **Hermetic Git.** Global and system Git configuration is disabled, so the benchmark host's hooks, signing, and filesystem monitor stay out of the measurements.
- **Wins must beat the noise.** A tool is called faster only when the gap between medians is larger than both tools' ranges across samples.
- **No partial results.** The page shows a run only if every tool produced the right files in every timed sample. Otherwise the harness is broken or a tool has a bug, and neither should be published as a timing.

## What this does not measure

- A real repository. The generated files are uniform. In a real project, results depend on your linters, how much their file patterns overlap, how many files change, and how many cores you have. [hk timing reports](/logging#a-run-is-slow) show where your own runs spend their time.
- Tools that already parallelize internally benefit less from running steps concurrently. black, for example, uses every core by itself. The more CPU-bound the workload, the less orchestration matters.
- Stashing of partially staged files. In the commit scenario no files have unstaged changes, so saving and restoring unstaged work costs almost nothing.
- Install time (pre-commit and prek install pre-commit-hooks before the first sample), first runs with cold caches, and hook-manager features beyond running fixers.

## Reproduce

The benchmark runs on Linux (its scripts use GNU sed) and needs [mise](https://mise.jdx.dev), which installs the pinned toolset, and a checkout of hk:

```sh
mise run benchmark
```

This builds hk, generates the project (outside the repository, in `~/.cache/hk-bench`), verifies every tool, times every scenario, and writes `benchmark/results.json`, which this page renders. Pass tak flags to narrow a run for diagnosis, for example `mise run benchmark -- --bench fix-staged --runs 5`; such a run is never marked publishable. To measure a released hk, set `HK_BIN` to its path. See [`benchmark/README.md`](https://github.com/jdx/hk/blob/main/benchmark/README.md) for how the pieces fit together.
