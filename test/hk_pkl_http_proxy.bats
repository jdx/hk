#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hide warnings: HK_HIDE_WARNINGS=missing-profiles suppresses profile skip warning" {
    cat <<EOF > hk.pkl
amends "package://example.com/v1.26.0/hk@1.26.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step"] {
                check = "echo 'I ran'"
            }
        }
    }
}
EOF

    HK_PKL_HTTP_REWRITE="https://example.com/=https://github.com/jdx/hk/releases/download/" run hk check
    assert_success
    assert_output --partial "I ran"
}

@test "failed package download does not print credentials from HK_PKL_HTTP_REWRITE" {
    # Local server that answers every request with 404 and reports its port.
    python3 -u -c '
import http.server
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(404)
        self.end_headers()
    def log_message(self, *args):
        pass
s = http.server.HTTPServer(("127.0.0.1", 0), H)
print(s.server_address[1], flush=True)
s.serve_forever()
' > server.port &
    server_pid=$!
    for _ in $(seq 50); do [ -s server.port ] && break; sleep 0.1; done
    port=$(cat server.port)

    cat <<EOF > hk.pkl
amends "package://example.com/v1.26.0/hk@1.26.0#/Config.pkl"
hooks { ["check"] { steps { ["step"] { check = "true" } } } }
EOF

    HK_PKL_EMBEDDED=0 HK_PKL_CACHE_DIR="$PWD/pkl-cache" \
        HK_PKL_HTTP_REWRITE="https://example.com/=http://alice:s3cret@127.0.0.1:$port/" run hk check
    kill "$server_pid"
    assert_failure
    assert_output --partial "127.0.0.1:$port"
    refute_output --partial "s3cret"
    refute_output --partial "alice"
}
