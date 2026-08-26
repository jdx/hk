# Migrating to hk v2

hk v2 removes deprecated configuration entry points and makes shared steps and
staging behavior explicit. No automatic rewrite is provided; hk reports a
targeted replacement when it detects a removed input.

## Builtins

Every builtin is now a class-as-a-function factory value. Default references
stay concise and do not need to change:

```pkl
["prettier"] = Builtins.prettier
```

If a local mapping contains builtin factories, let Pkl infer its value type (or
use `StepDefinition`) instead of declaring `Mapping<String, Step>`:

```pkl
local linters = new Mapping {
  ["prettier"] = Builtins.prettier
}
```

Replace the removed staged, strict, and versioned names as follows:

```pkl
["gitleaks"] = (Builtins.gitleaks) {
  staged = true
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

These replace `gitleaks_staged`, `knip_strict`, `pinact_v3`, and
`pinact_update_v3`, respectively. Put generic step customization under the
factory's nested `step` output. This syntax remains stable if a builtin gains
its own options later:

```pkl
["prettier"] = (Builtins.prettier) {
  step {
    batch = false
  }
}
```

Replace `Builtins.check_byte_order_marker` and
`Builtins.fix_byte_order_marker` with `Builtins.byte_order_marker`.

## Shared steps and staging

Move steps repeated across `check`, `fix`, and `pre-commit` to the top level:

```pkl
steps {
  ["prettier"] = Builtins.prettier
}
```

This creates implicit `check`, `fix`, and `pre-commit` hooks. Explicit hooks
inherit these steps and override same-named entries. Use `enabled = false` to
disable an implicit hook.

`pre-commit` fixes and stages by default. `hk fix` and every other hook leave
changes unstaged unless `stage = true` or `--stage` is supplied. A step's
`stage` patterns only filter paths after hook-level staging is enabled.

## Configuration files

| Removed in v2 | Replacement |
| --- | --- |
| `hk.toml`, `hk.yaml`, `hk.yml`, `hk.json` | `hk.pkl` amending `Config.pkl` |
| project `.hkrc.pkl` | `hk.local.pkl` |
| home `~/.hkrc.pkl` | `~/.config/hk/config.pkl` |
| `--hkrc <PATH>` | the XDG or project-local path above |
| `UserConfig.pkl` | `Config.pkl` |
| `environment { ... }` | `env { ... }` |
| `Types.Regex(...)` or `Config.Regex(...)` | Pkl's built-in `Regex(...)` |
| `hk generate` | `hk init` |

Project, local, and XDG configuration files must all be Pkl. Global and project
steps remain additive, with project definitions winning collisions.
