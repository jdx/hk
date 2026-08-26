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
    ["pre-commit"] {
        fix = true
        stage = read?("env:PRE_COMMIT_STAGE")?.toBoolean() ?? null
        stash = "none"
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

@test "non-pre-commit hooks do not stage by default" {
    echo "content  " > file.txt

    hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "pre-commit stages by default" {
    echo "content  " > file.txt
    git add file.txt

    hk run pre-commit

    run git status --porcelain
    assert_success
    assert_output 'M  file.txt'
}

@test "disabled in hook config" {
    echo "content  " > file.txt
    git add file.txt

    PRE_COMMIT_STAGE=false hk run pre-commit

    run git status --porcelain
    assert_success
    assert_output 'MM file.txt'
}

@test "disabled in config" {
    echo "stage = false" >> hk.pkl
    git commit -am "disabling stage in config"

    echo "content  " > file.txt
    git add file.txt

    hk run pre-commit

    run git status --porcelain
    assert_success
    assert_output 'MM file.txt'
}

@test "disabled in XDG config" {
    mkdir -p "$HOME/.config/hk"
    cat <<EOF > "$HOME/.config/hk/config.pkl"
amends "$PKL_PATH/Config.pkl"
stage = false
EOF

    echo "content  " > file.txt
    git add file.txt

    hk run pre-commit

    run git status --porcelain
    assert_success
    assert_output 'MM file.txt'
}

@test "disabled in git config" {
    git config hk.stage false
    echo "content  " > file.txt
    git add file.txt

    hk run pre-commit

    run git status --porcelain
    assert_success
    assert_output 'MM file.txt'
}

@test "disabled in envvar" {
    echo "content  " > file.txt
    git add file.txt

    HK_STAGE=0 hk run pre-commit

    run git status --porcelain
    assert_success
    assert_output 'MM file.txt'
}

@test "disabled on CLI" {
    echo "content  " > file.txt
    git add file.txt

    hk run -v pre-commit --no-stage

    run git status --porcelain
    assert_success
    assert_output 'MM file.txt'
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
