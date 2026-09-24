#!/usr/bin/env bash
# Runs the competitor benchmark end to end and writes benchmark/results.json,
# which docs/benchmarks.md renders.
#
#   1. setup.sh     fixture and one clone per subject
#   2. verify.py    did each subject produce the right files?
#   3. tak run      wall time, subjects interleaved
#   4. report.py    results.json, marked publishable only if the run is sound
#
# Usage: benchmark/run.sh [tak run flags...]   e.g. --bench fix-staged --runs 3
#
# A run narrowed with --bench is for diagnosis: it verifies and times only the
# named benchmarks, and results.json records that it is not publishable.
# Environment:
#   HK_BIN   hk binary to measure (default: target/release/hk, built first)
#   TRIALS   verification trials per subject (default: 5)
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
# Verify the same benchmarks tak will time.
benches=()
args=("$@")
for ((i = 0; i < ${#args[@]}; i++)); do
    case "${args[i]}" in
    --bench) benches+=(--bench "${args[i + 1]}") ;;
    --bench=*) benches+=(--bench "${args[i]#--bench=}") ;;
    esac
done
echo "Verifying..."
./verify.py --trials "${TRIALS:-5}" "${benches[@]}" || true

echo "Timing..."
# tak drops a failing subject, keeps measuring the rest and exits non-zero;
# report.py then names what is missing instead of the run stopping here.
status=0
tak run --no-counters --export-json .work/tak.json "$@" || status=$?

./report.py .work/tak.json .work/verify.json || status=$?
exit "$status"
