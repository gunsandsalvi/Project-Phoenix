/**
 * Credit default swaps: a market in whether a named party will pay.
 *
 * @spec CDS A1 CDS A1.a CDS A1.b CDS A1.c CDS A1.d CDS A2 CDS A3 CDS A4 CDS A4.a CDS B1 CDS B1.a CDS B2 CDS B3 CDS B4 CDS B5 CDS C1 CDS C2 CDS C3 CDS C3.a CDS C3.b CDS C4 CDS D1 CDS D2 CDS D2.a CDS D2.b CDS D3 CDS D4 CDS D5 CDS E1 CDS E2 CDS E3 CDS E4 Derivative D10 Derivative D10.a Derivative Layer B1 Derivative Layer C2 Corporate Credit H4 Corporate Credit H4.a XI-13 Law 3 Law 15 Law 19
 *
 * WHAT IT OWNS is the class and nothing underneath it: the kind, the books its curve prints on, the
 * reasons two sides have to be in one, and what happens when the reference actually fails. The
 * house, the margin, the capacity and the waterfall are 13a's and are not touched here — which is
 * the forecast 13a's record made, and this module is its first test.
 *
 * THE BOOKS ARE PER REFERENCE PER TENOR (A1.d). The term structure of credit is not a curve
 * somebody fits: it is the set of levels these books cleared, and a tenor nobody traded has no
 * point on it (Law 3, Law 19).
 */
import type { Family, Violation } from '../../audit/audit.js';
import { addYears } from '../../calendar/civil.js';
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { mul, sub, sum } from '../../core/num.js';
import { none } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

import { CDS_PARAMS, PROTECTED, cdsLineOf, cdsMarketOf } from './data.js';
import { CDS, cdsKind, isCds, type CdsTerms } from './contract.js';
import {
  CDS_INDEX,
  cdsIndexKind,
  isCdsIndex,
  seriesLineOf,
  seriesMarketOf,
  type CdsIndexTerms,
  type SeriesName,
} from './series.js';
import { middleGrade, rankOf } from '../../registry/grades.js';

export * from './data.js';
export * from './contract.js';
export * from './participants.js';
export * from './series.js';
export * from './measures.js';

function params(): ParamDecl[] {
  return [
    {
      id: CDS_PARAMS.tenors,
      value: 5,
      unit: 'years',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'CDS A1.d: the longest tenor this world writes protection to. The term structure is the set of books from one year out to this one, and how many points it has is a fact about the market rather than about anybody in it.',
    },
    {
      id: CDS_PARAMS.window,
      value: 8,
      unit: 'periods',
      kind: 'resolution',
      owner: 'model',
      why: "Derivative Layer D1: how much of a book's own record the initial margin is measured over. It is a resolution — the answer must not turn on it — and the invariance test is that a longer window changes the margin and not who can trade.",
    },
    {
      id: CDS_PARAMS.roll,
      value: 26,
      unit: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'CDS A5: how often a new series is published. The names are fixed when it is — that is what a series IS — so this is how long a line runs before the next one starts, which is a convention of the market and not anybody’s choice within it.',
    },
    {
      id: CDS_PARAMS.riskWeightSold,
      value: 1,
      unit: 'of the notional sold',
      kind: 'policy',
      owner: 'parliament',
      why: 'CDS B3: what protection SOLD weighs in a bank’s capital. A naked seller carries the reference’s whole credit without funding a bond, so what it consumes is stated by the regulator like every other weight (Banks Capital B1).',
    },
  ];
}

/** A1.d: the tenors this world's curve has points at — one year out to the longest it writes. */
export function cdsTenorsOf(ctx: Pick<MechanismContext, 'params'>): readonly number[] {
  const longest = ctx.params.get(CDS_PARAMS.tenors);
  const out: number[] = [];
  for (let y = 1; y <= longest; y += 2) out.push(y);
  return out;
}

/**
 * A1, A1.d, Derivative Layer B1, C2: open a book per reference per tenor, cleared through the house.
 *
 * A reference gets books when it has debt somebody could watch it default on (A4.a) and keeps them
 * while it does. Nothing opens a book on a party nobody lends to, and nothing closes one because
 * the spread went somewhere somebody did not like.
 */
function openBooks(ctx: MechanismContext, house: (ccy: CurrencyCode) => PartyId): void {
  const open = new Set(ctx.markets.map((m) => String(m.id)));
  const window = ctx.params.get(CDS_PARAMS.window);
  const tenors = cdsTenorsOf(ctx);
  // Law 19, Law 18: ONE PASS OVER WHAT EXISTS. Asking every party what it owes means asking the
  // whole register once per party, every period, which is the same answer arrived at N times —
  // and the traversal is free to change (Law 18) as long as the books that open are the same.
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some) continue;
    const kind = ctx.registry.instrumentKind(i.kind);
    if (!kind.liabilityOfIssuer || kind.pricing !== 'cleared') continue;
    const reference = i.issuer.value;
    if (!ctx.parties.has(reference)) continue;
    const p = ctx.parties.get(reference);
    if (!p.status.alive) continue;
    if (tenors.every((t) => open.has(String(cdsMarketOf(reference, t))))) continue;
    // B1.a, and it is not a limit: a book exists where somebody has a REASON for one. Protection
    // on a name nobody is exposed to is a book with no buyer in it. What makes this a mechanism
    // rather than a cap is that the condition is somebody's holding: the moment one party holds
    // another's paper the book opens, and nothing closes it.
    if (ctx.register.holdersOf(i.id).every((h) => h === reference)) continue;
    const ccy = ctx.registry.region(p.region).ccy;
    // C2: this book clears, so it needs the house that clears it. A world with no house in this
    // money has no cleared book in it — which is a fact about that world and not a reason to write
    // a market naming a party that is not there (Money E4).
    const clearer = house(ccy);
    if (!ctx.parties.has(clearer) || !ctx.parties.get(clearer).status.alive) continue;
    for (const tenorYears of tenors) {
      const id = cdsMarketOf(reference, tenorYears);
      if (open.has(String(id))) continue;
      open.add(String(id));
      const terms: CdsTerms = {
        kind: CDS,
        reference,
        obligation: i.id,
        book: cdsLineOf(reference, tenorYears),
        // D6: the term is a DATE, and the period it falls in is what the calendar says. A count
        // of periods a year is a second calendar (Law 4).
        maturity: ctx.calendar.periodOf(addYears(ctx.calendar.startOf(ctx.period), tenorYears)),
        tenorYears,
        buysProtection: true,
        window,
      };
      ctx.openMarket({
        id,
        name: `${p.name} ${tenorYears}y protection`,
        instrument: cdsLineOf(reference, tenorYears),
        ccy,
        rationing: 'proRata',
        kind: 'contract',
        contract: { kind: CDS, terms, house: clearer },
      });
    }
  }
}

/**
 * A5, A5.a, Indices A1.a: THE ROLL — a new series, its names fixed the day it is published.
 *
 * Who is in it is decided by the published opinions of this world's assessors, combined by a rule
 * stated in advance and the same for everybody (`registry/grades.ts`: the middle opinion counts).
 * Investment grade is one line and high yield is the other, and a name that is downgraded after
 * the roll STAYS IN THE SERIES IT WAS IN — which is the whole point of fixing the names, and why
 * the high-yield line is the one where a constituent's event actually fires.
 *
 * Nothing here is a weight somebody chose: A5 fixes the names, Indices B1 makes a weight a count,
 * and the count is what each name has outstanding of the paper the series is on.
 */
function rollSeries(ctx: MechanismContext, house: (ccy: CurrencyCode) => PartyId): void {
  const every = ctx.params.get(CDS_PARAMS.roll);
  if (ctx.period === 0 || ctx.period % every !== 0) return;
  const byGrade = new Map<string, SeriesName[]>();
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some) continue;
    const kind = ctx.registry.instrumentKind(i.kind);
    if (!kind.liabilityOfIssuer || kind.pricing !== 'cleared') continue;
    const reference = i.issuer.value;
    if (!ctx.parties.has(reference) || !ctx.parties.get(reference).status.alive) continue;
    const grade = publishedGradeOf(ctx, reference);
    if (grade === undefined) continue;
    const line = rankOf(grade) <= rankOf('bbb') ? 'ig' : 'hy';
    const weight = ctx.instruments.get(i.id).issued;
    if (weight <= 0) continue;
    const names = byGrade.get(line) ?? [];
    names.push({ reference, obligation: i.id, weight });
    byGrade.set(line, names);
  }
  const ccy = ctx.registry.currencies.keys().next().value;
  if (ccy === undefined) return;
  const clearer = house(ccy);
  if (!ctx.parties.has(clearer) || !ctx.parties.get(clearer).status.alive) return;
  const window = ctx.params.get(CDS_PARAMS.window);
  for (const [grade, names] of byGrade.entries()) {
    if (names.length === 0) continue;
    const series = `${grade}.${ctx.period}`;
    for (const tenorYears of cdsTenorsOf(ctx)) {
      const terms: CdsIndexTerms = {
        kind: CDS_INDEX,
        series,
        grade,
        names,
        book: seriesLineOf(series, tenorYears),
        maturity: ctx.calendar.periodOf(addYears(ctx.calendar.startOf(ctx.period), tenorYears)),
        tenorYears,
        buysProtection: true,
        window,
      };
      ctx.openMarket({
        id: seriesMarketOf(series, tenorYears),
        name: `${grade.toUpperCase()} series ${series} ${tenorYears}y`,
        instrument: seriesLineOf(series, tenorYears),
        ccy,
        rationing: 'proRata',
        kind: 'contract',
        contract: { kind: CDS_INDEX, terms, house: clearer },
      });
    }
    ctx.record(
      'cds.index.rolled',
      [series],
      { series, grade, names: names.map((n) => String(n.reference)), weights: names.map((n) => n.weight) },
      true,
    );
  }
}

/** A5.a, Indices A1.a: the grade this world's assessors agree on, by the rule they published. */
export function publishedGradeOf(ctx: MechanismContext, party: PartyId): string | undefined {
  const latest = new Map<string, string>();
  for (const e of ctx.journal.ofKind('rating.action')) {
    if (!e.subjects.includes(party)) continue;
    const assessor = e.subjects[0];
    const grade = e.data['grade'];
    if (typeof assessor !== 'string' || typeof grade !== 'string') continue;
    latest.set(assessor, grade);
  }
  return middleGrade([...latest.values()]);
}

/**
 * A5: A NAME'S EVENT SETTLES ITS WEIGHT, ONCE, FOR EVERY CONTRACT ON THE LINE — and the line runs
 * on over the survivors.
 *
 * "Once" is a read and not a second record: the journal already says which weights this contract
 * has paid, so nothing here keeps a list of its own (Law 4, Law 19).
 */
function settleSeriesNames(ctx: MechanismContext): void {
  const paid = new Set<string>();
  for (const e of ctx.journal.ofKind('cds.index.settled')) {
    paid.add(`${String(e.data['contract'])}|${String(e.data['reference'])}`);
  }
  for (const c of ctx.contracts.open_()) {
    if (!isCdsIndex(c.terms)) continue;
    const whole = sum(c.terms.names.map((n) => n.weight)).value;
    if (whole <= 0) continue;
    for (const n of c.terms.names) {
      if (paid.has(`${String(c.id)}|${String(n.reference)}`)) continue;
      if (!recoveryIsKnown(ctx, n.reference)) continue;
      const recovery = ctx.valuation.markPerUnit(n.obligation, ctx.period);
      const share = mul(c.notional, n.weight / whole, 'this name’s share of the line');
      const owed = mul(sub(1, recovery, 'par less recovery'), share, 'what this name owes');
      const buyer = c.terms.buysProtection ? c.a : c.b;
      const seller = c.terms.buysProtection ? c.b : c.a;
      const amount = ctx.registry.cashFor(c.ccy, owed);
      if (amount > 0) {
        ctx.settle({
          legs: [
            {
              kind: 'money',
              from: ctx.accountOf(seller, c.ccy),
              to: ctx.accountOf(buyer, c.ccy),
              ccy: c.ccy,
              amount,
              fromCell: none(),
              toCell: none(),
            },
          ],
          cause: 'corporateAction',
          reason: `${n.reference} settles its weight in ${c.terms.series}`,
        });
      }
      ctx.record(
        'cds.index.settled',
        [String(c.id), String(n.reference), c.terms.series],
        { contract: String(c.id), reference: String(n.reference), series: c.terms.series, paid: amount },
        true,
      );
    }
  }
}

/**
 * D2, D2.a, D3, D4: WHAT PROTECTION PAYS, when there is finally a recovery to pay it against.
 *
 * The event fires the moment the reference fails (D1) and the mark moves to the payoff that day —
 * but nobody can settle it until the estate has sold what it had and paid what it could, because
 * until then par less recovery has no second half (D2.a: there is no fixed recovery in this world
 * to stand in for it). So the row is held (D2.b), and when the estate closes the seller pays.
 *
 * D3: the payment is cash, from a named seller to a named buyer, and IT CAN FAIL — a seller of
 * protection that cannot pay a claim is in Money E1's state like anybody else.
 */
function settleEvents(ctx: MechanismContext): void {
  for (const c of ctx.contracts.open_()) {
    // Law 15: what makes a row one of these is the SHAPE OF ITS TERMS, not the id on it.
    if (!isCds(c.terms)) continue;
    if (!recoveryIsKnown(ctx, c.terms.reference)) continue;
    const owed = ctx.contracts.mark(c, ctx.period);
    const buyer = c.terms.buysProtection ? c.a : c.b;
    const seller = c.terms.buysProtection ? c.b : c.a;
    const amount = ctx.registry.cashFor(c.ccy, owed < 0 ? -owed : owed);
    const legs: Leg[] = [
      {
        kind: 'contract',
        act: 'close',
        contract: c.id,
        why: 'the reference defaulted and its estate closed',
      },
    ];
    if (amount > 0) {
      legs.push({
        kind: 'money',
        from: ctx.accountOf(owed > 0 ? seller : buyer, c.ccy),
        to: ctx.accountOf(owed > 0 ? buyer : seller, c.ccy),
        ccy: c.ccy,
        amount,
        fromCell: none(),
        toCell: none(),
      });
    }
    const r = ctx.settle({
      legs,
      cause: 'corporateAction',
      reason: `${c.id} settles at the realised recovery`,
    });
    ctx.record(
      'cds.settled',
      [String(c.id), buyer, seller, String(c.terms.reference)],
      {
        contract: String(c.id),
        reference: String(c.terms.reference),
        notional: c.notional,
        paid: amount,
        settled: r.outcome === 'settled',
      },
      true,
    );
  }
}

/**
 * D2, D2.a: has this reference defaulted AND has its estate finished paying out?
 *
 * Both halves are public events this world already records — item 5 writes the default, item 7's
 * estate writes its own opening and its close — so this is a read of them and never a second
 * record of the same fact (Law 4, Law 19).
 */
export function recoveryIsKnown(ctx: MechanismContext, reference: PartyId): boolean {
  const failed = ctx.journal
    .ofKind('credit.default')
    .some((e) => e.subjects.includes(reference));
  if (!failed) return false;
  const opened = ctx.journal
    .ofKind('estate.opened')
    .filter((e) => e.subjects.includes(reference))
    .at(-1);
  if (opened === undefined) return false;
  const estate = opened.data['estate'];
  if (typeof estate !== 'string') return false;
  return ctx.journal.ofKind('estate.closed').some((e) => e.subjects.includes(estate));
}

/**
 * A4.a, G1: a contract on a reference nobody can watch fail is a contract nobody can settle.
 *
 * It is a FAMILY and not a throw because a reference can stop having debt after the contract was
 * written — a firm repays its last bond — and that is a real state of the world to report, not an
 * impossibility to refuse. What it must never be is silent.
 */
function referencesAreObservable(): Family {
  return {
    name: 'names',
    contributor: 'cds',
    spec: 'CDS A4 CDS A4.a',
    built: true,
    check: (view): Violation[] => {
      const out: Violation[] = [];
      for (const c of view.contracts.open_()) {
        if (!isCds(c.terms)) continue;
        const i = c.terms.obligation;
        if (view.instruments.has(i) && view.instruments.get(i).status.live) continue;
        out.push({
          family: 'names',
          spec: 'CDS A4.a',
          owner: String(c.id),
          size: c.notional,
          unit: String(PROTECTED),
          period: view.period,
          message: `${c.id} is protection on ${c.terms.reference}, whose ${i} is no longer a live obligation`,
        });
      }
      return out;
    },
  };
}

export function cds(house: (ccy: CurrencyCode) => PartyId): SystemModule {
  return {
    id: 'cds',
    spec: 'CDS',
    requires: ['derivative-layer', 'ratings', 'credit-events', 'estate'],
    instrumentKinds: [],
    derivativeKinds: [cdsKind, cdsIndexKind],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: PROTECTED, name: 'of face protected', perUnit: 1 }],
    params: params(),
    phases: [
      {
        name: 'cds.books',
        spec: 'CDS A1 CDS A1.d CDS A4.a',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx): void => {
          openBooks(ctx, house);
        },
      },
      {
        name: 'cds.roll',
        spec: 'CDS A5 CDS A5.a Indices A1.a',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx): void => {
          rollSeries(ctx, house);
        },
      },
      {
        name: 'cds.settle',
        spec: 'CDS D2 CDS D2.a CDS D2.b CDS D3 CDS D4',
        cycle: 'anchor',
        // After the estate has paid what it could: what protection pays is par less what the
        // reference's own debt turned out to be worth, and that number does not exist until then.
        anchor: { after: 'estates.resolve' },
        run: (ctx): void => {
          settleEvents(ctx);
          settleSeriesNames(ctx);
        },
      },
    ],
    // Clearing A2, Law 4: A CLASS DECLARES NO PARTICIPANT OF ITS OWN. Its reasons live on its own
    // profile (`DerivativeKindProfile.orders`) and the layer that owns contract books asks for
    // them — one module speaking for one party in one book, which is the assembly half of the
    // kernel's refusal to let anybody cross themselves.
    participants: [],
    families: [referencesAreObservable()],
  };
}

/** What the observer shows of one reference: its curve, as the set of levels its books cleared. */
export function curveOf(
  view: Pick<ParticipantView, 'print' | 'period'>,
  reference: PartyId,
  tenors: readonly number[],
): readonly { readonly tenorYears: number; readonly spread: number }[] {
  const out: { tenorYears: number; spread: number }[] = [];
  for (const tenorYears of tenors) {
    const p = view.print(cdsLineOf(reference, tenorYears));
    if (p.some) out.push({ tenorYears, spread: p.value.price });
  }
  return out;
}

/** B3: what protection SOLD consumes, at the weight the regulator set (Banks Capital B1). */
export function soldWeight(ctx: Pick<MechanismContext, 'params'>): number {
  return ctx.params.get(CDS_PARAMS.riskWeightSold);
}
