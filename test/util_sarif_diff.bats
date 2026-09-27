#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# Writes a tool that prints $1 as its SARIF log and exits with $2.
fake_tool() {
    printf '%s' "$1" > sarif.json
    cat <<SCRIPT > tool.sh
#!/bin/sh
cat sarif.json
exit ${2:-1}
SCRIPT
    chmod +x tool.sh
}

fix_line() {
    # A result replacing line $2 of $1 with $3.
    printf '{"message":{"text":"pin it"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"%s"},"region":{"startLine":%s}}}],"fixes":[{"artifactChanges":[{"artifactLocation":{"uri":"%s"},"replacements":[{"deletedRegion":{"startLine":%s},"insertedContent":{"text":"%s"}}]}]}]}' "$1" "$2" "$1" "$2" "$3"
}

@test "util sarif-diff turns SARIF fixes into a patch that git apply accepts" {
    printf 'one\ntwo\nthree\n' > a.txt
    fake_tool "{\"runs\":[{\"results\":[$(fix_line a.txt 2 TWO)]}]}"

    run --separate-stderr hk util sarif-diff -- ./tool.sh
    assert_failure 1
    assert_output $'--- a.txt\n+++ a.txt\n@@ -1,3 +1,3 @@\n one\n-two\n+TWO\n three'

    echo "$output" | git apply -p0
    run cat a.txt
    assert_output $'one\nTWO\nthree'
}

@test "util sarif-diff exits 0 when there are no results" {
    fake_tool '{"runs":[{"results":[]}]}' 0

    run hk util sarif-diff -- ./tool.sh
    assert_success
    refute_output
}

@test "util sarif-diff prints no patch when any result has no fix" {
    printf 'one\ntwo\n' > a.txt
    fake_tool "{\"runs\":[{\"results\":[$(fix_line a.txt 1 ONE),{\"message\":{\"text\":\"unfixable\"},\"locations\":[{\"physicalLocation\":{\"artifactLocation\":{\"uri\":\"a.txt\"},\"region\":{\"startLine\":2}}}]}]}]}"

    run --separate-stderr hk util sarif-diff -- ./tool.sh
    assert_failure 1
    refute_output
    [[ "$stderr" == *"a.txt:2: unfixable (has no fix)"* ]]
}

@test "util sarif-diff passes on the tool's failure when it prints no SARIF log" {
    cat <<'SCRIPT' > tool.sh
#!/bin/sh
echo "network error" >&2
exit 4
SCRIPT
    chmod +x tool.sh

    run --separate-stderr hk util sarif-diff -- ./tool.sh
    assert_failure 4
    refute_output
    [[ "$stderr" == *"network error"* ]]
}

@test "a check_diff built with util sarif-diff is applied instead of running the fixer" {
    printf 'one\ntwo\n' > a.txt
    fake_tool "{\"runs\":[{\"results\":[$(fix_line a.txt 2 TWO)]}]}"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["sarif"] {
                glob = List("*.txt")
                check_diff = "hk util sarif-diff -- ./tool.sh"
                fix = "for f in {{files}}; do echo fixer-ran > \"\$f\"; done"
            }
        }
    }
}
EOF

    run hk fix a.txt
    assert_success
    run cat a.txt
    assert_output $'one\nTWO'
}
