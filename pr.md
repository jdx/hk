test(setup): exempt nanoid from aube trust-downgrade policy

## Problem

`ci-nogit` started failing with `ERR_AUBE_TRUST_DOWNGRADE` when installing
the npm-backed test tooling (`stylelint`, `vite-plus`):

```
ERR_AUBE_TRUST_DOWNGRADE
  × failed to resolve dependencies
  ╰─▶ trust downgrade for nanoid@3.3.14 (trustPolicy=no-downgrade): earlier
      published version 5.1.12 had trusted publisher but this version has no
      trust evidence
```

A recent aube release enabled `trustPolicy = no-downgrade` by default. It
rejects any version whose trust evidence is weaker than an earlier-published
version of the same package. `nanoid@3.3.14` (the 3.x maintenance line) lacks
the trusted-publisher evidence that the 5.x line carries, so aube treats it as
a possible supply-chain downgrade and aborts the install.

This is upstream behavior, not a hk change — runs before the aube release
passed; runs after fail.

## Why a version bump doesn't help

`nanoid@3.3.x` is pulled transitively via `postcss` (`nanoid@^3.3.12`), which
both `stylelint` and `vite-plus` depend on. Verified that even the latest
`stylelint` (17.13.0) still resolves `nanoid@3.3.14` and trips the same error,
because postcss stays on the CJS-compatible nanoid 3.x line. Updating the tools
does not avoid the issue.

## Fix

Add `nanoid` to `trustPolicyExclude` in the test aube config, mirroring the
targeted, per-package approach used for `allowedUnpopularPackages` in #1002.
This is test tooling and `nanoid@3.3.14` is not a confirmed compromise — just
an older release line without trusted-publisher metadata.

## Verification

- Reproduced `ERR_AUBE_TRUST_DOWNGRADE` locally with aube 1.18.2.
- Confirmed `trustPolicyExclude = ["nanoid"]` lets the install resolve
  (`+ nanoid@3.3.14`).
- Confirmed `aube config get` reads both keys correctly from the commented TOML.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
