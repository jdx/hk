---
description: Rig a Python ship with hk. Ruff checks and formats the cargo, and mypy type checking comes along when ye call for it.
sourceHash: e6c225a5d7b4
---

# A Python ship

Ruff keeps lookout and mends the canvas (linting and formatting), with mypy standing behind the `types` profile, a watch of its own.

**Before ye sail:** `ruff` and `mypy` on `PATH`, along with yer project's rules and type-checking configuration. Activate yer virtual environment, or have [mise, the quartermaster](/mise_integration), provide the tools.

<a href="/python-project.pkl" download>Download python-project.pkl</a> and save it as `hk.pkl`, the charts yer ship sails by.

## The charts {#configuration}

<<< @/public/python-project.pkl

## Take her out {#try-it}

```sh
hk validate
hk check --all --plan
hk check --all
hk check --all --profile types
hk fix
```

Ruff's formatter waits for Ruff's lint fixes before it takes up the canvas. mypy runs only when `types` is enabled; until then it stays below decks. Mark this well: the profile must be enabled for the hk invocation itself; setting `HK_PROFILE` in a hook's child-command environment does not select it.

## Refit her for yer own voyage {#adapt-it}

If ye'd rather sail with Black, replace the `ruff-format` entry with `Builtins.black`. Choose one primary formatter, one master sailmaker, to avoid formatting passes that fight each other.

For a push hook that always brings mypy along, add a `pre-push` hook that copies the top-level steps and clears mypy's profile requirement there:

```pkl
hooks {
  ["pre-push"] {
    steps = (module.steps) {
      ["mypy"] = (Builtins.mypy) {
        profiles = List()
      }
    }
  }
}
```

Add this block after the top-level `steps` block. `module.steps` refers to those shared steps, the same crew. Locally and in CI, `hk check --all --profile types` includes type checking without a separate hook.
