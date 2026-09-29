---
description: Ships in bottles, ready to rig. Complete hk configurations for JavaScript, Python, monorepos, and custom steps.
sourceHash: fe6e68669142
---

# Ships in bottles: configuration examples

Pick a ship to start from, save its downloadable Pkl file as `hk.pkl` (yer ship's charts), and fit the tools and paths to yer own project. Each page carries the exact file it offers for download.

| The ship in the bottle                                     | Tools and ideas aboard                                       |
| ---------------------------------------------------------- | ------------------------------------------------------------ |
| [JavaScript and TypeScript](./javascript-project)          | ESLint, Prettier, and TypeScript checking if ye want it      |
| [Python](./python-project)                                 | Ruff linting and formatting, plus mypy if ye want it         |
| [A monorepo, one ship, many crews](./monorepo)             | Component groups, inherited defaults, and several toolchains |
| [Custom steps, hands ye sign on yerself](./custom-linters) | Check/fix commands and self-contained step tests             |

Bring aboard any external tools the example calls on. Then validate and inspect the configuration before ye rig the hooks:

```sh
hk validate
hk check --all --plan
hk install
```

For package environments and hooks launched from an editor, see [mise integration, the quartermaster's berth](/mise_integration). For each property on its own, see the [configuration](/configuration) charts.
