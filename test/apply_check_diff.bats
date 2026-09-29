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

@test "read-only check_diff rechecks unnamed job inputs changed before apply" {
    cat <<'SCRIPT' > derived.sh
#!/bin/bash
touch derived.started
source=$(cat b.txt)
sleep 0.4
if [ "$(cat a.txt)" != "from-$source" ]; then
    printf '%s\n' '--- a.txt' '+++ a.txt' '@@ -1 +1 @@' "-$(cat a.txt)" "+from-$source"
    exit 1
fi
SCRIPT
    cat <<'SCRIPT' > writer.sh
#!/bin/bash
for _ in $(seq 50); do
    [ -e derived.started ] && break
    sleep 0.1
done
[ -e derived.started ] || exit 2
if [ "$(cat b.txt)" = "old" ]; then
    printf '%s\n' '--- b.txt' '+++ b.txt' '@@ -1 +1 @@' '-old' '+new'
    exit 1
fi
SCRIPT
    chmod +x derived.sh writer.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["derived"] {
                glob = List("*.txt")
                check_diff = new CommandSpec { command = "./derived.sh"; effect = "read" }
            }
            ["writer"] {
                glob = List("b.txt")
                check_diff = new CommandSpec { command = "./writer.sh"; effect = "read" }
            }
        }
    }
}
EOF
    echo "old" > a.txt
    echo "old" > b.txt

    run hk fix a.txt b.txt
    assert_success
    run cat a.txt
    assert_output "from-new"
    run cat b.txt
    assert_output "new"
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

# Runs a command under a pty, where hk draws its progress in place, and fails
# if the last frame hk drew still shows a job running: a step's command line
# and its spinner left on screen once the step is done. CLX_TRACE_LOG records
# each frame's job tree. The top-level header job (id 0) is ignored: clx skips
# redrawing a settled frame whose text is unchanged, so its final "done" is
# never traced even though nothing on screen differs. Needs util-linux
# script(1); BSD script takes other arguments.
assert_no_job_left_running_in_pty() {
    if ! script --version 2>/dev/null | grep -q util-linux; then
        skip "needs util-linux script(1) for a pty"
    fi
    local frames="$BATS_TEST_TMPDIR/frames.jsonl"
    rm -f "$frames"
    # hk draws in place only for an attended terminal outside CI.
    env -u CI -u GITHUB_ACTION TERM=xterm-256color CLX_TRACE_LOG="$frames" \
        script -q -e -c "$*" /dev/null >/dev/null
    assert_file_not_empty "$frames"
    run jq -c '[.jobs[] | select(.id != 0) | recurse(.children[]) | select(.status == "running")]' <(tail -n 1 "$frames")
    assert_success
    assert_output '[]'
}

# A fix hook with one step whose read-only check_diff is $1, and whose fixer
# writes "fixed" to a.txt.
write_read_only_diff_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("*.txt")
                check_diff = new CommandSpec { command = "$1 {{files}}"; effect = "read" }
                fix = new CommandSpec { command = "./fixer.sh {{files}}"; effect = "write" }
            }
        }
    }
}
EOF
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "fixed" > a.txt
SCRIPT
    chmod +x fixer.sh
    echo "a" > a.txt
}

@test "a read-only check_diff whose patch applies leaves no job running on screen" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
if [ "$(cat a.txt)" = "a" ]; then
    printf -- '--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-a\n+a2\n'
    exit 1
fi
SCRIPT
    chmod +x formatter.sh
    write_read_only_diff_config ./formatter.sh

    assert_no_job_left_running_in_pty hk fix a.txt
    run cat a.txt
    assert_output "a2"
}

@test "a read-only check_diff with nothing to apply leaves no job running on screen" {
    # Like `shellcheck --format=diff` with only findings it can't fix: it
    # fails without a patch, so hk runs the fixer.
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
echo "Issues were detected, but none were auto-fixable." >&2
exit 1
SCRIPT
    chmod +x formatter.sh
    write_read_only_diff_config ./formatter.sh

    assert_no_job_left_running_in_pty hk fix a.txt
    run cat a.txt
    assert_output "fixed"
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

@test "check_diff stages a file created by its patch without warning" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
if [ ! -e go.sum ]; then
    printf '%s\n' '--- /dev/null' '+++ b/go.sum' '@@ -0,0 +1 @@' '+sum'
    exit 1
fi
SCRIPT
    chmod +x formatter.sh
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "fixer ran unexpectedly" >&2
exit 1
SCRIPT
    chmod +x fixer.sh

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "none"
        steps {
            ["tidy"] {
                glob = List("go.mod")
                dir = "."
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "read" }
                fix = "./fixer.sh"
                stage = "<JOB_FILES>"
            }
        }
    }
}
EOF

    echo "module example.com/test" > go.mod
    git add go.mod formatter.sh fixer.sh hk.pkl
    git commit -m "test: create base fixture"
    echo "module example.com/changed" > go.mod
    git add go.mod

    run hk run pre-commit
    assert_success
    refute_output --partial "file in check output not found in original files"
    run git show :go.sum
    assert_success
    assert_output "sum"
}

@test "stomp check_diff applies a patch that creates a file" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
if [ ! -e created.txt ]; then
    printf '%s\n' '--- /dev/null' '+++ b/created.txt' '@@ -0,0 +1 @@' '+made'
    exit 1
fi
SCRIPT
    chmod +x formatter.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("source.txt")
                stomp = true
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "write" }
            }
        }
    }
}
EOF
    echo "source" > source.txt

    run hk fix source.txt
    assert_success
    refute_output --partial "file in check output not found in original files"
    run cat created.txt
    assert_output "made"
}

@test "absolute creation headers use the applied path for staging" {
    mkdir pkg
    cat <<'SCRIPT' > pkg/formatter.sh
#!/bin/bash
if [ ! -e created.txt ]; then
    printf '%s\n' '--- /dev/null' "+++ $(pwd)/b/created.txt" '@@ -0,0 +1 @@' '+made'
    exit 1
fi
SCRIPT
    chmod +x pkg/formatter.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "none"
        steps {
            ["fmt"] {
                glob = List("*.txt")
                dir = "pkg"
                check_diff = "./formatter.sh"
                fix = "false"
                stage = "<JOB_FILES>"
            }
        }
    }
}
EOF
    echo "base" > pkg/source.txt
    git add .
    git commit -m "test: create base fixture"
    echo "changed" > pkg/source.txt
    git add pkg/source.txt

    run hk run pre-commit
    assert_success
    run git show :pkg/created.txt
    assert_success
    assert_output "made"
    run test ! -e pkg/b/created.txt
    assert_success
}

@test "created paths under a symlinked step directory use the resolved file" {
    mkdir pkg
    ln -s pkg alias
    cat <<'SCRIPT' > pkg/formatter.sh
#!/bin/bash
if [ ! -e created.txt ]; then
    printf '%s\n' '--- /dev/null' '+++ created.txt' '@@ -0,0 +1 @@' '+made'
    exit 1
fi
SCRIPT
    chmod +x pkg/formatter.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = true
        steps {
            ["fmt"] {
                glob = List("*.txt")
                dir = "alias"
                check_diff = "./formatter.sh"
                fix = "false"
                stage = "<JOB_FILES>"
            }
        }
    }
}
EOF
    echo "source" > pkg/source.txt
    git add .
    git commit -m "test: create base fixture"

    run hk fix alias/source.txt
    assert_success
    run git show :pkg/created.txt
    assert_success
    assert_output "made"
}

@test "a failed creation patch passes only check-selected files to the fixer" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
echo "already here" > generated.txt
printf '%s\n' '--- /dev/null' '+++ generated.txt' '@@ -0,0 +1 @@' '+from-patch'
exit 1
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
printf '%s\n' "$@" > fixer-files.txt
SCRIPT
    chmod +x formatter.sh fixer.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("source.txt")
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "read" }
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF
    echo "source" > source.txt
    run hk fix source.txt
    assert_success
    run cat fixer-files.txt
    assert_output "source.txt"
}

@test "write-effect creation patches take all file locks together" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
sleep 0.2
case "$1" in
    source-a.txt) target=source-b.txt ;;
    source-b.txt) target=source-a.txt ;;
esac
printf '%s\n' '--- /dev/null' "+++ $target" '@@ -0,0 +1 @@' '+from-patch'
exit 1
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
touch "fixed-$1"
SCRIPT
    chmod +x formatter.sh fixer.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["a"] {
                glob = List("source-a.txt")
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "write" }
                fix = "./fixer.sh {{files}}"
            }
            ["b"] {
                glob = List("source-b.txt")
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "write" }
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF
    echo "a" > source-a.txt
    echo "b" > source-b.txt
    run python3 -c 'import subprocess, sys; sys.exit(subprocess.run(sys.argv[1:], timeout=10).returncode)' hk fix source-a.txt source-b.txt
    assert_success
    [ -e fixed-source-a.txt ]
    [ -e fixed-source-b.txt ]
}

@test "each created destination appearing during check_diff is rechecked" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
if [ ! -e generated-one.txt ]; then
    # Simulate another step creating the destination before hk takes its lock.
    echo "from-other-step" > generated-one.txt
    printf '%s\n' '--- /dev/null' '+++ generated-one.txt' '@@ -0,0 +1 @@' '+from-patch'
    exit 1
fi
if [ ! -e generated-two.txt ]; then
    echo "from-other-step" > generated-two.txt
    printf '%s\n' '--- /dev/null' '+++ generated-two.txt' '@@ -0,0 +1 @@' '+from-patch'
    exit 1
fi
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "fixer ran unexpectedly" >&2
exit 1
SCRIPT
    chmod +x formatter.sh fixer.sh
    echo "source" > source.txt
    for effect in read write; do
        rm -f generated-one.txt generated-two.txt
        cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("source.txt")
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "read" }
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF
        if [ "$effect" = "write" ]; then
            perl -pi -e 's/effect = "read"/effect = "write"/' hk.pkl
        fi
        run hk fix source.txt
        assert_success
        run cat generated-one.txt
        assert_output "from-other-step"
        run cat generated-two.txt
        assert_output "from-other-step"
    done
}

@test "check_diff falls back to the fixer when created destinations never settle" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
attempt=$(cat attempts 2>/dev/null || echo 0)
attempt=$((attempt + 1))
echo "$attempt" > attempts
if [ "$attempt" -le 12 ]; then
    created="generated-$attempt.txt"
    echo "from-other-step" > "$created"
    printf '%s\n' '--- /dev/null' "+++ $created" '@@ -0,0 +1 @@' '+from-patch'
    exit 1
fi
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "fixer ran" > fixed.txt
SCRIPT
    chmod +x formatter.sh fixer.sh
    echo "source" > source.txt
    for effect in read write; do
        rm -f attempts fixed.txt generated-*.txt
        cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("source.txt")
                check_diff = new CommandSpec { command = "./formatter.sh {{files}}"; effect = "read" }
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF
        if [ "$effect" = "write" ]; then
            perl -pi -e 's/effect = "read"/effect = "write"/' hk.pkl
        fi
        run hk fix source.txt
        assert_success
        run cat attempts
        assert_output "9"
        run cat fixed.txt
        assert_output "fixer ran"
    done
}

@test "creation-only check_diff still passes job files to the fixer" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf '%s\n' '--- /dev/null' '+++ created.txt' '@@ -0,0 +1 @@' '+from-patch'
exit 1
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
printf '%s\n' "$@" > fixer-files.log
echo "from-fixer" > created.txt
SCRIPT
    chmod +x formatter.sh fixer.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("go.mod")
                check_diff = "./formatter.sh {{files}}"
                apply_check_diff = false
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF
    echo "module example.com/test" > go.mod

    run hk fix go.mod
    assert_success
    run cat fixer-files.log
    assert_output "go.mod"
    run cat created.txt
    assert_output "from-fixer"
}

@test "a creation header for a job file does not reacquire its write lock" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf '%s\n' '--- /dev/null' '+++ existing.txt' '@@ -0,0 +1 @@' '+from-patch'
exit 1
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "from-fixer" > "$1"
SCRIPT
    chmod +x formatter.sh fixer.sh
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
    echo "old" > existing.txt

    run perl -e 'alarm 5; exec @ARGV' hk fix existing.txt
    assert_success
    run cat existing.txt
    assert_output "from-fixer"
}

@test "fixer receives an existing job file named by a creation header in a mixed patch" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf '%s\n' '--- /dev/null' '+++ existing.txt' '@@ -0,0 +1 @@' '+from-patch' \
    '--- other.txt' '+++ other.txt' '@@ -1 +1 @@' '-old' '+updated'
exit 1
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
printf '%s\n' "$@" > fixer-files.log
SCRIPT
    chmod +x formatter.sh fixer.sh
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
    echo "already here" > existing.txt
    echo "old" > other.txt

    run hk fix existing.txt other.txt
    assert_success
    run cat fixer-files.log
    assert_output --partial "existing.txt"
    assert_output --partial "other.txt"
}

@test "a mixed-prefix creation patch keeps unprefixed paths intact" {
    cat <<'SCRIPT' > formatter.sh
#!/bin/bash
printf '%s\n' '--- /dev/null' '+++ b/new.txt' '@@ -0,0 +1 @@' '+new' \
    '--- sub/existing.txt' '+++ sub/existing.txt' '@@ -1 +1 @@' '-old' '+updated'
exit 1
SCRIPT
    cat <<'SCRIPT' > fixer.sh
#!/bin/bash
echo "fixer ran unexpectedly" >&2
exit 1
SCRIPT
    chmod +x formatter.sh fixer.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["fmt"] {
                glob = List("**/*.txt")
                check_diff = "./formatter.sh {{files}}"
                fix = "./fixer.sh {{files}}"
            }
        }
    }
}
EOF
    mkdir sub
    echo "old" > sub/existing.txt

    run hk fix sub/existing.txt
    assert_success
    run cat sub/existing.txt
    assert_output "updated"
    run cat b/new.txt
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
