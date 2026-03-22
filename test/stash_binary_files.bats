#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

create_config_with_stash() {
    local method="$1"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "$method"
    steps = new Mapping<String, Step> {
      ["format-txt"] {
        glob = "*.txt"
        fix = "true"
      }
    }
  }
}
PKL
    git add hk.pkl
    git commit -m 'init'
    hk install
}

# Generate a binary file with known content (non-UTF-8 bytes)
create_binary_file() {
    local path="$1"
    # Write bytes including null bytes and high bytes that are not valid UTF-8
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01' > "$path"
}

@test "stash=git preserves unstaged binary files" {
    create_config_with_stash "git"

    # Create and commit a text file and a binary file
    echo "hello" > file.txt
    create_binary_file snapshot.png
    git add file.txt snapshot.png
    git commit -m "base"

    # Stage a text file change
    echo "world" > file.txt
    git add file.txt

    # Modify the binary file but do NOT stage it
    create_binary_file snapshot.png
    printf '\xff\xfe\x00\x01' >> snapshot.png

    # Record expected binary content
    local expected_size
    expected_size=$(wc -c < snapshot.png)
    local expected_md5
    expected_md5=$(md5sum snapshot.png | cut -d' ' -f1)

    # Commit (triggers pre-commit hook with stash)
    git commit -m "test"

    # Binary file should be preserved with original content
    local actual_size
    actual_size=$(wc -c < snapshot.png)
    local actual_md5
    actual_md5=$(md5sum snapshot.png | cut -d' ' -f1)

    [ "$actual_size" -eq "$expected_size" ]
    [ "$actual_md5" = "$expected_md5" ]
}

@test "stash preserves staged binary + unstaged binary simultaneously" {
    # Tests a different scenario: binary file is both staged (old version)
    # and has unstaged modifications (new version). The unstaged version
    # should be preserved after commit.
    # Note: stash=patch-file currently uses the same code path as stash=git.
    create_config_with_stash "git"

    # Create and commit a text file and a binary file
    echo "hello" > file.txt
    create_binary_file snapshot.png
    git add file.txt snapshot.png
    git commit -m "base"

    # Modify the binary file and stage it
    printf '\xde\xad\xbe\xef' >> snapshot.png
    git add snapshot.png

    # Modify the binary file again but do NOT stage this change
    printf '\xca\xfe\xba\xbe' >> snapshot.png

    # Record expected binary content (the unstaged version)
    local expected_size
    expected_size=$(wc -c < snapshot.png)
    local expected_md5
    expected_md5=$(md5sum snapshot.png | cut -d' ' -f1)

    # Commit (triggers pre-commit hook with stash)
    git commit -m "test"

    # Binary file should retain the unstaged version's content
    local actual_size
    actual_size=$(wc -c < snapshot.png)
    local actual_md5
    actual_md5=$(md5sum snapshot.png | cut -d' ' -f1)

    [ "$actual_size" -eq "$expected_size" ]
    [ "$actual_md5" = "$expected_md5" ]
}

@test "stash=git preserves untracked binary files" {
    create_config_with_stash "git"

    # Create and commit a text file
    echo "hello" > file.txt
    git add file.txt
    git commit -m "base"

    # Stage a text file change
    echo "world" > file.txt
    git add file.txt

    # Add an untracked binary file
    create_binary_file untracked.png

    local expected_size
    expected_size=$(wc -c < untracked.png)
    local expected_md5
    expected_md5=$(md5sum untracked.png | cut -d' ' -f1)

    # Commit
    git commit -m "test"

    # Untracked binary file should be preserved
    local actual_size
    actual_size=$(wc -c < untracked.png)
    local actual_md5
    actual_md5=$(md5sum untracked.png | cut -d' ' -f1)

    [ "$actual_size" -eq "$expected_size" ]
    [ "$actual_md5" = "$expected_md5" ]
}
