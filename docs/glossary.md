---
description: Definitions of hooks, steps, groups, profiles, file locks, stashing, and workspaces in hk.
---

# Glossary

Definitions of hooks, steps, groups, profiles, file locks, stashing, and workspaces in hk.

## Builtin

A reusable Pkl step definition supplied by hk. It describes how to invoke a linter or utility; external linter executables must be installed separately. [Browse builtins](/builtins).

## Check

A command that reports problems without modifying files. hk relies on this convention to run checks concurrently with shared read locks. `hk check` runs the configured `check` hook.

## Dependency

The `depends` property names steps that must finish before another step runs. Use it to establish order, such as `depends = "eslint"` on a formatter. [Dependencies](/configuration#dependencies-and-groups).

## File lock

Coordination for a file selected by a step within a hook run. Multiple checks can hold read locks; a writer needs exclusive access. Locks only cover the files hk knows the step uses. [Execution model](/why-hk#parallelism-needs-coordination).

## Fix

A command that may modify files to resolve problems. A fix can still fail if some findings need manual changes. `hk fix` runs the configured `fix` hook.

## Glob

A pattern that selects file paths, such as `*.py` or `src/**/*.ts`. A step’s patterns filter the files selected for the run; they do not force a changed-file run to scan the whole repository.

## Group

A collection of steps with a scheduling boundary: its children can run concurrently, while later groups wait. Groups can provide defaults such as `dir` and `prefix` for children. [Group defaults](/configuration#group).

## Hook

A named collection of steps. Git hooks include `pre-commit` and `pre-push`; custom hooks such as `check` can be invoked manually. [Git hooks](/hooks).

## Job

A unit of step execution. A step may create multiple jobs through batching or workspace selection. `--jobs` and `HK_JOBS` limit concurrency; tools may also start their own workers.

## Profile

A label used to enable or disable steps, such as `slow` or `types`. Activate one with `--profile types` or `HK_PROFILE=types`. A step requires all of its positive profile names. [Profiles](/configuration#profiles).

## Stage

To add file content to Git’s index for the next commit. A hook can stage fixes automatically, or leave them for review with `stage = false`. This differs from a step’s `stage` property, which specifies file patterns to stage.

## Stash

Temporarily saved unstaged work. A hook with `stash = "git"` isolates staged content before running its steps and restores saved changes afterward. `"patch-file"` currently uses the same implementation. [Stashing and partial commits](/hooks#stashing-and-partial-commits).

## Step

An individual check, formatter, or task within a hook. A step defines commands and can select files, declare dependencies, and require profiles. [Define a step](/configuration#define-a-step).

## Workspace

A project directory located through a marker such as `package.json` or `Cargo.toml`. `workspace_indicator` partitions selected files so a step can run once per matching workspace. [Workspaces](/configuration#workspaces).
