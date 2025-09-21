setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "validate" {
    setup_with_config 'amends "'"$PKL_PATH"'/Config.pkl"
import "'"$PKL_PATH"'/Builtins.pkl"
hooks {
    ["pre-commit"] { steps { ["tsc"] = Builtins.tsc } }
    ["pre-push"] { steps { ["tsc"] = Builtins.tsc } }
    ["fix"] { steps { ["tsc"] = Builtins.tsc } }
    ["check"] { steps { ["tsc"] = Builtins.tsc } }
}'
    assert_hk_success validate
}
