---
outline: "deep"
---

# Glossary

This glossary defines key terms used throughout hk documentation and configuration.

## Depends

A step configuration property that specifies other steps that must complete successfully before this step can run. Dependencies control execution order and ensure prerequisites are met.

Example:
```pkl
steps {
  ["typecheck"] {
    depends = List("lint", "format")  // Wait for lint and format to complete
    check = "tsc --noEmit"
  }
}
```

See: [Step Dependencies](/configuration.md#step-depends-list-string)

## Group

An organizational unit that contains multiple steps, allowing you to structure your configuration hierarchically. Groups help organize related steps together and can be used to create logical divisions like "frontend" and "backend" tasks.

Example:
```pkl
steps {
  ["frontend"] = new Group {
    steps {
      ["prettier"] = Builtins.prettier
      ["eslint"] = Builtins.eslint
    }
  }
}
```

See: [Group Configuration](/configuration.md#group)

## Hook

A git hook or custom command that runs a collection of steps. hk supports standard git hooks like `pre-commit`, `pre-push`, `commit-msg`, and `prepare-commit-msg`, as well as custom hooks like `check` and `fix` for manual execution.

Example:
```pkl
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = linters
  }
}
```

See: [Hooks](/hooks.md)

## Job

The number of parallel processes that hk will use to execute steps concurrently. This affects performance by controlling how many linting/formatting tasks can run simultaneously. Can be configured via the `-j/--jobs` CLI flag or `HK_JOBS` environment variable.

Example:
```bash
# Run with 4 parallel jobs
hk check --jobs 4

# Or via environment variable
HK_JOBS=8 hk fix
```

See: [HK_JOBS](/environment_variables.md#hk_jobs)

## Skip

A mechanism to bypass execution of specific steps or entire hooks. Steps can be skipped using the `HK_SKIP_STEPS` environment variable, while entire hooks can be skipped with `HK_SKIP_HOOK`.

Examples:
```bash
# Skip specific steps
HK_SKIP_STEPS=lint,test hk run pre-commit

# Skip entire hooks
HK_SKIP_HOOK=pre-commit,pre-push git commit
```

See: [HK_SKIP_STEPS](/environment_variables.md#hk_skip_steps), [HK_SKIP_HOOK](/environment_variables.md#hk_skip_hook)

## Stash

A strategy for temporarily saving unstaged changes before running hooks that might modify files. This prevents conflicts between your working directory changes and the automated fixes applied by linting tools.

Stash strategies:
- `git`: Uses `git stash`
- `patch-file`: Uses hk-generated patch files (faster, avoids lock conflicts)
- `none`: No stashing (fastest, but may cause staging conflicts)

Example:
```pkl
hooks {
  ["pre-commit"] {
    stash = "patch-file"  // Use patch-based stashing
    steps = linters
  }
}
```

See: [HK_STASH](/environment_variables.md#hk_stash)

## Step

An individual linting, formatting, or validation task that processes files. Steps are the fundamental units of work in hk, each defining commands to check and/or fix code. Steps can specify which files they operate on using glob patterns, and can have dependencies on other steps.

Example:
```pkl
steps {
  ["eslint"] {
    glob = List("*.js", "*.ts")
    check = "eslint {{files}}"
    fix = "eslint --fix {{files}}"
    depends = List("prettier")  // Run after prettier
  }
}
```

See: [Step Configuration](/configuration.md#hooks-hook-steps-step-group)
