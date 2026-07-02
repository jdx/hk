# About

hk is built by [@jdx](https://github.com/jdx).

## Why does this exist?

git hooks need to be fast above all else or else developers won't use them. Parallelism
is the best (and likely only) way to achieve acceptable performance at the git hook manager level.

Existing alternatives to hk such as [lefthook](https://github.com/evilmartians/lefthook) support
very basic parallel execution of shell scripts—however, because linters may edit files, this naive approach
can break down if multiple linters affect the same file.

I felt that a git hook manager that had tighter integration with the linters would be able to leverage
read/write file locks to enable more aggressive parallelism while preventing race conditions. This read/write locking is the primary reason
I built hk, however there are other design decisions worth noting that I think make hk a better experience than its peers:

- hk has a bunch of [builtins](https://github.com/jdx/hk/tree/main/pkl/builtins) you can use for common linters like `prettier` or `black`.
- hk can stash unstaged changes before running "fix" hooks (see the `stash` setting). This prevents a common issue with pre-commit hooks where files containing both staged and
  unstaged changes get modified and the unstaged changes end up being staged erroneously.
- By default, hk uses libgit2 to directly interact with git instead of shelling out many times to `git`.
  (This generally makes hk much faster but there are situations like `fsmonitor` where it may perform worse)
- hk is a Rust CLI which gives it great startup performance.
- hk is designed to work well with my other project [mise-en-place](https://mise.jdx.dev) which makes it easy to manage dependencies for hk linters.

## Contributing

Contributions are welcome! Please open an issue or submit a PR. I always encourage reaching out to me first before submitting a feature PR to make sure it's something I will be interested in
maintaining.
