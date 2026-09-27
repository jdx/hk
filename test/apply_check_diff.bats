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
    run hk fix test.txt
    assert_success
    # Output that isn't a patch at all hands the file to the fixer quietly.
    refute_output --partial "rejected"

    # The fixer should have run and added "FIXED"
    run cat test.txt
    assert_output "hello
FIXED"
}

@test "check_diff warns when its patch does not apply and the fixer runs instead" {
    # A well-formed patch whose context doesn't match the file, as a tool
    # that mangles its diff output would print.
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf -- '--- %s\n+++ %s\n@@ -1 +1 @@\n-something else\n+formatted\n' "$1" "$1"
exit 1
SCRIPT
    chmod +x formatter.sh

    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "FIXED" > "$1"
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
    run hk fix test.txt
    assert_success
    assert_output --partial "fmt: check_diff printed a patch that doesn't apply, so the fixer ran instead: test.txt: error applying hunk #1"

    run cat test.txt
    assert_output "FIXED"
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

@test "check_diff handles diffs that label each side with its own directory" {
    # `go mod tidy -diff` writes "--- current/go.mod" and "+++ tidy/go.mod".
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
echo "diff current/$file tidy/$file"
echo "--- current/$file"
echo "+++ tidy/$file"
echo "@@ -1 +1 @@"
echo "-$(cat "$file")"
echo "+relabelled_diff"
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

    run cat test.txt
    assert_output "relabelled_diff"
}

@test "check_diff applies a patch that changes lines ending in CRLF" {
    # The patch's removed lines end in \r\n, like the file. Dropping the \r
    # anywhere between the command and git apply makes the patch not match.
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf -- '--- %s\n+++ %s\n@@ -1,2 +1,2 @@\n-one  \r\n+one\r\n two\r\n' "$1" "$1"
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

    printf 'one  \r\ntwo\r\n' > test.txt
    run hk fix test.txt
    assert_success

    run od -c test.txt
    assert_output --partial 'o   n   e  \r  \n   t   w   o  \r  \n'
}

@test "check_diff leaves hunk lines that look like file headers alone" {
    # The file changes from "-- current/foo" to "++ tidy/foo", so the hunk's
    # removed and added lines read "--- current/foo" and "+++ tidy/foo".
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf -- '--- a/%s\n+++ b/%s\n@@ -1,2 +1,2 @@\n--- current/foo\n+++ tidy/foo\n x\n' "$1" "$1"
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

    printf -- '-- current/foo\nx\n' > test.txt
    run hk fix test.txt
    assert_success

    run cat test.txt
    assert_output $'++ tidy/foo\nx'
}

@test "check_diff applies a git-style patch that edits one file and creates another" {
    # `go mod tidy -diff` can create go.sum this way, from /dev/null.
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf -- '--- a/%s\n+++ b/%s\n@@ -1 +1 @@\n-old\n+new\n--- /dev/null\n+++ b/created.txt\n@@ -0,0 +1 @@\n+made\n' "$1" "$1"
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
                glob = List("test.txt")
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
    run cat created.txt
    assert_output "made"
}

@test "check_diff applies a patch that only creates a file with git's b/ prefix" {
    # No pair has both a/ and b/, and there is no b/ directory, so b/ is
    # git's prefix rather than part of the path.
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf -- '--- /dev/null\n+++ b/go.sum\n@@ -0,0 +1 @@\n+sum\n'
exit 1
SCRIPT
    chmod +x formatter.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["tidy"] {
                glob = List("go.mod")
                check_diff = "./formatter.sh {{files}}"
                fix = "echo fixer-ran > go.mod"
            }
        }
    }
}
EOF

    echo "module x" > go.mod
    run hk fix go.mod
    assert_success

    run cat go.sum
    assert_output "sum"
    assert [ ! -e b/go.sum ]
    run cat go.mod
    assert_output "module x"
}

@test "a check_diff patch for a file with a tab in its name fixes that file" {
    # hk's diff utils quote such a path the way git does, so the tab isn't read
    # as the start of a timestamp that would leave just `foo`.
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["trailing-whitespace"] = Builtins.trailing_whitespace
        }
    }
}
EOF
    printf 'tabbed  \n' > "$(printf 'foo\tbar.txt')"
    printf 'clean\n' > foo
    git add -A
    git commit -qm init

    run hk fix --all
    assert_success
    refute_output --partial "doesn't apply"
    run cat "$(printf 'foo\tbar.txt')"
    assert_output "tabbed"
    run cat foo
    assert_output "clean"
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

# A check_diff that meets the other step's before it prints anything: it
# marks that it started and waits up to 5 seconds for the other step's mark.
# Steps that ran one after the other leave a "$STEP.alone" file. A step whose
# first line lacks its mark prints a patch adding it.
write_rendezvous_formatter() {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
file="$1"
touch "$STEP.started"
for _ in $(seq 50); do
    [ -e "$OTHER.started" ] && break
    sleep 0.1
done
[ -e "$OTHER.started" ] || touch "$STEP.alone"
first=$(head -1 "$file")
[[ "$first" == *"-$STEP"* ]] && exit 0
new=$(mktemp)
sed "1s/\$/-$STEP/" "$file" > "$new"
diff -u -L "$file" -L "$file" "$file" "$new"
rm -f "$new"
exit 1
SCRIPT
    chmod +x formatter.sh
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "fixer ran unexpectedly" > "$1"
SCRIPT
    chmod +x fixer.sh
}

# $1: the effect both steps' check_diff declares
write_rendezvous_config() {
    local check_diff_effect=$1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["first"] {
                glob = List("*.txt")
                env { ["STEP"] = "1"; ["OTHER"] = "2" }
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "$check_diff_effect" }
                fix = "./fixer.sh {{files}}"
            }
            ["second"] {
                glob = List("*.txt")
                env { ["STEP"] = "2"; ["OTHER"] = "1" }
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "$check_diff_effect" }
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF
}

@test "read-only check_diff steps compute their patches alongside each other in fix mode" {
    write_rendezvous_formatter
    write_rendezvous_config read
    echo "line" > test.txt

    run hk fix test.txt
    assert_success
    # Both ran at once, and both patches were applied: the second to reach
    # its write locks found the file changed and computed its patch again.
    [ ! -e 1.alone ]
    [ ! -e 2.alone ]
    run cat test.txt
    assert_output --regexp '^line-(1-2|2-1)$'
}

@test "check_diff steps that declare a write effect keep write locks in fix mode" {
    write_rendezvous_formatter
    write_rendezvous_config write
    echo "line" > test.txt

    run hk fix test.txt
    assert_success
    # One ran after the other, so exactly one waited for the other in vain.
    [ -e 1.alone ] || [ -e 2.alone ]
    run cat test.txt
    assert_output --regexp '^line-(1-2|2-1)$'
}

@test "a read-only check_diff patch that also names a file outside the job is applied" {
    # Like `go mod tidy -diff` rewriting go.sum from all of a job's .go files:
    # the patch is computed again under write locks, from every job file.
    # outside.lock records how many files the formatter was given.
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
status=0
if [ "$(cat a.txt)" = "a" ]; then
    printf -- '--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-a\n+a2\n'
    status=1
fi
if [ "$(cat outside.lock)" != "count=$#" ]; then
    printf -- '--- outside.lock\n+++ outside.lock\n@@ -1 +1 @@\n-%s\n+count=%s\n' "$(cat outside.lock)" "$#"
    status=1
fi
exit $status
SCRIPT
    chmod +x formatter.sh
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "fixer ran unexpectedly" > outside.lock
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
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "read" }
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF
    echo "a" > a.txt
    echo "b" > b.txt
    echo "old" > outside.lock

    run hk fix a.txt b.txt
    assert_success
    run cat a.txt
    assert_output "a2"
    run cat outside.lock
    assert_output "count=2"
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

@test "apply_check_diff = false runs the fixer instead of the diff" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
echo "ran" >> formatter.log
for file in "$@"; do
    echo "--- a/$file"
    echo "+++ b/$file"
    echo "@@ -1 +1 @@"
    echo "-$(cat "$file")"
    echo "+from-diff"
done
exit 1
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
for file in "$@"; do
    echo "FIXED" > "$file"
done
SCRIPT
    chmod +x formatter.sh fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
// Top-level steps create the check and fix hooks.
steps {
    ["fmt"] {
        glob = List("*.txt")
        check_diff = "./formatter.sh {{files}}"
        fix = "./fixer.sh {{files}}"
        apply_check_diff = false
    }
}
EOF

    echo "hello" > test.txt

    run hk fix test.txt
    assert_success
    run cat test.txt
    assert_output "FIXED"
    # With no files to narrow for staging, fix mode doesn't run check_diff.
    assert_file_not_exists formatter.log

    # Check mode still shows the diff and leaves the file alone.
    echo "hello" > test.txt
    run hk check test.txt
    assert_failure
    assert_output --partial "+from-diff"
    run cat test.txt
    assert_output "hello"
}

@test "apply_check_diff = false fixes and stages only the files the diff names in pre-commit" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
status=0
for file in "$@"; do
    if [[ "$(cat "$file")" == "old" ]]; then
        echo "--- a/$file"
        echo "+++ b/$file"
        echo "@@ -1 +1 @@"
        echo "-old"
        echo "+new"
        status=1
    fi
done
exit $status
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
for file in "$@"; do
    echo "$file" >> fixer.log
    echo "new" > "$file"
done
SCRIPT
    chmod +x formatter.sh fixer.sh

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
                fix = "./fixer.sh {{files}}"
                apply_check_diff = false
            }
        }
    }
}
EOF

    echo "base" > a.txt
    echo "base" > b.txt
    git add .
    git commit -m "test: create base fixture"
    echo "old" > a.txt
    echo "fine" > b.txt
    git add a.txt b.txt

    run hk run pre-commit
    assert_success

    run cat fixer.log
    assert_output "a.txt"
    run git show :a.txt
    assert_output "new"
    run git show :b.txt
    assert_output "fine"
}

@test "apply_check_diff = false runs the fixer in pre-commit when check_diff is empty on this platform" {
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
for file in "$@"; do
    echo "new" > "$file"
done
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "none"
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = new Script {
                    linux = ""
                    macos = ""
                    windows = ""
                    other = "false"
                }
                fix = "./fixer.sh {{files}}"
                apply_check_diff = false
            }
        }
    }
}
EOF

    echo "base" > a.txt
    git add .
    git commit -m "test: create base fixture"
    echo "old" > a.txt
    git add a.txt

    HK_LOG=debug run hk run pre-commit
    assert_success
    # No check-first attempt without a command to run.
    refute_output --partial "failed check step first"
    run git show :a.txt
    assert_output "new"
}
