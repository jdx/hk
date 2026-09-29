---
description: Rig a JavaScript ship with hk. ESLint and Prettier on deck, and TypeScript checking when ye call for it.
sourceHash: f595d405debd
---

# A JavaScript and TypeScript ship

ESLint goes to work before Prettier, and ye call TypeScript checking on deck when ye need it.

**Before ye sail:** ESLint, Prettier, and TypeScript executables on `PATH`, plus their project configuration. If they're package dependencies, expose `node_modules/.bin` through [mise, the quartermaster](/mise_integration#install-tools), or through the environment ye already have.

<a href="/javascript-project.pkl" download>Download javascript-project.pkl</a> and save it as `hk.pkl`, the charts yer ship sails by.

## The charts {#configuration}

<<< @/public/javascript-project.pkl

## Take her out {#try-it}

```sh
hk validate
hk check --all --plan
hk check --all
hk check --all --profile types
hk fix
```

ESLint and Prettier can both mend JavaScript files. The dependency gives them a stable order, the same every time they haul; configure their rules to agree. TypeScript checking stays behind the `types` profile, a watch of its own, so it only comes on deck when ye opt in.

The top-level `steps` crews the default `pre-commit`, `check`, and `fix` hooks. The pre-commit hook stows yer unstaged work in the hold before it fixes and stages changes.

## Refit her for yer own voyage {#adapt-it}

Remove `tsc` from the crew for a JavaScript-only project. If yer tools live in multiple packages, use [workspaces](/configuration#workspaces) or the [monorepo example, one ship with many crews](./monorepo). Use `hk check --all --profile types` in CI, the harbour-master's inspection, to include type checking.
