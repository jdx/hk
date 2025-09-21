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

To improve test performance, the test suite enables caching for parsed PKL configurations. The test framework automatically sets `HK_CACHE=1` to enable caching (which is disabled by default in debug builds).

### How It Works

- `HK_CACHE` environment variable controls caching behavior
- In release builds: caching is enabled by default
- In debug builds: caching is disabled by default
- Tests override this by setting `HK_CACHE=1`

### Cache Location

The cache is stored in `/tmp/hk-test-cache` by default. This location:
- Persists between test runs for performance
- Gets cleared on system reboot
- Automatically removes entries older than 1 day

### Manual Control

```bash
# Run tests without cache (useful for debugging)
HK_CACHE=0 mise run test:bats

# Run hk with cache enabled in debug build
HK_CACHE=1 hk validate

# Clear the cache manually
rm -rf /tmp/hk-test-cache
```

### Per-Test Cache Control

Individual tests can control caching:

```bash
# In a test file
_disable_test_cache   # Sets HK_CACHE=0 for this test
_clear_test_cache     # Clear the cache directory
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
