setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtins tests run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["check"] {
    steps {
      ["newlines"] = Builtins.newlines
    }
  }
}
PKL
    run hk test --step newlines
    assert_success
    assert_output --partial "ok - newlines :: adds newline"
}
