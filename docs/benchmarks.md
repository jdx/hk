---
description: Compare hk, lefthook, pre-commit, and prek on fixing files, checking a repository, and running pre-commit hooks. Includes the benchmark method and reproduction steps.
---

# Benchmarks

This benchmark compares hk, lefthook, pre-commit, and prek on three everyday tasks: fixing files, checking a repository, and running pre-commit hooks. It measures elapsed time and verifies the files each tool produces. hk runs steps concurrently, using file locks to prevent fixers from writing the same file at once.

<BenchmarkResults />

## What the results show

hk is fastest in all three scenarios, and every tool produces the expected files in every timed sample.

- **Fix every file:** hk's lead over the next tool is smallest here. pre-commit and prek split each hook's files into batches that run across CPUs; hk can also run different fixers concurrently.
- **Check every file:** hk finishes ahead of lefthook even with lefthook's parallel mode enabled for read-only checks.
- **Commit:** hk has its largest relative lead when fixing about 60 staged files. hk runs independent fixers concurrently and stages each fixer's files as it finishes; the other configurations run hooks or jobs one at a time.

## Workload and correctness

The generated repository contains about 6,000 files: 4,000 Python, 500 JavaScript and TypeScript, 500 JSON, 500 shell, 250 YAML, 200 CSS, and 200 Markdown. Each configuration runs ten fixers: black, ruff format, ruff check, Prettier, ESLint, jq, yq, shfmt, trailing whitespace, and final newline.

The generator creates two commits: `clean`, the result of running the fixers sequentially, and `dirty`, with formatting defects in a quarter of the files. Each defective file needs both a language fixer and a whitespace fixer, so the workload exercises overlapping writes.

| Scenario | Starting state | Measured work |
| --- | --- | --- |
| Fix every file | `dirty` | `hk fix --all` and equivalent commands. |
| Check every file | `clean` | Read-only checks of the entire repository, as in CI. |
| Commit | About 60 files with defects staged | Each tool's pre-commit hook fixes the staged files. |

The commit scenario measures one hook invocation. hk and lefthook stage their fixes. pre-commit and prek leave fixes unstaged and return a failure, requiring the user to stage the changes and retry the commit. That manual work and retry are outside the measurement.

After every timed sample, [tak](https://github.com/jdx/tak) checks that the resulting tree matches `clean` byte for byte. A separate check verifies that each tool detects defects in `dirty`. A run is publishable only if every tool passes all required checks.

## Tool configurations

The configurations allow concurrency where it cannot cause overlapping writes: hk coordinates fixers with file locks, while pre-commit and prek give each batch different files.

| Tool | Fixing | Checking |
| --- | --- | --- |
| hk | Steps run concurrently with per-file locks. | Same configuration. |
| lefthook | Jobs run sequentially (the default). | Jobs run concurrently with `parallel: true`. |
| pre-commit | Hooks run sequentially; each hook's file batches run across CPUs. | Same configuration. |
| prek | Hooks run sequentially; each hook's file batches run across CPUs. | Same configuration. |

lefthook's `parallel: true` and prek hooks with a shared `priority` can run fixers that write the same file at once. Those configurations are excluded from fixing, even if a particular run happens to produce correct output. lefthook's parallel mode is used for read-only checks.

hk uses its builtins, including their check-before-fix behavior and the `hk util` whitespace fixers. The other configurations invoke the language fixers directly. pre-commit and lefthook use the whitespace fixers from [pre-commit-hooks](https://github.com/pre-commit/pre-commit-hooks); prek uses its bundled Rust replacements. See the complete [tool configurations](https://github.com/jdx/hk/tree/main/benchmark/subjects).

## Measurement method

- **Pinned versions.** Hook managers, linters, and runtimes are pinned in [`benchmark/mise.toml`](https://github.com/jdx/hk/blob/main/benchmark/mise.toml). The charts list the measured hook-manager versions and host.
- **Interleaved samples.** tak measures every tool once per round, in a new random order each round, to spread the effects of changing host conditions across tools.
- **Repeatable starting state.** Each tool has its own clone, reset before each sample outside the timed interval. Each clone keeps its own warm caches, including ruff's and black's.
- **Isolated Git configuration.** Global and system Git configuration is disabled to exclude the host's hooks, signing, and filesystem monitor.
- **Variation matters.** The chart calls a tool faster only when the gap between medians exceeds both tools' sample ranges. Otherwise, it calls them level. This is a conservative comparison rule, not a statistical significance test.

## Limitations

The generated files are uniform. Results in your repository depend on the linters, overlapping file patterns, number of changed files, and available CPU cores. Linters that already use all available cores, such as black, may gain less from running alongside other steps. Use [hk timing reports](/logging#a-run-is-slow) to see where your own runs spend their time.

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
