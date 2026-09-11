#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util check-shebang-scripts-are-executable - detects shebang without executable bit" {
    printf "#!/bin/bash\necho hello\n" > script.sh
    chmod 644 script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_failure
    assert_output --partial "script.sh"
    assert_output --partial "chmod +x"
}

@test "util check-shebang-scripts-are-executable - passes executable with shebang" {
    printf "#!/bin/bash\necho hello\n" > script.sh
    chmod +x script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - passes non-executable without shebang" {
    echo "echo hello" > notes.txt
    chmod 644 notes.txt

    run hk util check-shebang-scripts-are-executable notes.txt
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - accepts env shebang" {
    printf "#!/usr/bin/env python3\nprint('hello')\n" > script.py
    chmod 644 script.py

    run hk util check-shebang-scripts-are-executable script.py
    assert_failure
    assert_output --partial "script.py"
}

@test "util check-shebang-scripts-are-executable - detects multiple files" {
    printf "#!/bin/sh\necho one\n" > script1.sh
    printf "#!/bin/sh\necho two\n" > script2.sh
    chmod 644 script1.sh script2.sh

    run hk util check-shebang-scripts-are-executable script1.sh script2.sh
    assert_failure
    assert_output --partial "script1.sh"
    assert_output --partial "script2.sh"
}

@test "util check-shebang-scripts-are-executable - mixed executable and not" {
    printf "#!/bin/sh\necho one\n" > good.sh
    chmod +x good.sh
    printf "#!/bin/sh\necho two\n" > bad.sh
    chmod 644 bad.sh

    run hk util check-shebang-scripts-are-executable good.sh bad.sh
    assert_failure
    assert_output --partial "bad.sh"
    refute_output --partial "good.sh"
}

@test "util check-shebang-scripts-are-executable - passes empty file" {
    : > empty.sh
    chmod 644 empty.sh

    run hk util check-shebang-scripts-are-executable empty.sh
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["shebang-scripts"] = Builtins.check_shebang_scripts_are_executable
        }
    }
}
HK

    printf "#!/bin/bash\necho hello\n" > script.sh
    chmod 644 script.sh
    git add -A

    run hk check --all
    assert_failure
    assert_output --partial "script.sh"
}
