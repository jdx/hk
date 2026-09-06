---
description: Why hk exists, how it fits into a toolchain, and where to contribute.
---

# About hk

hk is a Git hook manager and project linting tool built by [@jdx](https://github.com/jdx). It is written in Rust and released under the [MIT license](https://github.com/jdx/hk/blob/main/LICENSE).

## Why it exists

Git hooks sit directly in the path of a commit. Their speed matters, but running formatters concurrently introduces a coordination problem: two tools can read the same file and then overwrite one another’s changes.

hk uses read/write file locks to coordinate those tools. Checks can share read access; fixes take exclusive access to the files they modify. Builtins expose tool features such as diff output and lists of files needing changes, which help hk keep more work running concurrently.

[Why hk?](/why-hk) explains the execution model and its tradeoffs.

## Where it fits

hk decides which checks to run, on which files, and when. Linters still own their rules and configuration. Your package manager provides their executables; [mise](/mise_integration) can manage those versions and the environment used by Git.

Configuration uses [Pkl](/pkl_introduction) for types, imports, and reusable step definitions. The default evaluator is included in hk.

## Get involved

Report bugs in [GitHub issues](https://github.com/jdx/hk/issues), discuss ideas in [GitHub Discussions](https://github.com/jdx/hk/discussions) or [Discord](https://discord.gg/UBa7pJUN7Z), and read the [contributing guide](/contributing) before starting a larger change.

For something less technical, there is also a [sea shanty](/shanty).
