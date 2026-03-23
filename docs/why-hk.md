# Why hk?

Tools like pre-commit, prek, and lefthook simply shell out to run linters. That means they can't safely run linters in parallel—if two linters try to modify the same file at the same time, there will be a race condition.

hk has a lot of tricks up its sleeve so that it can safely do this. hk maintains a read/write lock for every file being linted. If a linter needs to write, it needs an exclusive lock. Of course if every linter needed a write lock then we wouldn't be able to parallelize—but with hk we avoid write locks through several mechanisms depending on the capabilities of the linter.

## How hk avoids write locks

A couple of examples:

**ruff** – has diff output which is perfect for hk. hk runs `ruff format --diff` which outputs a diff if there are any changes that need to be made. This way hk can simply apply that diff later itself without needing to shell out to ruff again. (However, if there are conflicts because multiple linters edited the same file, it may need to.)

**prettier** – supports `--list-different`, which tells hk which files need formatting without modifying anything. hk uses this to run the check with only a read lock on all targeted files, then narrows down the set of files before running `prettier --write`.

For linters with none of these capabilities (ahem: eslint), it will fall back to fetching a write lock on all of its files. Other linters that don't touch the same files will still run in parallel. Optionally, you can use the `check_first` feature to run something like `eslint` on all files and if that fails run `eslint --fix` on all of those same files again.

For this reason, hk requires more integration work with each linter. However, since it comes with a large corpus of [builtins](/builtins) you likely won't need to deal with the nitty-gritty yourself.

## Smart stashing

Here's a scenario every developer hits: you've got a file with some changes staged for commit and other changes you're still working on. You run your pre-commit hooks—and suddenly your unstaged work-in-progress gets formatted, linted, and staged along with everything else. Your careful partial commit is ruined.

hk solves this properly. Before running any fixers, hk stashes your unstaged changes so linters only see what you're actually committing. After linters run, hk restores your unstaged work using a **three-way merge**:

1. **Before hooks run**: hk snapshots your working tree and stashes only the unstaged changes (the diff between your worktree and your index). Your working tree now matches your staged content exactly.
2. **Hooks run**: linters and fixers see only the staged content. If prettier reformats a file, it reformats the version you're committing—not your work-in-progress.
3. **After hooks run**: hk does a three-way merge to combine the fixer's changes with your unstaged work. If the fixer changed line 5 and your unstaged changes are on line 20, both are preserved. If there's a conflict, your unstaged changes take priority—hk never destroys your in-progress work.

This works even with partially staged hunks in a single file. If you `git add -p` to stage just one function, hk will lint that function, apply fixes to it, and leave your other unstaged changes in the file untouched.

Other tools either don't stash at all (lefthook), or do basic stashing that can lose changes when fixers and unstaged edits touch the same file (pre-commit, prek).

## Built-in utilities

hk ships with fast Rust-native utilities for common tasks like trailing whitespace removal, end-of-file fixing, and merge conflict detection. These run as part of hk itself—no extra tools to install. We plan to add more of these over time so common checks are as fast as possible.

## Plugin security

pre-commit and prek download hook implementations from external git repositories. Each hook repo contains its own environment setup, dependencies, and entry points—you're running code pulled from third-party repos.

hk builtins are just [Pkl](https://pkl-lang.org/) config hosted in the [hk repository](https://github.com/jdx/hk/tree/main/pkl/builtins) and fetched via the import statement in your `hk.pkl`. They define how to invoke linters already on your system—they don't download or execute third-party code. You can read exactly what each builtin does in a few lines of Pkl:

```pkl
// This is an entire hk builtin. That's it.
prettier = new Config.Step {
  glob = List("**/*.js", "**/*.ts", "**/*.css", "**/*.json", "**/*.md")
  check = "prettier --check {{ files }}"
  check_list_files = "prettier --list-different {{ files }}"
  fix = "prettier --write {{ files }}"
}
```

Compare that to pre-commit:

```yaml
# pre-commit: hooks are downloaded from external git repos
repos:
  - repo: https://github.com/pre-commit/mirrors-prettier
    rev: v3.1.0
    hooks:
      - id: prettier
```

## vs pre-commit

[pre-commit](https://pre-commit.com/) is the most popular hook manager. It's written in Python and requires Python as a runtime dependency.

- Runs hooks **sequentially**—hook B waits for hook A to finish completely
- Downloads hook code from external git repos
- Requires Python on the system
- Uses YAML for configuration

## vs prek

[prek](https://github.com/j178/prek) is pre-commit reimplemented in Rust. It's faster than pre-commit but fundamentally the same model:

- Still runs hooks **sequentially**—same execution model as pre-commit
- Still downloads hook code from external git repos
- prek itself doesn't require Python, but Python-based hooks (which are most of the pre-commit ecosystem) still need Python at runtime—prek just manages it automatically via uv
- prek does have some built-in Rust-native hooks for common checks, similar to hk's `hk util` commands

hk gets its speed from **parallelism**, not just language choice. Running 10 linters in parallel is fundamentally faster than running them one at a time, regardless of how fast each individual run is.

## vs lefthook

[lefthook](https://github.com/evilmartians/lefthook) is a hook manager written in Go. It's the closest to hk in philosophy—it supports parallel execution.

The problem: lefthook has **no file-level coordination**. If two parallel jobs modify the same file, you get a race condition—the last writer wins and the other's changes are silently lost. This means in practice you either accept the risk or configure your hooks to not overlap, which defeats the purpose.

hk solves this with read/write file locks. Multiple linters can safely run in parallel, even when they touch the same files.

Other differences:
- lefthook has no builtin linter definitions—you write shell commands directly in YAML
- lefthook does not stash unstaged changes before running fix hooks, which can cause unstaged changes to be erroneously staged
- lefthook uses YAML; hk uses [Pkl](https://pkl-lang.org/) for type-safe configuration

## vs husky + lint-staged

[husky](https://github.com/typicode/husky) (~35k stars) and [lint-staged](https://github.com/lint-staged/lint-staged) (~14k stars) are the most popular combination in the JavaScript ecosystem.

husky is intentionally minimal—it just wires up git hooks. lint-staged handles the actual linting orchestration. lint-staged does run tasks matched to *different* glob patterns in parallel, but tasks targeting the *same* glob run sequentially. There's no file-level coordination, so if two tasks end up touching the same file through different globs, you get a race condition.

Both require Node.js. If your project isn't a JS project, you're adding a runtime dependency just for your hook manager.

## Feature comparison

| Feature | hk | pre-commit | lefthook | prek |
|---------|-----|-----------|----------|------|
| Parallel execution | File-locked | No | Unsafe | No |
| Language | Rust | Python | Go | Rust |
| Requires Python | No | Yes | No | Often* |
| Config format | Pkl | YAML | YAML | YAML |
| Built-in linter configs | [120+](/builtins) | — | — | — |
| Plugin model | Pkl config (local) | Git repos (remote) | Shell commands | Git repos (remote) |
| check_diff support | Yes | No | No | No |
| check_list_files support | Yes | No | No | No |
| Stash management | Yes | Partial | No | Partial |
| Dependency resolution | Yes | No | No | No |
| Batched execution | Yes | No | No | No |

*prek itself is Rust but many hooks in the pre-commit ecosystem require Python environments to run.

See also: [Benchmarks](/benchmarks) for reproducible performance numbers.

Come give it a whirl: [Getting Started](/getting_started)
