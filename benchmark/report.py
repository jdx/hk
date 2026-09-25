#!/usr/bin/env python3
"""Turns a tak run into benchmark/results.json.

The docs page (docs/benchmarks.md) renders this file. It refuses to render
numbers unless `passed` is true, and this script only sets it when the run can
be trusted:

- every subject of every scenario in tak.toml was timed, and every timed
  sample was checked against the fixture's clean commit;
- every subject passed every check. Every configuration measured is safe by
  design, so a wrong result means the harness is broken (or the tool has a bug
  worth reporting);
- every subject reported failure on the dirty tree in the check-detects sanity
  benchmark. A read-only check that ran nothing would also pass on a clean tree.

Usage: report.py <tak-export.json> [--out PATH]
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent
# Bump when results from an older configuration should stop rendering: the docs
# page and benchmark-refresh.yml accept only this schema, and the refresh
# re-measures when the published results.json has another.
SCHEMA = 2

# Benchmarks in tak.toml that guard the run but aren't shown as scenarios.
SANITY = {"check-detects"}

# Display metadata. `mode` describes how the tool runs its hooks; `modes`
# overrides it for scenarios where the configuration runs them differently.
SUBJECTS = {
    "hk": {"tool": "hk", "label": "hk", "mode": "parallel, file locks"},
    "lefthook": {"tool": "lefthook", "label": "lefthook", "mode": "sequential",
                 "modes": {"check-all": "parallel: true"}},
    "pre-commit": {"tool": "pre-commit", "label": "pre-commit", "mode": "sequential hooks, batched files"},
    "prek": {"tool": "prek", "label": "prek", "mode": "sequential hooks, batched files"},
}

SCENARIOS = {
    "fix-all": {
        "title": "Fix every file",
        "summary": "A quarter of the files need two or three fixers each to write them.",
    },
    "check-all": {
        "title": "Check every file",
        "summary": "Every file is already clean and nothing writes.",
    },
    "fix-staged": {
        "title": "Commit",
        "summary": "About 60 staged files with defects, fixed by each tool's pre-commit hook.",
    },
}

# The workload's own tools. The hook managers' versions come from tak's export
# (`version_cmd` in tak.toml).
LINTERS = {
    "black": ["black", "--version"],
    "ruff": ["ruff", "--version"],
    "prettier": ["prettier", "--version"],
    "eslint": ["eslint", "--version"],
    "shfmt": ["shfmt", "--version"],
    "jq": ["jq", "--version"],
    "yq": ["yq", "--version"],
    "node": ["node", "--version"],
    "python": ["python3", "--version"],
    "git": ["git", "--version"],
}


def out(*cmd, cwd=None):
    try:
        return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return ""


def semver(text):
    m = re.search(r"\d+\.\d+(\.\d+)?([-+.][0-9A-Za-z.+-]+)?", text or "")
    return m.group(0) if m else None


def workload():
    fixture = ROOT / ".work" / "fixture"
    count = lambda *a: len(out("git", *a, cwd=fixture).splitlines())
    staged = ROOT / ".work" / "hk" / ".git" / "staged-files"
    return {
        "files": count("ls-files"),
        "dirty_files": count("diff", "--name-only", "clean", "dirty"),
        "staged_files": len(staged.read_text().split()) if staged.exists() else None,
        "fixers": ["black", "ruff format", "ruff check", "prettier", "eslint", "jq", "yq", "shfmt",
                   "trailing whitespace", "final newline"],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("tak_json")
    ap.add_argument("--out", default=str(ROOT / "results.json"))
    args = ap.parse_args()

    config = tomllib.loads((ROOT / "tak.toml").read_text())
    export = json.loads(Path(args.tak_json).read_text())
    samples = {(r["bench"], r["subject"]): r for r in export["results"]}

    def subjects_of(bench):
        b = config["bench"][bench]
        return b.get("subjects") or list(b.get("subject", {}))

    # A run narrowed with `--bench` is a diagnostic: never publishable, but not
    # a failure either. `problems` holds what went wrong in what did run.
    skipped = [b for b in config["bench"] if not any(k[0] == b for k in samples)]
    problems = []

    for bname in SANITY - set(skipped):
        for sname in subjects_of(bname):
            if (bname, sname) not in samples:
                problems.append(f"{sname}: its check passed on the dirty tree, or it could not run")

    versions = {}
    scenarios = []
    for bname in config["bench"]:
        if bname in SANITY or bname in skipped:
            continue
        results = {}
        for sname in subjects_of(bname):
            meta = SUBJECTS.get(sname)
            t = samples.get((bname, sname))
            if meta is None:
                problems.append(f"{sname}: no display metadata in report.py")
                continue
            if t is None:
                problems.append(f"{bname}/{sname}: dropped by tak (it failed to run or exited with an unexpected code)")
                continue
            checks = t.get("checks")
            if not checks or checks["total"] != len(t["times"]):
                problems.append(f"{bname}/{sname}: not every sample was checked")
                continue
            if checks["passed"] != checks["total"]:
                problems.append(
                    f"{bname}/{sname}: produced the wrong files in "
                    f"{checks['total'] - checks['passed']}/{checks['total']} samples"
                )
            if t.get("version"):
                versions.setdefault(meta["tool"], semver(t["version"]))
            results[sname] = {
                "mode": meta.get("modes", {}).get(bname, meta["mode"]),
                "mean": round(t["mean"], 4),
                "median": round(t["median"], 4),
                "stddev": round(t["stddev"], 4),
                "min": round(t["min"], 4),
                "max": round(t["max"], 4),
                "runs": len(t["times"]),
                "correct": {"passed": checks["passed"], "total": checks["total"]},
            }
        scenarios.append({"key": bname, **SCENARIOS.get(bname, {"title": bname, "summary": ""}), "results": results})

    versions["tak"] = semver(export.get("tak_version"))
    versions.update({name: semver(out(*cmd)) for name, cmd in LINTERS.items()})

    machine = export.get("machine") or {}
    unpublishable = ([f"partial run, not timed: {', '.join(skipped)}"] if skipped else []) + problems
    data = {
        "schema": SCHEMA,
        "generated": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "passed": not unpublishable,
        "problems": unpublishable,
        "commit": out("git", "rev-parse", "HEAD", cwd=ROOT.parent),
        "workflow_run": os.environ.get("BENCH_WORKFLOW_RUN") or None,
        "seed": export.get("seed"),
        "machine": {
            "runner": os.environ.get("BENCH_RUNNER") or export.get("runner") or "local",
            "os": machine.get("os_version") or machine.get("os"),
            "kernel": machine.get("kernel"),
            "arch": machine.get("arch"),
            "cpu": machine.get("cpu"),
            "cpus": machine.get("cpus"),
            "memory_gb": round(machine["memory_bytes"] / 2**30) if machine.get("memory_bytes") else None,
        },
        "versions": versions,
        "workload": workload(),
        "subjects": {k: {f: v[f] for f in ("tool", "label", "mode")} for k, v in SUBJECTS.items()},
        "scenarios": scenarios,
    }
    Path(args.out).write_text(json.dumps(data, indent=2) + "\n")
    print(f"wrote {args.out}")
    for p in unpublishable:
        print(f"  not publishable: {p}", file=sys.stderr)
    # Exit non-zero only when something that ran was wrong.
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
