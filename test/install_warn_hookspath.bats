#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk install warns when global core.hooksPath is set" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF

    # Set global core.hooksPath
    git config --global core.hooksPath "/some/global/hooks/path"

    # Run hk install and capture stderr
    run hk install

    # Should succeed
    assert_success

    # Should warn about global hooksPath
    assert_output --partial "core.hooksPath is set globally to '/some/global/hooks/path'"
    assert_output --partial "git config --global --unset-all core.hooksPath"

    # Clean up global config
    git config --global --unset core.hooksPath
}

@test "hk install warns when local core.hooksPath is set" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF

    # Set local core.hooksPath
    git config --local core.hooksPath "/some/local/hooks/path"

    # Run hk install and capture stderr
    run hk install

    # Should succeed
    assert_success

    # Should warn about local hooksPath
    assert_output --partial "core.hooksPath is set locally to '/some/local/hooks/path'"
    assert_output --partial "git config --local --unset-all core.hooksPath"

    # Clean up local config
    git config --local --unset core.hooksPath
}

@test "hk install warns when both global and local core.hooksPath are set" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF

    # Set both global and local core.hooksPath
    git config --global core.hooksPath "/some/global/hooks/path"
    git config --local core.hooksPath "/some/local/hooks/path"

    # Run hk install and capture stderr
    run hk install

    # Should succeed
    assert_success

    # Should warn about both
    assert_output --partial "core.hooksPath is set globally to '/some/global/hooks/path'"
    assert_output --partial "git config --global --unset-all core.hooksPath"
    assert_output --partial "core.hooksPath is set locally to '/some/local/hooks/path'"
    assert_output --partial "git config --local --unset-all core.hooksPath"

    # Clean up configs
    git config --global --unset core.hooksPath
    git config --local --unset core.hooksPath
}

@test "hk install does not warn when core.hooksPath is not set" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF

    # Make sure core.hooksPath is not set
    git config --global --unset core.hooksPath 2>/dev/null || true
    git config --local --unset core.hooksPath 2>/dev/null || true

    # Run hk install
    run hk install

    # Should succeed
    assert_success

    # Should not warn about hooksPath
    refute_output --partial "core.hooksPath"
}
