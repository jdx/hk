#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# A pre-commit-style fixer: rewrites "bad" to "good" and exits 1 when it
# changed anything, like the hooks `hk migrate pre-commit` generates.
write_precommit_fixer() {
    cat <<'EOF' > fixer.sh
#!/bin/sh
status=0
for f; do
    if grep -q bad "$f"; then
        sed -i.bak 's/bad/good/' "$f" && rm -f "$f.bak"
        status=1
    fi
done
exit $status
EOF
    chmod +x fixer.sh
}

@test "a step whose check and fix are the same command passes after fixing, even alone" {
    write_precommit_fixer
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["fixer"] {
        glob = "*.txt"
        check = "./fixer.sh {{files}}"
        fix = "./fixer.sh {{files}}"
      }
    }
  }
}
EOF
    echo bad > a.txt
    git add -A

    run hk fix --all
    assert_success
    assert_equal "$(cat a.txt)" "good"
}

@test "an overlapping fixer runs only its fix by default" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["a"] {
        glob = "*.txt"
        check = "echo check-a {{files}}"
        fix = "echo fix-a {{files}}"
      }
      ["b"] {
        glob = "*.txt"
        check = "echo check-b {{files}}"
        fix = "echo fix-b {{files}}"
      }
    }
  }
}
EOF
    echo x > a.txt
    git add -A

    HK_LOG=debug run hk fix --all
    assert_success
    assert_output --partial "fix-a a.txt"
    refute_output --partial "check-a a.txt"
}

write_listing_fixer_config() {
    cat <<'EOF' > list.sh
#!/bin/sh
status=0
for f; do
    if grep -q bad "$f"; then
        echo "$f"
        status=1
    fi
done
exit $status
EOF
    chmod +x list.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stage = true
    stash = "$1"
    steps {
      ["lister"] {
        glob = "*.txt"
        check = "./list.sh {{files}}"
        check_list_files = "./list.sh {{files}}"
        fix = "sed -i.bak s/bad/good/ {{files}} && rm -f *.bak"
      }
    }
  }
}
EOF
    # git cannot stash before the first commit.
    git add -A
    git commit -qm init
    echo bad > bad.txt
    echo fine > good.txt
    git add -A
    # An unstaged edit that the step's fix must not stage.
    echo edited >> good.txt
}

@test "a staging hook without a stash narrows a listing step to the files it would change" {
    write_listing_fixer_config none

    HK_LOG=debug run hk run pre-commit
    assert_success
    assert_output --partial "DEBUG $ ./list.sh"
    assert_output --partial "DEBUG $ sed -i.bak s/bad/good/ bad.txt"
    refute_output --partial "s/bad/good/ bad.txt good.txt"
    assert_equal "$(git show :bad.txt)" "good"
    assert_equal "$(git show :good.txt)" "fine"
}

@test "a staging hook that stashes fixes a listing step's files directly" {
    write_listing_fixer_config git

    HK_LOG=debug run hk run pre-commit
    assert_success
    refute_output --partial "DEBUG $ ./list.sh"
    assert_output --partial "DEBUG $ sed -i.bak s/bad/good/ bad.txt good.txt"
    assert_equal "$(git show :bad.txt)" "good"
    assert_equal "$(git show :good.txt)" "fine"
    assert_equal "$(cat good.txt)" $'fine\nedited'
}

@test "files fixed by a same-command step whose check passes are staged" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stage = true
    steps {
      // Fixes and exits 0, so the check hk runs first already did the fix.
      ["fixer"] {
        glob = "*.txt"
        check = "sed -i.bak s/bad/good/ {{files}} && rm -f *.bak"
        fix = "sed -i.bak s/bad/good/ {{files}} && rm -f *.bak"
      }
    }
  }
}
EOF2
    echo bad > a.txt
    git add -A

    run hk run pre-commit
    assert_success
    assert_equal "$(cat a.txt)" "good"
    assert_equal "$(git show :a.txt)" "good"
}

@test "a skipped same-command step stages nothing" {
    write_precommit_fixer
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stage = true
    stash = "none"
    steps {
      ["fixer"] {
        glob = "*.txt"
        condition = "false"
        check = "./fixer.sh {{files}}"
        fix = "./fixer.sh {{files}}"
      }
    }
  }
}
EOF2
    echo fine > a.txt
    git add -A
    echo "unstaged edit" >> a.txt

    run hk run pre-commit
    assert_success
    assert_equal "$(git show :a.txt)" "fine"
}
