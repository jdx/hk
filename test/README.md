# hk test suite

hk uses Rust tests for internal behavior, Bats for CLI integration, and Pkl step tests for builtins.

## Run tests

From the repository root:

```sh
mise run test                          # Full test suite
mise run test:cargo                    # Rust tests
mise run test:bats                     # Bats backend variants
mise run test:bats test/check.bats      # One integration-test file
```

The Bats task builds hk and runs variants for libgit2, the Git CLI, and operation outside a Git repository where supported.

After building, run a specific Bats test directly when debugging:

```sh
mise exec -- bats test/check.bats --filter "check files"
```

This direct command does not run every backend variant.

## Write a Bats test

Use the shared setup and teardown to get an isolated temporary repository:

```bash
setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "validates an empty configuration" {
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
EOF
    run hk validate
    assert_success
}
```

`$PKL_PATH` points to the local schema. Create only the files and Git state needed by the test, invoke hk through `run`, and assert the relevant output, exit status, file contents, or index state.

For stashing tests, distinguish working-tree content from staged content explicitly. A successful command alone does not prove a partial commit was preserved.

## Test builtins

Builtin tests belong in the step’s `tests` field in `pkl/builtins/<name>.pkl`. They run through `hk test`; the integration harness in [builtins_tests.bats](builtins_tests.bats) loads the builtin catalogue.

Use tool stubs in `builtin_tool_stubs/` to provide external tools and the `TestMaker` helper in `pkl/builtins/test/helpers.pkl` for standard check/fix cases. See [adding a builtin](../docs/contributing.md#add-a-builtin).

## Configuration cache

Shared setup enables `HK_CACHE=1` even for debug builds. The cache directory is `$BATS_TEST_TMPDIR/hk-test-cache` when Bats supplies that directory, otherwise `/tmp/hk-test-cache`. Setup removes cache files older than one day.

The helper sets `HK_CACHE` during setup, so setting `HK_CACHE=0` on the outer test command does not disable it. Call the helper **after** `_common_setup` in a test that needs uncached evaluation:

```bash
setup() {
    load 'test_helper/common_setup'
    _common_setup
    _disable_test_cache
}
```

Use `_clear_test_cache` to clear the test’s configured cache directory or `_test_cache_stats` for diagnostics. For a direct hk invocation, `HK_CACHE=0 hk validate` bypasses caching.
