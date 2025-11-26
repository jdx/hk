#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "git status AD: staged-added but deleted in worktree should not be included in files list" {
    # Force shell git path to reproduce porcelain parsing behavior
    export HK_LIBGIT2=0
    export NO_COLOR=1

    mkdir -p ml/py

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["list"] {
        // capture the list of files hk passes to the step
        check = "printf '%s\n' {{files}} > files_list.txt"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"

    # Create a staged file that exists so the hook runs
    echo "print('ok')" > ml/py/b.py
    git add ml/py/b.py

    # Create a file that is added to index, then remove it to produce AD
    echo "print('temp')" > ml/py/a.py
    git add ml/py/a.py
    rm ml/py/a.py

    # Sanity: ensure git shows AD for a.py
    run bash -lc "git status --porcelain --untracked-files=all | tr -d '\0'"
    assert_success
    assert_output --partial "AD ml/py/a.py"

    # Run pre-commit; 'list' step will write files_list.txt with the files hk selected
    run hk run pre-commit
    # Whether hk succeeds depends on tool config; we only care that it ran

    assert_file_exists files_list.txt
    # The deleted file should NOT be included in the files list
    assert_file_not_contains files_list.txt "ml/py/a.py"
}

