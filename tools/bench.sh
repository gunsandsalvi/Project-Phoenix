#!/usr/bin/env bash
# The bench: the world itself, run on this machine, and everything its run measures. The one tool for the budget
# (N8), a step's cost and a stage gate's run. Its numbers are costs, never the world's.
#
#   tools/bench.sh [-p persons] [-d days] [-s seed] [-w workers] [-c checks] [-g] [-o dir] [-k] [-B] [-- run arguments]
#
#   -p  persons the world holds (default: the setup's, the committed resolution, where the budget is judged)
#   -d  days run from day zero, settling cut short (default 20, the span the budget's ratchets are measured over)
#   -g  a stage gate's run instead: settled, then two years, every live check (owner, plan section 12)
#   -s  the run's seed (default 1)
#   -w  worker threads of the pool (default: the machine's)
#   -c  live checks: all, none or a list of identities (default none; all with -g)
#   -o  where the report and summary go (default target/bench)
#   -k  keep the report as perf/bench/<commit>-<persons>-<days>.json, the committed measure a resolution change cites
#   -B  run the last build, not building first
#
# Writes <dir>/report.json (the run's whole report, each day's stages in microseconds under core_days[].stages_us),
# <dir>/run.log and <dir>/summary.txt, and prints the summary: the run, the budget, every stage's median, worst and
# share of the day, memory, cores, page faults, the day's flows and the findings.
set -euo pipefail

persons="" days=20 gate=0 seed=1 workers="" checks="" out="target/bench" build=1 keep=0
while getopts "p:d:gs:w:c:o:kB" opt; do
    case $opt in
        p) persons=$OPTARG ;;
        d) days=$OPTARG ;;
        g) gate=1 ;;
        s) seed=$OPTARG ;;
        w) workers=$OPTARG ;;
        c) checks=$OPTARG ;;
        o) out=$OPTARG ;;
        k) keep=1 ;;
        B) build=0 ;;
        *) sed -n '2,19p' "$0"; exit 2 ;;
    esac
done
shift $((OPTIND - 1))
extra=("$@")

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

built=0
if [[ $build == 1 ]]; then
    start=$(date +%s)
    cargo build --release -q -p phx-cli
    built=$(( $(date +%s) - start ))
fi

args=(--seed "$seed" --data data --setup data/setup/default.toml --ratchets perf/ratchets.toml
      --run-dir "$out/run" --report "$out/report.json" --build-seconds "$built")
if [[ $gate == 1 ]]; then
    args+=(--days 730 --checks "${checks:-all}")
else
    args+=(--days "$days" --total-days "$days" --checks "${checks:-none}")
fi
# The budget's ratchets hold at the committed resolution only.
if [[ -n $persons ]]; then args+=(--persons "$persons"); else args+=(--budget perf/budget.toml); fi
if [[ -n $workers ]]; then args+=(--workers "$workers"); fi

mkdir -p "$out"
# A run that fails writes no report, so the last one is removed first and never read as this run's.
rm -rf "$out/report.json" "$out/run"
status=0
target/release/phx run "${args[@]}" "${extra[@]}" > "$out/run.log" 2>&1 || status=$?
if [[ ! -f "$out/report.json" ]]; then
    echo "bench: the run wrote no report (exit $status):"
    tail -20 "$out/run.log"
    exit 1
fi

commit="$(git rev-parse --short=12 HEAD)"
if [[ $keep == 1 ]]; then
    if [[ -n "$(git status --porcelain -- crates data Cargo.toml Cargo.lock)" ]]; then
        echo "bench: the code or data differ from $commit; commit them before keeping a measure" >&2
        exit 1
    fi
    mkdir -p perf/bench
    span=$([[ $gate == 1 ]] && echo gate || echo "${days}d")
    cp "$out/report.json" "perf/bench/$commit-${persons:-committed}-$span.json"
fi

python3 - "$out/report.json" "$commit" <<'PY' | tee "$out/summary.txt"
import json, statistics, sys

r = json.load(open(sys.argv[1]))
b = r.get("budget", {})
days = r.get("core_days", [])
print(f"bench {sys.argv[2]}: seed {r.get('seed')}, {r.get('persons')} persons held of {r.get('persons_opened')} "
      f"opened, {len(days)} days; built in {r.get('build_seconds')} s, assembled in {r.get('assembly_ms')} ms, "
      f"run in {r.get('run_ms')} ms, {r.get('turns')} turns")
print(f"budget: turn median {b.get('turn_ms_median')} ms, worst {b.get('turn_ms_worst')} ms (1000 and 2000); "
      f"{b.get('bytes_per_person')} bytes a person; peak {r.get('peak_resident_bytes', 0) // 2**20} MiB; "
      f"{b.get('busy_core_hundredths', 0) / 100} cores busy; {b.get('faults_per_day')} page faults a day")
for f in b.get("failures", []) or []:
    print(f"  ratchet broken: {f}")
for f in r.get("ratchet_failures", []) or []:
    print(f"  counter ratchet broken: {f}")

ms = [d["ms"] for d in days]
if ms:
    print(f"core days: median {statistics.median(ms):.0f} ms, mean {statistics.mean(ms):.0f} ms, worst {max(ms)} ms; "
          f"flows a day median {statistics.median(d['flows'] for d in days):.0f}, failed "
          f"{sum(d['failed'] for d in days)} in all, breaks {sum(d['breaks'] for d in days)}")
    print(f"{'day':>6} {'ms':>8} {'flows':>9} {'settled':>9} {'failed':>7} {'committed':>9}")
    for d in days:
        print(f"{d['day']:>6} {d['ms']:>8} {d['flows']:>9} {d['settled']:>9} {d['failed']:>7} {d['committed']:>9}")

stages = {}
for d in days:
    for name, us in d.get("stages_us", {}).items():
        stages.setdefault(name, []).append(us / 1000)
if stages:
    total = sum(ms) or 1
    print(f"{'stage':<26} {'median ms':>10} {'worst ms':>10} {'share':>7}")
    top = [n for n in stages if "." not in n]
    order = sorted(top, key=lambda n: -sum(stages[n]))
    for n in order:
        v = stages[n]
        print(f"{n:<26} {statistics.median(v):>10.1f} {max(v):>10.1f} {sum(v) / total:>7.1%}")
        subs = sorted((s for s in stages if s.startswith(n + ".")), key=lambda s: -sum(stages[s]))
        for s in subs:
            w = stages[s]
            print(f"  {s:<24} {statistics.median(w):>10.1f} {max(w):>10.1f} {sum(w) / total:>7.1%}")

print(f"findings: {r.get('findings')}; checks: "
      + ", ".join(f"{c.get('id', '?')} {c.get('verdict', c.get('status', '?'))}" for c in (r.get("checks") or [])[:40]))
PY
exit $status
