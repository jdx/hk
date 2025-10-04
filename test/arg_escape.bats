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
    # Provide local mise config so hk install doesn't try to discover tools
    cat <<'MTOML' > mise.toml
    [tools]
    "npm:prettier" = "latest"
    pkl = "latest"
MTOML
    # Ensure tools are ready in this repo; should be a quick no-op after global install
    mise install || true
    git add hk.pkl
    git status -sb || true
    git commit -m "install hk" --no-verify
    echo "[debug] hooks before install:"; ls -la .git/hooks || true
    echo "[debug] running hk install (with timeout + tracing)"
    if command -v strace >/dev/null 2>&1; then
        HK_LOG_LEVEL=trace HK_TRACE=1 GIT_TRACE=1 GIT_CURL_VERBOSE=1 run timeout 1s strace -f -tt -s 256 -o strace.log hk install
        echo "[debug] strace captured to: $PWD/strace.log"
    else
        HK_LOG_LEVEL=trace HK_TRACE=1 GIT_TRACE=1 GIT_CURL_VERBOSE=1 run timeout 1s hk install
    fi
    echo "[debug] hk install status=$status"
    echo "[debug] hk install output:\n$output"
    if [ -f strace.log ]; then
        echo "[debug] tail strace.log:"; tail -n 200 strace.log || true
    fi
    echo "[debug] hooks after install:"; ls -la .git/hooks || true
    echo 'console.log("test")' > '$test.js'
    git add '$test.js'
    run git commit -m "test"
    assert_failure
    assert_output --partial '[warn] $test.js'
}
