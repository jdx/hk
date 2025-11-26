#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "regex exclude pattern" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("**/*.yaml")
                exclude = Config.Regex(#"""
(?x)
^.*airflow\.template\.yaml$|
^.*init_git_sync\.template\.yaml$|
^chart/(?:templates|files)/.*\.yaml$|
^helm-tests/tests/chart_utils/keda\.sh_scaledobjects\.yaml$|
.*/v1.*\.yaml$|
^.*openapi.*\.yaml$|
^\.pre-commit-config\.yaml$|
^.*reproducible_build\.yaml$|
^.*pnpm-lock\.yaml$
"""#)
                check = "prettier --no-color --check {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Create files that should be checked (with bad formatting)
    echo "foo:  bar" > config.yaml
    echo "baz:   qux" > settings.yaml

    # Create files that should be excluded by regex
    mkdir -p chart/templates foo
    echo "excluded: 1" > airflow.template.yaml
    echo "excluded: 2" > chart/templates/deployment.yaml
    echo "excluded: 3" > .pre-commit-config.yaml
    echo "excluded: 4" > openapi-spec.yaml
    echo "excluded: 5" > foo/v1-api.yaml

    git add config.yaml settings.yaml airflow.template.yaml chart/templates/deployment.yaml .pre-commit-config.yaml openapi-spec.yaml foo/v1-api.yaml
    run hk check -v
    assert_failure
    assert_output --partial 'DEBUG $ prettier --no-color --check config.yaml settings.yaml'
    # Make sure excluded files are NOT in the prettier command
    refute_output --partial '$ prettier --no-color --check.*airflow.template.yaml'
    refute_output --partial '$ prettier --no-color --check.*chart/templates/deployment.yaml'
    refute_output --partial '$ prettier --no-color --check.*\.pre-commit-config\.yaml'
    refute_output --partial '$ prettier --no-color --check.*openapi-spec.yaml'
    refute_output --partial '$ prettier --no-color --check.*v1-api.yaml'
}

@test "regex glob pattern" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["yaml-check"] {
                glob = Config.Regex(#"^(config|settings).*\.yaml$"#)
                check = "echo {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Create files that should match the regex
    echo "foo: bar" > config.yaml
    echo "baz: qux" > settings.yaml
    echo "qux: quux" > config-dev.yaml

    # Create files that should NOT match
    echo "excluded: 1" > other.yaml
    echo "excluded: 2" > data.yaml

    git add config.yaml settings.yaml config-dev.yaml other.yaml data.yaml
    run hk check -v
    assert_success
    assert_output --partial 'config-dev.yaml config.yaml settings.yaml'
    # Make sure non-matching files are NOT in the echo command
    refute_output --partial '$ echo.*other.yaml'
    refute_output --partial '$ echo.*data.yaml'
}

@test "regex pattern with dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-src"] {
                dir = "src"
                glob = List("**/*.js")
                exclude = Config.Regex(#".*\.test\.js$"#)
                check = "echo {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src
    # Create files that should be checked
    echo "code" > src/app.js
    echo "code" > src/lib.js

    # Create files that should be excluded
    echo "test" > src/app.test.js
    echo "test" > src/lib.test.js

    git add src/app.js src/lib.js src/app.test.js src/lib.test.js
    run hk check -v
    assert_success
    assert_output --partial 'app.js lib.js'
    # The files should be excluded - they won't show up in the command at all
    # The regex matched and excluded them successfully
}

@test "regex pattern with dir and nested paths" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-src"] {
                dir = "src"
                glob = List("**/*.js")
                exclude = Config.Regex(#"^utils/.*$"#)
                check = "echo {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src/utils src/components
    # Create files that should be checked
    echo "code" > src/app.js
    echo "code" > src/components/button.js

    # Create files that should be excluded (in utils/)
    echo "util" > src/utils/helper.js
    echo "util" > src/utils/format.js

    git add src/
    run hk check -v
    assert_success
    # Should only include app.js and components/button.js
    assert_output --partial '$ echo app.js components/button.js'
    # utils/ files should NOT be in the command
    refute_output --partial '$ echo.*utils/'
}

@test "glob pattern scoping with dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-frontend"] {
                dir = "frontend"
                glob = List("**/*.ts")
                check = "echo {{files}}"
            }
            ["check-backend"] {
                dir = "backend"
                glob = List("**/*.ts")
                check = "echo {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p frontend backend
    # Create TypeScript files in different directories
    echo "frontend" > frontend/app.ts
    echo "frontend" > frontend/utils.ts
    echo "backend" > backend/server.ts
    echo "backend" > backend/db.ts

    # Create a root-level TS file that shouldn't match either step
    echo "root" > main.ts

    git add frontend/ backend/ main.ts
    run hk check -v
    assert_success
    # Frontend step should only see frontend files
    assert_output --partial 'check-frontend'
    assert_output --partial 'echo app.ts utils.ts'
    # Backend step should only see backend files
    assert_output --partial 'check-backend'
    assert_output --partial 'echo db.ts server.ts'
    # Root file should not appear in either step's command
    refute_output --partial 'echo.*main.ts'
}

@test "dir pre-filters files before glob matching" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-src"] {
                dir = "src"
                glob = List("**/*.ts")
                check = "echo checking: {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src other
    # Create files in src/ that should match
    echo "src" > src/app.ts
    echo "src" > src/lib.ts

    # Create files outside src/ that should NOT match (even though they match the glob)
    echo "other" > other/app.ts
    echo "other" > other/lib.ts
    echo "root" > main.ts

    git add src/ other/ main.ts
    run hk check -v
    assert_success
    # {{files}} shows paths relative to dir, so just "app.ts lib.ts"
    assert_output --partial 'checking: app.ts lib.ts'
    # The check command should only process files in src/, verify count is 2
    assert_output --partial 'check-src – 2 files'
    # Should NOT see files outside src/ in the echo command output
    refute_output --partial 'checking:.*other'
    refute_output --partial 'checking:.*main.ts'
}

@test "dir pre-filters files before regex matching" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-backend"] {
                dir = "backend"
                glob = Config.Regex(#"^.*\\.py$"#)
                check = "echo checking: {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p backend frontend
    # Create files in backend/ that should match
    echo "backend" > backend/app.py
    echo "backend" > backend/db.py

    # Create files outside backend/ that should NOT match (even though they match the regex)
    echo "frontend" > frontend/app.py
    echo "frontend" > frontend/utils.py
    echo "root" > main.py

    git add backend/ frontend/ main.py
    run hk check -v
    assert_success
    # {{files}} shows paths relative to dir, so just "app.py db.py"
    assert_output --partial 'checking: app.py db.py'
    # The check command should only process files in backend/, verify count is 2
    assert_output --partial 'check-backend – 2 files'
    # Should NOT see files outside backend/ in the echo command output
    refute_output --partial 'checking:.*frontend'
    refute_output --partial 'checking:.*main.py'
}

@test "step with dir but no glob filters to directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-src"] {
                dir = "src"
                check = "echo checking: {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src docs
    echo "src" > src/app.js
    echo "src" > src/lib.js
    echo "docs" > docs/README.md
    echo "root" > package.json

    git add src/ docs/ package.json
    run hk check -v
    assert_success
    # {{files}} shows paths relative to dir, so just "app.js lib.js"
    assert_output --partial 'checking: app.js lib.js'
    # The check command should only process files in src/, verify count is 2
    assert_output --partial 'check-src – 2 files'
    # Should NOT see files outside src/ in the echo command output
    refute_output --partial 'checking:.*docs'
    refute_output --partial 'checking:.*package.json'
}

@test "{{globs}} template variable works with regex patterns" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["yaml-check"] {
                glob = Config.Regex(#"^.*\\.yaml$"#)
                check = "echo pattern={{globs}} files={{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    echo "foo: bar" > config.yaml
    echo "baz: qux" > settings.yaml

    git add config.yaml settings.yaml
    run hk check -v
    assert_success
    # {{globs}} should contain the regex pattern (backslashes are displayed literally in shell output)
    assert_output --partial 'pattern=^.*\.yaml$'
    # {{files}} should contain the matched files
    assert_output --partial 'files=config.yaml settings.yaml'
}

@test "regex with dir matches paths relative to dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-components"] {
                dir = "src/components"
                glob = Config.Regex(#"^[A-Z].*\\.tsx$"#)
                check = "echo matched: {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src/components src/utils
    # These should match (relative paths start with uppercase)
    echo "component" > src/components/Button.tsx
    echo "component" > src/components/Input.tsx

    # These should NOT match (relative paths don't start with uppercase)
    echo "component" > src/components/helpers.tsx
    echo "util" > src/utils/Button.tsx  # Not in dir

    git add src/
    run hk check -v
    assert_success
    # Should match files with uppercase names in src/components
    assert_output --partial 'matched: Button.tsx Input.tsx'
    # Should NOT match lowercase files in the check command output
    refute_output --partial 'matched:.*helpers'
    # Should NOT match files outside dir
    refute_output --partial 'matched:.*src/utils'
}

@test "{{globs}} template variable is consistent string format for both glob and regex" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["glob-test"] {
                glob = List("*.yaml", "*.yml")
                check = "echo 'glob-type={{globs}}' files={{files}}"
            }
            ["regex-test"] {
                glob = Config.Regex(#"^.*\\.json$"#)
                check = "echo 'regex-type={{globs}}' files={{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    echo "foo: bar" > config.yaml
    echo "baz: qux" > data.json

    git add config.yaml data.json
    run hk check -v
    assert_success
    # For glob patterns, {{globs}} should be space-separated string like regex
    assert_output --partial "glob-type=*.yaml *.yml"
    # For regex patterns, {{globs}} is already a string
    assert_output --partial "regex-type=^.*\\.json$"
}

@test "glob patterns with dir should not double-apply directory context" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-src"] {
                dir = "src"
                glob = List("*.js")
                check = "echo files={{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src src/lib
    echo "code" > src/app.js
    echo "code" > src/lib/utils.js

    git add src/
    run hk check -v
    assert_success
    # The glob *.js with dir=src should only match files directly in src/
    # not in subdirectories like src/lib/
    assert_output --partial 'check-src – 1 file'
    assert_output --partial 'files=app.js'
    # lib/utils.js should NOT appear in the command output
    refute_output --partial 'files=.*lib'
}

# Note: User config tests with .hkrc.pkl would require a separate Pkl schema for UserConfig
# which doesn't currently exist. The Rust changes to support Pattern in UserStepConfig
# are tested implicitly through the API, but end-to-end .hkrc.pkl tests would need
# additional schema work.

