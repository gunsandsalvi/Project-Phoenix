#!/usr/bin/env bash
# The build run: after a step's build, the world runs on this machine with every live check and the read trace: an
# ordinary step's for 120 days from day zero, which holds a quarter's save; a stage gate's (--gate) settled and run
# two years. Its numbers test the code and are never read as the world's.
set -euo pipefail

span=(--days 730 --total-days 120)
if [[ "${1:-}" == "--gate" ]]; then
    span=(--days 730)
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

commit="$(git rev-parse --short=12 HEAD)"
if [[ -n "$(git status --porcelain -- crates data Cargo.toml Cargo.lock)" ]]; then
    echo "build-run: the code or data differ from $commit; commit them first" >&2
    exit 1
fi

start=$(date +%s)
cargo build --release -q -p phx-cli
built=$(( $(date +%s) - start ))

mkdir -p perf/build-run
exec target/release/phx run \
    --seed 1 \
    "${span[@]}" \
    --checks all \
    --read-trace \
    --data data \
    --setup data/setup/default.toml \
    --run-dir target/run \
    --ratchets perf/ratchets.toml \
    --report "perf/build-run/$commit.json" \
    --build-seconds "$built"
