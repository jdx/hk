#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "check_list_files with empty file list and stderr should display stderr" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["go-fmt"] {
        check_first = true
        glob = List("*.go")
        stage = List("*.go")
        check_list_files = """
# Simulate gofmt behavior: when gofmt has syntax errors, it writes to stderr
# but the wrapper script sees empty FILES and exits 0
echo "test.go:1:1: syntax error" 1>&2
echo "test.go:2:5: expected operand, found return" 1>&2
# Exit 0 because no files need formatting (FILES is empty)
"""
        fix = "echo 'would format' {{files}}"
      }
      ["other-step"] {
        glob = List("*.go")
        stage = List("*.go")
        fix = "echo 'other'"
      }
    }
  }
}
EOF
    # Create a go file
    echo 'package main' > test.go
    git add test.go

    run hk fix
    assert_failure
    # Should display the stderr output from check_list_files
    assert_output --partial "test.go:1:1: syntax error"
    assert_output --partial "test.go:2:5: expected operand, found return"
    assert_output --partial "check_list_files returned no files and produced errors"
}

@test "check_list_files with empty file list but no stderr should not fail" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["format"] {
        check_first = true
        glob = List("*.txt")
        stage = List("*.txt")
        check_list_files = """
# Return empty list with no stderr (files are already formatted)
# Exit 0 with empty stdout - no files need formatting
"""
        fix = "echo 'would format' {{files}}"
      }
      ["other-step"] {
        glob = List("*.txt")
        fix = "echo 'other'"
      }
    }
  }
}
EOF
    echo 'content' > test.txt
    git add test.txt

    run hk fix
    assert_success
    # Should not run format fix since no files need formatting
    refute_output --partial "would format"
    # But other-step should run
    assert_output --partial "other"
}

@test "check_list_files with files and stderr should process files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["format"] {
        check_first = true
        glob = List("*.txt")
        stage = List("*.txt")
        check_list_files = """
# Return file list with some warnings in stderr
echo "test.txt"
echo "warning: some formatting issue" 1>&2
exit 1
"""
        fix = "echo 'formatted' {{files}}"
      }
    }
  }
}
EOF
    echo 'content' > test.txt
    git add test.txt

    run hk fix
    assert_success
    # Should run fix since files were returned
    assert_output --partial "formatted test.txt"
}
