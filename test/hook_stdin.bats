#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "post-rewrite hook_stdin contains old and new SHAs on amend" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-rewrite"] {
        steps {
            ["capture"] {
                check = "cat > hook_stdin.txt"
                stdin = "{{ hook_stdin }}"
            }
        }
    }
}
EOF
    hk install
    echo "a" > a.txt && git add a.txt && git commit -m "init"
    OLD_SHA=$(git rev-parse HEAD)
    git commit --amend -m "amended"
    NEW_SHA=$(git rev-parse HEAD)
    run cat hook_stdin.txt
    assert_output --partial "$OLD_SHA"
    assert_output --partial "$NEW_SHA"
}

@test "pre-push hook_stdin forwards git ref data to steps" {
    if [ "$HK_LIBGIT2" = "0" ]; then
        skip "libgit2 is not installed"
    fi
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-push"] {
        steps {
            ["capture"] {
                check = "cat > hook_stdin.txt"
                stdin = "{{ hook_stdin }}"
            }
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
    run cat hook_stdin.txt
    assert_output --regexp "refs/heads/main [a-f0-9]+ refs/heads/main [a-f0-9]+"
}

@test "pre-push hook_stdin works with git-lfs" {
    if [ "$HK_LIBGIT2" = "0" ]; then
        skip "libgit2 is not installed"
    fi
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-push"] {
        steps {
            ["git-lfs"] {
                check = "git lfs pre-push {{ hook_args }}"
                stdin = "{{ hook_stdin }}"
            }
        }
    }
}
EOF
    echo "*.bin filter=lfs diff=lfs merge=lfs -text" > .gitattributes
    git lfs install --local
    git config lfs.standalonetransferagent lfs-standalone-file
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
    # Verify LFS objects were actually transferred to the remote
    local remote_lfs_count
    remote_lfs_count=$(find ../lfs-remote.git/lfs/objects -type f 2>/dev/null | wc -l | tr -d ' ')
    [ "$remote_lfs_count" -eq 2 ]
}
