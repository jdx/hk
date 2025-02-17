#!/usr/bin/env bash

set -euxo pipefail

# cargo build --profile serious
export PATH="$PWD/target/serious:$PATH"

# git stash || true

# echo "with no changes"
# hyperfine --warmup 1 "hk run pre-commit" "lefthook run pre-commit" "pre-commit run" --export-json tmp/benchmark-no-changes.json

# echo "with unstaged changed"
# echo "// foo" >> src/main.rs
# hyperfine --warmup 1 "hk run pre-commit" "lefthook run pre-commit" "pre-commit run" --export-json tmp/benchmark-unstaged.json

# echo "with staged changed"
# git add src/main.rs
# hyperfine --warmup 1 "hk run pre-commit" "lefthook run pre-commit" "pre-commit run" --export-json tmp/benchmark-staged.json

# echo "on all files"
# hyperfine --warmup 1 "hk run pre-commit --all" "lefthook run pre-commit -f --all-files" "pre-commit run --all-files" --export-json tmp/benchmark-all.json
# git reset src/main.rs
# git checkout -- src/main.rs

cat << 'EOF' | uv run --with matplotlib --with numpy -
import json
import matplotlib.pyplot as plt
import numpy as np

# Read all benchmark files
scenarios = {
    'No Changes': 'tmp/benchmark-no-changes.json',
    'Unstaged': 'tmp/benchmark-unstaged.json',
    'Staged': 'tmp/benchmark-staged.json',
    'All Files': 'tmp/benchmark-all.json'
}

# Setup data structures
tools = ['hk', 'lefthook', 'pre-commit']  # Define tools explicitly to ensure order
tool_times = {tool: [] for tool in tools}
tool_stddevs = {tool: [] for tool in tools}

def normalize_tool_name(command):
    if 'hk' in command:
        return 'hk'
    elif 'lefthook' in command:
        return 'lefthook'
    else:
        return 'pre-commit'

# Read data from each benchmark file
for scenario, filename in scenarios.items():
    try:
        with open(filename) as f:
            print(f"\nProcessing {filename}...")
            data = json.load(f)
            
            # Collect data for each tool
            for result in data['results']:
                command = result['command']
                tool_name = normalize_tool_name(command)
                print(f"Command: {command} -> Tool: {tool_name}")
                tool_times[tool_name].append(result['mean'])
                tool_stddevs[tool_name].append(result['stddev'])
    except FileNotFoundError:
        print(f"Warning: {filename} not found, skipping...")
        # Fill with dummy data for this scenario
        for tool in tools:
            tool_times[tool].append(0)
            tool_stddevs[tool].append(0)

# Plotting
fig, ax = plt.subplots(figsize=(12, 6))

# Set the positions for the bars
num_scenarios = len(scenarios)
width = 0.2  # Width of each bar
positions = np.arange(num_scenarios)

# Plot bars for each tool
for idx, tool in enumerate(tools):
    offset = (idx - (len(tools)-1)/2) * width
    bars = ax.bar(positions + offset, tool_times[tool], width, 
                  yerr=tool_stddevs[tool],
                  capsize=5,
                  label=tool)
    
    # Add value labels on top of bars
    for bar in bars:
        height = bar.get_height()
        ax.text(bar.get_x() + bar.get_width()/2., height,
                f'{height:.3f}s',
                ha='center', va='bottom',
                rotation=45,
                fontsize=8)

# Customize the plot
ax.set_ylabel('Time (seconds)')
ax.set_title('Pre-commit Tool Performance Comparison')
ax.set_xticks(positions)
ax.set_xticklabels(scenarios.keys())
ax.legend()

# Adjust layout to prevent label cutoff
plt.tight_layout()

plt.savefig('docs/public/benchmark.png')
print("Chart saved as docs/public/benchmark.png")
EOF

git stash pop || true
