# Contributing to hk

Thank you for your interest in contributing to hk! This guide will help you get started.

## Prerequisites

- [mise](https://mise.jdx.dev/)
- [Rust](https://www.rust-lang.org/)

## Setup

1. Clone the repository:

```sh
git clone --recurse-submodules https://github.com/jdx/hk.git
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
mise lint-fix
```
