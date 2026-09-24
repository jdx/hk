#!/usr/bin/env bats
setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "renovate_deps Flint checker tests run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["renovate_deps"] = Builtins.renovate_deps
    }
  }
}
PKL

    git status --short >/dev/null
    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step renovate_deps
    assert_success
    assert_output --partial "ok - renovate_deps :: check detects stale snapshot"
    assert_output --partial "ok - renovate_deps :: fix regenerates snapshot"
}

@test "renovate_deps fix stages its regenerated snapshot" {
    mkdir -p .github "$TEST_TEMP_DIR/bin"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["fix"] {
    stage = true
    steps {
      ["renovate_deps"] = Builtins.renovate_deps
    }
  }
}
PKL
    cat <<'SH' > "$TEST_TEMP_DIR/bin/flint"
#!/bin/sh
set -eu
case " $* " in
    *" --fix "*) printf '{"files": {"package.json": {"npm": ["express"]}}}\n' > .github/renovate-tracked-deps.json ;;
esac
SH
    chmod +x "$TEST_TEMP_DIR/bin/flint"
    printf '{"dependencies":{"express":"^4.18.0"}}\n' > package.json
    printf '{"files": {}}\n' > .github/renovate-tracked-deps.json
    git add package.json .github/renovate-tracked-deps.json
    git commit -m "baseline"
    printf '{"dependencies":{"express":"^4.19.0"}}\n' > package.json

    PATH="$TEST_TEMP_DIR/bin:$PATH"
    run hk fix --all --step renovate_deps
    assert_success
    run git diff --cached --name-only
    assert_output --partial ".github/renovate-tracked-deps.json"
}
