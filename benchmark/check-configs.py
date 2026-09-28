#!/usr/bin/env python3
"""Keep prek and pre-commit's hook workload equivalent across config formats."""

import difflib
import json
import subprocess
import sys
import tomllib
from pathlib import Path

PRE_COMMIT_HOOKS = "https://github.com/pre-commit/pre-commit-hooks"
TEXT_FIXERS = {"trailing-whitespace", "end-of-file-fixer"}


def workload(config):
    hooks = []
    for repo in config["repos"]:
        for original in repo["hooks"]:
            hook = dict(original)
            hook.pop("priority", None)
            hook.pop("require_serial", None)
            source = repo["repo"]
            if source == "builtin":
                if hook["id"] not in TEXT_FIXERS:
                    raise ValueError(f"no pre-commit equivalent for builtin {hook['id']}")
                source = PRE_COMMIT_HOOKS
                if "manual" in hook.get("stages", []):
                    args = list(hook.get("args", []))
                    if "--check" not in args:
                        raise ValueError(f"builtin {hook['id']} must use --check in the manual stage")
                    args.remove("--check")
                    if args:
                        hook["args"] = args
                    else:
                        hook.pop("args", None)
            hooks.append({"repo": source, **hook})
    normalized = dict(config)
    normalized.pop("minimum_prek_version", None)
    normalized.pop("priorities", None)
    normalized["repos"] = hooks
    return normalized


def main():
    reference = json.loads(subprocess.check_output(["yq", "-o=json", sys.argv[1]], text=True))
    prek = tomllib.loads(Path(sys.argv[2]).read_text())
    expected = json.dumps(workload(reference), indent=2, sort_keys=True).splitlines(keepends=True)
    actual = json.dumps(workload(prek), indent=2, sort_keys=True).splitlines(keepends=True)
    if expected != actual:
        sys.stderr.writelines(difflib.unified_diff(expected, actual, fromfile=sys.argv[1], tofile=sys.argv[2]))
        sys.exit("error: prek and pre-commit must run the same hooks on the same files")


if __name__ == "__main__":
    main()
