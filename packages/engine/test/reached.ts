/**
 * What this world DECLARED it could do, against what has ever come of it (Audit E1, E2).
 *
 * @spec Audit E1 Audit E2 Appendix C
 *
 * `docs/COVERAGE.md` answers completeness from the `@spec` citations in the source, which says a
 * module implementing the clause EXISTS and says nothing about whether it has ever run. Ninety-six
 * rows cite a module that has never produced an outcome — no corporate bond issued, no insurance
 * party created, no order at the tenancy venue, eight of nine derivative books never printed — and
 * the only way to learn that was to read the source three times over.
 *
 * This runs the two scale models and asks the kernel's own `Reach` register, which declares every
 * capability at assembly so "never" is a MEASURED state rather than an absence of evidence. It
 * repairs nothing and asserts nothing: what it prints goes into `docs/COVERAGE.md` by hand, where
 * completeness lives. It is a measurement, so it runs when a module is done and not in the suite
 * (Law 11) — `npm run coverage:reached [periods]`, default 52, which is a year.
 *
 * It lives beside the rig rather than in `tools/` because the rig is what it measures and `tools/`
 * compiles as its own project with its own root.
 */
import { abroadWorld, rigWorld } from './rig.js';
import type { Capability } from '../src/world/reach.js';
import type { World } from '../src/index.js';

/** `types: []` keeps the console out of the engine, which is the rule; a tool that prints says so. */
declare const console: { log: (line: string) => void };
declare const process: { argv: readonly string[] };

const PERIODS = Number(process.argv[2] ?? '52');

function walk(name: string, make: (seed: string) => World): readonly Capability[] {
  const w = make('reached');
  for (let i = 0; i < PERIODS; i += 1) w.step();
  const all = w.reach();
  const never = all.filter((c) => c.produced === 0).length;
  console.log(
    `${name}: ${all.length} declared, ${all.length - never} reached, ${never} never, over ${PERIODS} periods`,
  );
  return all;
}

/** Reached if EITHER world reached it: one country cannot exercise a currency pair (XI-12). */
function union(a: readonly Capability[], b: readonly Capability[]): Capability[] {
  const by = new Map<string, Capability>();
  for (const c of [...a, ...b]) {
    const k = `${c.kind}:${c.id}`;
    const held = by.get(k);
    if (held === undefined || held.produced < c.produced) by.set(k, c);
  }
  return [...by.values()];
}

const all = union(walk('rig', rigWorld), walk('abroad', abroadWorld));
const byOwner = new Map<string, { declared: number; never: number; dead: string[] }>();
for (const c of all) {
  const row = byOwner.get(c.owner) ?? { declared: 0, never: 0, dead: [] };
  row.declared += 1;
  if (c.produced === 0) {
    row.never += 1;
    row.dead.push(`${c.kind}:${c.id}`);
  }
  byOwner.set(c.owner, row);
}

console.log('\n| module | declared | never produced |');
console.log('|---|---|---|');
for (const [owner, row] of [...byOwner].sort((x, y) =>
  y[1].never === x[1].never ? x[0].localeCompare(y[0]) : y[1].never - x[1].never,
)) {
  console.log(`| ${owner} | ${row.declared} | ${row.never} |`);
}
console.log('\nnever produced, in full:\n');
for (const [owner, row] of [...byOwner].sort((x, y) => x[0].localeCompare(y[0]))) {
  if (row.dead.length === 0) continue;
  console.log(`${owner}: ${row.dead.sort().join(', ')}`);
}
