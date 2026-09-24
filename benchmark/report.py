#!/usr/bin/env python3
"""Combines a tak run and its verification into benchmark/results.json.

The docs page (docs/benchmarks.md) renders this file. It refuses to render
numbers unless `passed` is true, and this script only sets it when the run can
be trusted:

- every subject of every benchmark in tak.toml was both timed and verified;
- every subject that is safe by design produced the clean tree in every trial.
  A sequential tool getting it wrong means the harness is broken (or the tool
  has a bug worth reporting), not that parallelism is hard.

Subjects configured to run fixers concurrently without coordination
(lefthook-parallel, prek-parallel) are allowed to fail verification. That is
the finding the page exists to show, so their pass rate is published next to
their time.

Usage: report.py <tak-export.json> <verify.json> [--out PATH]
"""

import argparse
import json
import os
import platform
import re
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SCHEMA = 1

# Display metadata. `safe` marks configurations that cannot race by design.
SUBJECTS = {
    "hk": {"tool": "hk", "label": "hk", "mode": "parallel, file locks", "safe": True},
    "lefthook": {"tool": "lefthook", "label": "lefthook", "mode": "sequential", "safe": True},
    "lefthook-parallel": {"tool": "lefthook", "label": "lefthook", "mode": "parallel: true", "safe": False},
    "pre-commit": {"tool": "pre-commit", "label": "pre-commit", "mode": "sequential hooks, batched files", "safe": True},
    "prek": {"tool": "prek", "label": "prek", "mode": "sequential hooks, batched files", "safe": True},
    "prek-parallel": {"tool": "prek", "label": "prek", "mode": "priority: 0", "safe": False},
}

SCENARIOS = {
    "fix-all": {
        "title": "Fix every file",
        "summary": "A quarter of the files need two or three fixers each to write them.",
    },
    "check-all": {
        "title": "Check every file",
        "summary": "Every file is already clean and nothing writes, so parallel is safe for every tool.",
    },
    "fix-staged": {
        "title": "Commit",
        "summary": "About 60 staged files with defects, fixed by each tool's pre-commit hook.",
    },
}

# Command whose output contains each tool's version.
VERSIONS = {
    "hk": ["hk", "--version"],
    "lefthook": ["lefthook", "version"],
    "pre-commit": ["pre-commit", "--version"],
    "prek": ["prek", "--version"],
    "tak": ["tak", "--version"],
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


def version(cmd):
    m = re.search(r"\d+\.\d+(\.\d+)?([-+.][0-9A-Za-z.+-]+)?", out(*cmd))
    return m.group(0) if m else None


def machine():
    cpu = ""
    for line in out("lscpu").splitlines():
        if line.startswith("Model name:"):
            cpu = line.split(":", 1)[1].strip()
    mem_kb = 0
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemTotal:"):
                mem_kb = int(line.split()[1])
    except OSError:
        pass
    return {
        "runner": os.environ.get("BENCH_RUNNER", "local"),
        "os": f"{platform.system()} {platform.release()}",
        "arch": platform.machine(),
        "cpu": cpu,
        # The CPUs this process may run on, which is what every subject sees.
        "cpus": len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count(),
        "memory_gb": round(mem_kb / 1024 / 1024),
    }


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
    ap.add_argument("verify_json")
    ap.add_argument("--out", default=str(ROOT / "results.json"))
    args = ap.parse_args()

    config = tomllib.loads((ROOT / "tak.toml").read_text())
    samples = {(r["bench"], r["subject"]): r for r in json.loads(Path(args.tak_json).read_text())["results"]}
    verified = json.loads(Path(args.verify_json).read_text())

    problems = []
    scenarios = []
    # A run narrowed with `--bench` is a diagnostic: never publishable, but
    # not a failure either.
    skipped = [b for b in config["bench"] if not any(k[0] == b for k in samples)]
    for bname, bench in config["bench"].items():
        if bname in skipped:
            continue
        results = {}
        for sname in bench["subject"]:
            meta = SUBJECTS.get(sname)
            t = samples.get((bname, sname))
            v = verified.get(bname, {}).get(sname)
            if meta is None:
                problems.append(f"{sname}: no display metadata in report.py")
                continue
            if t is None:
                problems.append(f"{bname}/{sname}: not timed")
                continue
            if v is None:
                problems.append(f"{bname}/{sname}: not verified")
                continue
            correct = v["passed"] == v["trials"]
            if meta["safe"] and not correct:
                problems.append(
                    f"{bname}/{sname}: produced the wrong files in {v['trials'] - v['passed']}/{v['trials']} trials"
                    f" (up to {v['max_wrong_files']}, e.g. {', '.join(v['example_wrong_files']) or 'exit status'})"
                )
            results[sname] = {
                "mean": round(t["mean"], 4),
                "median": round(t["median"], 4),
                "stddev": round(t["stddev"], 4),
                "min": round(t["min"], 4),
                "max": round(t["max"], 4),
                "runs": len(t["times"]),
                "correct": {"passed": v["passed"], "trials": v["trials"], "max_wrong_files": v["max_wrong_files"]},
            }
        scenarios.append({"key": bname, **SCENARIOS.get(bname, {"title": bname, "summary": ""}), "results": results})

    repo = ROOT.parent
    # `problems` holds what went wrong in the benchmarks that ran.
    unpublishable = ([f"partial run, not timed: {', '.join(skipped)}"] if skipped else []) + problems
    data = {
        "schema": SCHEMA,
        "generated": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "passed": not unpublishable,
        "problems": unpublishable,
        "commit": out("git", "rev-parse", "HEAD", cwd=repo),
        "workflow_run": os.environ.get("BENCH_WORKFLOW_RUN") or None,
        "machine": machine(),
        "versions": {name: version(cmd) for name, cmd in VERSIONS.items()},
        "workload": workload(),
        "subjects": SUBJECTS,
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
