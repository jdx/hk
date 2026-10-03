---
description: Race hk against lefthook, pre-commit, and prek at fixing files, checking a whole repository, and running pre-commit hooks on large and small commits, with the benchmark's method and the steps to sail the course again yerself.
sourceHash: ccfa2aca07bc
---

# Benchmarks: speed trials of four hook managers

This benchmark races hk, lefthook, pre-commit, and prek over the same everyday course: fixing files, checking a repository, and running pre-commit hooks on a large and a small commit. It measures elapsed time and inspects the cargo, verifying the files each tool produces. hk sends all hands to haul at once, running steps concurrently, with file locks that keep two fixers from writing the same file at the same moment.

<BenchmarkResults />

## Reading the race {#what-the-results-show}

Every tool produces the expected files in every timed sample. Each chart states how hk compares with the fastest of the other tools in that scenario. The scenarios differ in how much of each tool's run can overlap:

- **Fix every file:** each hook has thousands of files in its cargo, so pre-commit and prek split them into batches that keep every CPU busy. With every CPU already hauling, running different fixers at the same time has little idle CPU to reclaim, and a quarter of the files need two or three fixers in turn.
- **Check every file:** nothing writes, so lefthook and prek also send every check aloft at once. The comparison is between tools that all run their checks concurrently, except pre-commit, which has no mode for it.
- **Commit and small commit:** each hook has only a few files, so batching cannot fill the CPUs, and much of each hook's time is spent starting its tool. hk calls independent steps on deck together and stages each step's files as it finishes. The other configurations run fixers one at a time.

## The cargo and the inspection {#workload-and-correctness}

The generated repository, our test ship, carries about 6,000 files: 4,000 Python, 500 JavaScript and TypeScript, 500 JSON, 500 shell, 250 YAML, 200 CSS, and 200 Markdown. Each configuration musters ten fixers: black, ruff format, ruff check, Prettier, ESLint, jq, yq, shfmt, trailing whitespace, and final newline. It also posts two type checkers as lookouts, mypy and tsc, which change no files. mypy checks the Python files it is handed. tsc checks the whole TypeScript project whenever a TypeScript file is selected, as it would in a real repository.

The generator lays down two commits: `clean`, the result of running the fixers one after another, and `dirty`, with defects in a quarter of the files: formatting for every file type, plus an unused import in each Python file that only ruff check removes. Each defective file needs a language fixer and a whitespace fixer, and a Python file needs black and ruff check as well, so several sailmakers must work the same canvas: the workload exercises overlapping writes.

| Scenario | Where she starts | The work that's timed |
| --- | --- | --- |
| Fix every file | `dirty` | `hk fix --all` and the equivalent commands. |
| Check every file | `clean` | Read-only checks of the entire repository, as the harbour-master (CI) runs them. |
| Commit | About 60 files with defects loaded aboard (staged) | Each tool's pre-commit hook fixes the staged files. |
| Small commit | One file with defects staged in each of eight file types | The same hook on a voyage of typical size. |

With about 60 files aboard, splitting each hook's files into batches keeps several CPUs busy. With one file per hook, there's nothing to split: most of the run is the time each tool takes to start, and a tool that runs its hooks one at a time waits for every start in turn.

The commit scenarios measure one sounding of the bosun's pipe: a single hook invocation. hk and lefthook stage their fixes. pre-commit and prek leave fixes unstaged and return a failure, so the user must stage the changes and retry the commit, setting sail a second time. That manual work and the retry are outside the measurement.

After every timed sample, [tak](https://github.com/jdx/tak) inspects the cargo: the resulting tree must match `clean` byte for byte. hk and lefthook must also exit successfully, since a failed type check changes no files. pre-commit and prek report failure whenever a hook modified files, so their exit codes cannot show a failed type check; they run the type checkers after every fixer has finished, and setup verifies that both commits type-check, including mypy on exactly the files each commit scenario stages. A separate check verifies that each tool spots the defects in `dirty`. A run is fit to publish only if every tool passes all required checks.

## How each tool is rigged {#tool-configurations}

Each configuration uses the tool's fastest setting that cannot cause overlapping writes. hk coordinates its fixers with file locks, a lock on each file taking the strain, and the builtin type checkers declare that they may write cache files, so under `hk fix` they hold write locks like the fixers (under `hk check` every step holds read locks). pre-commit and prek give each batch different files. lefthook and prek also run read-only work concurrently.

| Tool | Mending (fixing) | Inspecting (checking) |
| --- | --- | --- |
| hk | All hands haul at once: steps run concurrently with per-file locks. | The same configuration. |
| lefthook | Fixers run one after another (the default); then mypy and tsc run together in a `parallel: true` group. | Jobs run concurrently with `parallel: true`. |
| pre-commit | Hooks run one after another; each hook's file batches run across CPUs. | The same configuration. |
| prek | Fixers run one after another; then mypy and tsc run together with a shared `priority`. Each hook's file batches run across CPUs. | Hooks run concurrently with a shared `priority`. |

A word on that rigging: lefthook's `parallel: true` and prek hooks with a shared `priority` can run fixers that write the same file at once. Those modes are never used for fixers, even if a particular run happens to produce correct output. They're used only for hooks that write nothing to the files being checked. pre-commit has no mode for running hooks concurrently.

hk sails with its standing crew, the builtins, including their check-before-fix behavior and the `hk util` whitespace fixers. The other configurations invoke the language fixers directly. pre-commit and lefthook use the whitespace fixers from [pre-commit-hooks](https://github.com/pre-commit/pre-commit-hooks); prek uses its own bundled Rust replacements. See each tool's full rigging in the complete [tool configurations](https://github.com/jdx/hk/tree/main/benchmark/subjects).

## How the race is timed {#measurement-method}

- **Pinned versions.** Hook managers, linters, and runtimes are pinned in the quartermaster's [`benchmark/mise.toml`](https://github.com/jdx/hk/blob/main/benchmark/mise.toml). The charts list the measured hook-manager versions and the host.
- **Interleaved samples.** tak measures every tool once per round, in a fresh random order each round, to spread the effects of changing host conditions, the shifting weather, across all the tools.
- **Repeatable starting state.** Each tool has its own clone of the ship, reset before each sample, outside the timed interval. Each clone keeps its own warm caches, including ruff's and black's.
- **Isolated Git configuration.** Global and system Git configuration is disabled, so the host's own hooks, signing, and filesystem monitor stay ashore.
- **Variation matters.** The chart calls a tool faster only when the gap between medians exceeds both tools' sample ranges. Otherwise, it calls them level, sailing abreast. This is a conservative comparison rule, not a statistical significance test.

## Limitations: mind the shoals {#limitations}

The generated files are uniform, every barrel alike. Results aboard yer own ship depend on the linters, overlapping file patterns, the number of changed files, and the CPU cores ye have. When every tool already keeps all CPUs busy, as when fixing thousands of files split into batches, running steps concurrently has little idle time to reclaim. All hands hauling at once gains most when individual hooks cannot use every CPU: small commits, whole-project checks, and single-threaded linters. Linters that already use all available cores, such as black, may gain less from running alongside other steps. Use [hk timing reports](/logging#a-run-is-slow) to see where yer own runs spend their time.

The benchmark leaves out installation, first runs with cold caches, and hook-manager features beyond running fixers. Nor does it measure the cost of preserving partially staged work: the commit scenario has no unstaged changes to stow in the hold and bring back up.

## Sail the course yerself {#reproduce-the-benchmark}

On Linux, install [mise](https://mise.jdx.dev), the quartermaster, and run this command from a checkout of hk. The benchmark scripts require GNU sed; mise provisions the pinned toolset.

```sh
mise run benchmark
```

This builds hk from the checkout, generates the test ship in `~/.cache/hk-bench`, runs the scenarios and correctness checks, and writes `benchmark/results.json`, the tally that supplies the charts on this page.

To look into one scenario on its own, pass flags through to tak:

```sh
mise run benchmark -- --bench fix-staged --runs 5
```

Mind: a filtered run is not publishable. To time an hk binary ye already have instead of building the checkout, set the standing order `HK_BIN` to its path:

```sh
HK_BIN=/path/to/hk mise run benchmark
```

See the [benchmark maintainer guide](https://github.com/jdx/hk/blob/main/benchmark/README.md) for the scripts and the publication workflow.
