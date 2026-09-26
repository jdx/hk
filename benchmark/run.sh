#!/usr/bin/env bash
# Runs the competitor benchmark end to end and writes benchmark/results.json,
# which docs/benchmarks.md renders.
#
#   1. setup.sh     fixture and one clone per subject
#   2. tak run      wall time, subjects interleaved, every sample checked
#                   against the fixture's clean commit
#   3. report.py    results.json, marked publishable only if the run is sound
#
# Usage: benchmark/run.sh [tak run flags...]   e.g. --bench fix-staged --runs 3
#
# A run narrowed with --bench is for diagnosis: results.json records that it is
# not publishable.
# Environment:
#   HK_BIN   hk binary to measure (default: target/release/hk, built first)
#   BENCH_RUNNER, BENCH_WORKFLOW_RUN   provenance recorded in results.json
set -euo pipefail

BENCH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -z "${HK_BIN:-}" ]; then
    (cd "$BENCH/.." && mise run perf:build)
fi
# shellcheck source=env.sh
source "$BENCH/env.sh"
echo "Measuring $("$HK_BIN" --version) at $HK_BIN"

"$BENCH/setup.sh"

cd "$BENCH"
# .work outlives the run; never let report.py read an earlier run's export if
# tak fails before writing this one.
rm -f .work/tak.json

# tak drops a subject that fails to run, keeps measuring the rest and exits
# non-zero; report.py then names what is missing instead of the run stopping
# here.
status=0
tak run --config tak.toml --no-counters --export-json .work/tak.json "$@" || status=$?

./report.py .work/tak.json || status=$?
exit "$status"
