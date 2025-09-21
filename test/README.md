# HK Test Suite

This directory contains the test suite for HK, using the [Bats](https://github.com/bats-core/bats-core) testing framework.

## Running Tests

```bash
# Run all tests
mise run test

# Run only bats tests
mise run test:bats

# Run specific test file
bats test/check.bats

# Run specific test by name
bats test/check.bats --filter "check files"
```

## Test Caching

To improve test performance, the test suite uses a persistent cache for parsed PKL configurations. This cache survives between test runs and significantly speeds up test execution.

### Cache Location

By default, the cache is stored in `/tmp/hk-test-cache`. This location:
- Persists between test runs for performance
- Gets cleared on system reboot
- Automatically removes entries older than 1 day

### Environment Variables

You can control caching behavior with these environment variables:

- `HK_TEST_CACHE_DISABLED=1` - Disable test caching entirely (forces fresh PKL evaluation every time)
- `HK_TEST_CACHE_DIR=/custom/path` - Use a custom cache directory location
- `HK_TEST_CACHE_NO_CLEANUP=1` - Disable automatic cleanup of stale cache entries

### Examples

```bash
# Run tests without cache (useful for debugging)
HK_TEST_CACHE_DISABLED=1 mise run test:bats

# Use a custom cache directory
HK_TEST_CACHE_DIR=/tmp/my-cache mise run test:bats

# Keep all cache entries (no cleanup)
HK_TEST_CACHE_NO_CLEANUP=1 mise run test:bats
```

### Per-Test Cache Control

Individual tests can also control caching:

```bash
# In a test file
_disable_test_cache           # Disable cache for this test
_enable_isolated_test_cache   # Use isolated cache for this test file
_clear_test_cache             # Clear the cache
```

## Writing Tests

Tests are written using Bats syntax. Each test file should:

1. Include the common setup:
```bash
setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}
```

2. Define tests using `@test`:
```bash
@test "description of test" {
    # test code here
    run hk validate
    assert_success
}
```

See existing test files for examples of different testing patterns.
