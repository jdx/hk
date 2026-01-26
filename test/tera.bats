#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "tera vars" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step"] {
                check = "echo '{{ __tera_context }}'"
            }
        }
    }
}
EOF
    echo "content" > file.txt

    run hk check
    assert_success
    assert_output --partial "\"hook\": \"check\""
    assert_output --partial "\"step\": \"step\""
}
