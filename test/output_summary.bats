#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "output_summary default (stderr) prints only stderr" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["s"] {
        // default output_summary
        check = "echo OUT && echo ERR 1>&2"
      }
    }
  }
}
EOF
    HK_SUMMARY_TEXT=1 run hk check
    assert_success
    # Summary should include stderr header and content
    assert_output --partial "s stderr:"
    refute_output --partial "s stdout:"
    assert_output --partial "ERR"
}

@test "output_summary stdout prints only stdout" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["s"] {
        output_summary = "stdout"
        check = "echo OUT && echo ERR 1>&2"
      }
    }
  }
}
EOF
    HK_SUMMARY_TEXT=1 run hk check
    assert_success
    assert_output --partial "s stdout:"
    refute_output --partial "s stderr:"
    assert_output --partial "OUT"
}

@test "output_summary combined prints interleaved output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["s"] {
        output_summary = "combined"
        check = "echo O1; echo E1 1>&2; echo O2; echo E2 1>&2"
      }
    }
  }
}
EOF
    HK_SUMMARY_TEXT=1 run hk check
    assert_success
    assert_output --partial "s output:"
    # We can't perfectly assert interleaving with lines, but ensure both appear
    assert_output --partial "O1"
    assert_output --partial "E1"
    assert_output --partial "O2"
    assert_output --partial "E2"
}

@test "output_summary hide prints nothing" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["s"] {
        output_summary = "hide"
        check = "echo OUT && echo ERR 1>&2"
      }
    }
  }
}
EOF
    HK_SUMMARY_TEXT=1 run hk check
    assert_success
    refute_output --partial "s stdout:"
    refute_output --partial "s stderr:"
    refute_output --partial "s output:"
}

