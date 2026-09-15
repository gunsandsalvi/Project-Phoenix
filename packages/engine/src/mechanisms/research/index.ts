/**
 * Research: banks publish their own estimates of the companies they cover, and disagree.
 *
 * @spec Reporting C1 Reporting C2 Reporting C3 Reporting C4 Reporting C5 Reporting C6 Reporting D1 Reporting D2 Reporting D3 Reporting D3.a Reporting E1 Reporting E2 Reporting E3 Reporting F1 Reporting F2 Reporting F2.a Reporting F3 Reporting G1 Reporting G3 Expectations A2 Expectations A3 Expectations B2 Law 3 Law 4 Law 5 Law 19
 *
 * C3 IS THE POINT: estimates disagree, and the disagreement is load-bearing (§46 A3). Banks differ
 * because their memories differ — drawn at entry — and because what each has seen of a name differs.
 * Nothing here disperses them, and nothing hands any of them the answer (C5).
 *
 * D3.a IS THE FORBID THAT BREAKS QUIETLY: no universal coverage. A bank covers a name because its
 * OWN book needs the view — it makes a market in the share, it lends to the issuer, or it holds it
 * (D1) — and coverage costs it real money paid to a named payee (D2). So the count of estimates per
 * name is an OUTCOME of what banks' books look like, and a world where every bank covered every name
 * would have made that count a constant and deleted D3.
 *
 * F2.a IS WHAT THIS MODULE MUST NOT DO. The surprise a report settles is an OBSERVATION and nothing
 * else. Nowhere here is a price written, nudged or scaled: holders revise their own outlooks, their
 * reservations change, the book clears, and the move is whatever the changed schedules cleared at.
 */
import { forbid } from '../../core/assert.js';
import { partyId, type PartyId } from '../../core/ids.js';
import { sum } from '../../core/num.js';
import {
  absolute,
  asAmount,
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  minus,
  over,
  type PerPiece,
  scale,
  valueAt,
} from '../../core/measure.js';
import { none, some, type Option } from '../../core/option.js';
import { BANK, HOUSEHOLD } from '../../registry/profiles.js';
import { weightOf } from '../../parties/party.js';
import { cellSide, totalFor } from '../../ledger/settlement.js';
import type { MechanismContext } from '../../world/context.js';
import type { Event } from '../../journal/journal.js';
import type { SystemModule } from '../../world/module.js';
import type { Family, Violation } from '../../audit/audit.js';
import { estimateFrom, seenOf } from './estimate.js';
import { RESEARCH_PARAMS, researchParams, memoryOf } from './data.js';

export * from './data.js';
export * from './estimate.js';

/**
 * C4: WHAT THIS DESK HAS ALREADY COUNTED, and that is all it keeps (item 9.9b).
 *
 * What it SAID and when it INITIATED were here too, and both were mirrors: a research estimate is
 * published — that is what research IS — so `research.estimate`, `research.initiated` and
 * `research.dropped` already carry every one of them, and `consensus` in this same file was already
 * reading them that way. Two records of one fact, one of them private and invisible to the analysts
 * it exists for (Law 19).
 *
 * The last period whose observations are in the estimate is not published and must not be: it is
 * bookkeeping about a read, not a fact about a company, and publishing it would be telling the
 * market which reports this desk has got round to.
 */
interface Desk {
  /** C4: the last period whose observations are already in the estimate, so none is counted twice. */
  readonly seenTo: Map<string, number>;
}

/**
 * C1, C4, Law 19: WHAT THIS DESK LAST SAID ABOUT THIS NAME, and WHEN IT INITIATED — off what it
 * published. A drop ends the coverage, so an initiation before the last drop is not this one.
 */
interface Coverage {
  readonly said: Option<Cash>;
  readonly since: Option<number>;
}

function coverageOf(ctx: MechanismContext, bank: PartyId, company: string): Coverage {
  const mine = (kind: 'research.estimate' | 'research.initiated' | 'research.dropped'): Event[] =>
    ctx.journal.forSubject(kind, company).filter((e) => e.data['bank'] === String(bank));
  const dropped = mine('research.dropped');
  const lastDrop = dropped[dropped.length - 1]?.period;
  const after = (e: Event): boolean => lastDrop === undefined || e.period > lastDrop;
  const initiated = mine('research.initiated').filter(after);
  const estimates = mine('research.estimate').filter(after);
  const last = estimates[estimates.length - 1];
  const value = last?.data['perPeriod'];
  return {
    // Item 16: a published number re-enters the type system through its dimension's own door.
    said: typeof value === 'number' ? some(asCash(value, 'what it said the name makes')) : none<Cash>(),
    since: initiated[0] === undefined ? none<number>() : some(initiated[0].period),
  };
}

type Desks = Record<string, Desk>;

function desks(ctx: MechanismContext): Desks {
  return ctx.state<Desks>('research', () => ({}));
}

function deskOf(all: Desks, bank: PartyId): Desk {
  const held = all[String(bank)];
  if (held !== undefined) return held;
  const made: Desk = { seenTo: new Map() };
  all[String(bank)] = made;
  return made;
}

/**
 * D1: WHY THIS BANK WOULD COVER THIS NAME — because its own book already needs the view. Three
 * reads of its own position, none of them a preference and none of them assigned: it holds the
 * share, it has lent to the issuer, or it makes a market in the line.
 *
 * Everything here is the BANK's own state, so what a desk covers is what that bank's book looks
 * like — which is exactly why the count per name comes out uneven (D3) without anybody spreading it.
 */
function needsTheView(ctx: MechanismContext, bank: PartyId, company: PartyId): boolean {
  // D2, Money E4: A COMPANY THAT HAS CEASED IS NOT ONE ANYBODY COVERS. It has no next quarter to
  // have a view of, and a desk publishing an estimate of one is naming a party that is not there —
  // which the `names` family says out loud, and said in every year-long run in the suite. Coverage
  // ENDS rather than lapsing: the desk takes the same `research.dropped` path it takes for a name
  // it can no longer justify, so the last thing said about a dead company is that nobody is
  // covering it (D3).
  if (!ctx.parties.get(company).status.alive) return false;
  const view = ctx.participant(bank);
  // It HOLDS something the company issued — a share it took, or a loan row it wrote to it (§23).
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!i.issuer.some || i.issuer.value !== company) continue;
    if (view.quantity(i.id) > 0) return true;
  }
  // Or it MAKES A MARKET in the line (§26), which is read off what its own desk published about
  // itself (Law 19) rather than asked of the dealing module, which this one may not import.
  const said = view.lastOwn('bank.dealing');
  if (!said.some) return false;
  const lines = said.value.data['lines'];
  if (typeof lines !== 'object' || lines === null) return false;
  for (const id of Object.keys(lines)) {
    const issuer = view.instruments.get(id as never).issuer;
    if (issuer.some && issuer.value === company) return true;
  }
  return false;
}

/**
 * D2: COVERAGE COSTS, and the cost has a named payee. The analysts are people and this is what they
 * are paid — an instruction from the bank to the household cells whose members do the work, in the
 * bank's own money, every period it covers anything.
 *
 * The hours are TECHNOLOGY (`research.hoursPerName`): covering a name takes a person a stated amount
 * of time, which is a fact about the work and not a preference of anybody. What the time COSTS is
 * whatever the labour venue cleared at, read off this world's own wage prints — never a research
 * budget somebody wrote down, which would be the cost stated instead of paid.
 */
function pay(ctx: MechanismContext, bank: PartyId, names: number): void {
  if (names <= 0) return;
  const wage = wagePrinted(ctx, bank);
  if (!wage.some) return;
  const hours = scale(
    asAmount<'piece'>(ctx.params.count(RESEARCH_PARAMS.hoursPerName), 'the hours one name takes'),
    asRatio(names, 'the names it covers'),
    'the hours this desk takes',
  );
  const owed = valueAt(wage.value, hours, 'what the desk costs it');
  if (owed <= 0) return;
  // D2: THE ANALYSTS ARE PEOPLE AND THIS IS WHAT THEY ARE PAID. They are the members of the cells
  // that bank here, which is who is at hand to do the work, and the payment is split across them
  // per member exactly as a wage is (XI-15). A bank with no household at it pays nobody and has no
  // desk, which is a real answer and not a missing one.
  const cells = ctx.parties.ofKind(HOUSEHOLD).filter((c) => c.status.alive && c.bank === bank);
  const members = cells.reduce((t, c) => t + weightOf(c), 0);
  if (members <= 0) return;
  const ccy = ctx.registry.currencyOf(ctx.parties.get(bank).region);
  for (const cell of cells) {
    const share = ctx.registry.payable(over(owed, asRatio(members, 'the analysts there are'), "one analyst's share"),
    );
    if (share <= 0) continue;
    const side = cellSide(cell, share);
    if (side === undefined) continue;
    ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(bank, ccy),
          to: ctx.accountOf(cell.id, ccy),
          ccy,
          amount: totalFor(cell, share),
          fromCell: none(),
          toCell: some(side),
        },
      ],
      cause: 'transfer',
      reason: `${bank} pays its research desk for ${names} names`,
    });
  }
}

/** Law 19: what an hour of somebody's time last went for, read off the wage this world printed. */
function wagePrinted(ctx: MechanismContext, bank: PartyId): Option<PerPiece> {
  const region = ctx.parties.get(bank).region;
  let best: PerPiece | undefined;
  for (const e of ctx.journal.ofKind('labour.print')) {
    if (e.data['region'] !== String(region)) continue;
    const wage = e.data['wagePerHour'];
    // Item 16: a wage re-enters from what the labour venue published — a level per hour, in the
    // same money an hour is paid in.
    if (typeof wage === 'number' && wage > 0) best = asPerPiece(wage, 'what an hour cleared at');
  }
  return best === undefined ? none<PerPiece>() : some(best);
}

/** C1–C4: form, publish and revise. Every estimate is this bank's own and none of them is a price. */
function cover(seed: string, ctx: MechanismContext): void {
  const all = desks(ctx);
  const companies = reported(ctx);
  for (const bank of ctx.parties.ofKind(BANK)) {
    if (!bank.status.alive) continue;
    const desk = deskOf(all, bank.id);
    const memory = memoryOf(seed, bank.id);
    let covered = 0;
    for (const company of companies) {
      const wanted = needsTheView(ctx, bank.id, company);
      const cover = coverageOf(ctx, bank.id, String(company));
      const standing = cover.said.some ? cover.said.value : undefined;
      if (!wanted) {
        // D2: and it DROPS one it cannot justify. What it said stands until it says otherwise.
        if (!cover.since.some) continue;
        desk.seenTo.delete(String(company));
        ctx.record('research.dropped', [bank.id, company], { bank: bank.id, company }, true);
        continue;
      }
      covered += 1;
      let since = cover.since.some ? cover.since.value : undefined;
      if (since === undefined) {
        since = ctx.period;
        ctx.record('research.initiated', [bank.id, company], { bank: bank.id, company }, true);
      }
      // C1, C4: what this desk has not yet taken account of. A bank that initiated today reads
      // everything published about the name since it did; one that has been covering it reads what
      // has been published since it last spoke, and nothing it has already counted.
      const last = desk.seenTo.get(String(company));
      const from = last === undefined ? since : last + 1;
      const seen = seenOf(ctx, company, from as never, ctx.period);
      const now = estimateFrom(
        seen,
        memory,
        standing === undefined ? none<Cash>() : some(standing),
      );
      desk.seenTo.set(String(company), ctx.period);
      if (!now.some) continue;
      // C4: A REVISION IS ON INFORMATION. It is published when the bank's own view has moved, and
      // a view that has not moved says nothing — §46 B2.a: a revision no observation preceded is
      // the defect this is shaped to avoid.
      if (standing !== undefined && !moved(now.value, standing)) continue;
      ctx.record(
        'research.estimate',
        [bank.id, company],
        {
          bank: bank.id,
          company,
          perPeriod: now.value,
          reports: seen.reports.length,
          memory,
          revision: standing !== undefined,
          movedBy:
            standing === undefined
              ? asCash(0, 'a first view has not moved')
              : minus(now.value, standing, 'the revision'),
        },
        true,
      );
    }
    pay(ctx, bank.id, covered);
  }
}

/** Law 7: a view has moved when it has moved past the dust of the arithmetic that produced it. */
function moved(now: Cash, said: Cash): boolean {
  return (
    absolute(minus(now, said, 'the revision'), 'either way') >
    Number.EPSILON * (Math.abs(now) + Math.abs(said))
  );
}

/** G4: the companies there is anything to estimate — the ones that have published a report. */
function reported(ctx: MechanismContext): PartyId[] {
  const out = new Set<PartyId>();
  for (const said of ctx.published.statements()) out.add(said.company);
  return [...out];
}

/**
 * F1: THE REPORT SETTLES EVERY EXPECTATION STANDING AGAINST IT — the management's guidance and each
 * bank's estimate. Observed minus expected, per holder of a view, recorded with the name of the
 * party whose view it was (§46 B2's surprise with a name on it).
 *
 * F2: and that is ALL it does. The surprise is an observation; what it causes is holders revising
 * their own outlooks and therefore their reservations in the share book. Nothing here writes a
 * price, and there is no number anywhere in this module whose unit is a price move (F2.a).
 */
function settle(ctx: MechanismContext): void {
  const all = desks(ctx);
  for (const report of ctx.published.statements()) {
    if (report.at !== ctx.period) continue;
    const company = String(report.company);
    const observed = over(report.earned, asRatio(report.periods, 'the periods it covers'), 'what it made a period');
    // The desks that have a slot are the ones that have ever covered anything; whether THIS one
    // covers THIS name is what its published coverage says (Law 19).
    for (const bank of Object.keys(all)) {
      const said = coverageOf(ctx, partyId(bank), company).said;
      if (!said.some) continue;
      ctx.record(
        'research.surprise',
        [partyId(bank), partyId(company)],
        {
          bank,
          company,
          quarter: report.quarter,
          expected: said.value,
          observed,
          surprise: minus(observed, said.value, 'observed minus expected'),
        },
        true,
      );
    }
  }
}

/** E1: what the estimates that exist come to, with how stale the oldest of them is. */
export interface ConsensusRead {
  readonly count: number;
  readonly mean: Cash;
  /** C3, E1: how far apart they are, which is the read that says the disagreement is real. */
  readonly spread: Cash;
  /** §45 A5: the period the oldest estimate in it was published in, so a reader can see its age. */
  readonly oldest: number;
}

/**
 * E1, E3: THE CONSENSUS IS A READ, computed from the estimates that exist at the moment of reading
 * and stored nowhere — exactly as an index is a read of its constituents (§22 A2).
 *
 * E2 is what keeps it honest and it is not enforceable from here: nothing that DECIDES may consult
 * it. It reaches the observer surface, and it reaches a party only the way any published statistic
 * does — as one more thing observed (§46 A2.a). No module in this engine imports this function, and
 * `test/research.test.ts` is what says so.
 */
export function consensusOf(
  ctx: { readonly journal: Pick<MechanismContext['journal'], 'ofKind'> },
  company: PartyId,
): Option<ConsensusRead> {
  const latest = new Map<string, { value: Cash; period: number }>();
  for (const e of ctx.journal.ofKind('research.estimate')) {
    if (e.subjects[1] !== String(company)) continue;
    const bank = e.data['bank'];
    const value = e.data['perPeriod'];
    if (typeof bank !== 'string' || typeof value !== 'number') continue;
    // Item 16: a published number re-enters the type system here, through its dimension's own door.
    latest.set(bank, { value: asCash(value, 'what a desk said it makes in a period'), period: e.period });
  }
  for (const e of ctx.journal.ofKind('research.dropped')) {
    if (e.subjects[1] !== String(company)) continue;
    const bank = e.data['bank'];
    if (typeof bank === 'string') latest.delete(bank);
  }
  const values = [...latest.values()];
  const first = values[0];
  if (first === undefined) return none<ConsensusRead>();
  const mean = over(
    sum(values.map((v) => v.value)).value,
    asRatio(values.length, 'the desks that have a view'),
    'the consensus',
  );
  // C3, E1: HOW FAR APART THEY ARE — the widest minus the narrowest, which is a measurement of the
  // disagreement and not a limit on it. Walked rather than reduced through a helper, because the
  // helpers that take a maximum are the ones a bound hides in (Law 6).
  let widest = first.value;
  let narrowest = first.value;
  let oldest = first.period;
  for (const v of values) {
    if (v.value > widest) widest = v.value;
    if (v.value < narrowest) narrowest = v.value;
    if (v.period < oldest) oldest = v.period;
  }
  return {
    some: true,
    value: {
      count: values.length,
      mean,
      spread: minus(widest, narrowest, 'how far apart they are'),
      oldest,
    },
  };
}

/**
 * Audit B3, Law 5: every estimate names a bank and a company that ARE HERE, and every desk that was
 * paid was paid by somebody to somebody.
 *
 * An estimate about a party that has ceased is an opinion about nobody, and a research cost with no
 * payee is a one-sided flow even when nothing failed. Both break quietly: the numbers look the same.
 */
function researchNames(): Family {
  return {
    name: 'names',
    contributor: 'research',
    spec: 'Reporting C2 Reporting G4 Audit B3 Register F2',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const e of view.journal.ofKind('research.estimate')) {
        if (e.period !== view.period) continue;
        for (const who of e.subjects) {
          const there = view.parties.has(partyId(who));
          if (!there) {
            out.push({
              family: 'names',
              spec: 'Reporting C2',
              owner: who,
              size: 1,
              unit: 'estimates',
              period: view.period,
              message: `an estimate names ${who}, which does not exist`,
            });
            continue;
          }
          // C2, Money E4: alive WHEN IT WAS SAID, which is not the same question as alive now. This
          // audit runs at the close of the period and a party can cease inside one — a bank fails in
          // the resolution slot after its desk has published, an issuer is wound up in the period it
          // was last reported on. An estimate that named a living party and was overtaken by its
          // death is not an estimate about a party that is not there; it is the record of what
          // somebody thought before it happened, and C2 asks that a view have a real holder and a
          // real subject, not that both outlive the period.
          const status = view.parties.get(partyId(who)).status;
          if (status.alive || status.ceasedIn >= e.period) continue;
          out.push({
            family: 'names',
            spec: 'Reporting C2',
            owner: who,
            size: 1,
            unit: 'estimates',
            period: view.period,
            message: `an estimate names ${who}, which ceased in period ${status.ceasedIn}`,
          });
        }
      }
      return out;
    },
  };
}

/** Law 5, Reporting D2: what a research desk cost was paid, in full, to somebody with a name. */
function researchFlows(): Family {
  return {
    name: 'flows',
    contributor: 'research',
    spec: 'Reporting D2 Law 5',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const r of view.ledger.inPeriod(view.period)) {
        if (!r.instruction.reason.includes('research desk')) continue;
        if (r.outcome === 'settled') continue;
        out.push({
          family: 'flows',
          spec: 'Reporting D2',
          owner: r.instruction.legs[0]?.kind ?? 'a desk',
          size: 1,
          unit: 'instructions',
          period: view.period,
          // D2 asks for a cost that is PAID. A bank that could not pay its analysts has a funding
          // problem and that is a real state — but it is one somebody must be able to see.
          message: `a research desk cost did not settle: ${r.instruction.reason}`,
        });
      }
      return out;
    },
  };
}

/** C1, D1: the module. The banks that exist do this; there is no analyst party kind. */
export function research(seed: string): SystemModule {
  forbid(seed.length > 0, 'Seed A5', 'a research desk is drawn from the world seed');
  return {
    id: 'research',
    nouns: [
      {
        name: 'research',
        kind: 'working',
        holds:
          'the last period whose observations are already in each desk’s estimate',
        why:
          'C4, item 9.9b: the last period whose observations are already in each estimate, so none is counted twice. It is bookkeeping about a READ that one phase hands the next, not a fact about a company, and publishing it would be telling the market which reports this desk has got round to. What it SAID and when it INITIATED were here too and both were mirrors: an estimate is published — that is what research IS — and `consensus`, in this same file, was already reading them off the journal.',
      },
    ],
    spec: 'Reporting C Reporting D Reporting E Reporting F',
    requires: ['reporting', 'banks', 'expectations'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: researchParams(),
    phases: [
      {
        // F1: settling comes FIRST, so a surprise is measured against what the bank said BEFORE the
        // report it is being surprised by — a view revised on the report and then scored against it
        // would be surprised by nothing, every time.
        name: 'research.settle',
        spec: 'Reporting F1',
        // Item 0 (stop 18): it takes the cycle of the phase it is anchored to, because what it
        // reads is what that phase wrote. `reporting.publish` moved to the close of the period
        // (a balance sheet is struck at one) and a cycle stated here would have pinned this to
        // the top of it — which is the anchor design item 0a deletes.
        anchor: { after: 'reporting.publish' },
        reads: [
          { kind: 'event', name: 'research.dropped', of: 'anyPeriod' },
          { kind: 'event', name: 'research.estimate', of: 'anyPeriod' },
          { kind: 'event', name: 'research.initiated', of: 'anyPeriod' },
        ],
        writes: [],
        run: (ctx: MechanismContext): void => {
          settle(ctx);
        },
      },
      {
        name: 'research.cover',
        spec: 'Reporting C1 Reporting C4 Reporting D1 Reporting D2',
        // It follows the surprise it is anchored to, and takes its cycle (item 0, stop 18).
        anchor: { after: 'research.settle' },
        reads: [
          { kind: 'event', name: 'labour.print', of: 'anyPeriod' },
          { kind: 'event', name: 'research.dropped', of: 'anyPeriod' },
          { kind: 'event', name: 'research.estimate', of: 'anyPeriod' },
          { kind: 'event', name: 'research.initiated', of: 'anyPeriod' },
        ],
        writes: [
          { kind: 'event', name: 'research.estimate' },
          { kind: 'event', name: 'research.initiated' },
        ],
        run: (ctx: MechanismContext): void => {
          cover(seed, ctx);
        },
      },
    ],
    participants: [],
    families: [researchNames(), researchFlows()],
  };
}
