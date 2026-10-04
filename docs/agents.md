# Coding agents

hk gives coding agents a narrow, inspectable way to run project checks and fixes. Agents can use structured CLI output everywhere, or the MCP server for persistent runs, cancellation, paged logs, diagnostics, diffs, and the interactive dashboard.

Generate a starting point without editing any host configuration:

```sh
hk agent instructions --target codex
hk agent hooks --target claude-code
hk agent mcp --target vscode
```

Claude Desktop uses global host configuration and does not reliably launch servers from the project directory. Replace `/absolute/path/to/project` in its generated snippet before installing it.

Every generator writes only to stdout. Review the result and place or merge it into the appropriate host configuration yourself.

## Safe agent workflow

1. Inspect the project and request a plan.
2. Scope execution to changed files. Use `--files0-from` when exact filenames matter and `--cd` to choose the project root.
3. Inspect command effects and prefer safe execution. `--safe` rejects a run before any step starts if a runnable command is unknown or destructive. That includes the hook's `report` command, so declare its effect with a `CommandSpec` (`report = new CommandSpec { command = "node scripts/report-timings.js"; effect = "read" }`). `--safe` checks declared effects only. It is not a sandbox, and hk does not verify that a command behaves as declared.
4. Consume JSON or JSONL diagnostics and retain raw output when investigating parser warnings. A step with no `diagnostic_format` has no diagnostics, so read its raw `output` field instead, when present (see [diagnostics](/configuration#diagnostics)).
5. Review the resulting diff before accepting a fix.

Without MCP, a portable invocation is:

```sh
{
  git diff --name-only -z
  git diff --cached --name-only -z
  git ls-files --others --exclude-standard -z
} | hk run check --files0-from - --safe --format jsonl
```

## MCP

`hk mcp --root <project>` starts a STDIO MCP server. It does not listen on a network port. The root is fixed at startup (plus roots supplied by the host), and tools cannot introduce arbitrary filesystem paths or unrestricted shell commands.

Generate configuration for a supported host:

```sh
hk agent mcp --target codex
hk agent mcp --target claude-desktop
hk agent mcp --target claude-code
hk agent mcp --target vscode
```

`start_check`, `start_safe_check`, and `start_safe_fix` take an optional `scope` argument that selects the files a run covers. It defaults to `all`, and any other value is rejected.

| `scope`    | Files                                                                                     | hk flag        |
| ---------- | ----------------------------------------------------------------------------------------- | -------------- |
| `all`      | Every tracked file, plus untracked files unless stashing is enabled (see below)           | `--all`        |
| `changed`  | Staged and unstaged files, plus untracked files (see below): all that differs from `HEAD` | `--stash none` |
| `unstaged` | Unstaged files, plus untracked files (see below), without staged files                    | `--unstaged`   |

`changed` turns stashing off for the run, so it always covers staged and unstaged edits even when the project or `HK_STASH` enables stashing. `--stash none` does not override `HK_STASH_UNTRACKED=0`: with that setting hk skips untracked-file discovery, so `changed` and `unstaged` leave untracked files out. `all` keeps the project's stash setting, and hk leaves untracked files out of `--all` while stashing is enabled. Use `changed` or `unstaged` to lint only what an agent just edited instead of the whole project.

Codex, Claude Code, Claude Desktop, and VS Code can use the structured MCP tools. Hosts that implement MCP Apps also receive the hk dashboard; other hosts receive the same structured content and a useful text fallback.

## Rich dashboard

The dashboard shows the current run, step status and timing, normalized diagnostics grouped by file, searchable logs, and the resulting diff. Mutating actions are limited to confirmed safe fixes, and the server remains the source of truth.

![hk MCP dashboard showing a completed project check](/agent-dashboard.png)

_A completed project check in an MCP Apps host, with step timing and command effects._

See the [dashboard demo runbook](./agents/mcp-dashboard.md) for a deterministic project fixture.

## ChatGPT Desktop

ChatGPT Desktop can connect the local STDIO server through OpenAI's [Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels). Build hk, create the demo fixture, and initialize a local-STDIO tunnel whose MCP command is the absolute `hk mcp --root <fixture>` command. Run `tunnel-client doctor` before starting the tunnel, then create a development-mode app in ChatGPT and select the tunnel.

The tunnel is a development bridge. hk itself ships no HTTP listener, hosted service, or authentication system.

## Instructions and hooks

Use `hk agent instructions` for a concise block suitable for `AGENTS.md`, `CLAUDE.md`, or a generic agent prompt. Use `hk agent hooks` for an optional Codex or Claude Code stop hook, or a VS Code task. The Codex and Claude Code snippets run `hk agent stop-hook`, which reads the Stop hook input from stdin, does nothing when `stop_hook_active` is true, and runs `hk run check --safe`. It always exits 0; if the check fails or `--safe` refuses to run, it prints only `{"decision":"block","reason":"..."}` so the agent keeps working with a short diagnosis. When the check passes, Claude Code's documented pass is exit 0 with no output, so the Claude Code snippet prints nothing; Codex's documentation is inconsistent about whether empty stdout is accepted, so the Codex snippet passes `--target codex` and prints `{}`, which is valid either way. Both agents document this top-level `decision`/`reason` JSON for Stop hooks, and Codex rejects any other stdout, so hk's own `run_result` JSON is never printed from these hooks. A check that runs longer than `--timeout` seconds (default 100, under Codex's 120 second hook timeout) is stopped, steps included, and reported as a block; the Claude Code snippet passes `--timeout 570` beside a hook `timeout` of 600 seconds, which is Claude Code's default for command hooks. Hook output is deliberately a snippet rather than an automatic installation: inspect its effect on your workflow before enabling it.
