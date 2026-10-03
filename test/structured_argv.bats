#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "structured argv supports a literal argv prefix" {
    cat <<'EOF' > capture-prefix
#!/bin/sh
printf '%s\n' "$@" > argv.log
EOF
    chmod +x capture-prefix
    touch 'a b.txt' 'semi;colon.txt'
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["capture"] {
                glob = "*.txt"
                prefix = List("{{root}}/capture-prefix", "prefix value", "\$HOME", "*")
                check = new Command {
                    argv = List("tool", "--flag", "{{files}}")
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_success

    run cat argv.log
    assert_success
    assert_line --index 0 'prefix value'
    assert_line --index 1 '$HOME'
    assert_line --index 2 '*'
    assert_line --index 3 'tool'
    assert_line --index 4 '--flag'
    assert_line 'a b.txt'
    assert_line 'semi;colon.txt'
    assert_equal "${#lines[@]}" 7
}

@test "structured argv preserves literal arguments and file boundaries" {
    cat <<'EOF' > capture-args
#!/bin/sh
printf '%s\n' "$@" > argv.log
EOF
    chmod +x capture-args
    touch 'a b.txt' 'semi;colon.txt'
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["capture"] {
                glob = "*.txt"
                check = new Command {
                    argv = List("{{root}}/capture-args", "\$HOME", "*", "{{files}}")
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_success

    run cat argv.log
    assert_success
    assert_line --index 0 '$HOME'
    assert_line --index 1 '*'
    assert_line 'a b.txt'
    assert_line 'semi;colon.txt'
    assert_equal "${#lines[@]}" 4
}

@test "structured argv prefix composes with builtins" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["ruff"] = (Builtins.ruff) {
                prefix = List("mise", "x", "--")
            }
            ["ruff_format"] = (Builtins.ruff_format) {
                prefix = List("mise", "x", "--")
            }
        }
    }
}
EOF

    run hk validate
    assert_success
}

@test "an argv prefix leaves out a builtin's shell check_diff instead of failing" {
    # yamlfmt's and taplo's check_diff and black's check_list_files are shell
    # scripts. They only make a fix faster, so under an argv prefix hk leaves
    # them out and runs `check` and `fix`.
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["yamlfmt"] = (Builtins.yamlfmt) {
                prefix = List("mise", "x", "--")
            }
            ["taplo_format"] = (Builtins.taplo_format) {
                prefix = List("mise", "x", "--")
            }
            ["black"] = (Builtins.black) {
                prefix = List("mise", "x", "--")
            }
            // Every go_lines command is argv, so nothing is left out.
            ["go_lines"] = (Builtins.go_lines) {
                prefix = List("mise", "x", "--")
            }
        }
    }
}
EOF

    run hk validate
    assert_success
}

@test "go_lines runs its check under an argv prefix" {
    # The prefix wraps go_lines' check, `hk util format-diff ... -- golines`,
    # which reports a file golines would change.
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["go_lines"] = (Builtins.go_lines) {
                prefix = List("env")
            }
        }
    }
}
EOF
    printf 'package name\n\nfunc a(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa int, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb int, cccccccccccccccccccccccccccccc int) int {\n\treturn aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n}\n' > a.go

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk check a.go
    assert_failure
    assert_output --partial "+	aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa int,"
}

@test "a missing argv command says it was not found" {
    touch a.txt
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = "*.txt"
                check = new Command {
                    argv = List("hk-no-such-tool-xyz", "{{files}}")
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_failure
    assert_output --partial "hk-no-such-tool-xyz: command not found; is it installed and on PATH?"
    refute_output --partial "No such file or directory"
}

@test "a missing argv command in an existing dir is still reported as a command" {
    mkdir sub
    touch sub/a.txt
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = "*.txt"
                dir = "sub"
                check = new Command {
                    argv = List("hk-no-such-tool-xyz")
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_failure
    assert_output --partial "hk-no-such-tool-xyz: command not found"
}
