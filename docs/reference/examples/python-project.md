---
description: Configure Ruff checks and formatting with optional mypy type checking.
---

# Python

Use Ruff for linting and formatting, with mypy behind the `types` profile.

**Prerequisites:** `ruff` and `mypy` on `PATH`, along with your project’s rules and type-checking configuration. Activate your virtual environment or use [mise](/mise_integration) to provide the tools.

<a href="/python-project.pkl" download>Download python-project.pkl</a> and save it as `hk.pkl`.

## Configuration

<<< @/public/python-project.pkl

## Try it

```sh
hk validate
hk check --all --plan
hk check --all
hk check --all --profile types
hk fix
```

Ruff’s formatter waits for Ruff’s lint fixes. mypy runs only when `types` is enabled. The profile must be enabled for the hk invocation; setting `HK_PROFILE` in a hook’s child-command environment does not select it.

## Adapt it

If you prefer Black, replace the `ruff-format` entry with `Builtins.black`. Choose one primary formatter to avoid conflicting formatting passes.

For a push hook that always includes mypy, add a `pre-push` hook using an amended linter mapping and clear mypy’s profile requirement there:

```pkl
["pre-push"] {
  steps = (linters) {
    ["mypy"] = (Builtins.mypy) {
      profiles = List()
    }
  }
}
```

Place this fragment inside `hooks`. Locally and in CI, `hk check --all --profile types` includes type checking without a separate hook.
