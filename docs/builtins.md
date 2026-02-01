---
outline: "deep"
---

# Built-in Linters Reference

hk provides 90+ pre-configured linters and formatters through the `Builtins` module. These are production-ready configurations that work out of the box.

## Usage

Import and use builtins in your `hk.pkl`:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.35.0/hk@1.35.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.35.0/hk@1.35.0#/Builtins.pkl"

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

- [Configuration Guide](configuration.md)
- [Getting Started](getting_started.md)
