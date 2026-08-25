#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    # shellcheck is provided by a mise tool-stub
    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
}

teardown() {
    _common_teardown
}

write_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["shellcheck"] = Builtins.shellcheck()
        }
    }
}
EOF
}

@test "shellcheck fix applies autofixable findings and still reports the rest" {
    write_config
    # SC2006 (backticks) is autofixable; SC2034 (unused variable) is not.
    printf '#!/bin/sh\nunused=value\nx=`date`\necho "$x"\n' > mixed.sh
    git add .
    git commit -m init

    run hk fix --all
    # The leftover SC2034 must fail the step rather than being masked by the
    # successful patch application.
    assert_failure
    assert_output --partial "SC2034"

    # The autofixable finding was still applied.
    run cat mixed.sh
    assert_output '#!/bin/sh
unused=value
x=$(date)
echo "$x"'
}

@test "shellcheck fix rewrites a fully autofixable file and succeeds" {
    write_config
    printf '#!/bin/sh\nx=`date`\necho "$x"\n' > auto.sh
    git add .
    git commit -m init

    run hk fix --all
    assert_success

    run cat auto.sh
    assert_output '#!/bin/sh
x=$(date)
echo "$x"'
}

@test "shellcheck fix leaves a file with no autofixable findings unchanged and fails" {
    write_config
    printf '#!/bin/sh\nunused=value\n' > only_unfixable.sh
    git add .
    git commit -m init

    run hk fix --all
    assert_failure
    assert_output --partial "SC2034"

    run cat only_unfixable.sh
    assert_output '#!/bin/sh
unused=value'
}
