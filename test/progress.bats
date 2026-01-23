#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "progress spinner starts with job" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["checker"] {
                glob = List("**/*")
                workspace_indicator = "workspace"
                check = "sleep 1"
            }
        }
    }
}
EOF

    mkdir subdir1 subdir2 subdir3 subdir4
    touch subdir1/workspace subdir2/workspace subdir3/workspace subdir4/workspace


    run hk check --jobs 2 subdir1/workspace subdir2/workspace subdir3/workspace subdir4/workspace
    assert_success
    assert_output --partial '
❯ checker  [                  ] 0/4
  checker – 1 file – **/* – sleep 1
  checker – 1 file – **/* – sleep 1
❯ checker  [========>         ] 2/4
❯ checker  [========>         ] 2/4
  checker – 1 file – **/* – sleep 1
  checker – 1 file – **/* – sleep 1
❯ checker  [=============>    ] 3/4
❯ checker  [==================] 4/4
'
}
