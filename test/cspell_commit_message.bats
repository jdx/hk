setup() {
    load 'test_helper/common_setup'
    _common_setup
    export PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["commit-msg"] {
    steps {
      ["cspell"] = Builtins.cspell_commit_message
    }
  }
}
PKL
}
teardown() {
    _common_teardown
}

@test "cspell_commit_message fails on a misspelled commit message" {
    printf 'fix: corect the parser\n' > "commit message.txt"
    run hk run commit-msg "commit message.txt"
    assert_failure
    assert_output --partial "COMMIT_EDITMSG:1:6 - Unknown word (corect)"
}

@test "cspell_commit_message ignores git comments and the verbose diff" {
    cat <<'MSG' > "commit message.txt"
fix: correct the parser

# Please enter the commit message for your changes.
# On branch zzqxbranch
# ------------------------ >8 ------------------------
# Do not modify or remove the line above.
diff --git a/zzqx.rs b/zzqx.rs
+let qqzzxx = 1;
MSG
    run hk run commit-msg "commit message.txt"
    assert_success
}

@test "cspell_commit_message uses git's core.commentChar" {
    git config core.commentChar ';'
    cat <<'MSG' > "commit message.txt"
fix: correct the parser

; On branch zzqxbranch
; ------------------------ >8 ------------------------
diff --git a/zzqx.rs b/zzqx.rs
+let qqzzxx = 1;
MSG
    run hk run commit-msg "commit message.txt"
    assert_success

    printf 'fix: corect the parser\n; zzqxbranch\n' > "commit message.txt"
    run hk run commit-msg "commit message.txt"
    assert_failure
    assert_output --partial "Unknown word (corect)"
    refute_output --partial "zzqxbranch"
}

@test "cspell_commit_message treats any candidate marker as a comment with core.commentChar=auto" {
    git config core.commentChar auto
    cat <<'MSG' > "commit message.txt"
fix: correct the parser

@ On branch zzqxbranch
@ ------------------------ >8 ------------------------
diff --git a/zzqx.rs b/zzqx.rs
+let qqzzxx = 1;
MSG
    run hk run commit-msg "commit message.txt"
    assert_success
}

@test "cspell_commit_message fails when the message file can't be read" {
    run hk run commit-msg "missing message.txt"
    assert_failure
    assert_output --partial "missing message.txt"
}

@test "cspell_commit_message uses the comment marker git reads last" {
    git config --global core.commentString '%'
    git config core.commentChar ';'
    printf 'fix: correct the parser\n; zzqxbranch\n' > "commit message.txt"
    run hk run commit-msg "commit message.txt"
    assert_success

    git config --unset core.commentChar
    printf 'fix: correct the parser\n%% zzqxbranch\n' > "commit message.txt"
    run hk run commit-msg "commit message.txt"
    assert_success
}

@test "cspell_commit_message with core.commentChar=auto skips only the marker git chose" {
    git config core.commentChar auto
    cat <<'MSG' > "commit message.txt"
fix: correct the parser

# On branch zzqxbranch
MSG
    run hk run commit-msg "commit message.txt"
    assert_success

    cat <<'MSG' > "commit message.txt"
fix: correct the parser

@ corect the parser
# On branch zzqxbranch
MSG
    run hk run commit-msg "commit message.txt"
    assert_failure
    assert_output --partial "Unknown word (corect)"
    refute_output --partial "zzqxbranch"
}
