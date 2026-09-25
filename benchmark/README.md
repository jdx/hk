# Competitor benchmark

Compares hk with lefthook, pre-commit, and prek on wall time and on whether each
tool produces the right files. `docs/benchmarks.md` renders the results and
explains the method for readers; this file is for maintainers.

```sh
mise run benchmark                                  # everything
mise run benchmark -- --bench fix-staged --runs 5   # diagnose one scenario (flags go to `tak run`)
HK_BIN=~/Downloads/hk mise run benchmark            # measure a specific binary
```

## Pieces

| File                    | Role                                                                                                     |
| ----------------------- | -------------------------------------------------------------------------------------------------------- |
| `mise.toml`             | Pins every hook manager, linter, and runtime the benchmark runs.                                         |
| `env.sh`                | Puts the pinned tools, then the hk under test, first on `PATH`.                                          |
| `generate-project.sh`   | Generates the fixture: `clean` and `dirty` tags, where fixing `dirty` must give `clean` byte for byte.   |
| `lib/reference-fix.sh`  | The ten fixers, run one after another. It defines what `clean` is.                                       |
| `lib/inject-defects.sh` | Breaks a quarter of the files so that two or three fixers must write each one.                           |
| `subjects/`             | Each tool's configuration.                                                                               |
| `setup.sh`              | One clone of the fixture per subject in `~/.cache/hk-bench` (`.work` links to it).                       |
| `tak.toml`              | Scenarios, commands and the per-sample `check`, timed by [tak](https://github.com/jdx/tak) 0.0.13.       |
| `report.py`             | Writes `results.json` from tak's export, publishable only if every subject always passed.               |

This `tak.toml` is separate from the repository root's. The root one records
hk's instruction counts on every push to main, while this one compares wall
time against other programs. `run.sh` passes it to tak with `--config`.

## Adding a tool or scenario

1. Add the configuration under `subjects/<name>/` and list it in `SUBJECTS` in
   `setup.sh`. Use the tool's fastest settings in which two fixers can never
   write the same file at once: concurrency for read-only checks, and fixers
   one at a time unless the tool coordinates writes to the same file.
2. In `tak.toml`, add a shared `[subject.<name>]` with its `version_cmd`, list
   it in each benchmark's `subjects`, and give each benchmark a
   `[bench.<scenario>.subject.<name>]` with the command for that scenario.
3. Add display metadata to `SUBJECTS` in `report.py`, with `modes` for any
   scenario the configuration runs differently.
4. Pin the tool in `mise.toml`.

The workload's yq fixer also formats each tool's own YAML configuration, so
`setup.sh` normalizes those files with `yq -P` before committing them into the
fixture.

## Publishing

`.github/workflows/benchmark-refresh.yml` runs the benchmark against the
released hk on the pinned `jdx-perf-v1` runner. It then opens or updates a single
pull request that changes only `benchmark/results.json`. Review the numbers
before merging, because the docs page is rebuilt from that file.
