#!/usr/bin/env bash

# Common setup helpers for hk bats tests

# Create a test environment with a specific hk.pkl configuration
# Usage: setup_with_config '<pkl_content>'
setup_with_config() {
    local config_content="$1"

    # Ensure we're in the test directory
    cd "$TEST_TEMP_DIR/src/proj" || return 1

    # Write the config
    cat > hk.pkl <<< "$config_content"

    # Initialize git if needed
    if [[ ! -d .git ]]; then
        git init .
    fi
}

# Create a test environment with a specific builtin
# Usage: setup_with_builtin <builtin_name> [additional_config]
setup_with_builtin() {
    local builtin="$1"
    local additional_config="${2:-}"

    local config='amends "'"$PKL_PATH"'/Config.pkl"
import "'"$PKL_PATH"'/builtins/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["'"$builtin"'"] = Builtins.'"$builtin"'
    }
  }
}'

    if [[ -n "$additional_config" ]]; then
        config="$config
$additional_config"
    fi

    setup_with_config "$config"
}

# Set up a git repository with specific file states
# Usage: setup_with_git_state <state> [files...]
# States: staged, unstaged, untracked, committed, conflict
setup_with_git_state() {
    local state="$1"
    shift
    local files=("$@")

    # Default to test.txt if no files specified
    if [[ ${#files[@]} -eq 0 ]]; then
        files=("test.txt")
    fi

    # Ensure we're in a git repo
    if [[ ! -d .git ]]; then
        git init .
    fi

    case "$state" in
        staged)
            for file in "${files[@]}"; do
                echo "staged content" > "$file"
                git add "$file"
            done
            ;;
        unstaged)
            for file in "${files[@]}"; do
                echo "initial content" > "$file"
                git add "$file"
                git commit -m "Initial commit"
                echo "modified content" > "$file"
            done
            ;;
        untracked)
            for file in "${files[@]}"; do
                echo "untracked content" > "$file"
            done
            ;;
        committed)
            for file in "${files[@]}"; do
                echo "committed content" > "$file"
                git add "$file"
                git commit -m "Commit $file"
            done
            ;;
        conflict)
            # Create a merge conflict scenario
            echo "main content" > "${files[0]}"
            git add "${files[0]}"
            git commit -m "Main commit"
            git checkout -b feature
            echo "feature content" > "${files[0]}"
            git add "${files[0]}"
            git commit -m "Feature commit"
            git checkout main
            echo "different main content" > "${files[0]}"
            git add "${files[0]}"
            git commit -m "Different main commit"
            # Attempt merge (will create conflict)
            git merge feature || true
            ;;
        mixed)
            # Create a mix of states
            echo "staged" > staged.txt
            git add staged.txt

            echo "committed" > committed.txt
            git add committed.txt
            git commit -m "Initial"
            echo "modified" > committed.txt

            echo "untracked" > untracked.txt
            ;;
        *)
            echo "Unknown git state: $state" >&2
            return 1
            ;;
    esac
}

# Set up a multi-file project with specific extensions
# Usage: setup_project_with_files <type> [count]
# Types: javascript, typescript, python, rust, mixed
setup_project_with_files() {
    local type="$1"
    local count="${2:-5}"

    case "$type" in
        javascript)
            for ((i=1; i<=count; i++)); do
                mkdir -p "src"
                cat > "src/file$i.js" << 'EOF'
function example() {
    console.log("Hello from file");
}
EOF
            done
            ;;
        typescript)
            for ((i=1; i<=count; i++)); do
                mkdir -p "src"
                cat > "src/file$i.ts" << 'EOF'
function example(): void {
    console.log("Hello from file");
}
EOF
            done
            ;;
        python)
            for ((i=1; i<=count; i++)); do
                cat > "file$i.py" << 'EOF'
def example():
    print("Hello from file")
EOF
            done
            ;;
        rust)
            for ((i=1; i<=count; i++)); do
                mkdir -p "src"
                cat > "src/file$i.rs" << 'EOF'
fn example() {
    println!("Hello from file");
}
EOF
            done
            ;;
        mixed)
            setup_project_with_files javascript 2
            setup_project_with_files python 2
            setup_project_with_files rust 1
            ;;
        *)
            echo "Unknown project type: $type" >&2
            return 1
            ;;
    esac

    # Add all files to git
    git add -A
    git commit -m "Initial project setup" || true
}

# Set up a test with specific environment variables
# Usage: setup_with_env VAR1=value1 VAR2=value2 ...
setup_with_env() {
    for env_var in "$@"; do
        export "$env_var"
    done
}

# Set up a test with a specific hook configuration
# Usage: setup_with_hook <hook_name> <step_config>
setup_with_hook() {
    local hook_name="$1"
    local step_config="$2"

    local config='amends "'"$PKL_PATH"'/Config.pkl"

hooks {
  ["'"$hook_name"'"] {
    steps {
      $step_config
    }
  }
}'

    setup_with_config "$config"
}

# Set up test with multiple steps and dependencies
# Usage: setup_with_dependent_steps
setup_with_dependent_steps() {
    local config='amends "'"$PKL_PATH"'/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["step1"] {
        shell = "echo step1"
      }
      ["step2"] {
        shell = "echo step2"
        depends = ["step1"]
      }
      ["step3"] {
        shell = "echo step3"
        depends = ["step1", "step2"]
      }
    }
  }
}'

    setup_with_config "$config"
}

# Set up a test with a specific profile configuration
# Usage: setup_with_profile <profile_name> [active]
setup_with_profile() {
    local profile_name="$1"
    local active="${2:-false}"

    local config='amends "'"$PKL_PATH"'/Config.pkl"

hooks {
  [\"check\"] {
    steps {
      ["test_step"] {
        shell = "echo test"
        profiles = ["'"$profile_name"'"]
      }
    }
  }
}'

    setup_with_config "$config"

    if [[ "$active" == "true" ]]; then
        export HK_PROFILE="$profile_name"
    fi
}

# Set up with a failing step for error testing
# Usage: setup_with_failing_step [step_name] [error_message]
setup_with_failing_step() {
    local step_name="${1:-failing_step}"
    local error_msg="${2:-Error occurred}"

    local config='amends "'"$PKL_PATH"'/Config.pkl"

hooks {
  [\"check\"] {
    steps {
      ["'"$step_name"'"] {
        shell = "echo '"'"'$error_msg'"'"' >&2; exit 1"
      }
    }
  }
}'

    setup_with_config "$config"
}

# Set up with custom cache directory
# Usage: setup_with_custom_cache [cache_dir]
setup_with_custom_cache() {
    local cache_dir="${1:-$TEST_TEMP_DIR/custom_cache}"

    mkdir -p "$cache_dir"
    export HK_CACHE_DIR="$cache_dir"
    export HK_CACHE=1
}

# Clean up any test artifacts
# Usage: cleanup_test_artifacts
cleanup_test_artifacts() {
    # Remove any test-specific environment variables
    unset HK_PROFILE
    unset HK_SKIP_STEPS
    unset HK_FAIL_FAST

    # Clean up any background processes
    jobs -p | xargs -r kill 2>/dev/null || true

    # Reset to test directory
    cd "$TEST_TEMP_DIR/src/proj" 2>/dev/null || true
}

# Set up a test with timing tracking enabled
# Usage: setup_with_timing [json_file]
setup_with_timing() {
    local json_file="${1:-$TEST_TEMP_DIR/timing.json}"
    export HK_TIMING_JSON="$json_file"
}

# Create test files with specific content patterns
# Usage: create_test_files <pattern> <count>
# Patterns: valid, invalid, mixed
create_test_files() {
    local pattern="$1"
    local count="${2:-3}"

    case "$pattern" in
        valid)
            for ((i=1; i<=count; i++)); do
                echo "// Valid file $i" > "file$i.js"
                echo "console.log('test');" >> "file$i.js"
            done
            ;;
        invalid)
            for ((i=1; i<=count; i++)); do
                echo "// Invalid file $i" > "file$i.js"
                echo "console.log('test')" >> "file$i.js"  # Missing semicolon
                echo "const x = " >> "file$i.js"  # Syntax error
            done
            ;;
        mixed)
            echo "// Valid file" > "valid.js"
            echo "console.log('test');" >> "valid.js"

            echo "// Invalid file" > "invalid.js"
            echo "const x = " >> "invalid.js"
            ;;
        *)
            echo "Unknown pattern: $pattern" >&2
            return 1
            ;;
    esac
}

# Wait for background process with timeout
# Usage: wait_for_process <pid> [timeout_seconds]
wait_for_process() {
    local pid="$1"
    local timeout="${2:-10}"
    local count=0

    while kill -0 "$pid" 2>/dev/null; do
        if ((count >= timeout)); then
            echo "Process $pid did not complete within $timeout seconds" >&2
            return 1
        fi
        sleep 1
        ((count++))
    done
}

# Create a complex directory structure for testing
# Usage: setup_complex_directory_structure
setup_complex_directory_structure() {
    mkdir -p src/{components,utils,tests}
    mkdir -p docs
    mkdir -p config
    mkdir -p scripts

    echo "export default {}" > src/components/App.js
    echo "export const util = () => {}" > src/utils/helper.js
    echo "test('sample', () => {})" > src/tests/App.test.js
    echo "# Documentation" > docs/README.md
    echo "{}" > config/settings.json
    echo "#!/bin/bash" > scripts/build.sh
    chmod +x scripts/build.sh

    # Create some dotfiles
    echo "node_modules/" > .gitignore
    echo "{}" > .prettierrc
}
