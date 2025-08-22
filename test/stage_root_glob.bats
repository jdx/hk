#!/usr/bin/env bats

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
