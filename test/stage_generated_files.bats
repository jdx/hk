#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "fix step stages generated files outside glob" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["fooment-protos"] {
        glob = List("config/logging/schema_foomake.ts")
        stage = List("fooment/schemas/**", "config/logging/generated/frontend_schema.ts")
        fix = "mkdir -p fooment/schemas config/logging/generated && echo generated > fooment/schemas/generated.proto && echo schema > config/logging/generated/frontend_schema.ts"
      }
    }
  }
}
EOF
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"

    mkdir -p config/logging
    cat <<'TS' > config/logging/schema_foomake.ts
export const schema = 'initial'
TS
    git add config/logging/schema_foomake.ts
    git -c commit.gpgsign=false commit -m "add schema"

    cat <<'TS' > config/logging/schema_foomake.ts
export const schema = 'changed'
TS

    run hk fix -v
    assert_success

    run git status --porcelain -- fooment/schemas/generated.proto config/logging/generated/frontend_schema.ts
    assert_success
    assert_line --regexp '^A  fooment/schemas/generated\.proto$'
    assert_line --regexp '^A  config/logging/generated/frontend_schema\.ts$'
}
