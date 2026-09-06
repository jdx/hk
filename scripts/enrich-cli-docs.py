#!/usr/bin/env python3
"""Add maintained examples and guide links to usage-cli's generated Markdown.

Run after usage generates docs/cli. Safe to rerun on the same output.
Command syntax and flag descriptions still come from the Rust CLI.
"""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "docs/cli"
MARKER = "<!-- hk documentation examples -->"

EXAMPLES = {
    "agent": ("Generate integration snippets for review before adding them to an agent host.", "hk agent instructions --target codex\nhk agent hooks --target claude-code\nhk agent mcp --target vscode"),
    "agent/instructions": ("Print project instructions without editing an agent configuration.", "hk agent instructions --target codex\nhk agent instructions --target generic"),
    "agent/hooks": ("Print a hook or task snippet to review and merge into your host configuration.", "hk agent hooks --target claude-code\nhk agent hooks --target vscode"),
    "agent/mcp": ("Print an MCP configuration for your host, then review its project path.", "hk agent mcp --target codex\nhk agent mcp --target claude-desktop"),
    "mcp": ("Start the STDIO MCP server with a fixed project root. Configure your host to launch this command.", "hk mcp --root /absolute/path/to/project"),
    "check": ("Check a full repository or inspect one step before running it.",
              "hk check --all\nhk check --step eslint\nhk check --why eslint\nhk check --plan --json"),
    "fix": ("Fix commands may also stage changes. Use --no-stage to leave fixes unstaged, then review both working-tree and staged diffs.",
            "hk fix --no-stage\nhk fix --all --no-stage\nhk fix --step prettier\ngit diff\ngit diff --cached"),
    "init": ("Generate a configuration, then validate it. Install selected tools separately.",
             "hk init\nhk validate\n\n# Choose linters and hooks interactively\nhk init --interactive"),
    "install": ("Choose a local installation or a global one on Git 2.54+. Add --mise when Git needs mise to provide tools.",
                "hk install\n\n# Alternative: install once for all repositories\nhk install --global"),
    "uninstall": ("Remove the installation in the matching scope; this does not delete hk.pkl.",
                  "hk uninstall\nhk uninstall --global"),
    "run": ("Run a configured hook by name. A plan uses its settings without executing its steps.",
            "hk run pre-commit --plan\nhk run pre-commit\n\n# A custom hook defined in hk.pkl\nhk run my-hook"),
    "run/pre-commit": ("Inspect the hook first. An actual run can fix, stage, and stash files according to its configuration.",
                       "hk run pre-commit --plan\nhk run pre-commit"),
    "config": ("Inspect effective runtime settings and their sources. Use hk check --plan for hook and step selection.",
               "hk config dump\nhk config get jobs\nhk config explain jobs"),
    "config/dump": ("Choose JSON or TOML for the effective runtime settings.",
                    "hk config dump\nhk config dump --format toml"),
    "config/get": ("Read an individual runtime setting by its underscore-separated name.",
                   "hk config get jobs\nhk config get skip_steps"),
    "config/explain": ("Find which source supplies a runtime setting.",
                       "hk config explain jobs\nhk config explain exclude"),
    "config/sources": ("Show precedence, then inspect the resolved values.",
                       "hk config sources\nhk config dump"),
    "cache/clear": ("Clear hk's cache when diagnosing stale evaluated configuration.",
                    "hk cache clear\nhk validate"),
    "builtins": ("List builtin names, then use them in hk.pkl. External tools must be installed separately.",
                 "hk builtins"),
    "completion": ("Generate a completion script, or install it in the location used by your shell. hk prints any remaining shell setup instructions.",
                   "hk completion bash\nhk completion zsh --install\nhk completion fish --install"),
    "test": ("List step-defined tests, or run only the tests belonging to one configured step.",
             "hk test --list\nhk test --step whitespace\nhk test --name 'accepts clean text'"),
    "validate": ("Evaluate the selected configuration without running its linter commands.",
                 "hk validate\nHK_FILE=./hk.local.pkl hk validate"),
    "version": ("Print the installed version for diagnostics or a bug report.", "hk version"),
    "sponsors": ("List the sponsors supporting hk and related open source tools.", "hk sponsors"),
    "migrate": ("Convert a pre-commit configuration, then review the generated file and tool requirements.",
                "hk migrate pre-commit --output hk.migrated.pkl\nHK_FILE=./hk.migrated.pkl hk validate"),
    "migrate/pre-commit": ("Write to a separate file while evaluating a migration. Review filters, tool versions, and any unsupported hooks before installing.",
                           "hk migrate pre-commit --output hk.migrated.pkl\nHK_FILE=./hk.migrated.pkl hk check --all --plan"),
    "util": ("Utilities run directly on the arguments you provide. Several also have ready-to-use Builtins definitions.",
             "hk util trailing-whitespace README.md\nhk util end-of-file-fixer README.md"),
    "util/check-added-large-files": ("Check files against a size limit in kilobytes.",
                                     "hk util check-added-large-files --maxkb 1024 assets/logo.png"),
    "util/check-byte-order-marker": ("Check for a byte order marker without changing the file.",
                                     "hk util check-byte-order-marker README.md"),
    "util/check-case-conflict": ("Compare the supplied paths for case-insensitive naming collisions.",
                                "hk util check-case-conflict src/App.ts src/app.ts"),
    "util/check-conventional-commit": ("Validate a file containing a commit message.",
                                      "hk util check-conventional-commit .git/COMMIT_EDITMSG"),
    "util/check-executables-have-shebangs": ("Check an executable script's shebang.",
                                           "hk util check-executables-have-shebangs scripts/check.sh"),
    "util/check-merge-conflict": ("Scan for merge markers even when Git is not currently merging.",
                                 "hk util check-merge-conflict --assume-in-merge src/main.rs"),
    "util/check-symlinks": ("Check the supplied symbolic link targets.",
                           "hk util check-symlinks bin/tool"),
    "util/detect-private-key": ("Check the supplied files for recognized private-key markers.",
                               "hk util detect-private-key config/example.env"),
    "util/end-of-file-fixer": ("Check first, or request an in-place fix explicitly.",
                              "hk util end-of-file-fixer README.md\nhk util end-of-file-fixer --fix README.md"),
    "util/fix-byte-order-marker": ("This command modifies the supplied files to remove a UTF-8 BOM.",
                                  "hk util fix-byte-order-marker README.md"),
    "util/fix-smart-quotes": ("This utility fixes by default; use --check to inspect without editing.",
                             "hk util fix-smart-quotes --check README.md\nhk util fix-smart-quotes README.md"),
    "util/mixed-line-ending": ("Check for mixed line endings, or normalize them to LF.",
                              "hk util mixed-line-ending README.md\nhk util mixed-line-ending --fix README.md"),
    "util/no-commit-to-branch": ("Protect a named branch instead of the default main/master list.",
                               "hk util no-commit-to-branch --branch production"),
    "util/python-check-ast": ("Validate Python syntax using python3 (or python). If neither interpreter is available, the utility skips the files. Python may create __pycache__ files.",
                             "hk util python-check-ast app.py"),
    "util/python-debug-statements": ("Check Python files for debugging statements.",
                                    "hk util python-debug-statements app.py"),
    "util/trailing-whitespace": ("Check first, or remove trailing whitespace in place.",
                                 "hk util trailing-whitespace README.md\nhk util trailing-whitespace --fix README.md"),
}

OVERVIEW = """Run the same configured steps locally, in Git hooks, and in CI.

| Task | Command |
| --- | --- |
| Set up a project | [hk init](/cli/init) and [hk install](/cli/install) |
| Check or fix code | [hk check](/cli/check) and [hk fix](/cli/fix) |
| Run a named hook | [hk run](/cli/run) |
| Inspect configuration | [hk config](/cli/config) and [hk validate](/cli/validate) |
| Test a step definition | [hk test](/cli/test) |
| Integrate a coding agent | [hk agent](/cli/agent) and [hk mcp](/cli/mcp) |
| Use included file checks | [hk util](/cli/util) |

Start with [getting started](/getting_started) for an end-to-end setup. Command-specific flags are listed on each page; the flags below apply globally.

"""

for path in sorted(CLI.rglob("*.md")):
    content = path.read_text().split(MARKER)[0].rstrip() + "\n"
    content = re.sub(r"\A---\n.*?\n---\n\n", "", content, count=1, flags=re.S)
    key = path.relative_to(CLI).with_suffix("").as_posix()
    title_match = re.search(r"^# (.+)$", content, re.M)
    if not title_match:
        raise ValueError(f"Missing title: {path}")
    title = title_match[1].replace("`", "")
    if key == "index":
        title = "CLI reference"
        content = content.replace("# `hk`\n", "# CLI reference\n", 1)
        content = re.sub(
            r"Run the same configured steps locally, in Git hooks, and in CI\.\n\n.*?"
            r"the flags below apply globally\.\n\n",
            "",
            content,
            flags=re.S,
        )
        content = content.replace("# CLI reference\n\n", "# CLI reference\n\n" + OVERVIEW, 1)
        content = re.sub(r"^- \*\*Usage(?::\*\*|\*\*:).*\n", "", content, flags=re.M)
    else:
        content = content.replace("- **Usage**:", "**Usage**:")
    description = "Commands, options, and examples for the hk CLI." if key == "index" else EXAMPLES.get(key, (f"Arguments and options for {title}.", ""))[0]
    metadata = "---\ntitle: " + json.dumps(title) + "\ndescription: " + json.dumps(description) + "\n---\n\n"
    extra = "\n" + MARKER + "\n"
    if key in EXAMPLES:
        intro, commands = EXAMPLES[key]
        extra += "\n## Examples\n\n" + intro + "\n\n```sh\n" + commands + "\n```\n"
    if key.startswith("agent") or key == "mcp":
        guide = "[Coding agents](/agents)"
    elif key.startswith("run/"):
        guide = "[Git hooks and stashing](/hooks)"
    elif key.startswith("util"):
        guide = "[Built-in linters and utilities](/builtins)"
    elif key.startswith("config"):
        guide = "[Configuration guide](/configuration)"
    else:
        guide = "[Getting started](/getting_started)"
    extra += "\n## Learn more\n\n" + guide + " · [Troubleshooting](/logging)"
    if key != "index":
        extra += " · [All commands](/cli/)"
    path.write_text(metadata + content.rstrip() + "\n" + extra + "\n")

print(f"Prepared {len(list(CLI.rglob('*.md')))} CLI reference pages")
