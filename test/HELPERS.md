# Bats Test Helpers

This document describes the new test helper functions added to improve the bats test suite.

## Setup

The helpers are automatically loaded in tests that use `common_setup`:

```bash
setup() {
    load 'test_helper/common_setup'
    _common_setup
}
```

## Assertion Helpers (`test_helper/assertions.bash`)

### Command Execution

- `assert_hk_success <command> [args...]` - Assert hk command succeeds
- `assert_hk_failure <command> [args...]` - Assert hk command fails

### File Assertions

- `assert_file_modified <file> <old_mtime>` - Check if file was modified
- `assert_file_not_modified <file> <old_mtime>` - Check if file was NOT modified
- `get_file_mtime <file>` - Get file modification time
- `assert_file_contains <file> <content>` - Check file contains content
- `assert_file_not_contains <file> <content>` - Check file doesn't contain content
- `assert_file_permissions <file> <perms>` - Check file permissions

### Git Assertions

- `assert_git_clean` - Assert working directory is clean
- `assert_git_dirty` - Assert working directory has changes

### Step Execution Assertions

- `assert_step_executed <step_name>` - Check if step was executed
- `assert_step_skipped <step_name> [reason]` - Check if step was skipped
- `assert_step_failed <step_name>` - Check if step failed

### Output Assertions

- `assert_output_matches <regex>` - Assert output matches regex pattern
- `assert_line_matches <regex>` - Assert any line matches regex pattern

## Setup Helpers (`test_helper/setup_helpers.bash`)

### Configuration Setup

- `setup_with_config <pkl_content>` - Create test with custom hk.pkl
- `setup_with_builtin <builtin_name> [extra_config]` - Set up with a builtin linter
- `setup_with_hook <hook_name> <step_config>` - Configure specific hook

### Git State Setup

- `setup_with_git_state <state> [files...]` - Create specific git states:
  - `staged` - Files staged for commit
  - `unstaged` - Files modified but not staged
  - `untracked` - Untracked files
  - `committed` - Files committed
  - `conflict` - Merge conflict state
  - `mixed` - Mix of different states

### Project Setup

- `setup_project_with_files <type> [count]` - Create project with files:
  - `javascript` - JavaScript project
  - `typescript` - TypeScript project
  - `python` - Python project
  - `rust` - Rust project
  - `mixed` - Mix of different file types

- `setup_complex_directory_structure` - Create complex directory tree

### Test Environment

- `setup_with_env VAR1=val1 VAR2=val2 ...` - Set environment variables
- `setup_with_profile <profile> [active]` - Configure profiles
- `setup_with_failing_step [name] [error]` - Create failing step
- `setup_with_custom_cache [dir]` - Use custom cache directory
- `setup_with_timing [json_file]` - Enable timing tracking

### Utility Functions

- `create_test_files <pattern> <count>` - Create test files:
  - `valid` - Valid syntax files
  - `invalid` - Files with syntax errors
  - `mixed` - Mix of valid and invalid

- `cleanup_test_artifacts` - Clean up test environment
- `wait_for_process <pid> [timeout]` - Wait for process completion

## Usage Examples

### Simple Test

```bash
@test "example test with helpers" {
    setup_with_config "amends \"$PKL_PATH/Config.pkl\"
hooks {
  [\"check\"] {
    steps {
      [\"lint\"] {
        shell = \"echo linting\"
        glob = List(\"*.js\")
      }
    }
  }
}"

    echo "test" > file.js
    assert_hk_success check --all
    assert_line --partial "linting"
}
```

### Testing with Git States

```bash
@test "test with staged files" {
    setup_with_git_state staged file1.txt file2.txt

    # Files are now staged
    assert_git_dirty

    run git status --short
    assert_output --partial "A  file1.txt"
}
```

### Testing with Builtins

```bash
@test "test prettier builtin" {
    echo "console.log('test')" > test.js
    setup_with_builtin prettier

    run hk check --all
    assert_failure  # Will fail due to missing semicolon
}
```

### Complex Test Scenario

```bash
@test "complex test with multiple helpers" {
    # Set environment
    setup_with_env HK_FAIL_FAST=true

    # Create project structure
    setup_project_with_files javascript 3

    # Configure hook
    setup_with_hook "check" '[\"format\"] { fix = \"prettier --write {{files}}\" }'

    # Run and verify
    assert_hk_success fix --all
    assert_git_dirty
}
```

## Notes

- PKL configuration requires proper syntax with quoted keys in mappings: `["check"]` not `check`
- Steps need glob patterns to match files, otherwise they won't execute
- Use `List(...)` for arrays in PKL, not `[...]`
- The helpers handle common setup/teardown automatically
- All helpers preserve the test isolation provided by bats

## Migration Guide

To migrate existing tests:

1. Replace repetitive setup code with helper functions
2. Replace `run hk` + `assert_success` with `assert_hk_success`
3. Use setup helpers instead of manual file creation
4. Simplify assertions with the new assertion helpers

Before:
```bash
cat > hk.pkl << EOF
# Complex multi-line config
EOF
echo "test" > file.js
run hk check --all
assert_success
assert_line --partial "✓"
```

After:
```bash
setup_with_config "..." # Simplified config
create_test_files valid 1
assert_hk_success check --all
assert_step_executed "step_name"
```
