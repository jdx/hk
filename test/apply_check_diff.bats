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

    git add .
    git commit -m "initial"

    # Run fix
    run hk fix test.txt
    assert_success

    # The file should have a newline added (from the diff), NOT "FIXED:" prefix
    # If git apply worked, content is "hello\n"
    # If fixer ran, content would be "FIXED:hello"
    run cat test.txt
    assert_output "hello"  # With newline from diff

    # Verify it does NOT have the FIXED prefix (which would mean fixer ran)
    run grep -c "FIXED:" test.txt
    assert_failure
}

@test "check_diff falls back to fixer when git apply fails" {
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

    git add .
    git commit -m "initial"

    # Run fix - should fall back to fixer since diff is invalid
    run hk fix test.txt
    assert_success

    # The fixer should have run and added "FIXED"
    run cat test.txt
    assert_output "hello
FIXED"
}
