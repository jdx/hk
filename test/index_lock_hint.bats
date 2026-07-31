#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "index lock failure suggests serializing concurrent steps once" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
  ["check"] {
    steps {
      ["writer-a"] {
        check = "echo \"fatal: Unable to create '/repo/.git/index.lock': File exists.\" >&2; exit 128"
        glob = "*.txt"
      }
      ["writer-b"] {
        check = "echo \"fatal: Unable to create '/repo/.git/index.lock': File exists.\" >&2; exit 128"
        glob = "*.txt"
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    run hk check
    assert_failure
    assert_output --partial "hint: this may be contention between concurrent steps that write the Git index."
    assert_output --partial 'serialize index-writing steps with `exclusive = true`, `depends`, or separate groups.'
    count=$(echo "$output" | grep -c "hint: this may be contention" || true)
    [ "$count" -eq 1 ] || fail "hint appeared $count times, expected 1"
}

@test "ordinary command failure does not show index lock hint" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] {
        check = "echo 'lint failed' >&2; exit 1"
        glob = "*.txt"
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    run hk check
    assert_failure
    refute_output --partial "hint: this may be contention"
}

@test "silent mode suppresses index lock hint" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["writer"] {
        check = "echo \"fatal: Unable to create '/repo/.git/index.lock': File exists.\" >&2; exit 128"
        glob = "*.txt"
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    run hk check --silent
    assert_failure
    refute_output --partial "hint: this may be contention"
}
