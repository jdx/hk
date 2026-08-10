## Using hk from a coding agent

Inspect and plan before running. Scope checks to changed files with `--files0-from` and use `--cd` to select the project root. Prefer `--safe`, inspect command effects, and require approval for unknown or destructive commands. Consume JSON or JSONL diagnostics while retaining raw output, and always review the diff produced by a fix. MCP clients should use `inspect_project`, `plan`, safe run tools, paged output, and `get_diff` rather than invoking arbitrary shell commands.
