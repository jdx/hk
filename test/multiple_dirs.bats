#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "steps with different dir settings can process same files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["packwerk-check"] {
                dir = "sinatra"
                glob = List("**/*.rb")
                check = "echo 'checking {{files}} from sinatra dir'"
            }
            ["prettier-check"] {
                glob = List("**/*.rb")
                check = "echo 'checking {{files}} from root dir'"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Create the directory structure
    mkdir -p sinatra/controllers/api
    echo "class ApiFolder" > sinatra/controllers/api/api_folder.rb
    echo "end" >> sinatra/controllers/api/api_folder.rb

    git add sinatra/controllers/api/api_folder.rb

    run hk check -v
    assert_success

    # Verify packwerk-check runs from sinatra dir with relative path
    assert_output --partial "checking controllers/api/api_folder.rb from sinatra dir"

    # Verify prettier-check runs from root dir with full path
    assert_output --partial "checking sinatra/controllers/api/api_folder.rb from root dir"
}

@test "steps with dir setting only process files in that directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["sinatra-only"] {
                dir = "sinatra"
                glob = List("**/*.rb")
                check = "echo 'found {{files}}'"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Create files in different directories
    mkdir -p sinatra/models
    mkdir -p other/models
    echo "class SinatraModel" > sinatra/models/test.rb
    echo "class OtherModel" > other/models/test.rb
    echo "class RootModel" > root.rb

    git add sinatra/models/test.rb other/models/test.rb root.rb

    run hk check -v
    assert_success

    # Should only process the file in sinatra directory
    assert_output --partial "found models/test.rb"
    # Should not process files outside sinatra directory (in the actual step output)
    refute_output --partial "found other/models/test.rb"
    refute_output --partial "found root.rb"
}

@test "no path stripping issues with multiple dir steps" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step-with-dir"] {
                dir = "subdir"
                glob = List("*.txt")
                check = "test -f {{files}}"
            }
            ["step-without-dir"] {
                glob = List("**/*.txt")
                check = "test -f {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Create test file
    mkdir -p subdir
    echo "test content" > subdir/test.txt
    git add subdir/test.txt

    run hk check -v
    assert_success

    # Both steps should succeed - no "file not found" errors
    refute_output --partial "No such file or directory"
    refute_output --partial "not found"
}

@test "complex scenario with multiple nested dirs" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["backend-check"] {
                dir = "backend"
                glob = List("**/*.py")
                check = "echo 'backend: {{files}}'"
            }
            ["frontend-check"] {
                dir = "frontend"
                glob = List("**/*.js")
                check = "echo 'frontend: {{files}}'"
            }
            ["global-check"] {
                glob = List("**/*.{py,js}")
                check = "echo 'global: {{files}}'"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Create complex directory structure
    mkdir -p backend/api/controllers
    mkdir -p frontend/src/components
    echo "def hello():" > backend/api/controllers/main.py
    echo "console.log('hello');" > frontend/src/components/main.js

    git add backend/api/controllers/main.py frontend/src/components/main.js

    run hk check -v
    assert_success

    # Verify each step processes files correctly
    assert_output --partial "backend: api/controllers/main.py"
    assert_output --partial "frontend: src/components/main.js"
    # Global step should process both files (may be in one command)
    assert_output --partial "global: backend/api/controllers/main.py"
    assert_output --partial "frontend/src/components/main.js"
}

