/**
 * THE CHRONICLE: what actually happens in the assembled world, period by period, in words.
 *
 * `npm run chronicle -- <periods> [rig|abroad]`
 *
 * Every diagnosis this project has written was a probe built for one question and thrown away. The
 * material to answer all of them has been in the stores for months — the journal has every event
 * with its subjects, the ledger every instruction with its cause and its failure, `reach` what was
 * declared, the liveness family what has produced nothing, and `why(party, period)` what a party
 * wanted and did not get (0h.4). This is the read that puts them on one page.
 *
 * It is a REPORT and asserts nothing (Law 11): what it prints is what the world did, and a number
 * that looks wrong in it is a finding for `docs/IMPLEMENTATION.md`, not a thing to fix here.
 */
import {
  type PartyId,
  type World,
  foundationWorld,
  isMoneyLeg,
  isAssetLeg,
} from '../src/index.js';
import { abroadWorld, rigWorld } from './rig.js';

/** `types: []` keeps the console out of the engine; a tool whose output IS its value says so. */
declare const console: { log: (line: string) => void };
declare const process: { argv: readonly string[] };

/**
 * A LONG READ SAYS WHAT IT HAS AS IT HAS IT. The full world is thirty banks and nine thousand firms
 * in four countries, and a period of it is minutes: a report that prints when it is finished is a
 * report nobody sees until then, and a run that dies in period two prints nothing at all.
 */
const out: string[] = [];
const say = (line: string): void => {
  out.push(line);
  console.log(line);
};

/** The top `n` of a tally, biggest first, as `name×count` — the shape every line here uses. */
function top(counts: ReadonlyMap<string, number>, n: number): string {
  return [...counts]
    .sort((a, b) => (b[1] === a[1] ? a[0].localeCompare(b[0]) : b[1] - a[1]))
    .slice(0, n)
    .map(([k, v]) => `${k}×${v}`)
    .join(' ');
}

function add(counts: Map<string, number>, key: string, by = 1): void {
  const had = counts.get(key);
  counts.set(key, had === undefined ? by : had + by);
}

/**
 * WHO IS IN IT, before anything runs: the world as the seed left it.
 *
 * XI-15: A PARTY ROW IS NOT A COUNT OF ANYTHING. A cell is one possible party carried with a
 * multiplicity, so sixty household rows are thirty million people a nation and a thousand
 * small-firm rows are a few million businesses. Printing the rows alone said this world had sixty
 * households in it, which is the one reading of a cell that is always wrong.
 */
function opening(w: World, name: string): void {
  const kinds = new Map<string, number>();
  const members = new Map<string, number>();
  for (const p of w.parties.all()) {
    add(kinds, String(p.kind));
    add(members, String(p.kind), p.representation === 'cell' ? p.weight : 1);
  }
  const instruments = new Map<string, number>();
  for (const i of w.instruments.all()) add(instruments, String(i.kind));
  say(`— ${name} —`);
  const people = [...members].reduce((n, [, v]) => n + v, 0);
  say(`party rows ${w.parties.all().length}: ${top(kinds, 10)}`);
  say(`who they ARE (weights) ${people}: ${top(members, 10)}`);
  say(`instruments ${w.instruments.all().length}: ${top(instruments, 8)}`);
  say(`markets ${w.markets.length}, money ${JSON.stringify(w.moneyStock())}`);
}

/** WHAT THE PERIOD DID: the books, the wire, the work, and what refused. */
function period(w: World, report: ReturnType<World['step']>, since: number): void {
  const at = report.period;
  const books = new Map<string, number>();
  for (const m of report.markets) add(books, m.outcome);
  // THE WIRE: every instruction of the period by what it was for, and every failure by its reason.
  const causes = new Map<string, number>();
  const failures = new Map<string, number>();
  let settled = 0;
  let failed = 0;
  const paid = new Map<string, number>();
  const moved = new Map<string, number>();
  for (const r of w.ledger.inPeriod(at)) {
    if (r.outcome === 'settled') {
      settled += 1;
      add(causes, r.instruction.cause);
      for (const leg of r.instruction.legs) {
        // What kind of money this was — a wage, a coupon, a tax — as the PAYER declared it.
        if (isMoneyLeg(leg)) add(paid, leg.receipt.of);
        else if (isAssetLeg(leg)) add(moved, String(w.instruments.get(leg.instrument).kind));
      }
      continue;
    }
    failed += 1;
    add(failures, `${r.reason.kind}/${r.instruction.cause}`);
  }
  // WHAT WAS SAID: the world's own events, which is where the mechanisms speak.
  const events = new Map<string, number>();
  for (const e of w.journal.inPeriod(at)) add(events, e.kind);
  // WHAT NOBODY COULD HAVE: the reads that came back Missing, over every living party (0h.4).
  const wanted = new Map<string, number>();
  let quiet = 0;
  for (const p of w.parties.alive()) {
    const y = w.why(p.id, at);
    for (const want of y.wanted) add(wanted, want.read.split('.')[0] ?? want.read, want.times);
    if (y.settled.length === 0) quiet += 1;
  }
  const families = new Map<string, number>();
  for (const f of report.audit.families) if (f.count > 0) families.set(f.family, f.count);
  say('');
  let alive = 0;
  for (const p of w.parties.alive()) alive += p.representation === 'cell' ? p.weight : 1;
  say(
    `period ${at} — ${w.parties.alive().length} rows carrying ${alive}, ` +
      `${quiet} rows a side of nothing (${Date.now() - since}ms)`,
  );
  say(`  books   ${top(books, 6)}`);
  say(`  wire    ${settled} settled (${top(causes, 6)}), ${failed} failed (${top(failures, 4)})`);
  say(`  money   ${top(paid, 8)}`);
  if (moved.size > 0) say(`  things  ${top(moved, 6)}`);
  say(`  said    ${top(events, 8)}`);
  say(`  wanted  ${top(wanted, 6)}`);
  say(`  audit   ${top(families, 8)}`);
}

/** AND WHAT HAS NEVER HAPPENED AT ALL: the standing absences, once, at the end. */
function closing(w: World): void {
  const never = w.reach().filter((c) => c.produced === 0);
  const byOwner = new Map<string, number>();
  for (const c of never) add(byOwner, c.owner);
  say('');
  say(`never reached: ${never.length} of ${w.reach().length} declared — ${top(byOwner, 8)}`);
  // ONE PARTY OF EACH KIND, ASKED WHY (0h.4). A kind is a row; the party is whichever came first.
  const seen = new Set<string>();
  for (const p of w.parties.alive()) {
    if (seen.has(String(p.kind))) continue;
    seen.add(String(p.kind));
    const y = w.why(p.id, w.period);
    const wants = y.wanted.slice(0, 3).map((x) => `${x.read}×${x.times}`).join(' ');
    say(
      `  ${String(p.kind)} ${String(p.id)}: ${y.settled.length} instructions, ` +
        `${y.events.length} kinds of event, quiet in ${y.quietIn.length} books` +
        (wants.length > 0 ? `, wanted ${wants}` : ''),
    );
  }
}

/**
 * `full` is THE WORLD ITSELF — thirty banks, nine thousand firms, four countries (`foundationWorld`)
 * — and not a scale model of it. The rig is what the suite can afford; this is what the model IS,
 * and a reading of the first periods of it is a different fact from a reading of the rig's.
 */
export function chronicle(periods: number, which: 'rig' | 'abroad' | 'full'): string {
  const began = Date.now();
  const w =
    which === 'full'
      ? foundationWorld('chronicle')
      : which === 'abroad'
        ? abroadWorld('chronicle')
        : rigWorld('chronicle');
  out.length = 0;
  opening(w, `${which}, ${periods} periods (assembled in ${Date.now() - began}ms)`);
  for (let i = 0; i < periods; i += 1) {
    const at = Date.now();
    period(w, w.step(), at);
  }
  closing(w);
  return out.join('\n');
}

const [periodsArg, whichArg] = process.argv.slice(2);
if (periodsArg !== undefined) {
  const which = whichArg === 'abroad' ? 'abroad' : whichArg === 'full' ? 'full' : 'rig';
  chronicle(Number(periodsArg), which);
}

/** So the party type is not imported for nothing when a reader adds a per-party line. */
export type Named = PartyId;
