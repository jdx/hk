## hk

- Ask the hk MCP server to inspect the project and plan checks before execution.
- Scope work to changed files (`--files0-from` accepts exact NUL-delimited paths) and use `--cd` for another project root.
- Prefer safe checks and safe fixes. Inspect command effects and ask before any unknown or destructive command.
- Read normalized diagnostics from structured results, then inspect the patch before reporting or committing a fix.
- If MCP is unavailable, run `hk run check --format jsonl --safe`; the final event is the authoritative summary.
