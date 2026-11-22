setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# `lint` passes unless a file contains BAD; `fmt` runs after it and appends
# `$1` to each file.
_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
local linters = new Mapping<String, Step> {
    ["lint"] {
        glob = "*.txt"
        check = "echo ran >> '$BATS_TEST_TMPDIR/lint-runs'; ! grep -l BAD {{files}}"
    }
    ["fmt"] {
        glob = "*.txt"
        depends = "lint"
        fix = "for f in {{files}}; do printf '$1' >> \"\$f\"; done"
    }
}
hooks {
    ["check"] { steps = linters }
    ["fix"] { fix = true; steps = linters }
    ["pre-commit"] { fix = true; steps = linters }
}
EOF
    git add hk.pkl
    git commit -qm "initial commit"
}

_lint_runs() {
    wc -l < "$BATS_TEST_TMPDIR/lint-runs" | tr -d ' '
}

@test "a check that passed runs again when a later fix changes its files, and fails" {
    _config 'BAD\\n'
    echo ok > a.txt
    git add a.txt

    run hk fix
    assert_failure
    assert_output --partial "a.txt"
    assert_equal "$(_lint_runs)" 2
}

@test "the commit is blocked when a fix breaks a check that already passed" {
    _config 'BAD\\n'
    hk install
    echo ok > a.txt
    git add a.txt

    run git commit -m "add a"
    assert_failure
    assert_equal "$(_lint_runs)" 2
    run git log --format=%s -1
    assert_output "initial commit"
}

@test "the check runs again and passes when the fix keeps its files passing" {
    _config 'fine\\n'
    echo ok > a.txt
    git add a.txt

    hk fix
    assert_equal "$(_lint_runs)" 2
    assert_equal "$(cat a.txt)" "$(printf 'ok\nfine')"
}

@test "a fix that leaves the files unchanged does not run the check again" {
    _config ''
    echo ok > a.txt
    git add a.txt

    hk fix
    assert_equal "$(_lint_runs)" 1
}

@test "hk check does not run checks twice" {
    _config 'BAD\\n'
    echo ok > a.txt
    git add a.txt

    hk check
    assert_equal "$(_lint_runs)" 1
}
