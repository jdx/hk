#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util check-executables-have-shebangs - detects executable without shebang" {
    echo "echo hello" > script.sh
    chmod +x script.sh

    run hk util check-executables-have-shebangs script.sh
    assert_failure
    assert_output "script.sh"
}

@test "util check-executables-have-shebangs - passes executable with shebang" {
    printf "#!/bin/bash\necho hello\n" > script.sh
    chmod +x script.sh

    run hk util check-executables-have-shebangs script.sh
    assert_success
    refute_output
}

@test "util check-executables-have-shebangs - passes non-executable without shebang" {
    echo "echo hello" > script.sh
    chmod 644 script.sh

    run hk util check-executables-have-shebangs script.sh
    assert_success
    refute_output
}

@test "util check-executables-have-shebangs - detects multiple executables" {
    echo "echo hello" > script1.sh
    chmod +x script1.sh
    echo "echo world" > script2.sh
    chmod +x script2.sh

    run hk util check-executables-have-shebangs script1.sh script2.sh
    assert_failure
    assert_output --partial "script1.sh"
    assert_output --partial "script2.sh"
}

@test "util check-executables-have-shebangs - accepts env shebang" {
    printf "#!/usr/bin/env python\nprint('hello')\n" > script.py
    chmod +x script.py

    run hk util check-executables-have-shebangs script.py
    assert_success
    refute_output
}

@test "util check-executables-have-shebangs - skips binary files" {
    # Create a fake binary file (starts with ELF magic)
    printf "\x7fELF\x02\x01\x01\x00binary data" > binary
    chmod +x binary

    run hk util check-executables-have-shebangs binary
    assert_success
    refute_output
}

@test "util check-executables-have-shebangs - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["executable-shebangs"] = Builtins.check_executables_have_shebangs
        }
    }
}
HK

    echo "echo hello" > test.sh
    chmod +x test.sh

    run hk check
    assert_failure
    assert_output --partial "test.sh"
}
