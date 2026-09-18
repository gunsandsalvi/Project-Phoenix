/**
 * THE WORLD ITSELF, RUN. Not a scale model and not a test: `foundationWorld` at its declared scale
 * — `BANK_COUNT` banks, `FIRM_COUNT` firms, every country — stepped until it stops.
 *
 * It exists because every performance and liveness figure this project has ever quoted was taken on
 * a rig of a few hundred parties, and the world the targets are about was never run. It assembles
 * into 10,318 parties and did not reach the end of period 1. That is the single most important
 * fact about this engine and it was invisible, because the only things that ran were scale models.
 *
 * `npm run world [periods]`. It prints what the world is, how far it got, and WHERE THE PERIOD
 * WENT — in questions rather than milliseconds (`src/world/work.ts`), because the questions are
 * what a period costs and the duration is only their shadow. It exits non-zero the moment the
 * simulation throws: the simulation IS the error, which is the point of it.
 */
import { foundationWorld, HOUSEHOLD, SMALL_FIRM } from '../src/index.js';
import { moved, ops, resetMoved, resetOps, totalOps } from '../src/core/ops.js';

declare const console: { log: (l: string) => void; error: (l: string) => void };
declare const process: {
  argv: readonly string[];
  exit: (code: number) => never;
  memoryUsage: () => { heapUsed: number };
};

const periods = Number(process.argv[2] ?? '52');
const mb = (): string => `${(process.memoryUsage().heapUsed / 1048576).toFixed(0)}MB`;

const began = Date.now();
const w = foundationWorld('world');
let named = 0;
let cells = 0;
let people = 0;
let small = 0;
for (const p of w.parties.all()) {
  if (!p.status.alive) continue;
  if (p.representation !== 'cell') {
    named += 1;
    continue;
  }
  cells += 1;
  if (p.kind === HOUSEHOLD) people += p.weight;
  if (p.kind === SMALL_FIRM) small += p.weight;
}
console.log(
  `WORLD assembled in ${((Date.now() - began) / 1000).toFixed(1)}s: ${named + cells} parties ` +
    `(${named} named, ${cells} cells), ${people} people, ${small} small firms, ` +
    `${w.instruments.all().length} instruments, ${w.markets.length} markets, ${w.venues.length} venues, ${mb()}`,
);

for (let i = 1; i <= periods; i += 1) {
  const at = Date.now();
  try {
    const r = w.step();
    const alive = w.parties.all().filter((p) => p.status.alive).length;
    console.log(
      `p${i} ${((Date.now() - at) / 1000).toFixed(1)}s | parties ${alive} | events ${w.journal.inPeriod(r.period).length}` +
        ` | audit ${r.audit.total} | ${mb()}`,
    );
    console.log(
      `   ops ${totalOps()} = instrument ${ops.instrument} + holding ${ops.holding}` +
        ` + price ${ops.price} + party ${ops.party} + measure ${ops.measure}`,
    );
    resetOps();
    const holdings = w.register.allHoldings();
    let lots = 0;
    for (const h of holdings) lots += h.lots.length;
    console.log(
      `   state holdings ${holdings.length} lots ${lots} instruments ${w.instruments.all().length}` +
        ` prints ${w.prices.instruments().length} events ${w.journal.all().length}`,
    );
    console.log(
      `   moved holdings ${moved.holdings.size} of ${holdings.length} · prices ${moved.prices.size} of ${w.instruments.all().length}` +
        ` · parties ${moved.parties.size} of ${alive} · legs ${moved.legs}`,
    );
    resetMoved();
    const byKind = new Map<string, number>();
    for (const e of w.journal.inPeriod(r.period)) {
      byKind.set(e.kind, (byKind.get(e.kind) ?? 0) + 1);
    }
    console.log('   events by kind, dearest:');
    for (const [k, n] of [...byKind].sort((a, b) => b[1] - a[1]).slice(0, 12)) {
      console.log(`   ${String(n).padStart(8)}  ${k}`);
    }
    const rows = w.work.all();
    console.log(`   asks ${w.work.asks()} over ${rows.length} declarations, dearest:`);
    for (const q of rows.slice(0, 22)) {
      console.log(
        `   ${String(q.asks).padStart(9)} asks ${String(q.narrows).padStart(7)} narrows` +
          ` -> ${String(q.orders).padStart(6)} orders ${String(q.reads).padStart(10)} reads  ${q.at} ${q.capability}`,
      );
    }
  } catch (e) {
    // The simulation's own refusal, reported as the failure it is. No test asserts this and none
    // should: what is wrong is the world, and the world is what says so.
    console.error(`p${i} STOPPED after ${((Date.now() - at) / 1000).toFixed(1)}s: ${String((e as { message?: string }).message ?? e)}`);
    if (e instanceof Error && e.stack !== undefined) {
      console.error(e.stack.split('\n').slice(1, 7).join('\n'));
    }
    process.exit(1);
  }
}
console.log(`WORLD ran ${periods} periods without stopping.`);
