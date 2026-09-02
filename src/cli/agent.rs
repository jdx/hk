use crate::Result;

/// Generate integration snippets for coding agents
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Agent {
    #[usage(subcommand)]
    command: Command,
}

#[derive(Debug, usage_rs::Subcommands)]
enum Command {
    /// Print a hook configuration for an agent or editor
    #[usage(effect = "read")]
    Hooks {
        /// Agent or editor to generate the hook configuration for
        #[usage(long, value_enum)]
        target: HookTarget,
    },
    /// Print project instructions for a coding agent
    #[usage(effect = "read")]
    Instructions {
        /// Agent to generate the instructions for
        #[usage(long, value_enum)]
        target: InstructionTarget,
    },
    /// Print an MCP server configuration
    #[usage(effect = "read")]
    Mcp {
        /// MCP host to generate the server configuration for
        #[usage(long, value_enum)]
        target: McpTarget,
    },
}

#[derive(Clone, Copy, Debug, usage_rs::ValueEnum, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
enum InstructionTarget {
    Codex,
    ClaudeCode,
    Generic,
}

#[derive(Clone, Copy, Debug, usage_rs::ValueEnum, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
enum HookTarget {
    Codex,
    ClaudeCode,
    Vscode,
}

#[derive(Clone, Copy, Debug, usage_rs::ValueEnum, strum::EnumString)]
#[strum(serialize_all = "kebab-case")]
enum McpTarget {
    Codex,
    ClaudeDesktop,
    ClaudeCode,
    Vscode,
}

impl Agent {
    pub async fn run(self) -> Result<()> {
        let output = match self.command {
            Command::Instructions { target } => instructions(target),
            Command::Hooks { target } => hooks(target),
            Command::Mcp { target } => mcp(target),
        };
        print!("{output}");
        Ok(())
    }
}

fn instructions(target: InstructionTarget) -> &'static str {
    match target {
        InstructionTarget::Codex => include_str!("agent/codex-instructions.md"),
        InstructionTarget::ClaudeCode => include_str!("agent/claude-code-instructions.md"),
        InstructionTarget::Generic => include_str!("agent/generic-instructions.md"),
    }
}

fn hooks(target: HookTarget) -> &'static str {
    match target {
        HookTarget::Codex => include_str!("agent/codex-hooks.json"),
        HookTarget::ClaudeCode => include_str!("agent/claude-code-hooks.json"),
        HookTarget::Vscode => include_str!("agent/vscode-hooks.json"),
    }
}

fn mcp(target: McpTarget) -> &'static str {
    match target {
        McpTarget::Codex => include_str!("agent/codex-mcp.toml"),
        McpTarget::ClaudeDesktop => include_str!("agent/claude-desktop-mcp.json"),
        McpTarget::ClaudeCode => include_str!("agent/claude-code-mcp.txt"),
        McpTarget::Vscode => include_str!("agent/vscode-mcp.json"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_generator_has_a_trailing_newline() {
        for output in [
            instructions(InstructionTarget::Codex),
            instructions(InstructionTarget::ClaudeCode),
            instructions(InstructionTarget::Generic),
            hooks(HookTarget::Codex),
            hooks(HookTarget::ClaudeCode),
            hooks(HookTarget::Vscode),
            mcp(McpTarget::Codex),
            mcp(McpTarget::ClaudeDesktop),
            mcp(McpTarget::ClaudeCode),
            mcp(McpTarget::Vscode),
        ] {
            assert!(output.ends_with('\n'));
        }
    }
}
