---
description: Refit yer ship for hk v2 by swapping out the v1 configuration files, builtin variants, and CLI entry points that v2 struck off.
sourceHash: cd07ad881676
---

# Refitting the ship for hk v2

hk v2 strikes off the deprecated configuration entry points, the old gangways into yer charts, and makes shared steps and staging behavior explicit. There be no automatic rewrite, so ye refit by hand; but when hk spots a removed input, it tells ye the replacement for that very input.

## The standing crew: builtins {#builtins}

Every builtin, every hand of the standing crew, is a `Config.Step`. A plain reference that takes the defaults stays short, and ye needn't change it:

```pkl
["prettier"] = Builtins.prettier
```

A muster of builtins is still typed as `Mapping<String, Step>`:

```pkl
local linters = new Mapping<String, Step> {
  ["prettier"] = Builtins.prettier
}
```

The staged, strict, and versioned names have been removed, struck off the crew roster. Replace them like so:

```pkl
["gitleaks"] = (Builtins.gitleaks) {
  scan = "staged"
}
["knip"] = (Builtins.knip) {
  strict = true
}
["pinact"] = (Builtins.pinact) {
  version = "3"
}
["pinact_update"] = (Builtins.pinact_update) {
  version = "3"
}
```

These stand in for `gitleaks_staged`, `knip_strict`, `pinact_v3`, and `pinact_update_v3`, in that order. Put generic step customization, the settings any hand can take, under that same amended step object, matey:

```pkl
["prettier"] = (Builtins.prettier) { batch = false }
```

Two hands become one: swap `Builtins.check_byte_order_marker` and `Builtins.fix_byte_order_marker` for `Builtins.byte_order_marker`.

## Shared hands and staging the cargo {#shared-steps-and-staging}

Top-level `steps` be optional, not something the refit demands. Existing charts that define their steps only inside `hooks`, shared `local linters` mappings included, are still supported. Ye needn't move those steps when ye upgrade.

For a new set of charts, we recommend top-level `steps` when `check`, `fix`, and `pre-commit` should share the same linters:

```pkl
steps {
  ["prettier"] = Builtins.prettier
}
```

When it isn't empty, it rigs implicit `check`, `fix`, and `pre-commit` hooks. Without top-level steps, only the hooks ye declare outright are aboard. Explicit hooks inherit these steps, and an entry the hook defines under the same name replaces the inherited one entirely. Use `enabled = false` to stand down an implicit hook ye don't want.

`pre-commit` mends the canvas and loads the fixes aboard (stages them) by default. `hk fix` and every other hook leave their changes on the dock, unstaged, unless `stage = true` or `--stage` is given. A step's `stage` patterns only filter which paths get loaded, and only once staging is turned on at the hook level.

Ye'll find exactly how the three materialized hook names behave, and the settings a custom hook needs, under [hook defaults](/configuration#hook-defaults).

## Configuration files: the charts ye sail by {#configuration-files}

| Struck off in v2                          | Sail with this instead                                                                          |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `hk.toml`, `hk.yaml`, `hk.yml`, `hk.json` | `hk.pkl` amending `Config.pkl`                                                                  |
| project `.hkrc.pkl`, aboard the ship      | `hk.local.pkl`                                                                                  |
| home `~/.hkrc.pkl`, in yer home port      | `~/.config/hk/config.pkl`                                                                       |
| `--hkrc <PATH>`                           | the XDG or project-local path above                                                             |
| `UserConfig.pkl`                          | `Config.pkl`                                                                                    |
| `UserConfig.pkl`'s `environment { ... }`  | `Config.pkl`'s `env { ... }`                                                                    |
| `defaults { jobs = ... }`                 | haul `jobs`, `skip_steps`, `skip_hooks`, `profiles`, and the other settings up to the top level |
| `Types.Regex(...)` or `Config.Regex(...)` | Pkl's own built-in `Regex(...)`                                                                 |
| `hk generate`                             | `hk init`                                                                                       |
| `HK_PKL_BACKEND=pkl`                      | strike the variable; `pklr` is still accepted, as a compatibility no-op                         |

Project, local, and XDG configuration files must all be written in Pkl, the chart-maker's language. Global and project steps still add together, and where they collide, the project's definition wins.

hk no longer calls on the pkl CLI when it runs, neither directly nor through mise. The standalone pkl CLI is still a handy spyglass for inspecting a Pkl module, but hk doesn't need it aboard to run, and won't fall back on it to evaluate yer charts.
