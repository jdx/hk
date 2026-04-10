#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "hook_args is empty for hooks without a dedicated handler" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-applypatch"] {
        steps {
            ["capture"] { check = "echo '{{ hook_args }}' > hook_args.txt" }
        }
    }
}
EOF
    echo "a" > a.txt && git add a.txt && git commit -m "init"
    run hk run post-applypatch
    assert_success
    run cat hook_args.txt
    assert_output ""
}

@test "post-checkout hook_args contains refs and branch flag" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-checkout"] {
        steps {
            ["capture"] { check = "echo {{ hook_args }} > hook_args.txt" }
        }
    }
}
EOF
    hk install
    echo "test" > test.txt && git add test.txt && git commit -m "init"
    git checkout -b feature
    run cat hook_args.txt
    assert_output --regexp "^[a-f0-9]+ [a-f0-9]+ 1$"
}

@test "post-checkout hook_args works with git-lfs" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-checkout"] {
        steps {
            ["git-lfs"] { check = "git lfs post-checkout {{ hook_args }}" }
        }
    }
}
EOF
    echo "*.bin filter=lfs diff=lfs merge=lfs -text" > .gitattributes
    git lfs install --local
    hk install
    dd if=/dev/urandom bs=1024 count=1 of=test.bin 2>/dev/null
    git add .gitattributes test.bin && git commit -m "init with lfs"
    run git checkout -b feature
    assert_success
}

@test "post-merge hook_args contains squash flag" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-merge"] {
        steps {
            ["capture"] { check = "echo {{ hook_args }} > hook_args.txt" }
        }
    }
}
EOF
    hk install
    echo "a" > a.txt && git add a.txt && git commit -m "init"
    git checkout -b feature
    echo "b" > b.txt && git add b.txt && git commit -m "feature"
    git checkout main
    echo "c" > c.txt && git add c.txt && git commit -m "main"
    git checkout feature
    git merge main
    run cat hook_args.txt
    assert_output "0"
}

@test "post-merge hook_args works with git-lfs" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-merge"] {
        steps {
            ["git-lfs"] { check = "git lfs post-merge {{ hook_args }}" }
        }
    }
}
EOF
    echo "*.bin filter=lfs diff=lfs merge=lfs -text" > .gitattributes
    git lfs install --local
    hk install
    dd if=/dev/urandom bs=1024 count=1 of=test.bin 2>/dev/null
    git add .gitattributes test.bin && git commit -m "init with lfs"
    git checkout -b feature
    echo "x" > x.txt && git add x.txt && git commit -m "feature"
    git checkout main
    echo "y" > y.txt && git add y.txt && git commit -m "main"
    git checkout feature
    run git merge main
    assert_success
}

@test "post-rewrite hook_args contains amend" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-rewrite"] {
        steps {
            ["capture"] { check = "echo {{ hook_args }} > hook_args.txt" }
        }
    }
}
EOF
    hk install
    echo "a" > a.txt && git add a.txt && git commit -m "init"
    git commit --amend -m "amended"
    run cat hook_args.txt
    assert_output "amend"
}

@test "post-rewrite hook_args contains rebase" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-rewrite"] {
        steps {
            ["capture"] { check = "echo {{ hook_args }} > hook_args.txt" }
        }
    }
}
EOF
    hk install
    echo "a" > a.txt && git add a.txt && git commit -m "init"
    git checkout -b feature
    echo "b" > b.txt && git add b.txt && git commit -m "feature"
    git checkout main
    echo "c" > c.txt && git add c.txt && git commit -m "main"
    git checkout feature
    git rebase main
    run cat hook_args.txt
    assert_output "rebase"
}

@test "pre-push hook_args contains remote and url" {
    if [ "$HK_LIBGIT2" = "0" ]; then
        skip "libgit2 is not installed"
    fi
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-push"] {
        steps {
            ["capture"] { check = "echo {{ hook_args }} > hook_args.txt" }
        }
    }
}
EOF
    echo "a" > a.txt && git add a.txt && git commit -m "init"
    git init --bare ../remote.git
    git remote add origin ../remote.git
    git push -u origin main
    hk install
    echo "b" > b.txt && git add b.txt && git commit -m "second"
    git push origin main
    run cat hook_args.txt
    assert_output --partial "origin"
}

@test "pre-push hook_args works with git-lfs" {
    if [ "$HK_LIBGIT2" = "0" ]; then
        skip "libgit2 is not installed"
    fi
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-push"] {
        steps {
            ["git-lfs"] { check = "git lfs pre-push {{ hook_args }}" }
        }
    }
}
EOF
    echo "*.bin filter=lfs diff=lfs merge=lfs -text" > .gitattributes
    git lfs install --local
    dd if=/dev/urandom bs=1024 count=1 of=test.bin 2>/dev/null
    git add .gitattributes test.bin && git commit -m "init with lfs"
    git init --bare ../lfs-remote.git
    git remote add origin ../lfs-remote.git
    git push -u origin main
    hk install
    dd if=/dev/urandom bs=1024 count=1 of=test2.bin 2>/dev/null
    git add test2.bin && git commit -m "second lfs file"
    run git push origin main
    assert_success
}

@test "commit-msg hook_args contains commit message file path" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["commit-msg"] {
        steps {
            ["capture"] { check = "echo {{ hook_args }} > hook_args.txt" }
        }
    }
}
EOF
    hk install
    echo "a" > a.txt && git add a.txt
    git commit -m "init"
    run cat hook_args.txt
    assert_output --partial "COMMIT_EDITMSG"
}

@test "pre-commit hook_args is empty" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["capture"] {
                glob = "**/*"
                check = "echo '{{ hook_args }}' > hook_args.txt"
            }
        }
    }
}
EOF
    hk install
    echo "a" > a.txt && git add a.txt
    git commit -m "init"
    run cat hook_args.txt
    assert_output ""
}
