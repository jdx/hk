#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "check mode with check_diff still reports check errors" {
    # Mock linter: --diff exits 0 (no auto-fixable issues), but regular check exits 1
    mkdir -p bin
    cat <<'SCRIPT' > bin/mock-linter
#!/usr/bin/env bash
case "$1" in
    --diff) exit 0 ;;
    *)      echo "error: non-auto-fixable violation" >&2; exit 1 ;;
esac
SCRIPT
    chmod +x bin/mock-linter
    export PATH="$PWD/bin:$PATH"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = List("*.txt")
                check = "mock-linter"
                check_diff = "mock-linter --diff"
            }
        }
    }
}
EOF
    git add -A
    git commit -m "init"

    echo "hello" > test.txt
    git add test.txt

    run hk check
    assert_failure
}
