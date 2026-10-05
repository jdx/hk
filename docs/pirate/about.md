---
description: Why hk was built, where it stands among the rest o' yer tools, and where to sign aboard and lend a hand.
sourceHash: 5f59aa4ed25d
---

# About hk: the bosun's tale

hk is a Git hook manager and project linting tool, built by [@jdx](https://github.com/jdx). It is written in Rust and sails under the [MIT license](https://github.com/jdx/hk/blob/main/LICENSE).

## Why hk was launched {#why-it-exists}

A Git hook, the bosun's pipe, sits right in the path of a commit, and the voyage waits on it. So its speed matters. But send formatters aloft all at once and ye've a coordination problem: two tools can read the same file, then each write over the other's changes.

hk lashes the files with read/write locks so those tools can haul together without colliding. Checks can share read access, many lookouts to one file; fixes take exclusive access to the files they modify. Builtins put tool features to work, such as diff output and lists of files needing changes, which help hk keep more hands hauling at once.

[Why sail with hk?](/why-hk) explains the execution model (how the crew is worked) and its tradeoffs.

## Where hk stands on deck {#where-it-fits}

hk is the bosun: it decides which checks to run, on which files, and when. The linters, yer lookouts, still own their rules and configuration. Yer package manager provides their executables; [mise](/mise_integration), the quartermaster, can manage those versions and the environment Git uses.

The charts are drawn in [Pkl](/pkl_introduction), the chart-maker's language, for types, imports, and reusable step definitions. The default evaluator comes aboard inside hk itself.

## Sign aboard {#get-involved}

So all you hands who would sign aboard: sing out bugs in [GitHub issues](https://github.com/jdx/hk/issues), talk over ideas in [GitHub Discussions](https://github.com/jdx/hk/discussions) or on [Discord](https://discord.gg/UBa7pJUN7Z), and read the [contributing guide](/contributing) before ye start on a larger change.

For something less technical, there's also a [sea shanty](/shanty) to haul to.
