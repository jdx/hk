#!/usr/bin/env python3
"""Checks that every subject in tak.toml produces the right files.

tak measures how long a command takes, not what it did. A tool that exits
early, skips files, or races two fixers onto one file can post a fast time, so
this runs every subject of every benchmark again, exactly as tak.toml declares
it, and compares the working tree afterwards with the fixture's clean commit.

A trial passes when the tree is byte-identical to `clean` and the command's
exit status is one tak.toml's wrapper accepts. Races are intermittent, so each
subject gets several trials; the report shows how many passed.

A read-only check that ran nothing would also leave a clean tree clean, so
subjects of the check benchmarks get one more trial against the dirty commit,
where they must report failure.

Usage: verify.py [--trials N] [--bench NAME] [--subject NAME] [--out PATH]
"""

import argparse
import json
import os
import subprocess
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def git(cwd, *args):
    return subprocess.run(
        ["git", *args], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout.strip()


def resolve(argv):
    # Same rule as tak: a relative program path containing "/" is anchored at
    # tak.toml's directory; bare names are looked up on PATH.
    prog = argv[0]
    if "/" in prog and not os.path.isabs(prog):
        prog = str(ROOT / prog)
    return [prog, *argv[1:]]


# Benchmarks whose subjects only read files. They must fail on the dirty tree.
CHECKS = {"check-all"}
DIRTY = ["sh", "-c", "git reset -q --hard dirty && git clean -qfd"]


def trial(bench, subject, prepare=None):
    cwd = ROOT / subject["dir"]
    env = {**os.environ, **bench.get("env", {}), **subject.get("env", {})}
    for name in CONFIG.get("env", {}).get("deny", []):
        if name not in bench.get("env", {}) and name not in subject.get("env", {}):
            env.pop(name, None)

    prepare = prepare or subject.get("prepare", bench.get("prepare"))
    if prepare:
        subprocess.run(resolve(prepare), cwd=cwd, env=env, check=True)

    start = time.monotonic()
    proc = subprocess.run(resolve(subject["cmd"]), cwd=cwd, env=env, capture_output=True, text=True)
    elapsed = time.monotonic() - start

    git(cwd, "add", "-A")
    wrong = git(cwd, "diff", "--cached", "--name-only", "clean").splitlines()
    return {
        "ok": proc.returncode == 0 and not wrong,
        "exit": proc.returncode,
        "wrong_files": len(wrong),
        "example": wrong[:3],
        "seconds": round(elapsed, 3),
        "stderr": proc.stderr[-2000:] if proc.returncode else "",
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--trials", type=int, default=5)
    ap.add_argument("--bench", action="append")
    ap.add_argument("--subject", action="append")
    ap.add_argument("--out", default=str(ROOT / ".work" / "verify.json"))
    args = ap.parse_args()

    results = {}
    failed = False
    for bname, bench in CONFIG["bench"].items():
        if args.bench and bname not in args.bench:
            continue
        results[bname] = {}
        for sname, subject in bench["subject"].items():
            if args.subject and sname not in args.subject:
                continue
            trials = [trial(bench, subject) for _ in range(args.trials)]
            passed = sum(t["ok"] for t in trials)
            worst = max(trials, key=lambda t: t["wrong_files"])
            detects = None
            if bname in CHECKS:
                detects = trial(bench, subject, prepare=DIRTY)["exit"] != 0
                if not detects:
                    passed = 0
            results[bname][sname] = {
                "trials": len(trials),
                "passed": passed,
                "detects_defects": detects,
                "max_wrong_files": worst["wrong_files"],
                "example_wrong_files": worst["example"],
                "exit_codes": sorted({t["exit"] for t in trials}),
                "seconds": [t["seconds"] for t in trials],
            }
            mark = "ok  " if passed == len(trials) else "FAIL"
            if detects is False:
                print(f"FAIL {bname:<11} {sname:<18} passed on the dirty tree", flush=True)
            print(
                f"{mark} {bname:<11} {sname:<18} {passed}/{len(trials)} correct"
                f"  worst: {worst['wrong_files']} wrong files"
                f"  ~{sum(t['seconds'] for t in trials) / len(trials):.2f}s",
                flush=True,
            )
            for t in trials:
                if t["stderr"]:
                    print(f"     exit {t['exit']}: {t['stderr'].strip()[-400:]}", file=sys.stderr)
                    break
            failed |= passed != len(trials)

    Path(args.out).write_text(json.dumps(results, indent=2) + "\n")
    # Incorrect subjects are an expected finding, not an error: the page shows
    # them. The exit status only reports whether everything was correct.
    return 1 if failed else 0


CONFIG = tomllib.loads((ROOT / "tak.toml").read_text())

if __name__ == "__main__":
    sys.exit(main())
