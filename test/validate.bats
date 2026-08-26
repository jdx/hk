setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "validate" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] { steps { ["newlines"] = Builtins.newlines } }
    ["pre-push"] { steps { ["newlines"] = Builtins.newlines } }
    ["fix"] { steps { ["newlines"] = Builtins.newlines } }
    ["check"] { steps { ["newlines"] = Builtins.newlines } }
}
EOF
    hk validate
}
