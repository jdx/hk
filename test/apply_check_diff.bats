#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "check_diff applies diff directly instead of running fixer" {
    # Create a simple "formatter" that outputs a unified diff
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
# When called with --diff, output a unified diff that adds a newline
for file in "$@"; do
    if [[ "$file" != "--diff" && "$file" != "--check" ]]; then
        content=$(cat "$file")
        if [[ "$content" != *$'\n' ]]; then
            echo "--- a/$file"
            echo "+++ b/$file"
            echo "@@ -1 +1 @@"
            echo "-$content"
            echo "\\ No newline at end of file"
            echo "+$content"
            exit 1  # Non-zero = needs fixing
        fi
    fi
done
exit 0  # All files OK
SCRIPT
    chmod +x formatter.sh

    # Create a fixer that would do something DIFFERENT (add "FIXED:" prefix)
    # This lets us verify the diff was applied, not the fixer
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
for file in "$@"; do
    echo "FIXED:$(cat "$file")" > "$file"
done
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh --diff {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    # Create a file without trailing newline
    printf "hello" > test.txt

    # Run fix
    run hk fix test.txt
    assert_success

    # The file should have a newline added (from the diff), NOT "FIXED:" prefix
    # If the apply worked, content is "hello\n"
    # If fixer ran, content would be "FIXED:hello"
    run cat test.txt
    assert_output "hello"  # With newline from diff

    # Verify it does NOT have the FIXED prefix (which would mean fixer ran)
    run grep -c "FIXED:" test.txt
    assert_failure
}

@test "check_diff falls back to fixer when apply fails" {
    # Create a "formatter" that outputs invalid diff
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
echo "this is not a valid diff format"
exit 1
SCRIPT
    chmod +x formatter.sh

    # Create a fixer that adds a marker
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
for file in "$@"; do
    echo "FIXED" >> "$file"
done
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    echo "hello" > test.txt

    # Run fix - should fall back to fixer since diff is invalid
    run env HK_LOG_LEVEL=warn hk fix test.txt
    assert_success
    refute_output --partial "cannot safely apply diff"

    # The fixer should have run and added "FIXED"
    run cat test.txt
    assert_output "hello
FIXED"
}

@test "check_diff applies diff when command exits nonzero with valid diff" {
    # Some tools like ruff, black, shfmt exit nonzero when files need changes
    # but still output a valid diff that can be applied
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
# Output a valid diff and exit nonzero to indicate changes needed
file="$1"
if [ -f "$file" ]; then
    content=$(cat "$file")
    # Output a diff that changes "old" to "new"
    echo "--- a/$file"
    echo "+++ b/$file"
    echo "@@ -1 +1 @@"
    echo "-$content"
    echo "+modified"
fi
exit 1  # Nonzero = changes needed
SCRIPT
    chmod +x formatter.sh

    # Fixer that adds different content to verify diff was applied instead
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "FIXER_RAN" > "$1"
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    echo "original" > test.txt

    run hk fix test.txt
    assert_success

    # Verify the diff was applied (content should be "modified")
    # NOT "FIXER_RAN" which would indicate the fixer ran instead
    run cat test.txt
    assert_output "modified"
}

@test "check_diff does not modify files during check mode" {
    # Regression test: check mode should be read-only
    # Even with check_diff defined, files should not be modified during `hk check`
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
if [ -f "$file" ]; then
    echo "--- a/$file"
    echo "+++ b/$file"
    echo "@@ -1 +1 @@"
    echo "-$(cat "$file")"
    echo "+modified_content"
fi
exit 1  # Indicates files need changes
SCRIPT
    chmod +x formatter.sh

    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "FIXER_RAN" > "$1"
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    echo "original" > test.txt

    # Run CHECK (not fix) - should fail but NOT modify the file
    run hk check test.txt
    assert_failure

    # File should be unchanged - neither diff applied nor fixer ran
    run cat test.txt
    assert_output "original"
}

@test "check_diff handles diff output mixed with extra diagnostic text" {
    # Some tools like ruff output diagnostic information alongside the diff
    # e.g., "Would reformat: file.py" or fix summaries
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
if [ -f "$file" ]; then
    # Output extra diagnostic text before and after the diff
    echo "Checking $file..."
    echo "Found 1 issue"
    echo ""
    echo "--- a/$file"
    echo "+++ b/$file"
    echo "@@ -1 +1 @@"
    echo "-$(cat "$file")"
    echo "+fixed_content"
    echo ""
    echo "Would reformat 1 file"
    echo "Done."
fi
exit 1
SCRIPT
    chmod +x formatter.sh

    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "FIXER_RAN" > "$1"
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    echo "original" > test.txt

    run hk fix test.txt
    assert_success

    # Verify the diff was applied despite extra output
    run cat test.txt
    assert_output "fixed_content"
}

@test "check_diff handles diffs without a/b prefixes" {
    # Some tools output diffs without the a/ and b/ prefixes
    # e.g., "--- src/file.py" instead of "--- a/src/file.py"
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
if [ -f "$file" ]; then
    # Output diff WITHOUT a/ and b/ prefixes
    echo "--- $file"
    echo "+++ $file"
    echo "@@ -1 +1 @@"
    echo "-$(cat "$file")"
    echo "+no_prefix_diff"
fi
exit 1
SCRIPT
    chmod +x formatter.sh

    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "FIXER_RAN" > "$1"
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    echo "original" > test.txt
    run hk fix test.txt
    assert_success

    # Verify the diff was applied (uses -p0 since no a/b prefixes)
    run cat test.txt
    assert_output "no_prefix_diff"
}

@test "check_diff handles diffs with .orig suffix on --- line" {
    # Go tools like gofmt output diffs with .orig suffix on the --- line
    # e.g., "--- file.go.orig" instead of "--- file.go"
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
if [ -f "$file" ]; then
    # Output diff with .orig suffix (like gofmt -d)
    echo "--- $file.orig"
    echo "+++ $file"
    echo "@@ -1 +1 @@"
    echo "-$(cat "$file")"
    echo "+gofmt_fixed"
fi
exit 1
SCRIPT
    chmod +x formatter.sh

    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "FIXER_RAN" > "$1"
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("*.go")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    echo "original" > test.go

    run hk fix test.go
    assert_success

    # Verify the diff was applied (should strip .orig suffix)
    run cat test.go
    assert_output "gofmt_fixed"
}

@test "check_diff works if the file has .orig suffix" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
if [ -f "$file" ]; then
    echo "--- $file"
    echo "+++ $file"
    echo "@@ -1 +1 @@"
    echo "-$(cat "$file")"
    echo "+diffed"
fi
exit 1
SCRIPT
    chmod +x formatter.sh

    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "FIXER_RAN" > "$1"
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("*.orig")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    echo "original" > test.orig

    run hk fix test.orig
    assert_success

    run cat test.orig
    assert_output "diffed"
}

@test "check_after_diff rechecks the original batch after applying a partial fix" {
    cat <<'SCRIPT' > partial-linter.sh
#!/bin/bash
mode="$1"
shift
failed=0
for file in "$@"; do
    content=$(cat "$file")
    if [[ "$mode" == "--diff" && "$content" == "fixable" ]]; then
        echo "--- a/$file"
        echo "+++ b/$file"
        echo "@@ -1 +1 @@"
        echo "-fixable"
        echo "+fixed"
        failed=1
    elif [[ "$content" == "unfixable" ]]; then
        echo "unfixable finding in $file"
        failed=1
    fi
done
exit "$failed"
SCRIPT
    chmod +x partial-linter.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["partial"] {
                glob = List("*.txt")
                check = "./partial-linter.sh --check {{files}}"
                check_diff = "./partial-linter.sh --diff {{files}}"
                check_after_diff = true
            }
        }
    }
}
EOF

    echo "fixable" > fixable.txt
    echo "unfixable" > unfixable.txt

    run hk fix --all
    assert_failure
    assert_output --partial "unfixable finding in unfixable.txt"

    run cat fixable.txt
    assert_output "fixed"

    rm unfixable.txt
    echo "fixable" > fixable.txt

    run hk fix --all
    assert_success

    run cat fixable.txt
    assert_output "fixed"
}

@test "check_after_diff validates required commands" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["partial"] {
                glob = List("*.txt")
                check_diff = "./partial-linter.sh --diff {{files}}"
                check_after_diff = true
            }
        }
    }
}
EOF
    touch test.txt

    run hk validate
    assert_failure
    assert_output --partial \
        "check_after_diff = true\` requires both \`check\` and \`check_diff"
}

@test "check_diff fix mode serializes writers for the same file" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
content=$(cat "$file")
if [[ "$content" == "old" ]]; then
    sleep 1
    echo "--- a/$file"
    echo "+++ b/$file"
    echo "@@ -1 +1 @@"
    echo "-old"
    echo "+new"
    exit 1
fi
SCRIPT
    chmod +x formatter.sh

    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "fixer ran unexpectedly" > "$1"
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["first"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
            ["second"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF

    echo "old" > test.txt

    run hk fix test.txt
    assert_success

    run cat test.txt
    assert_output "new"
}

@test "check_diff-only step stages an applied patch" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
if [[ "$(cat "$file")" == "old" ]]; then
    echo "--- a/$file"
    echo "+++ b/$file"
    echo "@@ -1 +1 @@"
    echo "-old"
    echo "+new"
    exit 1
fi
SCRIPT
    chmod +x formatter.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "none"
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = "./formatter.sh {{files}}"
            }
        }
    }
}
EOF

    echo "base" > test.txt
    git add .
    git commit -m "test: create base fixture"
    echo "old" > test.txt
    git add test.txt

    run hk run pre-commit
    assert_success

    run git show :test.txt
    assert_output "new"
}

_setup_partial_apply_fixture() {
    mkdir locked
    printf 'before\n' > locked/skill.md
    printf 'before\n' > 'writable file.md'
    chmod +x 'writable file.md'
    cp 'writable file.md' expected.md
    cat <<'PATCH' > changes.patch
--- locked/skill.md
+++ locked/skill.md
@@ -1 +1 @@
-before
+after
--- writable file.md
+++ writable file.md
@@ -1 +1 @@
-before
+after
PATCH
    cat <<'SCRIPT' > fixer.sh
#!/bin/sh
set -eu
cmp expected.md locked/skill.md
cmp expected.md 'writable file.md'
test -x 'writable file.md'
touch fixer-ran
SCRIPT
    chmod +x fixer.sh
    cat <<EOF_CONFIG > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = "*.md"
                check_diff = "cat changes.patch; exit 1"
                fix = "./fixer.sh"
            }
        }
    }
}
EOF_CONFIG
    # Originals must come from the working tree, including unstaged edits.
    printf 'staged\n' > locked/skill.md
    printf 'staged\n' > 'writable file.md'
    git add locked/skill.md 'writable file.md'
    git write-tree > index-before
    printf 'before\n' > locked/skill.md
    printf 'before\n' > 'writable file.md'
}

@test "check_diff restores files after git apply fails during a write" {
    case "$OSTYPE" in
        msys*|cygwin*|win*) skip "requires Unix directory permissions" ;;
    esac
    _setup_partial_apply_fixture
    chmod 0555 locked
    # Root and some filesystems can still write here. Do not claim coverage on
    # those systems; the injected failure tests below run without chmod.
    if touch locked/write-probe 2>/dev/null; then
        skip "directory permissions do not prevent writes for this user"
    fi

    # This succeeds even though the actual application will fail while writing.
    git apply --check -p0 changes.patch
    run git apply -p0 changes.patch
    assert_failure
    assert_file_not_exists 'writable file.md'
    cp expected.md 'writable file.md'
    chmod +x 'writable file.md'

    run hk fix locked/skill.md 'writable file.md'
    assert_success
    assert_file_exists fixer-ran
    cmp expected.md locked/skill.md
    cmp expected.md 'writable file.md'
    git write-tree > index-after
    cmp index-before index-after
}

_mock_partial_git_apply() {
    case "$OSTYPE" in
        msys*|cygwin*|win*) skip "requires a Unix executable shim; Rust tests cover restoration on Windows" ;;
    esac
    export REAL_GIT
    REAL_GIT=$(command -v git)
    mkdir mock-bin
    cat <<'SCRIPT' > mock-bin/git
#!/bin/sh
if [ "$1" = apply ]; then
    case " $* " in
        *" --numstat "*|*" --check "*) exec "$REAL_GIT" "$@" ;;
    esac
    cat >/dev/null
    printf 'partly written\n' > locked/skill.md
    rm 'writable file.md'
    if [ "${BLOCK_DIFF_RESTORE:-}" = 1 ]; then
        rm locked/skill.md
        mkdir locked/skill.md
        printf 'unrelated\n' > locked/skill.md/keep
    fi
    echo 'injected write-time failure' >&2
    exit 1
fi
exec "$REAL_GIT" "$@"
SCRIPT
    chmod +x mock-bin/git
    export PATH="$PWD/mock-bin:$PATH"
}

@test "check_diff warns on backup preparation I/O failures and still falls back" {
    case "$OSTYPE" in
        msys*|cygwin*|win*) skip "requires a Unix executable shim" ;;
    esac
    _setup_partial_apply_fixture
    export REAL_GIT
    REAL_GIT=$(command -v git)
    mkdir mock-bin backups
    export TMPDIR="$PWD/backups"
    cat <<'SCRIPT' > mock-bin/git
#!/bin/sh
if [ "$1" = apply ]; then
    case " $* " in
        *" --reverse "*)
            "$REAL_GIT" "$@" || exit
            # The patch is already open. Make the subsequent backup creation fail.
            mv "$TMPDIR" "$TMPDIR-unavailable"
            exit 0
            ;;
    esac
fi
exec "$REAL_GIT" "$@"
SCRIPT
    chmod +x mock-bin/git
    export PATH="$PWD/mock-bin:$PATH"

    run env HK_LOG_LEVEL=warn hk fix locked/skill.md 'writable file.md'
    assert_success
    assert_file_exists fixer-ran
    assert_dir_exists "$TMPDIR-unavailable"
    assert_output --partial 'cannot safely apply diff'
    cmp expected.md locked/skill.md
    cmp expected.md 'writable file.md'
}

@test "check_diff restores modified and deleted files before fallback" {
    _setup_partial_apply_fixture
    _mock_partial_git_apply

    run hk fix locked/skill.md 'writable file.md'
    assert_success
    assert_file_exists fixer-ran
    cmp expected.md locked/skill.md
    cmp expected.md 'writable file.md'
    git write-tree > index-after
    cmp index-before index-after
}

@test "check_diff stops and retains originals when restoration fails" {
    _setup_partial_apply_fixture
    _mock_partial_git_apply
    export BLOCK_DIFF_RESTORE=1
    # Keep the deliberately retained recovery files inside this test's temp dir.
    mkdir backups
    export TMPDIR="$PWD/backups"

    run hk fix locked/skill.md 'writable file.md'
    assert_failure
    assert_output --partial 'failed to restore files after git apply; refusing to run fixer'
    assert_output --partial 'Original files retained in'
    assert_file_not_exists fixer-ran
    # The first target fails restoration; later targets must still be recovered.
    cmp expected.md 'writable file.md'
    local backup
    for backup in backups/hk-diff-backup-*; do
        cmp expected.md "$backup/files/locked/skill.md"
        cmp expected.md "$backup/files/writable file.md"
        test -x "$backup/files/writable file.md"
    done
}

_setup_structural_diff_fixture() {
    mkdir locked expected
    printf 'delete\n' > delete.txt
    printf 'rename\n' > old.txt
    printf 'copy\n' > source.txt
    printf '#!/bin/sh\n' > script.sh
    chmod 0644 script.sh
    printf 'before\n' > locked/skill.md
    cp delete.txt old.txt source.txt script.sh expected/
    cp locked/skill.md expected/skill.md
    git add delete.txt old.txt source.txt script.sh locked/skill.md
    git write-tree > index-before

    cat <<'PATCH' > changes.patch
--- /dev/null
+++ new/nested/created.txt
@@ -0,0 +1 @@
+created
--- delete.txt
+++ /dev/null
@@ -1 +0,0 @@
-delete
diff --git old.txt renamed.txt
similarity index 100%
rename from old.txt
rename to renamed.txt
diff --git source.txt copied.txt
similarity index 100%
copy from source.txt
copy to copied.txt
diff --git script.sh script.sh
old mode 100644
new mode 100755
diff --git locked/skill.md locked/skill.md
--- locked/skill.md
+++ locked/skill.md
@@ -1 +1 @@
-before
+after
PATCH
    cat <<'SCRIPT' > verify-original.sh
#!/bin/sh
set -eu
for file in delete.txt old.txt source.txt script.sh; do
    cmp "expected/$file" "$file"
done
cmp expected/skill.md locked/skill.md
test ! -x script.sh
test ! -e renamed.txt
test ! -e copied.txt
test ! -e new
git write-tree > index-after
cmp index-before index-after
touch fixer-ran
SCRIPT
    chmod +x verify-original.sh
    cat <<EOF_CONFIG > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                check_diff = "cat changes.patch; exit 1"
            }
        }
    }
}
EOF_CONFIG
}

@test "check_diff-only preserves creation deletion rename copy and mode changes" {
    _setup_structural_diff_fixture

    run hk fix locked/skill.md
    assert_success
    assert_file_not_exists fixer-ran
    assert_file_not_exists delete.txt
    assert_file_not_exists old.txt
    cmp expected/old.txt renamed.txt
    cmp expected/source.txt source.txt
    cmp expected/source.txt copied.txt
    assert_file_contains new/nested/created.txt created
    assert_file_contains locked/skill.md after
    test -x script.sh
    git write-tree > index-after
    cmp index-before index-after
}

@test "check_diff rolls back structural changes after an actual write failure" {
    case "$OSTYPE" in
        msys*|cygwin*|win*) skip "requires Unix directory permissions" ;;
    esac
    _setup_structural_diff_fixture
    cat <<EOF_CONFIG > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                check_diff = "cat changes.patch; exit 1"
                fix = "./verify-original.sh"
            }
        }
    }
}
EOF_CONFIG
    chmod 0555 locked
    if touch locked/write-probe 2>/dev/null; then
        skip "directory permissions do not prevent writes for this user"
    fi

    # Prove this fixture writes structural changes before the final failure.
    git apply --check -p0 changes.patch
    run git apply -p0 changes.patch
    assert_failure
    assert_file_exists new/nested/created.txt
    assert_file_exists renamed.txt
    assert_file_exists copied.txt
    assert_file_not_exists delete.txt
    test -x script.sh

    # Reset only the disposable fixture, then exercise hk's recovery path.
    cp expected/delete.txt expected/old.txt expected/source.txt expected/script.sh .
    chmod 0644 script.sh
    rm renamed.txt copied.txt new/nested/created.txt
    rmdir new/nested new
    run hk fix locked/skill.md
    assert_success
    assert_file_exists fixer-ran
    run ./verify-original.sh
    assert_success
}

_setup_concurrent_diff_fixture() {
    case "$OSTYPE" in
        msys*|cygwin*|win*) skip "requires a Unix executable shim" ;;
    esac
    printf 'input\n' > a.in
    printf 'input\n' > b.in
    cat <<'PATCH' > a.patch
--- /dev/null
+++ shared.txt
@@ -0,0 +1 @@
+A
--- /dev/null
+++ a-side.txt
@@ -0,0 +1 @@
+partial
PATCH
    cat <<'PATCH' > b.patch
--- /dev/null
+++ shared.txt
@@ -0,0 +1 @@
+B
PATCH
    cat <<'SCRIPT' > wait-for-apply.sh
#!/bin/sh
# Bounded rendezvous: a correctly serialized transaction cannot start until
# this command finishes. Without coordination, A starts and B writes during it.
i=0
while [ ! -e a-applying ] && [ "$i" -lt 100 ]; do
    sleep 0.01
    i=$((i + 1))
done
SCRIPT
    chmod +x wait-for-apply.sh
    export REAL_GIT
    REAL_GIT=$(command -v git)
    mkdir mock-bin
    cat <<'SCRIPT' > mock-bin/git
#!/bin/sh
if [ "$1" != apply ]; then
    exec "$REAL_GIT" "$@"
fi
case " $* " in
    *" --numstat "*|*" --check "*) exec "$REAL_GIT" "$@" ;;
esac
patch=$(cat)
if printf '%s\n' "$patch" | grep -q '^+A$'; then
    printf 'partial\n' > a-side.txt
    touch a-applying
    i=0
    while [ ! -e b-written ] && [ "$i" -lt 100 ]; do
        sleep 0.01
        i=$((i + 1))
    done
    echo 'injected A write-time failure' >&2
    exit 1
fi
printf '%s\n' "$patch" | "$REAL_GIT" "$@" || exit
# Preserve evidence that B succeeded even if A subsequently deletes its output.
cp shared.txt b-written
SCRIPT
    chmod +x mock-bin/git
    export PATH="$PWD/mock-bin:$PATH"
}

_run_concurrent_fix() {
    # Bound lock-ordering regressions rather than hanging the integration suite.
    run python3 -c 'import subprocess, sys; sys.exit(subprocess.run(["hk", "fix", "a.in", "b.in"], timeout=15).returncode)'
}

@test "check_diff rollback preserves another patch job's new target" {
    _setup_concurrent_diff_fixture
    cat <<EOF_CONFIG > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["a"] {
                glob = "a.in"
                check_diff = "cat a.patch; exit 1"
                fix = "touch a-fixer-ran"
            }
            ["b"] {
                glob = "b.in"
                check_diff = "./wait-for-apply.sh; cat b.patch; exit 1"
            }
        }
    }
}
EOF_CONFIG
    _run_concurrent_fix
    assert_success
    assert_file_exists a-applying
    assert_file_exists a-fixer-ran
    assert_file_not_exists a-side.txt
    assert_file_contains b-written B
    assert_file_contains shared.txt B
}

@test "check_diff backup and rollback exclude ordinary fixer writes" {
    _setup_concurrent_diff_fixture
    cat <<EOF_CONFIG > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["a"] {
                glob = "a.in"
                check_diff = "cat a.patch; exit 1"
                fix = "touch a-fixer-ran"
            }
            ["b"] {
                glob = "b.in"
                fix = "./wait-for-apply.sh; echo B > shared.txt; cp shared.txt b-written"
            }
        }
    }
}
EOF_CONFIG
    _run_concurrent_fix
    assert_success
    assert_file_exists a-fixer-ran
    assert_file_not_exists a-side.txt
    assert_file_contains b-written B
    assert_file_contains shared.txt B
}

@test "ordinary fixers on disjoint inputs still run concurrently" {
    _setup_concurrent_diff_fixture
    cat <<'SCRIPT' > fixer.sh
#!/bin/sh
set -eu
file="$1"
touch "$file-started"
i=0
while [ ! -e a.in-started ] || [ ! -e b.in-started ]; do
    i=$((i + 1))
    test "$i" -lt 200 || exit 1
    sleep 0.01
done
printf 'fixed\n' > "$file"
SCRIPT
    chmod +x fixer.sh
    cat <<EOF_CONFIG > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["a"] {
                glob = "a.in"
                fix = "./fixer.sh {{files}}"
            }
            ["b"] {
                glob = "b.in"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF_CONFIG
    _run_concurrent_fix
    assert_success
    assert_file_contains a.in fixed
    assert_file_contains b.in fixed
}
