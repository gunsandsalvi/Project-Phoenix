/**
 * Insurers and pensions: the sector whose liability is a SCHEDULE, and therefore has duration.
 *
 * @spec Insurers A1 Insurers A2 Insurers A2.a Insurers A3 Insurers A4 Insurers A4.a Insurers A4.b Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Insurers C2 Insurers C2.a Insurers D2 Insurers E1 Insurers E2 XI-3 XI-11 Law 2 Law 3 Law 4 Law 6 Law 19
 *
 * B2.b IS THE CLAUSE THIS EXISTS FOR, and it says what the easy version gets wrong: a liability that
 * accumulates contributions minus benefits plus investment income has no schedule, no discount rate
 * and no discounting — so it never moves when rates move, and THE SECTOR'S DEFINING RISK
 * DISAPPEARS. This is the model's largest holder of duration; it must have duration.
 *
 * So a policy here is a promise to pay STATED AMOUNTS AT STATED FUTURE TIMES, and what it is worth
 * is those amounts discounted at a rate read from a market (B2). Falling rates raise it. That is
 * why a rate move is a solvency event for this sector and a P&L event for everybody else (B2.a,
 * D2), and it falls out of the arithmetic rather than being asserted anywhere.
 *
 * AND THE INSURER WEARS IT, which is the whole difference between this sector and a fund (A2.a).
 * The beneficiary does not absorb the investment result: the promise is fixed and the institution's
 * equity is what moves. In this world that is one declared fact — the kind says its issuer owes the
 * VALUE of what it promised rather than a face — and the revaluation pass does the rest. A sector
 * that passed the investment result straight through would be a fund wearing an insurer's name.
 *
 * A3, XI-3: equity is assets minus liabilities, it is a READ, and it can go negative — which is a
 * solvency event with consequences, because these institutions can fail like anything else.
 */
import { compareCivil, type Civil } from '../../calendar/civil.js';
import {
  instrumentId,
  instrumentKindId,
  partyKindId,
  unitId,
  venueId,
  type CurrencyCode,
  type CurveFamilyId,
  type InstrumentId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import { add, div, mul, sub, sum } from '../../core/num.js';
import { downTick } from '../../core/tick.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { FACE_TICK, MONEY_PIECES } from '../../registry/grid.js';
import type { CashFlow, InstrumentKindProfile, PartyKindProfile } from '../../registry/kinds.js';
import type { Instrument, Terms } from '../../register/instruments.js';
import { issuerName } from '../../registry/naming.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

/**
 * Law 9: AN INSURANCE COMPANY, named as the world names one. The deposit insurer this world already
 * has is a different institution with a different job — it stands behind a bank's depositors and
 * charges the bank for it — and two things called the same thing is how two facts become one.
 */
export const INSURANCE = partyKindId('insurance');
export const POLICY = instrumentKindId('policy');
export const COVER = unitId('cover');

/** A4: a claim this insurer actually paid, which is the only experience it has (A4.c). */
export const CLAIM_PAID = 'insurer.claim';

/** Law 9: named for who owes it and which book it is on. */
export const policyId = (insurer: PartyId, n: number): InstrumentId =>
  instrumentId(`policy:${insurer}:${n}`);

export const coverVenue = (ccy: CurrencyCode): VenueId => venueId(`cover:${ccy}`);

export interface PolicyTerms extends Terms {
  readonly kind: typeof POLICY;
  readonly insurer: PartyId;
  /**
   * B1: HOW MUCH IS OWED IN EACH FUTURE PERIOD. This is the schedule, and it is the thing B2.b says
   * must exist: without it there is no discounting and the sector has no duration.
   */
  readonly schedule: readonly CashFlow[];
  /** B2: the market whose prices say what money later is worth. Stated, so a reader can check it. */
  readonly discountedAt: CurveFamilyId;
}

export const isPolicy = (t: Terms): t is PolicyTerms =>
  'schedule' in t && 'discountedAt' in t && 'insurer' in t;

export function policyTerms(i: Instrument): PolicyTerms {
  if (!isPolicy(i.terms)) throw new InvalidRegistry('Insurers A2', `${i.id} is not a policy`);
  return i.terms;
}

export const insuranceKind: PartyKindProfile = {
  /** item 15: what somebody else set it up to do, and it does not get to change it. */
  objective: 'itsMandate',
  id: INSURANCE,
  representation: 'named',
  moneyIssuer: null,
  /**
   * A3, XI-3: THESE INSTITUTIONS CAN FAIL, both ways. It can be unable to pay a claim that fell due
   * today while holding assets it cannot sell in time, and it can owe more than it has — which is
   * what a rate move does to it, and is the consequence B2.a exists to produce.
   */
  fails: ['cash', 'solvency'],
  borrows: true,
  depositClass: null,
};

export const policyKind: InstrumentKindProfile = {
  id: POLICY,
  /**
   * B2, Law 3: WHAT IT IS WORTH IS ITS SCHEDULE AT A MARKET'S OWN RATE. Nobody makes a market in a
   * pension promise, so there is no price to clear — but there IS a price for money later, and the
   * curve is where this world keeps it. Discounting at it is reading a price, not inventing one.
   */
  pricing: 'derived',
  priceTick: FACE_TICK,
  carry: 'cost',
  // C4, D4: it moves both ways with the rate. An insurer that could only be written up would be an
  // insurer with a hidden guarantor on the way down.
  fairValueThroughIncome: true,
  liabilityOfIssuer: true,
  /**
   * A2.a, B2.a, D2, Register B3: THE INSTITUTION WEARS THE MOVE. What it owes IS the present value
   * of what it promised, so when rates fall the liability rises and its equity falls — which is the
   * sector's defining risk and the thing B2.b says must not disappear. This is the second kind in
   * this world whose issuer owes a value rather than a face, and it is the opposite case from the
   * first: a fund share moves because the ASSETS moved, a policy because the DISCOUNT RATE did.
   */
  owes: 'value',
  unit: (ccy) => ccy as unknown as ReturnType<InstrumentKindProfile['unit']>,
  validateTerms: (t) => {
    if (!isPolicy(t)) throw new InvalidRegistry('Insurers A2', 'not policy terms');
    if (t.schedule.length === 0) {
      // B2.b: a liability with no schedule is a cash balance, which never moves when rates move.
      throw new InvalidRegistry('Insurers B2.b', 'a policy with no schedule has no duration');
    }
    for (const flow of t.schedule) {
      if (flow.perUnit <= 0) {
        throw new InvalidRegistry('Insurers B1', 'a scheduled payment of nothing is not a promise');
      }
    }
  },
  displayName: (i, namer) =>
    isPolicy(i.terms)
      ? `${issuerName(namer, i.id)} cover to ${lastDate(i.terms.schedule)}`
      : String(i.id),
  ranking: () => ({
    seniority: 0,
    // A1, E1: a beneficiary is a named creditor of the institution and ranks like one.
    secured: [],
    claim: 'a beneficiary of a promise to pay stated amounts on stated dates',
  }),
  // A4: the payments are real flows to named parties, and the kernel's own phase makes them.
  due: (i, period, cal) => {
    if (!isPolicy(i.terms)) return [];
    const from = cal.startOf(period);
    const to = cal.endOf(period);
    return i.terms.schedule
      .filter((f) => compareCivil(f.date, from) >= 0 && compareCivil(f.date, to) <= 0)
      .map((f) => ({ kind: 'coupon' as const, date: f.date, amountPerUnit: f.perUnit }));
  },
  accrued: () => 0,
  cashFlows: (i, after) =>
    isPolicy(i.terms) ? i.terms.schedule.filter((f) => compareCivil(f.date, after) > 0) : [],
  /**
   * B2, B2.a: THE PRESENT VALUE, and it is the market's own arithmetic. `priceOf` is the same
   * function a bond's price is checked against, so a claim discounted here and a bond priced there
   * are reading one curve (Law 4) — and when that curve falls, this number rises.
   */
  derive: (i, at, reads) => {
    if (!isPolicy(i.terms)) return 0;
    const curve = reads.curve(i.terms.discountedAt, at);
    const priced = curve.priceOf(i.terms.schedule, reads.on(at));
    // XI-6: a curve with nothing on it prices nothing. Missing is Missing — never a zero that would
    // read as a liability the institution has discharged.
    return priced.some ? priced.value : 0;
  },
};

/**
 * B1: the last day this promise runs to. A policy with no schedule is refused at assembly (B2.b),
 * so an empty one cannot reach here — and if it ever did, a made-up date would be a lie about when
 * somebody is owed money. Missing is Missing (Law 2).
 */
const lastDate = (s: readonly CashFlow[]): string => {
  const d = s[s.length - 1]?.date;
  if (d === undefined) throw new InvalidRegistry('Insurers B1', 'a policy with no schedule');
  return `${d.y}-${d.m}-${d.d}`;
};

/* --------------------------------------------------------------------------------------------
 * WHAT IT CHARGES
 * ------------------------------------------------------------------------------------------ */

/**
 * A4.a, A4.b, Law 3: THE PRICE OF COVER, and a policy goes to the insurer that prices lower.
 *
 * A4.b says what the price is made of and it is two things, both the insurer's own: the claims a
 * unit of cover is expected to bring, which is ITS OWN experience of ITS OWN book (A4.c), and the
 * return required on the capital held against it. Worse experience or dearer capital quotes higher,
 * and neither is a number anybody typed — the first is a read of what its periods actually cost it
 * and the second is what its own money costs it.
 *
 * A4.a: an insurer with no surplus writes nothing and loses its renewals. It loses book before it
 * loses its licence, which is a consequence of having nothing to stand behind cover with rather
 * than a rule about solvency.
 */
export function quoteCover(view: ParticipantView, ccy: CurrencyCode): readonly Order[] {
  const surplus = view.equity();
  // A4.a: nothing to stand behind it with, so nothing written. Not a threshold — an insurer with no
  // surplus has no capacity, which is arithmetic (Law 6).
  if (surplus <= 0) return [];
  const experience = claimsSeen(view);
  const capital = view.owedIn(ccy);
  const required =
    capital > 0 ? div(capital, add(capital, surplus, 'what funds it'), 'what its capital costs') : 0;
  const price = add(experience, required, 'what a unit of cover costs it to write');
  if (price <= 0) return [];
  const capacity = downTick(surplus);
  if (capacity <= 0) return [];
  // Clearing C3: it posts a price and a size, and the book decides whose cover gets written.
  return [{ party: view.self.id, side: 'sell', price, qty: capacity }];
}

/**
 * A4.c, Law 19: ITS OWN CLAIMS OFF ITS OWN BOOK. What a unit of cover has actually cost it, read
 * off the payments it has made — never a loss ratio, never an industry number, never a draw from a
 * distribution somebody stated.
 */
function claimsSeen(view: ParticipantView): number {
  const paid: number[] = [];
  const written: number[] = [];
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!isPolicy(i.terms) || i.terms.insurer !== view.self.id) continue;
    written.push(view.quantity(h.instrument));
  }
  // A4.c: its OWN claims, which are the ones it was a side of. `lastOwn` asks that question
  // directly rather than filtering everything public by a name (Observer A4).
  const claim = view.lastOwn(CLAIM_PAID);
  if (claim.some) {
    const amount = claim.value.data['amount'];
    if (typeof amount === 'number') paid.push(amount);
  }
  const cover = sum(written).value;
  return cover > 0 ? div(sum(paid).value, cover, 'what a unit of cover has cost it') : 0;
}

/**
 * E1, E2, A1: what must be true of this sector, MEASURED and never repaired. Each breaks silently:
 * a liability with no beneficiary is a promise to nobody, and a policy with no schedule is the cash
 * balance B2.b forbids — the shape that makes the sector's defining risk disappear.
 */
function promises(): Family {
  return {
    name: 'names',
    contributor: 'insurers',
    spec: 'Insurers A1 Insurers B2.b Insurers E1 Insurers E2',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live || !isPolicy(i.terms)) continue;
        if (view.register.holdersOf(i.id).length === 0 && view.register.heldTotal(i.id).value > 0) {
          out.push({
            family: 'names',
            spec: 'Insurers E1',
            owner: i.id,
            size: view.register.heldTotal(i.id).value,
            unit: i.unit,
            period: view.period,
            message: `${i.id}: a promise outstanding and nobody it is owed to`,
          });
        }
        if (i.terms.schedule.length === 0) {
          out.push({
            family: 'names',
            spec: 'Insurers B2.b',
            owner: i.id,
            size: 0,
            unit: i.unit,
            period: view.period,
            message: `${i.id}: a liability with no schedule, which is a cash balance`,
          });
        }
      }
      return out;
    },
  };
}

export function insurers(): SystemModule {
  return {
    id: 'insurers',
    spec: 'Insurers',
    requires: ['sovereign-curve'],
    instrumentKinds: [policyKind],
    partyKinds: [insuranceKind],
    curveFamilies: [],
    units: [{ id: COVER, name: 'units of cover', perUnit: MONEY_PIECES }],
    params: [],
    phases: [],
    participants: [],
    families: [promises()],
  };
}

/**
 * B2.a, D2: WHAT A RATE MOVE DOES TO A BOOK OF PROMISES — the number the whole sector turns on,
 * exposed so a test can put a case to it. Falling rates raise the liability; the institution's
 * equity falls by the same amount, because what it owes is what the promise is worth.
 */
export const presentValueOf = (
  schedule: readonly CashFlow[],
  discount: (date: Civil) => number,
): number => sum(schedule.map((f) => mul(f.perUnit, discount(f.date), 'discounted'))).value;

/** A4.b: the two halves of a cover price, which are the only two things in it. */
export const coverPrice = (experience: number, requiredOnCapital: number): number =>
  add(experience, requiredOnCapital, 'what a unit of cover costs to write');

/** D1: the mismatch, in the one unit that makes it comparable — what each side is worth now. */
export const gapOf = (assets: number, liabilities: number): number =>
  sub(assets, liabilities, 'what it has against what it owes');

/** C2.a: whether this book is one a matching buyer wants — long assets against long liabilities. */
export const wantsDuration = (schedule: readonly CashFlow[], horizon: Civil): boolean =>
  schedule.some((f) => compareCivil(f.date, horizon) > 0);

/** Kept where the venue is named, so the observer and a test spell it one way (Law 4). */
export const venueForCover = coverVenue;

export const runCover = (ctx: MechanismContext, ccy: CurrencyCode, bids: readonly Order[]): void => {
  const venue = coverVenue(ccy);
  if (!ctx.venues.some((v) => v.id === venue)) {
    ctx.openVenue({
      id: venue,
      name: `cover in ${ccy}`,
      clearedBy: 'insurers',
      unit: COVER,
      ccy,
      key: { ccy },
    });
  }
  for (const b of bids) ctx.post(venue, b);
  const outcome = clear(ctx.posted(venue), 'proRata', 'sellersCompete');
  if (!isCleared(outcome)) return;
  ctx.record(
    'cover.cleared',
    [],
    { ccy, price: outcome.price, written: outcome.fills.length },
    true,
  );
};
