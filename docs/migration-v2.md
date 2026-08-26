# Migrating to hk v2

hk v2 removes deprecated configuration entry points and makes shared steps and
staging behavior explicit. No automatic rewrite is provided; hk reports a
targeted replacement when it detects a removed input.

## Builtins

Every builtin is now a factory function:

```pkl
// v1
["prettier"] = Builtins.prettier

// v2
["prettier"] = Builtins.prettier()
```

Replace `Builtins.gitleaks_staged` with:

```pkl
["gitleaks"] = (Builtins.gitleaks()) { staged = true }
```

Replace `Builtins.check_byte_order_marker()` and
`Builtins.fix_byte_order_marker()` with `Builtins.byte_order_marker()`.

## Shared steps and staging

Move steps repeated across `check`, `fix`, and `pre-commit` to the top level:

```pkl
steps {
  ["prettier"] = Builtins.prettier()
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
