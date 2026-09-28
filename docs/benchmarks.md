---
description: Compare hk, lefthook, pre-commit, and prek on fixing files, checking a repository, and running pre-commit hooks on large and small commits. Includes the benchmark method and reproduction steps.
---

# Benchmarks

This benchmark compares hk, lefthook, pre-commit, and prek on everyday tasks: fixing files, checking a repository, and running pre-commit hooks on a large and a small commit. It measures elapsed time and verifies the files each tool produces. hk runs steps concurrently, using file locks to prevent fixers from writing the same file at once.

<BenchmarkResults />

## What the results show

Every tool produces the expected files in every timed sample. Each chart states how hk compares with the fastest other tool in that scenario. The scenarios differ in how much of each tool's run can overlap:

- **Fix every file:** each hook has thousands of files, and a quarter of the files need two or three fixers in turn. Tools can split files into batches or use their own internal parallelism. prek also overlaps fixers whose file types are disjoint.
- **Check every file:** nothing writes, so lefthook and prek also run every check at once. The comparison is between tools that all run their checks concurrently, except pre-commit, which has no mode for it.
- **Commit and small commit:** each hook has few files, so batching cannot fill the CPUs, and much of each hook's time is spent starting its tool. hk starts independent steps together and stages each step's files as it finishes. prek overlaps fixers for disjoint file types using priority groups. lefthook and pre-commit run fixers one at a time.

## Workload and correctness

The generated repository contains about 6,000 files: 4,000 Python, 500 JavaScript and TypeScript, 500 JSON, 500 shell, 250 YAML, 200 CSS, and 200 Markdown. Each configuration runs ten fixers: black, ruff format, ruff check, Prettier, ESLint, jq, yq, shfmt, trailing whitespace, and final newline. It also runs two type checkers, mypy and tsc, which change no files. mypy checks the Python files it is given. tsc checks the whole TypeScript project whenever a TypeScript file is selected, as it would in a real repository.

The generator creates two commits: `clean`, the result of running the fixers sequentially, and `dirty`, with defects in a quarter of the files: formatting for every file type, plus an unused import in each Python file that only ruff check removes. Each defective file needs a language fixer and a whitespace fixer, and a Python file needs black and ruff check as well, so the workload exercises overlapping writes.

| Scenario | Starting state | Measured work |
| --- | --- | --- |
| Fix every file | `dirty` | `hk fix --all` and equivalent commands. |
| Check every file | `clean` | Read-only checks of the entire repository, as in CI. |
| Commit | About 60 files with defects staged | Each tool's pre-commit hook fixes the staged files. |
| Small commit | One file with defects staged in each of eight file types | The same hook on a commit of typical size. |

With about 60 files, splitting each hook's files into batches keeps several CPUs busy. With one file per hook, there is nothing to split: most of the run is the time each tool takes to start, and a tool that runs its hooks one at a time waits for every start in turn.

The commit scenarios measure one hook invocation. hk and lefthook stage their fixes. pre-commit and prek leave fixes unstaged and return a failure, requiring the user to stage the changes and retry the commit. That manual work and retry are outside the measurement.

After every timed sample, [tak](https://github.com/jdx/tak) checks that the resulting tree matches `clean` byte for byte. hk and lefthook must also exit successfully, since a failed type check changes no files. pre-commit and prek report failure whenever a hook modified files, so their exit codes cannot show a failed type check; they run the type checkers after every fixer has finished, and setup verifies that both commits type-check, including mypy on exactly the files each commit scenario stages. A separate check verifies that each tool detects defects in `dirty`. A run is publishable only if every tool passes all required checks.

## Tool configurations

Each configuration uses the tool's fastest setting that cannot cause overlapping writes. hk coordinates fixers with file locks and holds read locks for the type checkers. pre-commit and prek give each batch different files. lefthook and prek also run read-only work concurrently.

| Tool | Fixing | Checking |
| --- | --- | --- |
| hk | Steps run concurrently with per-file locks. | Same configuration. |
| lefthook | Fixers run sequentially (the default); then mypy and tsc run together in a `parallel: true` group. | Jobs run concurrently with `parallel: true`. |
| pre-commit | Hooks run sequentially; each hook's file batches run across CPUs. | Same configuration. |
| prek | Disjoint file types share a `priority`; fixers that share files run in order, followed by mypy and tsc. | Read-only hooks share a `priority`, including native `--check` modes for the text fixers. |

A shared prek `priority` does not coordinate writes. Its configuration therefore groups only fixers with disjoint file types: Black, Prettier, jq, yq and shfmt. Ruff format and Ruff check follow Black, ESLint follows Prettier, and the text fixers run after the language fixers. lefthook uses `parallel: true` only for read-only work. pre-commit has no mode for running hooks concurrently.

hk uses its builtins, including their check-before-fix behavior and the `hk util` whitespace fixers. The other configurations invoke the language fixers directly. pre-commit and lefthook use the whitespace fixers from [pre-commit-hooks](https://github.com/pre-commit/pre-commit-hooks); prek uses its bundled Rust replacements. prek uses a standalone `prek.toml`. Black and Ruff use `require_serial` as their upstream hooks do, shfmt also runs in one invocation, and the remaining batched hooks have a limit of two concurrent batches. This limit applies per hook; it does not disable concurrency between hooks or the tools' internal parallelism. See the complete [tool configurations](https://github.com/jdx/hk/tree/main/benchmark/subjects).

## Measurement method

- **Pinned versions.** Hook managers, linters, and runtimes are pinned in [`benchmark/mise.toml`](https://github.com/jdx/hk/blob/main/benchmark/mise.toml). The charts list the measured hook-manager versions and host.
- **Interleaved samples.** tak measures every tool once per round, in a new random order each round, to spread the effects of changing host conditions across tools.
- **Repeatable starting state.** Each tool has its own clone, reset before each sample outside the timed interval. Each clone keeps its own warm caches, including ruff's and black's.
- **Isolated Git configuration.** Global and system Git configuration is disabled to exclude the host's hooks, signing, and filesystem monitor.
- **Variation matters.** The chart calls a tool faster only when the gap between medians exceeds both tools' sample ranges. Otherwise, it calls them level. This is a conservative comparison rule, not a statistical significance test.

## Limitations

The generated files are uniform. Results in your repository depend on the linters, overlapping file patterns, number of changed files, and available CPU cores. When every tool already keeps all CPUs busy, as when fixing thousands of files split into batches, running steps concurrently has little idle time to reclaim. It gains most when individual hooks cannot use every CPU: small commits, whole-project checks, and single-threaded linters. Linters that already use all available cores, such as black, may gain less from running alongside other steps. Use [hk timing reports](/logging#a-run-is-slow) to see where your own runs spend their time.

The benchmark excludes installation, first runs with cold caches, and hook-manager features beyond running fixers. It also does not measure the cost of preserving partially staged work: the commit scenario has no unstaged changes to save and restore.

## Reproduce the benchmark

On Linux, install [mise](https://mise.jdx.dev) and run this command from a checkout of hk. The benchmark scripts require GNU sed; mise installs the pinned toolset.

```sh
mise run benchmark
```

This builds hk from the checkout, generates the test repository in `~/.cache/hk-bench`, runs the scenarios and correctness checks, and writes `benchmark/results.json`, which supplies the charts on this page.

To investigate one scenario, pass flags through to tak:

```sh
mise run benchmark -- --bench fix-staged --runs 5
```

A filtered run is not publishable. To measure an existing hk binary instead of building the checkout, set `HK_BIN` to its path:

```sh
HK_BIN=/path/to/hk mise run benchmark
```

See the [benchmark maintainer guide](https://github.com/jdx/hk/blob/main/benchmark/README.md) for the scripts and publication workflow.
