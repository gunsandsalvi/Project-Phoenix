#!/usr/bin/env bash
# The build run: after a step's build, the world is settled and run two years on this machine, with every live check
# and the read trace. Its numbers test the code and are never read as the world's.
set -euo pipefail

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
    --settle 1 \
    --days 730 \
    --checks all \
    --read-trace \
    --data data \
    --ratchets perf/ratchets.toml \
    --report "perf/build-run/$commit.json" \
    --build-seconds "$built"
