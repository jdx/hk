---
description: The crew's words for hooks, steps, groups, profiles, file locks, stashing, and workspaces in hk.
sourceHash: 4ad40291bdd6
---

# The crew's glossary

What the crew means by hooks, steps, groups, profiles, file locks, stashing, and workspaces in hk. Learn these, sailor, and ye'll follow every call on deck.

## Builtin: the standing crew {#builtin}

A reusable Pkl step definition that comes with the ship, supplied by hk. It describes how to call on a linter or utility; ye must bring the external linter executables aboard separately, for a builtin brings the know-how, not the tool. [Read the crew roster](/builtins).

## Check: inspecting the cargo {#check}

A command that reports problems without modifying files: a lookout only looks. hk relies on that convention to run checks all at once under shared read locks. `hk check` runs the configured `check` hook.

## Dependency: waiting on another hand {#dependency}

The `depends` property names the steps that must finish before another step runs. Use it to set the order, such as `depends = "eslint"` on a formatter, so the sailmaker waits until eslint is done. [Dependencies](/configuration#dependencies-and-groups).

## File lock: a lashing {#file-lock}

The lashing on a file a step was handed, within one run of a hook: it coordinates who may touch that file. Many checks can hold read locks on it at once; a writer needs the file to itself. Locks only cover the files hk knows the step uses, so a lashing can't hold what hk was never told about. [How all hands haul at once](/why-hk#parallelism-needs-coordination).

## Fix: mending the canvas {#fix}

A command that may modify files to set problems right. A fix can still fail when some findings need mending by hand. `hk fix` runs the configured `fix` hook.

## Glob: picking out the cargo {#glob}

A pattern that selects file paths, such as `*.py` or `src/**/*.ts`. A step's patterns filter the cargo already selected for the run; they do not force a changed-file run to scan the whole ship.

## Group: a gang of hands {#group}

A collection of steps with a scheduling boundary, a gang that goes aloft together: its children can all haul at once, while later groups wait their turn. A group can hand its children defaults such as `dir` and `prefix`. [Group defaults](/configuration#group).

## Hook: the bosun's pipe {#hook}

A named collection of steps: the hands one pipe calls on deck. Git hooks include `pre-commit` and `pre-push`; custom hooks such as `check` can be invoked by hand. [Git hooks, the bosun's pipes](/hooks).

## Job: one haul on the line {#job}

A unit of step execution. A step may take several hauls, creating multiple jobs through batching or workspace selection. `--jobs` and `HK_JOBS` limit how many run at once; tools may also start their own workers.

## Profile: a watch {#profile}

A label used to enable or disable steps, such as `slow` or `types`: the slow watch, the types watch. Call one up with `--profile types` or `HK_PROFILE=types`. A step requires all of its positive profile names: it turns to only when every one of those watches is on duty. [Profiles, the watches](/configuration#profiles).

## Stage: loading cargo aboard {#stage}

To add file content to Git's index, loading it aboard for the next voyage (the next commit). A hook can stage fixes automatically, or leave them on the dock for ye to review with `stage = false`. This differs from a step's `stage` property, which specifies the file patterns to stage.

## Stash: stowed in the hold {#stash}

Unstaged work, stowed in the hold for a spell. A hook with `stash = "git"` sets the staged cargo apart before running its steps, and brings the stowed changes back up afterward. `"patch-file"` currently uses the same implementation. [Stashing and partial commits: stowing the hold](/hooks#stashing-and-partial-commits).

## Step: a hand {#step}

One hand of the crew: an individual check, formatter, or task within a hook. A step defines its commands, and can select its cargo (files), declare dependencies, and require profiles. [Sign on a hand: define a step](/configuration#define-a-step).

## Workspace: a cabin of the ship {#workspace}

A project directory found by a marker such as `package.json` or `Cargo.toml`: a cabin, known by the marker on its door. `workspace_indicator` divides the selected files by cabin, so a step can run once per matching workspace. [Workspaces](/configuration#workspaces).
