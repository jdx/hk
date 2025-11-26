#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "pre-commit with cargo-fmt does not stage Cargo.toml" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            // Simulate cargo-fmt without requiring Rust toolchain
            ["fake-fmt"] {
                glob = "*.rs"
                stage = "*.rs"
                workspace_indicator = "Cargo.toml"
                // Overwrite the file with a formatted version
                fix = "printf 'pub fn add(a: i32, b: i32) -> i32 { a + b }\n' > src/lib.rs"
            }
        }
    }
}
EOF

    # Minimal Rust workspace
    cat <<EOF > Cargo.toml
[package]
name = "tmp"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
EOF
    mkdir -p src
    # Initial unformatted content
    cat <<'EOF' > src/lib.rs
pub fn add(a:i32,b:i32)->i32{a+b}
EOF

    touch other.txt

    git add hk.pkl Cargo.toml
    git commit -m "init"
    hk install

    # Stage only src changes, not Cargo.toml
    git add src other.txt

    # Introduce an unstaged change on the same line rustfmt will touch
    # to exercise the stash/apply path
    cat <<'EOF' > src/lib.rs
pub fn add(a:i32,b:i32)->i32{a+b} // unstaged-change
EOF

    # introduce an unstaged change to Cargo.toml
    echo "# unstaged-change" >> Cargo.toml
    echo "# untracked-file" > untracked.txt
    echo "# unstaged-file" > other.unstaged

    # Run the hook
    hk run pre-commit -v

    # Verify the Rust file is staged (formatted or restaged by the step)
    run git diff --name-only --cached
    assert_success
    assert_output --partial "src/lib.rs"

    # Critically, Cargo.toml must not be staged
    refute_output --partial "Cargo.toml"

    # And other.txt should be staged
    assert_output --partial "other.txt"

    # And Cargo.toml should not be modified in the working tree either
    run git status --porcelain --untracked-files=all
    assert_success
    assert_output --partial " M Cargo.toml"
    assert_output --partial "A  other.txt"
    assert_output --partial "?? other.unstaged"
    assert_output --partial "?? untracked.txt"
}

