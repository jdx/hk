# Contributing

Thank you for your interest in contributing to hk! This guide will help you get started.

## Contribution Expectations

Before opening a PR, unless it is something obvious, consider creating a
discussion or mentioning what you plan to do in
[Discord](https://discord.gg/UBa7pJUN7Z). The important part is to settle the
direction before much review happens. hk has a specific scope and design
taste. I am comfortable saying no to changes that do not clearly fit.

Before I review a PR, CI must be passing and all automated AI review comments
must be addressed. If those are still open, assume I will wait to look at the
PR.

If I am on the fence about a contribution, I will probably reject it for that
reason alone. If I did not do this, hk would suffer from feature bloat. I
may also reject a PR if the quality is poor enough that I do not have confidence
the contributor can get it across the finish line. I do not have time to coach
contributors.

I get hundreds of PRs per week across my projects, so I do not have time to
respond to every PR with detailed context. A rejection may be brief.

## Prerequisites

- [mise](https://mise.jdx.dev/)
- [Rust](https://www.rust-lang.org/)

## Setup

1. Clone the repository:

```sh
git clone https://github.com/jdx/hk.git
cd hk
```

2. Install required tools and dependencies:

```sh
mise install
```

## Running Tests

To run the test suite, use the following command:

```sh
mise run test
```

This will run all tests, including Bats shell tests and any other checks defined in the project.

To run a specific test, use the following command:

```sh
mise run test:bats -- test/workspace_indicator.bats
```

## Code Style

Check/format code with hk:

```sh
hk fix --all
```

Or with the mise task:

```sh
mise run lint-fix
```

## Commit and PR Titles

Use Conventional Commits for commit messages and PR titles. Examples:

- `fix: handle missing config file`
- `docs: clarify installation steps`
- `feat: add quiet output mode`
