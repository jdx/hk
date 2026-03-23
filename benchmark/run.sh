#!/usr/bin/env bash
# Runs benchmarks comparing hk, lefthook, pre-commit, and prek.
#
# lefthook is run in safe mode (parallel: false) since parallel: true
# has race conditions with overlapping file globs. hk is the only tool
# that can safely run linters in parallel on shared files.
#
# Prerequisites:
#   - hk, lefthook, pre-commit, prek, hyperfine must be on PATH
#   - prettier, eslint, black, ruff, shfmt, jq, yq must be on PATH
#
# Usage:
#   benchmark/run.sh [project-dir]
#
# Environment variables:
#   BUILD=1         Build hk from source first (default: 0)
#   WARMUP=1        Number of warmup runs (default: 1)
#   RUNS=10         Number of benchmark runs (default: 10)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_DIR="${1:-/tmp/hk-bench}"
RESULTS_DIR="$REPO_DIR/benchmark/results"
WARMUP="${WARMUP:-1}"
RUNS="${RUNS:-10}"

if [ "${BUILD:-0}" != "0" ]; then
    echo "Building hk..."
    cargo build --release --manifest-path "$REPO_DIR/Cargo.toml"
fi

if [ ! -d "$PROJECT_DIR/.git" ]; then
    echo "Project directory not found. Generating..."
    "$SCRIPT_DIR/generate-project.sh" "$PROJECT_DIR"
fi

mkdir -p "$RESULTS_DIR"

cd "$PROJECT_DIR"

HK_FILE="$SCRIPT_DIR/parallel/hk.pkl"
export HK_FILE

# Prime the pkl cache so the first benchmark run doesn't include pkl eval.
# Subsequent runs will use the cached config automatically.
echo "Priming pkl cache..."
hk validate --quiet 2>/dev/null || true

# lefthook must run in safe (sequential) mode to avoid race conditions
# with overlapping file globs like trailing-whitespace (**/*).
# Commit tool configs into the benchmark repo so they're tracked (not untracked).
# This prevents hk's stash from trying to stash them.
cp "$SCRIPT_DIR/parallel/lefthook.yml" "$PROJECT_DIR/lefthook.yml"
cp "$SCRIPT_DIR/parallel/.pre-commit-config.yaml" "$PROJECT_DIR/.pre-commit-config.yaml"
cd "$PROJECT_DIR"
git add lefthook.yml .pre-commit-config.yaml
git commit -q -m "add tool configs" --allow-empty

FILE_COUNT=$(find . -type f -not -path './.git/*' | wc -l | tr -d ' ')
echo ""
echo "=== Benchmark: parallel linters ==="
echo "Project: $PROJECT_DIR ($FILE_COUNT files)"
echo "Linters: black, ruff-format, ruff-check, prettier, eslint, shfmt, trailing-whitespace, newlines"
# Write file count for chart generation
echo "$FILE_COUNT" > "$RESULTS_DIR/file-count.txt"
echo ""

# --- Benchmark 1: All files ---
echo ">> All files"
hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    -i \
    --export-json "$RESULTS_DIR/all-files.json" \
    --prepare "cd $PROJECT_DIR && git checkout -f HEAD -- . 2>/dev/null; git reset --hard HEAD -q 2>/dev/null; true" \
    -n "hk" "HK_STASH=false hk run pre-commit --all" \
    -n "lefthook" "lefthook run pre-commit --all-files --force" \
    -n "pre-commit" "pre-commit run --all-files" \
    -n "prek" "prek run --all-files"

# --- Benchmark 2: Staged changes ---
# Stage ~500 files across diverse languages with only a handful dirty.
# Language diversity maximizes hk's parallel advantage since each language
# triggers different linters that all run concurrently.
echo ""
echo ">> Staged changes"
STAGE_SCRIPT='
cd '"$PROJECT_DIR"' && git reset --hard HEAD -q
# Stage 500 files across all languages (most are clean)
for f in $(find src -name "*.js" | sort | head -150); do git add "$f"; done
for f in $(find src -name "*.ts" | sort | head -50); do git add "$f"; done
for f in $(find lib -name "*.py" ! -name "__init__.py" | sort | head -150); do git add "$f"; done
for f in $(find scripts -name "*.sh" | sort | head -100); do git add "$f"; done
for f in $(find config data -name "*.json" | sort | head -30); do git add "$f"; done
for f in $(find config -name "*.yml" | sort | head -20); do git add "$f"; done
# Dirty ~50 files across different languages
for f in $(find src -name "*.js" | sort | head -15); do echo "   " >> "$f"; git add "$f"; done
for f in $(find src -name "*.ts" | sort | head -5); do echo "   " >> "$f"; git add "$f"; done
for f in $(find lib -name "*.py" ! -name "__init__.py" | sort | head -15); do echo "x=1+2" >> "$f"; git add "$f"; done
for f in $(find scripts -name "*.sh" | sort | head -10); do echo "" >> "$f"; git add "$f"; done
for f in $(find config data -name "*.json" | sort | head -5); do sed -i "s/  /    /" "$f"; git add "$f"; done
'
eval "$STAGE_SCRIPT"
STAGED_COUNT=$(cd "$PROJECT_DIR" && eval "$STAGE_SCRIPT" 2>/dev/null && git diff --cached --name-only | wc -l | tr -d ' ')
echo "$STAGED_COUNT" > "$RESULTS_DIR/staged-count.txt"
hyperfine \
    --warmup "$WARMUP" \
    --runs "$RUNS" \
    -i \
    --export-json "$RESULTS_DIR/staged-changes.json" \
    --prepare "$STAGE_SCRIPT" \
    -n "hk" "HK_STASH=false hk run pre-commit" \
    -n "lefthook" "lefthook run pre-commit" \
    -n "pre-commit" "pre-commit run" \
    -n "prek" "prek run"

# --- Cleanup ---
cd "$PROJECT_DIR"
git reset --hard HEAD -q
rm -f lefthook.yml .pre-commit-config.yaml

echo ""
echo "=== Results saved to $RESULTS_DIR ==="
echo ""

# --- Generate chart and data file ---
echo "Generating chart and data..."
export RESULTS_DIR REPO_DIR
cat << 'PYEOF' | uv run --with matplotlib -
import json
import os
from datetime import datetime, timezone

import matplotlib.pyplot as plt

results_dir = os.environ.get("RESULTS_DIR", "benchmark/results")
repo_dir = os.environ.get("REPO_DIR", ".")
chart_path = os.path.join(repo_dir, "docs", "public", "benchmark.png")
data_path = os.path.join(repo_dir, "docs", "public", "benchmark-data.json")

scenario_files = {
    "all_files": "all-files.json",
    "staged_changes": "staged-changes.json",
}

# Read dynamic file counts
file_count_path = os.path.join(results_dir, "file-count.txt")
try:
    with open(file_count_path) as f:
        total_files = f.read().strip()
except FileNotFoundError:
    total_files = "?"

staged_count_path = os.path.join(results_dir, "staged-count.txt")
try:
    with open(staged_count_path) as f:
        staged_files = f.read().strip()
except FileNotFoundError:
    staged_files = "?"

scenario_labels = {
    "all_files": f"All Files ({total_files} files)",
    "staged_changes": f"Staged Changes\n({staged_files} files)",
}

tools = ["hk", "lefthook", "pre-commit", "prek"]
colors = {"hk": "#4CC9F0", "lefthook": "#F72585", "pre-commit": "#7209B5", "prek": "#FF9E00"}

# Collect data for chart and JSON export
data_export = {
    "generated": datetime.now(timezone.utc).isoformat(),
    "total_files": int(total_files) if total_files.isdigit() else 0,
    "staged_files": int(staged_files) if staged_files.isdigit() else 0,
    "scenarios": {},
}
tool_times = {t: [] for t in tools}
tool_stddevs = {t: [] for t in tools}

for key, filename in scenario_files.items():
    filepath = os.path.join(results_dir, filename)
    scenario_data = {}
    try:
        with open(filepath) as f:
            data = json.load(f)
        for result in data["results"]:
            name = result["command"]
            for tool in tools:
                if tool == name:
                    tool_times[tool].append(result["mean"])
                    tool_stddevs[tool].append(result["stddev"])
                    scenario_data[tool] = {
                        "mean": round(result["mean"], 4),
                        "stddev": round(result["stddev"], 4),
                        "min": round(result["min"], 4),
                        "max": round(result["max"], 4),
                    }
                    break
    except FileNotFoundError:
        print(f"Warning: {filepath} not found")
        for tool in tools:
            tool_times[tool].append(0)
            tool_stddevs[tool].append(0)
    data_export["scenarios"][key] = scenario_data

# Compute speedups for data export
for key in scenario_files:
    s = data_export["scenarios"].get(key, {})
    if "hk" in s and s["hk"]["mean"] > 0:
        for tool in ["lefthook", "pre-commit", "prek"]:
            if tool in s and s[tool]["mean"] > 0:
                s[f"{tool}_vs_hk"] = round(s[tool]["mean"] / s["hk"]["mean"], 2)

# Write JSON data file
os.makedirs(os.path.dirname(data_path), exist_ok=True)
with open(data_path, "w") as f:
    json.dump(data_export, f, indent=2)
    f.write("\n")
print(f"Data saved to {data_path}")

# --- Generate chart (one subplot per scenario, each with its own y-axis) ---
plt.style.use("dark_background")
fig, axes = plt.subplots(1, 2, figsize=(14, 6))
fig.patch.set_facecolor("#1a1a2e")
fig.suptitle(
    "Hook Manager Performance — Safe Parallel Execution",
    fontsize=16,
    fontweight="bold",
    color="white",
    y=0.98,
)

scenario_keys = list(scenario_files.keys())
width = 0.6

for ax_idx, key in enumerate(scenario_keys):
    ax = axes[ax_idx]
    ax.set_facecolor("#1a1a2e")

    times = [tool_times[t][ax_idx] for t in tools]
    stddevs = [tool_stddevs[t][ax_idx] for t in tools]
    tool_colors = [colors[t] for t in tools]

    bars = ax.bar(
        range(len(tools)),
        times,
        width,
        yerr=stddevs,
        capsize=5,
        color=tool_colors,
        edgecolor="white",
        linewidth=0.5,
        alpha=0.9,
    )

    # Value labels
    for bar, t in zip(bars, times):
        if t > 0:
            label = f"{t:.2f}s" if t >= 0.1 else f"{t * 1000:.0f}ms"
            ax.text(
                bar.get_x() + bar.get_width() / 2.0,
                bar.get_height(),
                label,
                ha="center",
                va="bottom",
                fontsize=11,
                fontweight="bold",
                color="white",
            )

    ax.set_title(scenario_labels[key], fontsize=12, color="white", pad=10)
    ax.set_xticks(range(len(tools)))
    ax.set_xticklabels(tools, fontsize=10)
    ax.set_ylabel("Time" if ax_idx == 0 else "", fontsize=11, color="white")
    ax.grid(axis="y", alpha=0.2)
    ax.spines["top"].set_visible(False)
    ax.spines["right"].set_visible(False)

plt.tight_layout(rect=[0, 0, 1, 0.93])
os.makedirs(os.path.dirname(chart_path), exist_ok=True)
plt.savefig(chart_path, dpi=150, bbox_inches="tight")
print(f"Chart saved to {chart_path}")
PYEOF

echo "Done!"
