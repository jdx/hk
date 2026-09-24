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
| `tak.toml`              | The scenarios and the command each subject runs. Timing is done by [tak](https://github.com/jdx/tak).    |
| `verify.py`             | Runs every subject again as `tak.toml` declares it and checks the resulting tree against `clean`.        |
| `report.py`             | Writes `results.json`, and marks it publishable only if every safe-by-design subject was always correct. |
| `lib/allow-fixes`       | Treats exit 1 as success, because pre-commit and prek exit 1 whenever they fix a file.                   |

This `tak.toml` is separate from the repository root's. The root one records
hk's instruction counts on every push to main, while this one compares wall
time against other programs. tak uses the nearest `tak.toml`, so run it from
this directory.

## Adding a tool or scenario

1. Add the configuration under `subjects/<name>/` and map it in `setup.sh`.
2. Add `[bench.<scenario>.subject.<name>]` entries to `tak.toml`.
3. Add display metadata to `SUBJECTS` in `report.py`. Mark it `safe` only if
   the configuration cannot race by design.
4. Pin the tool in `mise.toml`, and add it to `VERSIONS` in `report.py`.

The workload's yq fixer also formats each tool's own YAML configuration, so
`setup.sh` normalizes those files with `yq -P` before committing them into the
fixture.

## Publishing

`.github/workflows/benchmark-refresh.yml` runs the benchmark against the
released hk on the pinned `jdx-perf-v1` runner. It then opens or updates a single
pull request that changes only `benchmark/results.json`. Review the numbers
before merging, because the docs page is rebuilt from that file.
