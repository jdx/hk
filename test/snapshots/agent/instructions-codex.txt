## hk

- Before changing files, inspect the project with `hk mcp` or `hk run check --format json`.
- Scope checks to the files you changed. For exact filenames, write a NUL-delimited list and use `--files0-from`; use `--cd` instead of changing hk's process-wide directory.
- Inspect each planned command's effect. Prefer `--safe`; never run an unknown or destructive command without explicit user approval.
- Consume normalized diagnostics from JSON/JSONL, preserve raw tool output for debugging, and review the resulting diff after fixes.
- Use `hk run check --format jsonl` for streaming lifecycle events. A final summary is emitted even when a step fails.
