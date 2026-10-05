#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test runs step-defined tests" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo checking {{files}}"
        fix = "sh -c 'echo hi > out.txt'"
        tests {
          ["check stdout"] {
            run = "check"
            expect { stdout = "checking" }
          }
          ["writes file"] {
            run = "fix"
            expect { files { ["out.txt"] = "hi\n" } }
          }
        }
      }
    }
  }
}
PKL

    run hk test
    assert_success
    assert_output --partial "ok - demo :: check stdout"
    assert_output --partial "ok - demo :: writes file"
}

@test "hk test --list lists tests" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo checking {{files}}"
        tests { ["t1"] {} ["t2"] {} }
      }
    }
  }
}
PKL
    run hk test --list
    assert_success
    assert_output --partial "demo :: t1"
    assert_output --partial "demo :: t2"
}
@test "hk test supports before/after hooks" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "sh -c 'echo main >> {{tmp}}/f.txt'"
        tests {
          ["pre and post mutate file"] {
            before = "sh -c 'echo before > {{tmp}}/f.txt'"
            after = "sh -c 'echo after >> {{tmp}}/f.txt'"
            files = List("{{tmp}}/f.txt")
            expect { files { ["{{tmp}}/f.txt"] = "before\nmain\nafter\n" } }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo --name "pre and post mutate file"
    assert_success
    assert_output --partial "ok - demo :: pre and post mutate file"
}

@test "hk test fails when before exits nonzero" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo should-not-run"
        tests {
          ["before fails"] {
            before = "sh -c 'exit 2'"
            expect { code = 0 }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo --name "before fails"
    assert_failure
}

@test "hk test fails when after exits nonzero" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "true"
        tests {
          ["after fails"] {
            after = "sh -c 'exit 3'"
            expect { code = 0 }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo --name "after fails"
    assert_failure
}

@test "hk test diff tests apply check_diff's patch" {
    cat <<'SCRIPT' > upper.sh
#!/bin/sh
for f; do
    new=$(tr a-z A-Z < "$f")
    [ "$new" = "$(cat "$f")" ] && continue
    printf -- '--- %s\n+++ %s\n@@ -1 +1 @@\n-%s\n+%s\n' "$f" "$f" "$(cat "$f")" "$new"
    status=1
done
exit ${status:-0}
SCRIPT
    chmod +x upper.sh
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["upper"] {
        check_diff = "$PWD/upper.sh {{files}}"
        fix = "for f in {{files}}; do echo fixer-ran > \"\$f\"; done"
        tests {
          ["diff changes the file"] {
            run = "diff"
            tmpdir = true
            write { ["a.txt"] = "hello\n" }
            expect { files { ["a.txt"] = "HELLO\n" } }
          }
          ["diff leaves a clean file"] {
            run = "diff"
            tmpdir = true
            write { ["a.txt"] = "HELLO\n" }
            expect { files { ["a.txt"] = "HELLO\n" } }
          }
        }
      }
    }
  }
}
PKL

    run hk test
    assert_success
    assert_output --partial "ok - upper :: diff changes the file"
    assert_output --partial "ok - upper :: diff leaves a clean file"
}

@test "hk test diff tests fail when check_diff's output is not a patch" {
    # Fix mode would fall back to the fixer here; a diff test must not.
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["viewer"] {
        check_diff = "echo 'Diff in {{files}}:'; echo '1 |-hello'; exit 1"
        fix = "for f in {{files}}; do echo HELLO > \"\$f\"; done"
        tests {
          ["diff"] {
            run = "diff"
            tmpdir = true
            write { ["a.txt"] = "hello\n" }
            expect { files { ["a.txt"] = "HELLO\n" } }
          }
        }
      }
    }
  }
}
PKL

    run hk test
    assert_failure
    assert_output --partial "not ok - viewer :: diff"
    assert_output --partial "check_diff exited 1 but hk could not apply its output as a patch"
}

@test "hk test diff tests rerun check after the patch for check_after_diff" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["partial"] {
        check = "grep -q unfixable {{files}} && exit 7 || exit 0"
        check_diff = "printf -- '--- a.txt\n+++ a.txt\n@@ -1,2 +1,2 @@\n-bad\n+good\n unfixable\n'; exit 1"
        check_after_diff = true
        fix = "true"
        tests {
          ["diff leaves unfixable findings"] {
            run = "diff"
            tmpdir = true
            write { ["a.txt"] = "bad\nunfixable\n" }
            expect {
              code = 7
              files { ["a.txt"] = "good\nunfixable\n" }
            }
          }
        }
      }
    }
  }
}
PKL

    run hk test
    assert_success
    assert_output --partial "ok - partial :: diff leaves unfixable findings"
}
