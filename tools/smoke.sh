#!/usr/bin/env bash
# The smoke: what a business day costs at the committed resolution, read at every step (N8.2, N8.4). Twenty days from
# day zero, no live checks; the report's budget block against perf/budget.toml. `--bench` also runs the full-load
# bench. Its numbers are costs, never the world's. Any further arguments go to the run.
set -euo pipefail

bench=0
if [[ "${1:-}" == "--bench" ]]; then
    bench=1
    shift
fi
extra=("$@")

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

cargo build --release -q -p phx-cli
out="target/smoke"
mkdir -p "$out"
# A run that fails writes no report, so the last one is removed first and never read as this run's.
rm -f "$out/report.json"
status=0
target/release/phx run \
    --seed 1 \
    --days 20 \
    --total-days 20 \
    --checks none \
    --data data \
    --setup data/setup/default.toml \
    --run-dir "$out/run" \
    --ratchets perf/ratchets.toml \
    --budget perf/budget.toml \
    --report "$out/report.json" \
    "${extra[@]}" > "$out/run.log" || status=$?
grep -E "^(budget:|[0-9]+ turns)" "$out/run.log" || true
if [[ ! -f "$out/report.json" ]]; then
    echo "smoke: the run wrote no report (exit $status):"
    tail -20 "$out/run.log"
    exit 1
fi
python3 - "$out/report.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
b = r["budget"]
print(f"smoke: {b['persons']} persons; turn median {b['turn_ms_median']} ms, worst {b['turn_ms_worst']} ms "
      f"(budget 1000 and 2000); {b['bytes_per_person']} bytes a person; {b['busy_core_hundredths'] / 100} cores busy; "
      f"{b['faults_per_day']} page faults a day")
days = sorted(d["ms"] for d in r["core_days"])
if days:
    print(f"smoke: the core's days: median {days[(len(days) - 1) // 2]} ms, worst {days[-1]} ms")
if b["failures"]:
    sys.exit(1)
PY
if [[ $bench == 1 ]]; then
    cargo test -q --release -p phx-ffi load_at_full_volumes -- --ignored --nocapture > "$out/bench.log" 2>&1
    tail -5 "$out/bench.log"
fi
# The run's own verdict includes the counters' ratchets and live checks, which the smoke does not judge.
exit 0
