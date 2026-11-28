---
description: Run hk checks in CI, select changed files, enable profiles, and keep diagnostics useful.
---

# Continuous integration

Use `hk check --all` to run the project’s checks against a checkout. CI must install hk and every tool used by the configured steps, just as a developer’s machine does.

## Share local and CI checks

Define a `check` hook that reuses your linter mapping:

```pkl
hooks {
  ["check"] { steps = linters }
}
```

Then run:

```sh
hk validate
hk check --all
```

No `hk install` step is needed to invoke hk directly in CI. Keep check commands read-only so failures report what needs to change. Use `hk fix` locally, review the changes, and commit them.

## Set up the environment

With a committed `mise.toml`, the essential CI commands are:

```sh
mise install
mise exec -- hk check --all
```

If your linters are project dependencies, also run the package manager’s install command and expose its executable directory to hk. The [mise integration guide](/mise_integration) shows a Node.js example.

Pin tool versions in the project, hk included, so local and CI runs use the same rules. `hk:` imports follow the installed hk.

## Check a branch’s changes

A full check is the simplest baseline. For large repositories, select files that differ between two references:

```sh
hk check --from-ref origin/main --to-ref HEAD
```

Replace `origin/main` with your target branch and ensure the checkout contains both references and enough history to compare them. Shallow clones may need an additional fetch.

Locally, `hk check --pr` selects changes against the detected default branch. For CI, explicit references make the comparison easier to inspect.

::: tip Changed files are a filter
Reference selection chooses file paths; the commands run against the current checkout. It does not check out historical versions. A changed-file check also cannot determine every downstream effect of a shared configuration or dependency change.
:::

## Enable additional checks

Use profiles for checks that are too expensive for every commit:

```pkl
["typecheck"] = (Builtins.tsc) {
  profiles = List("slow")
}
```

Enable them explicitly:

```sh
hk check --all --slow
hk check --all --profile ci --profile slow
```

A step with multiple positive profiles requires all of them. A profile named `ci` is a label you enable; do not rely on its name to activate it automatically.

## Collect useful diagnostics

```sh
hk check --all --no-fail-fast
hk check --all --plan --json
HK_TIMING_JSON=hk-timing.json hk check --all
```

`--no-fail-fast` collects failures from remaining steps. A plan shows selected steps without executing them. The timing file records total and per-step wall time; parallel step durations should not be added together as a total.

Use `hk check --all --format jsonl` for structured execution events, `--sarif hk.sarif` for normalized diagnostics (only steps that set `diagnostic_format` produce them; see [diagnostics](/configuration#diagnostics)), or `--junit-xml hk.junit.xml` to report each step as a JUnit test case for CI test-result viewers. See [coding agents](/agents) for command effects and exact file lists.

See [troubleshooting](/logging) for log levels, traces, and configuration inspection.

## Use the hk GitHub Action

[jdx/hk-action](https://github.com/jdx/hk-action) installs a pinned hk release and runs `hk check --all`. It checks the download against the release's SHA-256 digest and build attestation before running it.

```yaml
steps:
  - uses: actions/checkout@v7
  - uses: jdx/hk-action@v0
    with:
      version: 2.14.0
      mise: true
```

`version` is required, so a new hk release reaches CI only in a commit that changes it. Use the version your project pins locally. With `mise: true`, the action first runs [mise-action](https://github.com/jdx/mise-action), which installs the tools in `mise.toml` so the linters your steps call are on the `PATH`. If mise already installs hk, pass `install: false` instead of a `version`.

The action passes `--github-annotations` when the installed hk has it (see [below](#report-findings-in-github-actions)); with an older hk, it warns and runs without annotations. Other inputs check only a pull request's changed files (`files: changed`), pass extra arguments (`args: --slow`), or write `--sarif` and `--junit-xml` reports; its README lists them all. hk-action is not yet at 1.0, and its inputs may change between releases, including under the floating `v0` tag. Pin an exact tag such as `v0.1.0` or a commit SHA if that would break your workflow.

## Report findings in GitHub Actions

`--github-annotations` turns findings into inline annotations on the pull request's changed files and writes a job summary:

```yaml
- run: hk check --all --github-annotations
```

hk prints one [workflow command](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands) per finding (`::error`, `::warning` or `::notice`, with the file and, when the finding has them, its line and column; columns are left out for findings that span several lines), and one `::error` with the step's output for each failed step that produced no findings. A step whose failure is allowed gets no annotations. A run that fails before any step starts, such as with a broken `hk.pkl`, gets one `::error` with the reason.

When `GITHUB_STEP_SUMMARY` holds a nonempty path, as it does in every Actions step, hk also appends a Markdown job summary. Otherwise hk prints only the annotations. The summary has:

- the status and duration of each step that ran, and how many steps were skipped
- a table of findings, up to 500
- the output of each failed step that produced no findings, up to 16 KiB each

The summary file stays under 768 KiB, below GitHub's 1 MiB limit for a step's summary. When it would grow past that, hk leaves out whatever does not fit and says so. If earlier commands in the step have left too little room, hk writes no summary.

Findings come from steps that set `diagnostic_format` (see [diagnostics](/configuration#diagnostics)). Unlike SARIF upload to code scanning, annotations need no GitHub Advanced Security, so they work on private repositories. GitHub shows at most 10 error and 10 warning annotations per step. The job summary lists more of the findings, within the limits above; for all of them, also pass `--sarif` or use `--format json`.

With `--format json` or `jsonl`, the annotations go to stderr so stdout stays valid JSON. The runner reads workflow commands from both streams.

## Report findings in GitLab merge requests

`--gitlab-code-quality PATH` writes a [Code Quality](https://docs.gitlab.com/ci/testing/code_quality/) report, which GitLab shows in the merge request widget and diff:

```yaml
hk:
  script:
    - hk check --all --gitlab-code-quality gl-code-quality.json
  artifacts:
    when: always
    reports:
      codequality: gl-code-quality.json
```

Errors map to `major`, warnings to `minor` and notes to `info`. GitLab requires a path relative to the repository, so hk converts absolute paths inside the checkout and leaves out findings without a path. Fingerprints leave out the line number, so moving code does not make a finding look new. Set `when: always` so GitLab collects the report from a failed job.
