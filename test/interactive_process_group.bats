#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# A TUI that switches the terminal to raw mode is stopped by SIGTTOU/SIGTTIN when
# it runs in a background process group, so interactive steps must share hk's.
@test "interactive step runs in hk's process group" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["interactive"] {
                interactive = true
                check = "ps -o pgid= -p \$\$ > interactive.pgid; ps -o pgid= -p \$PPID > hk.pgid"
            }
            ["plain"] {
                depends = List("interactive")
                check = "ps -o pgid= -p \$\$ > plain.pgid; ps -o pgid= -p \$PPID > plain-hk.pgid"
            }
        }
    }
}
PKL
    echo "content" > file.txt
    git add hk.pkl file.txt

    run hk check
    assert_success

    [ -n "$(tr -d ' \n' < interactive.pgid)" ]
    [ "$(tr -d ' \n' < interactive.pgid)" = "$(tr -d ' \n' < hk.pgid)" ]
    # Control: other steps still get their own process group for tree kills.
    [ "$(tr -d ' \n' < plain.pgid)" != "$(tr -d ' \n' < plain-hk.pgid)" ]
}
