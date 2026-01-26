#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "types: matches python files by extension" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["python"] {
        types = List("python")
        check = "echo {{ files }}"
      }
    }
  }
}
EOF
    git init
    git add -A
    git commit -m "init"

    echo "print('hello')" > test.py
    echo "console.log('hello')" > test.js
    git add test.py test.js

    run hk check
    assert_success
    assert_output --partial "test.py"
    refute_output --partial "test.js"
}

@test "types: matches python files by shebang" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["python"] {
        types = List("python")
        check = "echo {{ files }}"
      }
    }
  }
}
EOF
    git init
    git add -A
    git commit -m "init"

    cat > script <<'SCRIPT'
#!/usr/bin/env python3
print('hello')
SCRIPT
    chmod +x script
    git add script

    run hk check
    assert_success
    assert_output --partial "script"
}

@test "types: matches shell scripts by shebang" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["shell"] {
        types = List("shell")
        check = "echo {{ files }}"
      }
    }
  }
}
EOF
    git init
    git add -A
    git commit -m "init"

    cat > script.sh <<'SCRIPT'
#!/bin/bash
echo hello
SCRIPT
    chmod +x script.sh

    cat > noscript <<'SCRIPT'
print('not a shell script')
SCRIPT

    git add script.sh noscript

    run hk check
    assert_success
    assert_output --partial "script.sh"
    refute_output --partial "noscript"
}

@test "types: OR logic - matches any type" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["scripts"] {
        types = List("python", "shell")
        check = "echo {{ files }}"
      }
    }
  }
}
EOF
    git init
    git add -A
    git commit -m "init"

    echo "print('hello')" > test.py
    cat > test.sh <<'SCRIPT'
#!/bin/bash
echo hello
SCRIPT
    echo "console.log('hello')" > test.js

    git add test.py test.sh test.js

    run hk check
    assert_success
    assert_output --partial "test.py"
    assert_output --partial "test.sh"
    refute_output --partial "test.js"
}

@test "types: combines with glob patterns" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["python-in-src"] {
        glob = "src/**/*"
        types = List("python")
        check = "echo {{ files }}"
      }
    }
  }
}
EOF
    git init
    git add -A
    git commit -m "init"

    mkdir -p src lib
    echo "print('hello')" > src/test.py
    echo "print('hello')" > lib/test.py
    echo "console.log('hello')" > src/test.js

    git add src lib

    run hk check
    assert_success
    assert_output --partial "src/test.py"
    refute_output --partial "lib/test.py"
    refute_output --partial "src/test.js"
}

@test "types: matches javascript and typescript" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["js-ts"] {
        types = List("javascript", "typescript")
        check = "echo {{ files }}"
      }
    }
  }
}
EOF
    git init
    git add -A
    git commit -m "init"

    echo "const x = 1;" > test.js
    echo "const x: number = 1;" > test.ts
    echo "print('hello')" > test.py

    git add test.js test.ts test.py

    run hk check
    assert_success
    assert_output --partial "test.js"
    assert_output --partial "test.ts"
    refute_output --partial "test.py"
}
