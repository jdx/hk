#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

write_capture_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["capture"] {
                check = "printf '%s\\n' {{files}} | sort > seen.txt"
            }
        }
    }
}
EOF
}

@test "--cd runs hk from the selected project" {
    mkdir nested
    cd nested
    git init .
    write_capture_config
    touch selected.txt ignored.txt
    git add .
    git commit -m init
    cd ..

    run hk --cd nested check selected.txt
    assert_success

    run cat nested/seen.txt
    assert_output "selected.txt"
}

@test "--cd reports an invalid directory" {
    run hk --cd missing check --all
    assert_failure
    assert_output --partial "missing"
}

@test "--files0-from reads exact paths including spaces" {
    write_capture_config
    touch "with space.txt" other.txt
    git add .
    git commit -m init
    printf 'with space.txt\0' > files.list

    run hk check --files0-from files.list
    assert_success

    run cat seen.txt
    assert_output "with space.txt"
}

@test "--files0-from accepts stdin" {
    write_capture_config
    touch first.txt second.txt
    git add .
    git commit -m init

    run bash -c "printf 'second.txt\\0' | hk check --files0-from -"
    assert_success

    run cat seen.txt
    assert_output "second.txt"
}

@test "--files0-from conflicts with other selection modes" {
    write_capture_config
    git add hk.pkl
    git commit -m init
    : > files.list

    run hk check --files0-from files.list --all
    assert_failure
    assert_output --partial "cannot be used with"
}

@test "--files0-from stdin is rejected for hooks that own stdin" {
    run bash -c "printf 'file.txt\\0' | hk run post-rewrite amend --files0-from -"
    assert_failure
    assert_output --partial "post-rewrite"
    assert_output --partial "reads rewrite data from stdin"
}
