---
description: Complete hk configurations for JavaScript, Python, monorepos, and custom steps.
---

# Configuration examples

Choose a starting point, save its downloadable Pkl file as `hk.pkl`, and adapt the tools and paths to your project. Each page includes the exact file it offers for download.

| Example                                           | Tools and concepts                                            |
| ------------------------------------------------- | ------------------------------------------------------------- |
| [JavaScript and TypeScript](./javascript-project) | ESLint, Prettier, and optional TypeScript checking            |
| [Python](./python-project)                        | Ruff linting and formatting, plus optional mypy               |
| [Monorepo](./monorepo)                            | Component groups, inherited defaults, and multiple toolchains |
| [Custom steps](./custom-linters)                  | Check/fix commands and self-contained step tests              |

Install any external tools the example invokes. Then validate and inspect the configuration before installing hooks:

```sh
hk validate
hk check --all --plan
hk install
```

For package environments and editor-launched hooks, see [mise integration](/mise_integration). For individual properties, see [configuration](/configuration).
