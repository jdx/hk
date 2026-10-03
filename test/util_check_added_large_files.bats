#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

_write_config() {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["large-files"] = Builtins.check_added_large_files
        }
    }
}
HK
}

@test "check_added_large_files builtin - fails on a large file containing NUL bytes" {
    _write_config
    # 501 KiB of NUL bytes: hk's default binary filter would drop this file.
    dd if=/dev/zero of=big.bin bs=1024 count=501 2>/dev/null
    git add -A

    run hk check
    assert_failure
    assert_output --partial "big.bin"
}

@test "check_added_large_files builtin - fails on a large text file" {
    _write_config
    yes "some text line" | head -c 513000 > big.txt
    git add -A

    run hk check
    assert_failure
    assert_output --partial "big.txt"
}

@test "check_added_large_files builtin - passes on a small binary file" {
    _write_config
    dd if=/dev/zero of=small.bin bs=1024 count=1 2>/dev/null
    git add -A

    run hk check
    assert_success
}
