/**
 * 0g.1: THE LADDER, one rung per process. Law 18: performance work changes layout and traversal
 * and never a mechanism, so every step of 0g reports this before and after, and a step that moves
 * a ratio is reverted. It is a measurement, so it runs when a step is done and not in the suite
 * (Law 11): `npm run ladder -- <banks> <firms> <grain>`; the invariants a rung must hold are in
 * `ladder.test.ts`, on the first rung, where the suite can afford them.
 *
 * What it prints is time and the shape of a year: ms per period at four marks, parties, cells,
 * people, events, sessions cleared, audit findings, money per member, the highest going rate.
 *
 * 0g.3: AND WHERE THE TIME GOES, with `--where`. The rung said how long a period took and never
 * what it was spent on, so every step of 0g so far built a throwaway harness to find out and no
 * two of those measurements were comparable. `whereItGoes` wraps each module's phases and each
 * participant's `orders` — module code the KERNEL calls, which is why it never showed up as a
 * phase — and times the audit. It answers the one question a step has to ask before it starts.
 */
import {
  HOUSEHOLD,
  SMALL_FIRM,
  assemble,
  moneyInstrumentId,
  type MechanismContext,
  type World,
} from '../src/index.js';
import { Audit } from '../src/audit/audit.js';
import { refined } from './grain.js';
import { rigSpec } from './rig.js';

/** `types: []` keeps the console out of the engine, which is the rule; a tool that prints says so. */
declare const console: { log: (line: string) => void };
declare const process: { argv: readonly string[]; memoryUsage: () => { heapUsed: number } };

export const YEAR = 52;
export const MARKS = [1, 13, 26, 52];

export interface Rung {
  readonly banks: number;
  readonly firms: number;
  readonly grain: 1 | 2;
  readonly msPerPeriodAt: Record<number, number>;
  readonly heapMbAt: Record<number, number>;
  readonly parties: number;
  readonly people: number;
  readonly smallFirms: number;
  readonly cells: number;
  readonly events: number;
  readonly sessionsCleared: number;
  readonly auditTotal: number;
  readonly moneyPerMember: number;
  readonly wagePerHour: number;
  /**
   * 0g.17: EVERY PERIOD'S OWN TIME, in order, so a claim about ms/period can be a distribution
   * rather than one number. `msPerPeriodAt` is CUMULATIVE elapsed over the mark — it carries
   * period 1's cost (640 ms against a steady 280) into every figure after it — and it is kept
   * unchanged because earlier 0g records quote it. This is what a step should be judged on.
   */
  readonly each: readonly number[];
}

export function runRung(banks: number, firms: number, grain: 1 | 2, periods = YEAR): Rung {
  const spec = rigSpec(`ladder-${banks}-${firms}`, banks, firms);
  const w: World = assemble(grain === 1 ? spec : refined(spec));
  const msPerPeriodAt: Record<number, number> = {};
  const heapMbAt: Record<number, number> = {};
  let sessionsCleared = 0;
  let auditTotal = 0;
  let events = 0;
  const started = Date.now();
  const each: number[] = [];
  for (let i = 1; i <= periods; i += 1) {
    const at = Date.now();
    const report = w.step();
    each.push(Date.now() - at);
    sessionsCleared += report.markets.filter((m) => m.outcome === 'cleared').length;
    auditTotal += report.audit.total;
    events += w.journal.inPeriod(report.period).length;
    if (MARKS.includes(i) || i === periods) {
      msPerPeriodAt[i] = (Date.now() - started) / i;
      heapMbAt[i] = process.memoryUsage().heapUsed / (1024 * 1024);
    }
  }
  let people = 0;
  let cells = 0;
  let smallFirms = 0;
  let money = 0;
  for (const p of w.parties.all()) {
    if (p.representation !== 'cell' || !p.status.alive) continue;
    cells += 1;
    if (p.kind === HOUSEHOLD) {
      people += p.weight;
      money += w.register.quantity(p.id, moneyInstrumentId(p.bank, w.registry.currencyOf(p.region)));
    }
    if (p.kind === SMALL_FIRM) smallFirms += p.weight;
  }
  let wagePerHour = 0;
  for (const e of w.journal.ofKind('labour.goingRate')) {
    for (const paid of Object.values((e.data['wagePerHour'] ?? {}) as Record<string, number>)) {
      if (paid > wagePerHour) wagePerHour = paid;
    }
  }
  return {
    banks,
    firms,
    grain,
    msPerPeriodAt,
    heapMbAt,
    parties: w.parties.all().filter((p) => p.status.alive).length,
    people,
    smallFirms,
    cells,
    events,
    sessionsCleared,
    auditTotal,
    moneyPerMember: people > 0 ? money / people : 0,
    wagePerHour,
    each,
  };
}

/**
 * 0g.17, Law 18 (21.118): A MEDIAN AND A SPREAD, because one run is not a measurement.
 *
 * Five runs of the (12, 48) rung at 26 periods on IDENTICAL code gave 274, 274, 286, 287 and
 * 303 ms/period — **±5%**. Every 0g step from 0g.4 to 0g.11 reported a single run against a single
 * run, and six of those deltas were 1–4%: inside the noise, and reported as if they were not. 0g's
 * own rule is that a step which moves a ratio is reverted, and that rule has never had an instrument
 * that could tell a moved ratio from moved weather.
 *
 * TWO distributions, because they answer different questions. WITHIN a run, the steady periods'
 * own times say what a period costs once the world has filled up — the first periods are a world
 * still being built and belong to no steady state. ACROSS runs, a fresh world per repeat catches
 * what differs between processes: the collector's mood, where the JIT got to, what the machine was
 * doing. A step is judged on the across-run median, and the spread is printed beside it so a
 * reader can see whether the claim is bigger than the noise.
 */
export interface Spread {
  readonly median: number;
  readonly lo: number;
  readonly hi: number;
  readonly n: number;
}

function spreadOf(xs: readonly number[]): Spread {
  const sorted = [...xs].sort((a, b) => a - b);
  const mid = sorted[Math.floor(sorted.length / 2)] ?? 0;
  return { median: mid, lo: sorted[0] ?? 0, hi: sorted[sorted.length - 1] ?? 0, n: sorted.length };
}

/** The steady periods of one run: everything after the world has stopped filling up. */
export function steadyOf(r: Rung, warm: number): Spread {
  return spreadOf(r.each.slice(warm));
}

export interface Rungs {
  readonly runs: readonly Rung[];
  /** The across-run distribution of each run's own steady median — what a step is judged on. */
  readonly steady: Spread;
  /** Whether every run agreed on the shape of the world; a step that moves one of these is reverted. */
  readonly sameShape: boolean;
}

export function runRungs(
  banks: number,
  firms: number,
  grain: 1 | 2,
  periods: number,
  repeats: number,
  warm: number,
): Rungs {
  const runs: Rung[] = [];
  for (let i = 0; i < repeats; i += 1) runs.push(runRung(banks, firms, grain, periods));
  const shape = (r: Rung): string =>
    `${r.parties}|${r.cells}|${r.people}|${r.smallFirms}|${r.events}|${r.sessionsCleared}|${r.auditTotal}|${r.moneyPerMember.toFixed(6)}|${r.wagePerHour.toFixed(6)}`;
  const first = runs[0];
  return {
    runs,
    steady: spreadOf(runs.map((r) => steadyOf(r, warm).median)),
    sameShape: first === undefined || runs.every((r) => shape(r) === shape(first)),
  };
}

/** Where a period's time went: the phases, the order generation, the audit. A measurement (Law 11). */
export interface Where {
  readonly perPeriod: number;
  readonly orders: number;
  readonly audit: number;
  readonly phases: readonly (readonly [string, number])[];
  readonly byParticipant: readonly (readonly [string, number])[];
}

/**
 * Law 18: WHAT A PERIOD IS SPENT ON, in its steady state. It instruments the spec before the world
 * is assembled, because a participant's `orders` is called by the kernel's market and venue
 * sessions — so it is module code that appears nowhere in the phase list, and it turned out to be
 * half of a period.
 *
 * `warm` periods run before the clock starts: a world still filling up is not in its steady state
 * and its first period is not the one that matters.
 */
export function whereItGoes(banks: number, firms: number, warm = 4, timed = 4): Where {
  const spec = rigSpec(`ladder-${banks}-${firms}`, banks, firms);
  const phases = new Map<string, number>();
  const byParticipant = new Map<string, number>();
  let orders = 0;
  for (const m of spec.modules) {
    for (const ph of m.phases) {
      const inner = ph.run.bind(ph);
      const name = `${m.id}/${ph.name}`;
      (ph as { run: (ctx: MechanismContext) => void }).run = (ctx: MechanismContext): void => {
        const at = Date.now();
        inner(ctx);
        phases.set(name, (phases.get(name) ?? 0) + (Date.now() - at));
      };
    }
    const wrap = (label: string, holder: { orders: (...a: never[]) => unknown }): void => {
      const inner = holder.orders.bind(holder);
      holder.orders = (...a: never[]): unknown => {
        const at = Date.now();
        const out = inner(...a);
        const took = Date.now() - at;
        byParticipant.set(label, (byParticipant.get(label) ?? 0) + took);
        orders += took;
        return out;
      };
    };
    for (const p of m.participants) wrap(`${m.id}/${String(p.partyKind)} [market]`, p);
    for (const p of m.venueParticipants ?? []) wrap(`${m.id}/${String(p.partyKind)} [venue]`, p);
  }
  let audit = 0;
  // The unbinding is the point: `this` is re-supplied by `apply` below, so the audit runs on the
  // real instance and only the clock is added. Binding it to the prototype loses the instance.
  // eslint-disable-next-line @typescript-eslint/unbound-method -- `this` is restored by `apply`
  const ran = Audit.prototype.run;
  Audit.prototype.run = function (this: Audit, ...a: Parameters<typeof ran>) {
    const at = Date.now();
    const out = ran.apply(this, a);
    audit += Date.now() - at;
    return out;
  };
  const w = assemble(spec);
  for (let i = 0; i < warm; i += 1) w.step();
  phases.clear();
  byParticipant.clear();
  orders = 0;
  audit = 0;
  const at = Date.now();
  for (let i = 0; i < timed; i += 1) w.step();
  const perPeriod = (Date.now() - at) / timed;
  Audit.prototype.run = ran;
  const per = (m: Map<string, number>): (readonly [string, number])[] =>
    [...m].map(([k, v]) => [k, v / timed] as const).sort((x, y) => y[1] - x[1]);
  return {
    perPeriod,
    orders: orders / timed,
    audit: audit / timed,
    phases: per(phases),
    byParticipant: per(byParticipant),
  };
}

export function line(r: Rung): string {
  const marks = Object.keys(r.msPerPeriodAt).map(Number);
  return (
    `${r.banks}b/${r.firms}f ×${r.grain}: ms/period ${marks.map((m) => `${m}:${(r.msPerPeriodAt[m] ?? 0).toFixed(0)}`).join(' ')}` +
    ` | heap MB ${marks.map((m) => `${m}:${(r.heapMbAt[m] ?? 0).toFixed(0)}`).join(' ')}` +
    ` | parties ${r.parties} cells ${r.cells} people ${r.people} small ${r.smallFirms}` +
    ` | events ${r.events} sessions ${r.sessionsCleared} audit ${r.auditTotal}` +
    ` | money/member ${r.moneyPerMember.toFixed(0)} wage/h ${r.wagePerHour.toFixed(2)}`
  );
}

const args = process.argv.slice(2);
const where = args.includes('--where');
/** 0g.17: `--repeat N` builds a fresh world N times and reports the median and the spread. */
const repeatArg = args.find((a) => a.startsWith('--repeat'))?.split('=')[1];
const repeats = repeatArg === undefined ? (args.includes('--repeat') ? 3 : 1) : Number(repeatArg);
/** How many opening periods are a world still filling up rather than a steady state. */
const warmArg = args.find((a) => a.startsWith('--warm='))?.split('=')[1];
const [banksArg, firmsArg, grainArg, periodsArg] = args.filter((a) => !a.startsWith('--'));
if (banksArg !== undefined && firmsArg !== undefined) {
  if (repeats > 1) {
    const grain: 1 | 2 = grainArg === '2' ? 2 : 1;
    const periods = periodsArg === undefined ? YEAR : Number(periodsArg);
    const warm = warmArg === undefined ? Math.min(4, periods - 1) : Number(warmArg);
    const rs = runRungs(Number(banksArg), Number(firmsArg), grain, periods, repeats, warm);
    const s = rs.steady;
    // The band as a share of the median, so a reader can see at once whether a claim can be heard.
    const band = s.median > 0 ? (100 * (s.hi - s.lo)) / s.median : 0;
    console.log(
      `LADDER ${banksArg}b/${firmsArg}f ×${grain}: steady ms/period MEDIAN ${s.median.toFixed(0)}` +
        ` (${s.lo.toFixed(0)}–${s.hi.toFixed(0)}, ±${(band / 2).toFixed(1)}%, ${s.n} runs of ${periods} periods, ${warm} warm)`,
    );
    for (const r of rs.runs) {
      const st = steadyOf(r, warm);
      console.log(`  run: steady ${st.median.toFixed(0)} (${st.lo}–${st.hi}) | ${line(r)}`);
    }
    // Law 18: the shape is the gate. A step that moves one of these is reverted, whatever the ms did.
    console.log(rs.sameShape ? '  SHAPE: identical across runs' : '  SHAPE: **DIFFERS ACROSS RUNS** — this rung is not deterministic');
    console.log(
      `  READ IT AS: a change smaller than ±${(band / 2).toFixed(1)}% on this rung is NOT evidence; judge the step on its counts.`,
    );
  } else if (where) {
    const w = whereItGoes(Number(banksArg), Number(firmsArg));
    const pct = (ms: number): string => `${((100 * ms) / w.perPeriod).toFixed(0)}%`;
    console.log(`WHERE ${banksArg}b/${firmsArg}f: ${w.perPeriod.toFixed(0)}ms/period`);
    console.log(`  order generation ${w.orders.toFixed(0)}ms (${pct(w.orders)})  audit ${w.audit.toFixed(0)}ms (${pct(w.audit)})`);
    for (const [k, ms] of w.byParticipant.slice(0, 6)) console.log(`  orders ${ms.toFixed(0).padStart(6)}ms ${pct(ms).padStart(4)}  ${k}`);
    for (const [k, ms] of w.phases.slice(0, 6)) console.log(`  phase  ${ms.toFixed(0).padStart(6)}ms ${pct(ms).padStart(4)}  ${k}`);
  } else {
    const grain: 1 | 2 = grainArg === '2' ? 2 : 1;
    const periods = periodsArg === undefined ? YEAR : Number(periodsArg);
    console.log(`LADDER ${line(runRung(Number(banksArg), Number(firmsArg), grain, periods))}`);
    console.log('  (one run: no median, no spread. Use --repeat=3 before claiming a delta — 0g.17.)');
  }
}
