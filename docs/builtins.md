---
outline: "deep"
---

# Built-in Linters Reference

hk provides 140+ pre-configured linters and formatters through the `Builtins` module. They provide the command, file matching, batching, and other hk behavior, while the corresponding tool must be available in the step's environment.

## Usage

Import and use builtins in your `hk.pkl`:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.57.0/hk@1.57.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.57.0/hk@1.57.0#/Builtins.pkl"

hooks {
  ["pre-commit"] {
    steps {
      ["prettier"] = Builtins.prettier
      ["eslint"] = Builtins.eslint
    }
  }
}
```

You can also customize builtins:

```pkl
["prettier"] = (Builtins.prettier) {
  batch = false  // Override the default batch setting
  glob = List("*.js", "*.ts")  // Override file patterns
}
```

### Tool availability

Builtins configure how hk invokes a tool; they do not install that tool. The
executable used by the builtin must be on `PATH`, or the step must use a `prefix`
that resolves it.

With [`HK_MISE=1`](/mise_integration#per-directory-environments-monorepos), hk
resolves the mise environment for the step's directory, so tools declared in a
subproject's `mise.toml` are available without a prefix. For a Node tool installed
locally by aube, prefix its builtin with `aube exec`:

```pkl
["eslint"] = (Builtins.eslint) {
  prefix = List("aube", "exec")
}
```

Use an argv list for builtins backed by structured commands. A string prefix such
as `"aube exec"` or `"mise x --"` cannot be combined with those commands.

The generated list below summarizes each builtin. The complete command and defaults
for each builtin are defined in the corresponding Pkl file in
[`pkl/builtins`](https://github.com/jdx/hk/tree/main/pkl/builtins).

## Available Builtins

<!--@include: ./gen/builtins.md-->

## Customizing Builtins

### Override Properties

```pkl
["prettier"] = (Builtins.prettier) {
  // Override glob patterns
  glob = List("src/**/*.js", "src/**/*.ts")

  // Disable batch processing
  batch = false

  // Add environment variables
  env {
    ["PRETTIER_CONFIG"] = ".prettierrc.json"
  }
}
```

### Add Dependencies

```pkl
["eslint"] = (Builtins.eslint) {
  // Run after prettier
  depends = "prettier"
}
```

### Workspace-Specific Configuration

```pkl
["cargo_clippy"] = (Builtins.cargo_clippy) {
  // Only run in directories with Cargo.toml
  workspace_indicator = "Cargo.toml"

  // Custom command using workspace
  check = "cargo clippy --manifest-path {{workspace}}/Cargo.toml"
}
```

### Profile-Based Configuration

```pkl
["mypy"] = (Builtins.mypy) {
  // Only run with "python" profile
  profiles = List("python")
}
```

## Creating Custom Steps

If a builtin doesn't exist for your tool:

```pkl
["custom-tool"] {
  glob = List("*.custom")
  check = "custom-tool --check {{files}}"
  fix = "custom-tool --fix {{files}}"
  batch = true  // Enable parallel processing
}
```

## See Also

- [Configuration Guide](/configuration)
- [Getting Started](/getting_started)
