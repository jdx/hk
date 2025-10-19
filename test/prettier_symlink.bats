#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "prettier should handle symlinks to markdown files without error" {
    # Create a properly formatted markdown file
    cat > README.md <<EOF
# Test

This is a test markdown file with some content.
EOF

    # Create a symlink pointing to the markdown file
    ln -s README.md README-symlink.md

    # Configure hk to run prettier
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["check"] {
    steps {
      ["prettier"] = Builtins.prettier
    }
  }
}
EOF

    git init
    git add .
    git commit -m "initial commit"

    # Run hk check --all (this should not fail due to symlink)
    run hk check --all
    
    # Should succeed
    assert_success
    
    # Should not contain error messages about symlinks
    refute_output --partial "symbolic link"
    refute_output --partial "duplicate"
    refute_output --partial "error"
}

@test "prettier should not process the same file twice via symlink" {
    # Create a properly formatted markdown file
    cat > README.md <<EOF
# Test

This is a test markdown file.
EOF

    # Create a symlink pointing to the markdown file
    ln -s README.md README-link.md

    # Configure hk to run prettier
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["check"] {
    steps {
      ["prettier"] = Builtins.prettier
    }
  }
}
EOF

    git init
    git add .
    git commit -m "initial commit"

    # Run hk check --all and capture output
    run hk check --all
    
    # Should succeed
    assert_success
    
    # Should not contain symlink errors
    refute_output --partial "symbolic link"
}

@test "prettier with symlinks to different files should work normally" {
    # Create two properly formatted markdown files
    cat > README.md <<EOF
# Test

This is the first markdown file.
EOF

    cat > docs.md <<EOF
# Documentation

This is the second markdown file.
EOF

    # Create symlinks
    ln -s README.md README-link.md
    ln -s docs.md docs-link.md

    # Configure hk to run prettier
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["check"] {
    steps {
      ["prettier"] = Builtins.prettier
    }
  }
}
EOF

    git init
    git add .
    git commit -m "initial commit"

    # Run hk check --all
    run hk check --all
    
    # Should succeed
    assert_success
    
    # Should not contain symlink errors
    refute_output --partial "symbolic link"
}
