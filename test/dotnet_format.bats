#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    export DOTNET_CLI_HOME="$HOME"
    export DOTNET_CLI_TELEMETRY_OPTOUT=1
}

teardown() {
    _common_teardown
}

@test "dotnet-format builtin tests" {
    cat > hk.pkl <<PKL
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
steps {
  ["dotnet-format"] = Builtins.dotnet_format
}
PKL
    run hk test --step dotnet-format
    assert_success
    assert_output --partial "ok - dotnet-format :: fix only selected file"
    assert_output --partial "ok - dotnet-format :: fix selected nested file with spaces through solution"
}

@test "dotnet-format dir rebases selected paths and leaves another project unchanged" {
    mkdir -p 'nested project' other
    for dir in 'nested project' other; do
        cat > "$dir/Test.csproj" <<'XML'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup>
</Project>
XML
        printf 'class C{void M(){}}\n' > "$dir/Code File.cs"
    done
    cat > .editorconfig <<'CONFIG'
root = true
[*]
indent_style = space
indent_size = 4
end_of_line = lf
insert_final_newline = true
CONFIG
    cat > hk.pkl <<PKL
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
steps {
  ["dotnet-format"] = (Builtins.dotnet_format) {
    dir = "nested project"
  }
}
PKL
    run hk check --check 'nested project/Code File.cs'
    assert_failure
    assert_file_contains 'nested project/Code File.cs' 'class C{void M(){}}'

    run hk fix --no-stage 'nested project/Code File.cs'
    assert_success
    assert_file_contains 'nested project/Code File.cs' 'class C { void M() { } }'
    assert_file_contains 'other/Code File.cs' 'class C{void M(){}}'

    run hk check --check 'nested project/Code File.cs'
    assert_success
}
