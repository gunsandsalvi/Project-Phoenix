/**
 * 0g.1: THE LADDER, one rung per process. Law 18: performance work changes layout and traversal
 * and never a mechanism, so every step of 0g reports this before and after, and a step that moves
 * a ratio is reverted. It is a measurement, so it runs when a step is done and not in the suite
 * (Law 11): `npm run ladder -- <banks> <firms> <grain>`; the invariants a rung must hold are in
 * `ladder.test.ts`, on the first rung, where the suite can afford them.
 *
 * What it prints is time and the shape of a year: ms per period at four marks, parties, cells,
 * people, events, sessions cleared, audit findings, money per member, the highest going rate.
 */
import { HOUSEHOLD, SMALL_FIRM, assemble, moneyInstrumentId, type World } from '../src/index.js';
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
  for (let i = 1; i <= periods; i += 1) {
    const report = w.step();
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

const [banksArg, firmsArg, grainArg, periodsArg] = process.argv.slice(2);
if (banksArg !== undefined && firmsArg !== undefined) {
  const grain: 1 | 2 = grainArg === '2' ? 2 : 1;
  const periods = periodsArg === undefined ? YEAR : Number(periodsArg);
  console.log(`LADDER ${line(runRung(Number(banksArg), Number(firmsArg), grain, periods))}`);
}
