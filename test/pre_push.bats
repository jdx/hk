#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    TEST_REPO_DIR="$(temp_make)"
    pushd "$TEST_REPO_DIR"
    git init --bare
    popd
    git remote add origin "$TEST_REPO_DIR"
}
teardown() {
    _common_teardown
    chmod -R u+w "$TEST_REPO_DIR"
    temp_del "$TEST_REPO_DIR"
}

@test "pre-push hook" {
    export NO_COLOR=1
    if [ "$HK_LIBGIT2" = "0" ]; then
        skip "libgit2 is not installed"
    fi
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-push"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    git add hk.pkl
    git commit -m "install hk"
    git push origin main
    # Use legacy shim mode: config-based pre-push hooks have different env/cwd
    # semantics that would need separate test coverage.
    hk install --legacy
    echo 'console.log("test")' > test.js
    git add test.js
    git commit -m "test"
    HK_LOG=trace run git push origin main
    assert_failure
    assert_output --partial "[warn] test.js"
}

@test "pre-push hook on new branch first push" {
    export NO_COLOR=1
    if [ "$HK_LIBGIT2" = "0" ]; then
        skip "libgit2 is not installed"
    fi
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-push"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    git add hk.pkl
    git commit -m "install hk"
    git push origin main
    hk install --legacy

    # Create a new branch with a file that needs linting and push it for the
    # first time. The pre-push hook receives a remote sha of all-zeros for a
    # new branch — the inverted filter regression caused this push to either
    # error out resolving refs/remotes/origin/HEAD or skip linting entirely.
    git checkout -b feature/new-thing
    echo 'console.log("new")' > new.js
    git add new.js
    git commit -m "add new.js"
    HK_LOG=trace run git push -u origin feature/new-thing
    assert_failure
    assert_output --partial "[warn] new.js"
}

@test "pre-push hook skips branch deletion" {
    export NO_COLOR=1
    if [ "$HK_LIBGIT2" = "0" ]; then
        skip "libgit2 is not installed"
    fi
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-push"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    git add hk.pkl
    git commit -m "install hk"
    git push origin main

    # Push a branch we'll later delete — install the hook only after this
    # setup push so the initial push isn't gated on linting.
    git checkout -b feature/to-delete
    echo 'console.log("x")' > x.js
    git add x.js
    git commit -m "add x.js"
    git push -u origin feature/to-delete
    git checkout main

    hk install --legacy

    # Add a file on main that WOULD fail linting if linted. This guards
    # against a regression where deletions slip past the EMPTY_REF guard
    # and end up running files_between_refs(default_branch, HEAD) — that
    # diff would include this file and trigger a lint failure.
    echo 'console.log("unformatted")' > unrelated.js
    git add unrelated.js
    git -c core.hooksPath=/dev/null commit -m "unrelated change on main"

    # Deleting a remote branch should not lint anything — the EMPTY_REF
    # guard in hook.rs short-circuits when to_ref is the all-zeros sha.
    run git push origin --delete feature/to-delete
    assert_success
    refute_output --partial "[warn] unrelated.js"
}
