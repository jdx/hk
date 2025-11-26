#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "binary file detection works correctly" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["test"] {
                glob = "**/*"
                check = "echo {{files}}"
            }
        }
    }
}
EOF

    # Create a text file
    echo "Hello World" > text.txt
    git add text.txt

    # Create a binary file (with null bytes)
    printf '\x00\x01\x02\x03' > binary.dat
    git add binary.dat

    run hk check
    assert_success

    # Text file should be in output
    assert_output --partial "text.txt"

    # Binary file should NOT be in output (filtered by default)
    refute_output --partial "binary.dat"
}

@test "binary file detection caches results" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["test"] {
                glob = "**/*"
                check = "echo {{files}}"
            }
        }
    }
}
EOF

    # Create multiple text files
    echo "File 1" > text1.txt
    echo "File 2" > text2.txt
    echo "File 3" > text3.txt
    git add text1.txt text2.txt text3.txt

    # Create multiple binary files (should be filtered)
    printf '\x00\x01\x02\x03' > binary1.dat
    printf '\x00\x01\x02\x03' > binary2.dat
    git add binary1.dat binary2.dat

    # Run check twice - second run should hit cache
    run hk check
    assert_success

    # Text files should be in output
    assert_output --partial "text1.txt"
    assert_output --partial "text2.txt"
    assert_output --partial "text3.txt"

    # Binary files should NOT be in output
    refute_output --partial "binary1.dat"
    refute_output --partial "binary2.dat"
}

