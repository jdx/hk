#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "fail_fast=true aborts on first failure" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
  ["check"] {
    steps {
      ["first"] {
        exclusive = true
        check = "sh -c 'echo FIRST && exit 2'"
      }
      ["second"] { check = "echo SECOND" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "test" > test.txt

    run hk check
    assert_failure
    assert_output --partial "FIRST"
    # Should not run the second step when fail_fast=true
    refute_output --partial "SECOND"
}

@test "fail_fast=false continues after failure" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
  ["check"] {
    steps {
      ["first"] {
        exclusive = true
        check = "sh -c 'echo FIRST && exit 2'"
      }
      ["second"] { check = "echo SECOND" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "test" > test.txt

    run hk check
    # Overall run still fails due to first step
    assert_failure
    assert_output --partial "FIRST"
    # With fail_fast=false, the second step should still run
    assert_output --partial "SECOND"
}

@test "fail-fast reports the failing step, not a sibling it cancelled" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
  ["check"] {
    steps {
      ["sleeper"] { check = "sleep 30" }
      ["broken"] { check = "sh -c 'echo REAL-FAILURE >&2; exit 3'" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "test" > test.txt

    # The cancelled sibling must never win the race to be the reported error.
    for libgit2 in 1 0; do
        for i in $(seq 1 15); do
            start=$SECONDS
            HK_LIBGIT2=$libgit2 run hk check
            assert_failure
            assert [ $((SECONDS - start)) -lt 15 ]
            assert_output --partial "REAL-FAILURE"
            assert_output --partial "broken"
            refute_output --partial "cancelled"
        done
    done
}

@test "allow_failure does not hide a job that could not run in another workspace" {
    mkdir -p services/api/sub services/web
    touch services/api/module.toml services/web/module.toml
    echo a > services/api/a.mod
    echo b > services/web/b.mod
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["vet"] {
        glob = "**/*.mod"
        workspace_indicator = "module.toml"
        dir = "{{workspace}}/sub"
        allow_failure = true
        check = new Command { argv = List("sh", "-c", "echo ALLOWED-FAILURE >&2; exit 1") }
      }
    }
  }
}
EOF
    git add .
    git commit -m "init"
    for libgit2 in 1 0; do
        HK_LIBGIT2=$libgit2 run hk check --all
        assert_failure
        assert_output --partial "working directory does not exist"
    done
}
