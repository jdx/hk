#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util check-shebang-scripts-are-executable - detects shebang without executable bit" {
    printf "#!/bin/bash\necho hello\n" > script.sh
    chmod 644 script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_failure
    assert_output --partial "script.sh"
    assert_output --partial "chmod +x"
}

@test "util check-shebang-scripts-are-executable - passes executable with shebang" {
    printf "#!/bin/bash\necho hello\n" > script.sh
    chmod +x script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - passes non-executable without shebang" {
    echo "echo hello" > notes.txt
    chmod 644 notes.txt

    run hk util check-shebang-scripts-are-executable notes.txt
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - accepts env shebang" {
    printf "#!/usr/bin/env python3\nprint('hello')\n" > script.py
    chmod 644 script.py

    run hk util check-shebang-scripts-are-executable script.py
    assert_failure
    assert_output --partial "script.py"
}

@test "util check-shebang-scripts-are-executable - detects multiple files" {
    printf "#!/bin/sh\necho one\n" > script1.sh
    printf "#!/bin/sh\necho two\n" > script2.sh
    chmod 644 script1.sh script2.sh

    run hk util check-shebang-scripts-are-executable script1.sh script2.sh
    assert_failure
    assert_output --partial "script1.sh"
    assert_output --partial "script2.sh"
}

@test "util check-shebang-scripts-are-executable - mixed executable and not" {
    printf "#!/bin/sh\necho one\n" > good.sh
    chmod +x good.sh
    printf "#!/bin/sh\necho two\n" > bad.sh
    chmod 644 bad.sh

    run hk util check-shebang-scripts-are-executable good.sh bad.sh
    assert_failure
    assert_output --partial "bad.sh"
    refute_output --partial "good.sh"
}

@test "util check-shebang-scripts-are-executable - passes empty file" {
    : > empty.sh
    chmod 644 empty.sh

    run hk util check-shebang-scripts-are-executable empty.sh
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["shebang-scripts"] = Builtins.check_shebang_scripts_are_executable
        }
    }
}
HK

    printf "#!/bin/bash\necho hello\n" > script.sh
    chmod 644 script.sh
    git add -A

    run hk check --all
    assert_failure
    assert_output --partial "script.sh"
}

@test "util check-shebang-scripts-are-executable - uses the index mode for tracked files" {
    printf '#!/bin/bash\necho hello\n' > script.sh
    chmod +x script.sh
    git add script.sh
    git update-index --chmod=-x script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_failure
    assert_output --partial "script.sh"
}

@test "util check-shebang-scripts-are-executable - works with core.fileMode=false" {
    printf '#!/bin/bash\necho hello\n' > script.sh
    git add script.sh
    git config core.fileMode false
    chmod +x script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_failure
    assert_output --partial "script.sh"

    git update-index --chmod=+x script.sh
    chmod 644 script.sh
    run hk util check-shebang-scripts-are-executable script.sh
    assert_success
}

@test "util check-shebang-scripts-are-executable - suggests update-index when git mode is not executable" {
    printf '#!/bin/bash\necho hello\n' > script.sh
    git add script.sh
    git config core.fileMode false
    chmod +x script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_failure
    assert_output --partial "git update-index --chmod=+x"
}

@test "util check-shebang-scripts-are-executable - skips tracked symlinks and directories" {
    printf '#!/bin/bash\necho hello\n' > target.sh
    chmod +x target.sh
    ln -s target.sh link.sh
    mkdir sub
    git add target.sh link.sh

    run hk util check-shebang-scripts-are-executable link.sh sub
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - backslash in a filename is not treated as a separator" {
    if [[ "$(uname)" != "Linux" && "$(uname)" != "Darwin" ]]; then
        skip "backslash filenames are unix only"
    fi
    mkdir scripts
    printf '#!/bin/bash\necho a\n' > 'scripts/foo\bar.sh'
    printf '#!/bin/bash\necho b\n' > scripts/other.sh
    git add scripts
    git update-index --chmod=+x 'scripts/foo\bar.sh'
    chmod +x 'scripts/foo\bar.sh'

    run hk util check-shebang-scripts-are-executable 'scripts/foo\bar.sh'
    assert_success
    run hk util check-shebang-scripts-are-executable scripts/other.sh
    assert_failure
}

@test "util check-shebang-scripts-are-executable - untracked files are told to chmod and add, not update-index alone" {
    printf '#!/bin/bash\necho hello\n' > script.sh
    chmod 644 script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_failure
    assert_output --partial "untracked files: run \`chmod +x <file>\`"
    refute_output --partial "  - tracked files"
}

@test "util check-shebang-scripts-are-executable - untracked hint leads with git add + update-index when core.fileMode=false" {
    printf '#!/bin/bash\necho hello\n' > script.sh
    git config core.fileMode false

    run hk util check-shebang-scripts-are-executable script.sh
    assert_failure
    assert_output --partial "untracked files: run \`git add <file>\`, then \`git update-index --chmod=+x <file>\`"
    refute_output --partial "run \`chmod +x"
}

@test "util check-shebang-scripts-are-executable - a file outside the repository does not fail the run" {
    outside="$(mktemp -d)"
    printf '#!/bin/bash\necho a\n' > "$outside/ext.sh"
    chmod +x "$outside/ext.sh"
    printf '#!/bin/bash\necho b\n' > inside.sh
    chmod +x inside.sh
    git add inside.sh
    git update-index --chmod=+x inside.sh

    run hk util check-shebang-scripts-are-executable "$outside/ext.sh" inside.sh
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - tracked path replaced by a dangling symlink is skipped" {
    printf '#!/bin/bash\necho a\n' > script.sh
    git add script.sh
    rm script.sh
    ln -s missing-target script.sh

    run hk util check-shebang-scripts-are-executable script.sh
    assert_success
    refute_output
}

@test "util check-shebang-scripts-are-executable - tracked file referenced as ../x.sh from a subdirectory uses the index mode" {
    printf '#!/bin/bash\necho a\n' > x.sh
    chmod +x x.sh
    git add x.sh
    git update-index --chmod=+x x.sh
    mkdir sub
    # worktree bit says non-executable, index says executable
    chmod 644 x.sh
    cd sub

    run hk util check-shebang-scripts-are-executable ../x.sh
    assert_success
    refute_output
}
