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
