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
