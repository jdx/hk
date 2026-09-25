#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "staging fixes to more paths than fit on one command line" {
    cat <<'EOF' > fix.sh
#!/bin/sh
for f; do
    sed s/bad/good/ "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done
EOF
    chmod +x fix.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stage = true
    steps {
      ["fixer"] {
        glob = "**/*.txt"
        check = "! grep -l bad {{files}}"
        fix = "./fix.sh {{files}}"
      }
    }
  }
}
EOF
    git add hk.pkl fix.sh
    git commit -qm init
    # About 2.5 MB of paths, more than Linux allows in one exec's arguments.
    dir="$(printf 'd%.0s' {1..200})"
    mkdir "$dir"
    for i in $(seq 10000); do
        echo bad > "$dir/file_$i.txt"
    done
    git add -A

    run hk run pre-commit
    assert_success
    run git grep --cached -l bad -- "$dir"
    assert_output ""
    run git diff --name-only
    assert_output ""
}
