#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "arg escape" {
    [[ "${BATS_DEBUG:-}" = "1" ]] && set -x
    echo "[debug] PATH=$PATH"
    command -v hk && hk --version || echo "[debug] hk not found or --version failed"
    git --version || true
    export NO_COLOR=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    git add hk.pkl
    git status -sb || true
    git commit -m "install hk"
    echo "[debug] running hk install"; hk install; echo "[debug] hk install done"
    echo 'console.log("test")' > '$test.js'
    git add '$test.js'
    run git commit -m "test"
    assert_failure
    assert_output --partial '[warn] $test.js'
}
