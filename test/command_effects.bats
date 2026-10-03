#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

write_config() {
    local first_effect="$1"
    local second_effect="$2"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["first"] {
                check = new CommandSpec {
                    command = "touch first-ran"
                    effect = "$first_effect"
                }
            }
            ["second"] {
                check = new CommandSpec {
                    command = "touch second-ran"
                    effect = "$second_effect"
                }
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init
}

@test "safe mode permits read and write effects" {
    write_config read write

    run hk check --all --safe
    assert_success
    assert_file_exists first-ran
    assert_file_exists second-ran
}

write_legacy_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["known"] {
                check = new CommandSpec {
                    command = "touch known-ran"
                    effect = "read"
                }
            }
            ["legacy"] {
                check = "touch legacy-ran"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init
}

@test "safe mode rejects legacy commands before any step starts" {
    write_legacy_config

    run hk check --all --safe
    assert_failure
    assert_output --partial "legacy.check: effect is unknown"
    assert_file_not_exists known-ran
    assert_file_not_exists legacy-ran
}

@test "safe mode ignores explicitly skipped legacy commands" {
    write_legacy_config

    run hk check --all --safe --skip-step legacy
    assert_success
    assert_file_exists known-ran
    assert_file_not_exists legacy-ran
}

@test "safe mode validates plans without running steps" {
    write_legacy_config

    run hk check --all --plan --safe

    assert_failure
    assert_output --partial "legacy.check: effect is unknown"
    assert_file_not_exists legacy-ran
}

@test "safe mode rejects destructive commands before any step starts" {
    write_config read destructive

    run hk check --all --safe
    assert_failure
    assert_output --partial "second.check: effect is destructive"
    assert_file_not_exists first-ran
    assert_file_not_exists second-ran
}

@test "plans and structured results include effects" {
    write_config read write

    run bash -c "hk check --all --plan --json 2>/dev/null"
    assert_success
    run jq -r '.steps[] | [.name, .metadata.effect] | @tsv' <<<"$output"
    assert_success
    assert_output $'first\tread\nsecond\twrite'

    run bash -c "hk --format json check --all --safe 2>/dev/null"
    assert_success
    run jq -r '.steps[] | [.name, .effects[0].effect] | @tsv' <<<"$output"
    assert_success
    assert_output $'first\tread\nsecond\twrite'
}

@test "safe mode validates dynamically enabled check-first commands" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["dynamic"] {
                check_diff = new CommandSpec {
                    command = "touch check-first-ran"
                    effect = "destructive"
                }
                fix = new CommandSpec {
                    command = "touch fix-ran"
                    effect = "write"
                }
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run hk check --fix --all --safe
    assert_failure
    assert_output --partial "dynamic.check_diff: effect is destructive"
    assert_file_not_exists check-first-ran
    assert_file_not_exists fix-ran
}

@test "safe mode validates checks configured after diff application" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["partial"] {
                check_diff = new CommandSpec {
                    command = "false"
                    effect = "read"
                }
                check = new CommandSpec {
                    command = "touch check-ran"
                    effect = "destructive"
                }
                check_after_diff = true
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m "test: initialize fixture"

    run hk fix --all --safe
    assert_failure
    assert_output --partial "partial.check: effect is destructive"
    assert_file_not_exists check-ran
}

@test "safe mode ignores check-first commands that no job will use" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["dynamic"] {
                check_first = true
                check = "touch check-first-ran"
                fix = new CommandSpec {
                    command = "touch fix-ran"
                    effect = "write"
                }
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run hk check --fix --all --safe
    assert_success
    assert_file_not_exists check-first-ran
    assert_file_exists fix-ran
}

@test "safe mode ignores a command unavailable on this platform" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["platform-only"] {
                check = new CommandSpec {
                    command = new Script { windows = "touch should-not-run" }
                    effect = "destructive"
                }
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run hk check --all --safe
    assert_success
    assert_file_not_exists should-not-run
}

@test "structured results report only commands selected for execution" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["dynamic"] {
                check_diff = new CommandSpec {
                    command = "true"
                    effect = "read"
                }
                fix = new CommandSpec {
                    command = "touch fix-ran"
                    effect = "write"
                }
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run bash -c "hk --format json check --fix --all --safe 2>/dev/null"
    assert_success
    run jq -r '.steps[0].effects | map([.command, .effect] | join(":")) | join(",")' <<<"$output"
    assert_success
    assert_output "check_diff:read"
    assert_file_not_exists fix-ran

    run bash -c "hk check --fix --all --plan --json 2>/dev/null"
    assert_success
    run jq -r '.steps[0].metadata.effect' <<<"$output"
    assert_success
    # The plan is conservative: check_diff can fail to apply and fall back to
    # the write-effect fixer even though this successful run did not.
    assert_output "write"
}

@test "safe mode rejects a hook report with no declared effect" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        report = "rm -f a.txt"
        steps {
            ["known"] {
                check = new CommandSpec {
                    command = "touch known-ran"
                    effect = "read"
                }
            }
        }
    }
}
EOF2
    touch input.txt a.txt
    git add .
    git commit -m init

    run hk check --all --safe
    assert_failure
    assert_output --partial "report: effect is unknown"
    assert_file_exists a.txt
    assert_file_not_exists known-ran
}

@test "safe mode rejects a destructive hook report" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        report = new CommandSpec { command = "rm -f a.txt"; effect = "destructive" }
        steps {
            ["known"] {
                check = new CommandSpec {
                    command = "touch known-ran"
                    effect = "read"
                }
            }
        }
    }
}
EOF2
    touch input.txt a.txt
    git add .
    git commit -m init

    run hk check --all --safe
    assert_failure
    assert_output --partial "report: effect is destructive"
    assert_file_exists a.txt
    assert_file_not_exists known-ran
}

@test "safe mode runs a hook report that declares a read effect" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        report = new CommandSpec { command = "echo \"\$HK_REPORT_JSON\" > report-ran"; effect = "read" }
        steps {
            ["known"] {
                check = new CommandSpec {
                    command = "touch known-ran"
                    effect = "read"
                }
            }
        }
    }
}
EOF2
    touch input.txt a.txt
    git add .
    git commit -m init

    run hk check --all --safe
    assert_success
    assert_file_exists report-ran
}

@test "hook report accepts a structured argv command with an effect" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        report = new CommandSpec { command = new Command { argv = List("touch", "report-ran") }; effect = "write" }
        steps {
            ["known"] {
                check = new CommandSpec {
                    command = "touch known-ran"
                    effect = "read"
                }
            }
        }
    }
}
EOF2
    touch input.txt a.txt
    git add .
    git commit -m init

    run hk check --all --safe
    assert_success
    assert_file_exists report-ran
}

@test "hook report rejects a structured argv command with no executable" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        report = new CommandSpec { command = new Command { argv = List() }; effect = "write" }
        steps {
            ["known"] {
                check = new CommandSpec {
                    command = "touch known-ran"
                    effect = "read"
                }
            }
        }
    }
}
EOF2
    touch input.txt a.txt
    git add .
    git commit -m init

    run hk check --all --safe
    assert_failure
    assert_output --partial "report: structured argv command must contain an executable"
    assert_file_not_exists known-ran
}

@test "plain string hook report still runs outside safe mode" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        report = "touch report-ran"
        steps {
            ["known"] {
                check = new CommandSpec {
                    command = "touch known-ran"
                    effect = "read"
                }
            }
        }
    }
}
EOF2
    touch input.txt a.txt
    git add .
    git commit -m init

    run hk check --all
    assert_success
    assert_file_exists report-ran
}
