#!/usr/bin/env bash
# The bench: the world itself, run on this machine, and everything its run measures. The one tool for the budget
# (N8), a step's cost and a stage gate's run. Its numbers are costs, never the world's.
#
#   tools/bench.sh [-p persons] [-d days] [-s seed] [-w workers] [-c checks] [-g] [-o dir] [-k] [-B] [-t seconds]
#                  [-- run arguments]
#
#   -p  persons the world holds (default: the setup's, the committed resolution, where the budget is judged)
#   -d  days run from day zero, settling cut short (default 20, the span the budget's ratchets are measured over)
#   -g  a stage gate's run instead: settled, then two years, every live check (owner, plan section 12)
#   -s  the run's seed (default 1)
#   -w  worker threads of the pool (default: the machine's)
#   -c  live checks: all, none or a list of identities (default none; all with -g)
#   -o  where the log, report and summary go (default target/bench)
#   -k  keep the report as perf/bench/<commit>-<persons>-<days>.json, the committed measure a resolution change cites
#   -B  run the last build, not building first
#   -t  stop the world after this many seconds; the summary is still read from its log
#
# The run's trace is printed and written to <dir>/run.log as it happens: every span of the opening and of each day —
# stages, sub-stages, settlement's passes, each product's meeting — as it begins and ends with its own time, and notes
# of what each did (parties, flows, buyers, stalls, rounds, sales, failures), each line with the time since the run
# began and the memory held. Loops note how far they are at each power of two of their rounds, so a runaway shows.
# Ctrl-C or -t stops the world, not the bench. Then <dir>/summary.txt: the run and the budget from <dir>/report.json
# where the run finished; the spans still open where it did not; every span's count, total, mean and worst; the
# heaviest meetings; settlement's and the day's notes; the parties opened; the findings and checks.
set -euo pipefail

persons="" days=20 gate=0 seed=1 workers="" checks="" out="target/bench" build=1 keep=0 limit=""
while getopts "p:d:gs:w:c:o:kBt:" opt; do
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
        t) limit=$OPTARG ;;
        *) sed -n '2,/^set -euo/p' "$0" | sed '$d'; exit 2 ;;
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
target/release/phx run "${args[@]}" "${extra[@]}" > "$out/run.log" 2>&1 &
pid=$!
# Ctrl-C or the time limit stops the world, never the bench: the summary is still read from what the log holds.
trap 'kill "$pid" 2>/dev/null || true' INT TERM
tail -n +1 -f --pid="$pid" "$out/run.log" &
tailer=$!
watchdog=""
if [[ -n $limit ]]; then
    ( sleep "$limit"; echo "bench: stopped at the time limit of $limit s" >> "$out/run.log"; kill "$pid" 2>/dev/null ) &
    watchdog=$!
fi
wait "$pid" || status=$?
wait "$tailer" 2>/dev/null || true
if [[ -n $watchdog ]]; then kill "$watchdog" 2>/dev/null || true; fi
trap - INT TERM

commit="$(git rev-parse --short=12 HEAD)"
if [[ $keep == 1 && -f "$out/report.json" ]]; then
    if [[ -n "$(git status --porcelain -- crates data Cargo.toml Cargo.lock)" ]]; then
        echo "bench: the code or data differ from $commit; commit them before keeping a measure" >&2
        exit 1
    fi
    mkdir -p perf/bench
    span=$([[ $gate == 1 ]] && echo gate || echo "${days}d")
    cp "$out/report.json" "perf/bench/$commit-${persons:-committed}-$span.json"
fi

python3 - "$out" "$commit" "$status" <<'SUMMARY' | tee "$out/summary.txt"
import json, os, re, statistics, sys

out, commit, status = sys.argv[1], sys.argv[2], int(sys.argv[3])
log = open(os.path.join(out, "run.log"), errors="replace").read().splitlines()
path = os.path.join(out, "report.json")
r = json.load(open(path)) if os.path.exists(path) else None

# The trace: every line "<seconds>s <MiB> MiB <indent><mark> <name> <rest>".
line_re = re.compile(r"^\s*([\d.]+)s\s+(\S+) MiB (\s*)([<>=]) (\S+) ?(.*)$")
inner = ("goods.meet", "goods.buyers", "goods.stalls", "goods.sales")
spans, open_spans, notes, meetings, rounds = {}, [], {}, [], {}
peak, last_t, context, pending = 0, 0.0, {}, None
for text in log:
    m = line_re.match(text)
    if not m:
        continue
    t, mib, _, mark, name, rest = m.groups()
    last_t = float(t)
    if mib.isdigit():
        peak = max(peak, int(mib))
    if mark == ">":
        open_spans.append((name, float(t)))
    elif mark == "<":
        for i in range(len(open_spans) - 1, -1, -1):
            if open_spans[i][0] == name:
                del open_spans[i]
                break
        ms = re.match(r"([\d.]+) ms", rest)
        if ms:
            v = float(ms.group(1))
            s = spans.setdefault(name, [0, 0.0, 0.0])
            s[0] += 1
            s[1] += v
            s[2] = max(s[2], v)
            if name == "goods.meet" and pending is not None:
                meetings.append((v, context.get("stage", "?"), context.get("product", "?"), pending))
                pending = None
    else:
        fields = dict(kv.split("=", 1) for kv in rest.split() if "=" in kv)
        notes.setdefault(name, []).append((float(t), fields))
        if name == "goods.product":
            context["product"] = fields.get("product")
            context["stage"] = next(
                (n for n, _ in reversed(open_spans) if n.startswith("goods.") and n not in inner), "?")
        elif name == "meet":
            pending = fields
        elif name in ("meet.round", "settle.round"):
            rounds[name] = fields

print(f"bench {commit}: exit {status}; the log's last mark at {last_t:.1f} s; peak {peak} MiB by the trace")
if r:
    b = r.get("budget", {})
    print(f"run: seed {r.get('seed')}, {r.get('persons')} persons held of {r.get('persons_opened')} opened, "
          f"{len(r.get('core_days', []))} days; built in {r.get('build_seconds')} s, assembled in "
          f"{r.get('assembly_ms')} ms, run in {r.get('run_ms')} ms, {r.get('turns')} turns")
    print(f"budget: turn median {b.get('turn_ms_median')} ms, worst {b.get('turn_ms_worst')} ms (1000 and 2000); "
          f"{b.get('bytes_per_person')} bytes a person; peak {r.get('peak_resident_bytes', 0) // 2**20} MiB; "
          f"{b.get('busy_core_hundredths', 0) / 100} cores busy; {b.get('faults_per_day')} page faults a day")
    for f in (b.get("failures") or []) + (r.get("ratchet_failures") or []):
        print(f"  ratchet broken: {f}")
else:
    print("run: no report; the run did not finish, so what follows is read from its log alone")

if open_spans:
    print("still open when the log ended, outermost first (where the run stood):")
    for name, t0 in open_spans:
        print(f"  {name:<30} begun at {t0:>10.3f} s, open {last_t - t0:>10.3f} s")
    for name, fields in rounds.items():
        print(f"  last {name}: " + " ".join(f"{k}={v}" for k, v in fields.items()))

days = (r or {}).get("core_days", [])
if days:
    ms = [d["ms"] for d in days]
    print(f"core days: median {statistics.median(ms):.0f} ms, worst {max(ms)} ms; flows a day median "
          f"{statistics.median(d['flows'] for d in days):.0f}, failed {sum(d['failed'] for d in days)}, "
          f"breaks {sum(d['breaks'] for d in days)}")
    print(f"{'day':>6} {'ms':>8} {'flows':>9} {'settled':>9} {'failed':>7} {'committed':>9}")
    for d in days:
        print(f"{d['day']:>6} {d['ms']:>8} {d['flows']:>9} {d['settled']:>9} {d['failed']:>7} {d['committed']:>9}")

if spans:
    print(f"{'span':<30} {'times':>6} {'total ms':>11} {'mean ms':>9} {'worst ms':>10}")
    for name, (n, total, worst) in sorted(spans.items(), key=lambda kv: -kv[1][1])[:60]:
        print(f"{name:<30} {n:>6} {total:>11.1f} {total / n:>9.1f} {worst:>10.1f}")

if meetings:
    print("heaviest meetings: ms, stage, product, buyers, stalls, places, rounds, sales, unserved")
    for v, stage, product, f in sorted(meetings, key=lambda x: -x[0])[:20]:
        print(f"  {v:>9.1f} {stage:<20} {product:>5} " + " ".join(
            f"{f.get(k, '?'):>8}" for k in ("buyers", "stalls", "places", "rounds", "sales", "unserved")))

for name in ("settle.gathered", "settle.currency", "settle.outcome", "settle.banks_short", "settle.funding", "hazards",
             "goods", "settle"):
    for t, fields in notes.get(name, [])[-6:]:
        print(f"note {name} at {t:.1f} s: " + " ".join(f"{k}={v}" for k, v in fields.items()))
dues = {n: v[-1][1]["dues"] for n, v in notes.items() if set(v[-1][1]) == {"dues"}}
if dues:
    print("the last day's dues by family: " + ", ".join(f"{n} {d}" for n, d in dues.items()))
kinds = [(n, v) for n, v in notes.items() if len(v) == 1 and set(v[0][1]) == {"parties"}]
if kinds:
    print("parties at the opening: " + ", ".join(f"{n} {v[0][1]['parties']}" for n, v in kinds))

if r:
    print(f"findings: {r.get('findings')}; checks: " + ", ".join(
        f"{c.get('id', '?')} {c.get('verdict', c.get('status', '?'))}" for c in (r.get("checks") or [])[:40]))
print("the log's last lines:")
for text in log[-12:]:
    print(f"  {text}")
SUMMARY
exit $status
