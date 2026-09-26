#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Run hk with the given stream (1 or 2) piped to a reader that has already
# closed it, so hk's first write to that stream fails with EPIPE. hk starts only
# after the reader signals through the FIFO, so the order is deterministic.
# Prints hk's exit status; hk's other output stream goes to other.txt.
run_with_closed_reader() {
    local stream=$1
    shift
    mkfifo reader_closed
    local redirect='2>other.txt'
    [[ $stream == 2 ]] && redirect='2>&1 >other.txt'
    set +o pipefail
    {
        read -r _ <reader_closed
        eval 'exec hk "$@"' "$redirect"
    } | {
        exec 0<&-
        echo >reader_closed
    }
    echo "${PIPESTATUS[0]}"
}

@test "hk exits like SIGPIPE when stdout's reader has gone" {
    for args in "--version" "--help" "usage" "completion bash" \
        "__complete_word__ --shell bash --line hk\ c --bash-word c"; do
        rm -f reader_closed
        # shellcheck disable=SC2086
        run run_with_closed_reader 1 $args
        assert_output "141"
        run cat other.txt
        refute_output --partial "panicked"
    done
}

@test "hk exits like SIGPIPE when stderr's reader has gone" {
    run run_with_closed_reader 2 --no-such-flag
    assert_output "141"
}
