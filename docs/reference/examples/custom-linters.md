# Example: custom-linters

```pkl
/// Example configuration with custom linters and platform-specific commands
/// * Shows how to define custom linters not in builtins
/// * Demonstrates platform-specific commands
/// * Uses conditions and workspace indicators
/// * Shows test configuration

amends "package://github.com/jdx/hk/releases/download/v1.36.0/hk@1.36.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.36.0/hk@1.36.0#/Builtins.pkl"

local custom_linters = new Mapping<String, Step> {
  // Custom SQL formatter
  ["sql_formatter"] {
    glob = List("**/*.sql")
    exclude = List("**/migrations/**")
    check = "sql-formatter --check {{files}}"
    fix = "sql-formatter --write {{files}}"
    batch = true
  }

  // Platform-specific security scanner
  ["security_scan"] {
    check = new Script {
      linux = "security-scanner-linux --scan {{files}}"
      macos = "security-scanner-mac --scan {{files}}"
      windows = "security-scanner.exe /scan {{files}}"
    }
    // Only run if security config exists
    condition = "test -f .security-config.yml"
    // Run exclusively to avoid conflicts
    exclusive = true
  }

  // Custom workspace-based build tool (pure formatter)
  // Use check_diff only for pure formatters where the diff covers all issues.
  // Do not use check_diff on linters that detect non-fixable errors.
  ["custom_build"] {
    workspace_indicator = "build.toml"
    check_diff = "cd {{workspace}} && custom-build diff"
    fix = "cd {{workspace}} && custom-build fix"
  }

  // Interactive migration tool
  ["migrate"] {
    glob = List("**/migrations/*.sql")
    check = "migrate validate {{files}}"
    fix = "migrate apply {{files}}"
    // Enable interactive mode for prompts
    interactive = true
  }

  // Custom linter with tests
  ["custom_validator"] {
    glob = List("**/*.custom")
    check = "validator {{files}}"
    fix = "validator --fix {{files}}"

    // Define tests for this step
    tests {
      ["validates correct syntax"] {
        run = "check"
        write {
          ["{{tmp}}/test.custom"] = #"valid content"#
        }
        files = List("{{tmp}}/test.custom")
        expect {
          code = 0
        }
      }
      ["fixes invalid syntax"] {
        run = "fix"
        write {
          ["{{tmp}}/broken.custom"] = #"broken  content"#
        }
        files = List("{{tmp}}/broken.custom")
        expect {
          files {
            ["{{tmp}}/broken.custom"] = #"broken content"#
          }
        }
      }
    }
  }
}

// Import some builtins and mix with custom
local all_linters = new Mapping<String, Step> {
  ...custom_linters
  ["prettier"] = Builtins.prettier
  ["shellcheck"] = Builtins.shellcheck
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "patch-file"  // Use patch file instead of git stash
    steps = all_linters
  }
  ["check"] {
    steps = all_linters
    // Generate a report after checking
    report = #"""
      echo "Check completed at $(date)"
      echo "Results: $HK_REPORT_JSON" | jq '.'
    """#
  }
}

// Show additional skip reasons for debugging
display_skip_reasons = List(
  "profile-not-enabled",
  "no-files-to-process",
  "condition-false"
)

// Environment variables for all steps
env {
  ["CUSTOM_VALIDATOR_STRICT"] = "true"
  ["SQL_FORMATTER_CONFIG"] = ".sql-format.yml"
}
```

## Description

Example configuration with custom linters and platform-specific commands
* Shows how to define custom linters not in builtins
* Demonstrates platform-specific commands
* Uses conditions and workspace indicators
* Shows test configuration

## Key Features

- Standard configuration
