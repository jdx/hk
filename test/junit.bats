#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

write_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
    ["check"] {
        steps {
            ["pass"] {
                check = "echo passed"
                output_summary = "combined"
            }
            ["fail"] {
                check = "echo diagnostic >&2; exit 1"
                output_summary = "combined"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init
}

@test "JUnit XML export works independently of the human output format" {
    write_config

    run hk check --all --junit-xml junit.xml
    assert_failure
    assert_file_exists junit.xml
    run python3 -c "import xml.etree.ElementTree as ET; ET.parse('junit.xml')"
    assert_success
    run bash -c "grep -F '<testsuite name=\"check\" tests=\"2\" failures=\"1\" errors=\"0\" skipped=\"0\"' junit.xml"
    assert_success
    run bash -c "grep -F '<testcase name=\"fail\"' junit.xml"
    assert_success
    run bash -c "grep -F '<failure message=\"step failed\">diagnostic' junit.xml"
    assert_success
}

@test "JUnit write errors are reported even when the hook also fails" {
    write_config

    run bash -c "hk --format json check --all --junit-xml hk.pkl/junit.xml 2>machine-errors.log"
    assert_failure
    run jq -e '.schema_version == 1 and .status == "failed"' <<<"$output"
    assert_success
    run grep -F "failed to emit result after hook also failed" machine-errors.log
    assert_success
    assert_file_not_exists hk.pkl/junit.xml
}

@test "skipped steps are reported as JUnit skipped test cases" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["files"] {
                check = "true"
                glob = List("*.rs")
            }
        }
    }
}
EOF
    git add .
    git commit -m init

    run hk check --all --junit-xml junit.xml
    assert_success
    run python3 -c "import xml.etree.ElementTree as ET; ET.parse('junit.xml')"
    assert_success
    run bash -c "grep -F '<skipped message=\"skipped: no files to process\"/>' junit.xml"
    assert_success
}

@test "early no-op runs still write a JUnit report" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps {} }
}
EOF
    git add .
    git commit -m init

    run hk check --all --junit-xml junit.xml
    assert_success
    assert_file_exists junit.xml
    run python3 -c "import xml.etree.ElementTree as ET; ET.parse('junit.xml')"
    assert_success
    run bash -c "grep -F '<testsuite name=\"check\" tests=\"0\" failures=\"0\" errors=\"0\" skipped=\"0\"' junit.xml"
    assert_success
}

@test "setup failures produce a synthetic JUnit error test case" {
    write_config
    mv .git .git.saved

    run hk check --all --junit-xml junit.xml
    assert_failure
    assert_file_exists junit.xml
    run python3 -c "import xml.etree.ElementTree as ET; ET.parse('junit.xml')"
    assert_success
    run bash -c "grep -F '<testsuite name=\"check\" tests=\"1\" failures=\"0\" errors=\"1\"' junit.xml"
    assert_success
    run bash -c "grep -F '<testcase name=\"check\" classname=\"hk\"' junit.xml"
    assert_success
}
