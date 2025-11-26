#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "stage '**/maintainers.yml' matches root-level file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["stage-maintainers"] {
        fix = "echo done"
        glob = "maintainers.yml"
        stage = "**/maintainers.yml"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"

    # create root-level file that should match the stage glob
    echo name: a > maintainers.yml

    # ensure it's untracked before running
    run git status --porcelain -- maintainers.yml
    assert_success
    [[ "$output" == '?? maintainers.yml' ]]

    # run fix using hk in PATH (added by test helper)
    hk fix -v

    # file should be staged after hk fix
    run git status --porcelain -- maintainers.yml
    assert_success
    [[ "$output" =~ ^A\  ]]
}

@test "stage '**/maintainers.yml' with step.dir matches file at dir root" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["stage-maintainers"] {
        fix = "echo done"
        dir = "pkg"
        glob = "maintainers.yml"
        stage = "**/maintainers.yml"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"

    mkdir -p pkg
    echo name: a > pkg/maintainers.yml

    run git status --porcelain -- pkg/maintainers.yml
    assert_success
    [[ "$output" == '?? pkg/maintainers.yml' ]]

    hk fix -v

    run git status --porcelain -- pkg/maintainers.yml
    assert_success
    [[ "$output" =~ ^A\  ]]
}

@test "stage '**/' alone should not stage anything" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["noop"] {
        fix = "echo done"
        glob = "*"
        stage = "**/"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"

    echo a > a.txt
    echo b > b.txt

    run git status --porcelain -- a.txt b.txt
    assert_success
    [[ "$output" == $'?? a.txt\n?? b.txt' ]]

    hk fix -v

    run git status --porcelain -- a.txt b.txt
    assert_success
    [[ "$output" == $'?? a.txt\n?? b.txt' ]]
}
