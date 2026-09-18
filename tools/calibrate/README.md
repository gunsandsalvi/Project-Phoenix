# What the runtime costs

A CALIBRATION, not a model of the world, and not a test world: it answers the one question that
cannot be answered in TypeScript, because measuring a language's cost requires the other language.

It builds the register's real shape at its real scale — 10,318 parties, 16,750 instruments,
**544,104 holdings in 657,785 lots**, all taken from `npm run world 1` — and runs the engine's own
three hot operations at the engine's own measured call counts. **The TypeScript side of every
comparison is a measured self time from the engine's CPU profile, never a re-implementation here.**

    cargo build --release && ./target/release/calibrate

What it found (2026-09-18, `docs/IMPLEMENTATION.md` 0g.30):

| operation | per period | TypeScript, measured | native SoA | ratio |
|---|---|---|---|---|
| a record fetched by id | 38,676,668 | 47.80 ns | 0.77 ns | **62×** |
| a full traversal of the holdings | ~10 passes | 95.70 ms | 2.23 ms | **43×** |
| the hot read, row index in hand | 23,304,012 | 67.20 ns | 10.62 ns | **6.3×** |
| the same, hashing a packed pair | 23,304,012 | 67.20 ns | 30.29 ns | 2.2× |
| the same, with the standard hasher | 23,304,012 | 67.20 ns | **75.22 ns** | **0.9×** |

**The last row is why this exists.** A native rewrite that keeps the current data model — a hash map
from an id pair to a record — is SLOWER than V8. The win is the language AND the layout AND ids that
are row indices, and a proposal that claims the language alone is quoting a number nobody measured.

It does NOT measure the module code, which is 32.8 s of a 54.7 s period. That is 0g.40's job and
the migration is gated on it.

## And what module code costs (0g.40)

The kernel's ratios above apply to 22.6 s of a profiled 54.7 s period. The other **32.8 s is module
code**, and nothing had measured it — so the migration is gated on this number and dies under 6×.

`cargo run --release --bin module` ports `capital-programme`'s `plantMoves` audit family, which is
**2,049 ms of self time: 3.75% of a period and 95% of its own module.** It was chosen because it is
arithmetic and bookkeeping rather than orchestration, which is what the other forty-nine are made
of. Its inputs are probed from the full world: **497,338 legs walked, 496,246 classified, 1,034,257
distinct (party, instrument) keys, 21,490 holdings read over 479 capital lines.**

| | ms | ratio |
|---|---|---|
| TypeScript, measured in the engine | 2,049.0 | — |
| **A. the same algorithm, in Rust** — a hash map to a growable list per key | **173.6** | **11.8×** |
| **B. the native shape** — keys packed in a `u64`, terms in one flat column, nothing allocated per key | **127.1** | **16.1×** |

Both ports keep the terms per key rather than a running total, because Law 7's dust is derived from
the terms; dropping them would be a different check, and both print the same checksum.

**Correction, stated because it goes the wrong way:** the generated map holds 953,992 keys against
the world's 1,034,257 — 92% — so the Rust side does slightly less work and the ratios are
overstated by that much. Corrected: **A 10.9×, B 14.8×.**

**A is the number that matters**, because it is what a port that translates rather than redesigns
gets. Both clear 6× by a wide margin.
