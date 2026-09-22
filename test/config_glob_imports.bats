#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# A generated step module contributing its own STEPS mapping. The dotted file
# name guards against deriving the step key from the path.
_write_generated_step() {
    mkdir -p generated
    cat <<EOF > generated/one.step.pkl
import "$PKL_PATH/Config.pkl"

STEPS: Mapping<String, Config.Step> = new {
    ["one"] {
        glob = List("*.js")
        check = "echo $1 {{files}}"
    }
}
EOF
}

@test "glob import expression defines steps" {
    _write_generated_step one
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

local generated = import*("generated/*.pkl")

hooks {
    ["check"] {
        steps = new Mapping<String, Step> {
            for (_, mod in generated) {
                ...mod.STEPS
            }
        }
    }
}
EOF
    echo "test" > test.js
    git init
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "one test.js"
}

@test "glob import declaration defines steps" {
    _write_generated_step one
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

import* "generated/*.pkl" as Generated

hooks {
    ["check"] {
        steps = new Mapping<String, Step> {
            for (_, mod in Generated) {
                ...mod.STEPS
            }
        }
    }
}
EOF
    echo "test" > test.js
    git init
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "one test.js"
}

@test "editing a glob-imported file invalidates the config cache" {
    _write_generated_step one
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

local generated = import*("generated/*.pkl")

hooks {
    ["check"] {
        steps = new Mapping<String, Step> {
            for (_, mod in generated) {
                ...mod.STEPS
            }
        }
    }
}
EOF
    echo "test" > test.js
    git init
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "one test.js"

    _write_generated_step edited
    run hk check --all
    assert_success
    assert_output --partial "edited test.js"
}

@test "glob import matching nothing leaves the hook empty" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

local generated = import*("generated/*.pkl")

hooks {
    ["check"] {
        steps = new Mapping<String, Step> {
            for (_, mod in generated) {
                ...mod.STEPS
            }
        }
    }
}
EOF
    echo "test" > test.js
    git init
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    # An empty steps mapping means hk has nothing to run at all, which is a
    # stronger claim than the generated step merely being absent.
    assert_output --partial "no steps to run"
    refute_output --partial "one test.js"
}
