#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = read?("env:FIX_STAGE")?.toBoolean() ?? null
        steps {
            ["trailing-whitespace"] = Builtins.trailing_whitespace
        }
    }
}
EOF
    touch file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit"
}

teardown() {
    _common_teardown
}

@test "stages by default" {
    echo "content  " > file.txt

    hk run fix

    run git status --porcelain
    assert_success
    assert_output 'M  file.txt'
}

@test "disabled in hook config" {
    echo "content  " > file.txt

    FIX_STAGE=false hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "disabled in config" {
    echo "stage = false" >> hk.pkl
    git commit -am "disabling stage in config"

    echo "content  " > file.txt

    hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "disabled in user config" {
    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

stage = false
EOF
    echo ".hkrc.pkl" > .git/info/exclude

    echo "content  " > file.txt

    hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "disabled in git config" {
    git config hk.stage false
    echo "content  " > file.txt

    hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "disabled in envvar" {
    echo "content  " > file.txt

    HK_STAGE=0 hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "disabled on CLI" {
    echo "content  " > file.txt

    hk run -v fix --no-stage

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "CLI enable overrides env disable" {
    echo "content  " > file.txt

    HK_STAGE=0 hk run -v fix --stage

    run git status --porcelain
    assert_success
    assert_output 'M  file.txt'
}

@test "CLI enable overrides hook disable" {
    echo "content  " > file.txt

    FIX_STAGE=false hk run -v fix --stage

    run git status --porcelain
    assert_success
    assert_output 'M  file.txt'
}

# This case is a bit weird. Intuitively you'd think hook config would win out,
# but root config values are akin to CLI/Env in that they are "global".
@test "config disable overrides hook config disable" {
    echo "stage = true" >> hk.pkl
    git commit -am "disabling stage in config"
    echo "content  " > file.txt

    FIX_STAGE=false hk run fix

    run git status --porcelain
    assert_success
    assert_output 'M  file.txt'
}

@test "env var enable overrides hook config disable" {
    echo "content  " > file.txt

    HK_STAGE=1 FIX_STAGE=false hk run fix

    run git status --porcelain
    assert_success
    assert_output 'M  file.txt'
}
