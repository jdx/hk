---
description: Configure ESLint, Prettier, and optional TypeScript checking with hk.
---

# JavaScript and TypeScript

Run ESLint before Prettier, and enable TypeScript checking when you need it.

**Prerequisites:** ESLint, Prettier, and TypeScript executables on `PATH`, plus their project configuration. If they are package dependencies, expose `node_modules/.bin` through [mise](/mise_integration#install-tools) or your existing environment.

<a href="/javascript-project.pkl" download>Download javascript-project.pkl</a> and save it as `hk.pkl`.

## Configuration

<<< @/public/javascript-project.pkl

## Try it

```sh
hk validate
hk check --all --plan
hk check --all
hk check --all --profile types
hk fix
```

ESLint and Prettier can both change JavaScript files. The dependency gives them a stable order; configure their rules to agree. TypeScript checking stays behind the `types` profile so it is opt-in.

The same linter mapping powers `pre-commit`, `check`, and `fix`. The pre-commit hook saves unstaged work before fixing the staged versions.

## Adapt it

Remove `tsc` for a JavaScript-only project. If your tools live in multiple packages, use [workspaces](/configuration#workspaces) or the [monorepo example](./monorepo). Use `hk check --all --profile types` in CI to include type checking.
