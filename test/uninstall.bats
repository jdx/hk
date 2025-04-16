setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "uninstall" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] { steps { ["tsc"] = Builtins.tsc } }
    ["pre-push"] { steps { ["tsc"] = Builtins.tsc } }
    ["fix"] { steps { ["tsc"] = Builtins.tsc } }
    ["check"] { steps { ["tsc"] = Builtins.tsc } }
}
EOF
    rm -f .git/hooks/*
    hk install
    assert_file_exists .git/hooks/pre-commit
    assert_file_exists .git/hooks/pre-push
    assert_file_not_exists .git/hooks/fix
    assert_file_not_exists .git/hooks/check
    hk uninstall
    assert_file_not_exists .git/hooks/pre-commit
    assert_file_not_exists .git/hooks/pre-push
}
