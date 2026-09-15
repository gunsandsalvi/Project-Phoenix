/**
 * Insurers and pensions: the sector whose liability is a SCHEDULE, and therefore has duration.
 *
 * @spec Insurers B2 Insurers B2.b Insurers A1 Insurers A2 Insurers A2.a Insurers A3 Insurers A4 Insurers A4.a Insurers A4.b Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Insurers C2 Insurers C2.a Insurers D2 Insurers E1 Insurers E2 XI-3 XI-11 Law 2 Law 3 Law 4 Law 6 Law 19
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
import { none, some } from '../../core/option.js';
import { period, type Period } from '../../calendar/calendar.js';
import { valueAt } from '../../core/measure.js';
import { Missing } from '../../core/errors.js';
import type { Fill } from '../../clearing/solver.js';
import type { SeedContext } from '../../world/context.js';
import type { RegionId } from '../../core/ids.js';
import {
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  heldAsMoney,
  minus,
  type PerPiece,
  plus,
  pricedAt,
  type Ratio,
  ratioOf,
  scale,
} from '../../core/measure.js';
import type { Qty } from '../../core/tick.js';
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
  paramId,
} from '../../core/ids.js';
import { InvalidRegistry, Unpriced } from '../../core/errors.js';
import { sum } from '../../core/num.js';
import { downTick } from '../../core/tick.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import { FACE_TICK, MONEY_PIECES } from '../../registry/grid.js';
import type { CashFlow, InstrumentKindProfile, PartyKindProfile } from '../../registry/kinds.js';
import type { Instrument, Terms } from '../../register/instruments.js';
import { issuerName } from '../../registry/naming.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { allocate, meetCalls } from './allocate.js';

/**
 * Law 9: AN INSURANCE COMPANY, named as the world names one. The deposit insurer this world already
 * has is a different institution with a different job — it stands behind a bank's depositors and
 * charges the bank for it — and two things called the same thing is how two facts become one.
 */
export const INSURANCE = partyKindId('insurance');
export const POLICY = instrumentKindId('policy');
export const COVER = unitId('cover');

/** B1: how long a unit of cover runs for, which is a convention of the contract (Law 2). */
export const COVER_TERM = paramId('insurers.coverTermPeriods');

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
  /**
   * A-9, Law 8, Appendix A: A POLICY IS COUNTED IN UNITS OF COVER, which this module declares.
   *
   * This returned the CURRENCY CODE where a `UnitId` is wanted, through the file's one
   * `as unknown as` cast — so `Instruments.add` threw `Missing [Appendix A] unit USD does not
   * exist` and NO POLICY COULD BE REGISTERED IN ANY WORLD. The module declared `COVER` in its own
   * `units` and never used it. The cast is what let it compile; without it the type said so.
   */
  unit: () => COVER,
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
    /**
     * A-9, XI-6: A CURVE WITH NOTHING ON IT PRICES NOTHING, and this returned 0 directly under a
     * comment forbidding exactly that. A zero here is not "unknown": it reads as a liability the
     * institution has DISCHARGED, and it would flow into its equity as a solvent book.
     *
     * `derive` answers a number or it does not answer, so the honest form is the throw the kernel
     * already makes for a derived kind that derives nothing — at the site, with a citation, never
     * caught in the engine (§5).
     */
    if (!priced.some) {
      throw new Unpriced('Insurers B2', `${i.id} is discounted at a curve with nothing on it`, {
        instrument: String(i.id),
        curve: String(i.terms.discountedAt),
      });
    }
    return priced.value;
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
    capital > 0
      ? ratioOf(
          heldAsMoney(capital, 'what it owes'),
          plus(heldAsMoney(capital, 'what it owes'), surplus, 'what funds it'),
          'what its capital costs',
        )
      : asRatio(0, 'it owes nothing, so its capital costs it nothing');
  const price = coverPrice(experience, required);
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
function claimsSeen(view: ParticipantView): PerPiece {
  const paid: Cash[] = [];
  const written: Qty[] = [];
  /**
   * A-9, Register B3, Law 19: THE POLICIES IT HAS WRITTEN ARE ITS LIABILITIES, NOT ITS HOLDINGS.
   *
   * This walked `view.holdings()` for policies whose `insurer` is itself — and an insurer does not
   * HOLD the cover it wrote: the beneficiary does. So `written` was always empty, `experience` was
   * always the claims over nothing, and the price of cover was never made of anything it had seen.
   * What it has written is what it ISSUED, which the register indexes both ways (Register B2).
   */
  for (const i of view.instruments.issuedBy(view.self.id)) {
    if (!i.status.live || !isPolicy(i.terms)) continue;
    written.push(i.issued);
  }
  // A4.c: its OWN claims, which are the ones it was a side of. `lastOwn` asks that question
  // directly rather than filtering everything public by a name (Observer A4).
  const claim = view.lastOwn(CLAIM_PAID);
  if (claim.some) {
    const amount = claim.value.data['amount'];
    if (typeof amount === 'number') paid.push(asCash(amount, 'what it paid on a claim'));
  }
  const cover = sum(written).value;
  return cover > 0
    ? pricedAt(sum(paid).value, cover, 'what a unit of cover has cost it')
    : asPerPiece(0, 'it has written no cover, so nothing has cost it anything');
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
    // Item 14.0: AND FUNDS, because an institution does not invest itself — what it does with its
    // assets is hand them to a manager, so the doors it subscribes at have to exist before it looks
    // for one. It is a dependency of the ALLOCATION and not of the cover it writes.
    requires: ['sovereign-curve', 'funds'],
    instrumentKinds: [policyKind],
    partyKinds: [insuranceKind],
    curveFamilies: [],
    units: [{ id: COVER, name: 'units of cover', perUnit: MONEY_PIECES }],
    params: [
      {
        id: COVER_TERM,
        value: 52,
        unit: 'periods',
        dimension: 'periods',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Insurers B1: how long one unit of cover runs for. A convention of the contract, stated with it — a year, which is what a policy is written for. It is not a forecast of when a claim arrives: what a claim COSTS is read off what this insurer has actually paid (A4.c), and it is the price that carries it.',
      },
    ],
    phases: [
      {
        /**
         * B2, B2.b (item 14.0): AN INSTITUTION PUTS ITS ASSETS UNDER MANAGEMENT.
         *
         * *"Insurance companies and pension funds don't invest themselves. Their assets are always
         * third party managed"* (the owner). So this posts ONE subscription at the door of the pool
         * whose mandate is shaped like what this insurer promised, and everything after that is the
         * manager's. There is no portfolio here, no allocation rule and no view about a price.
         *
         * BEFORE the fund strikes, because a subscription is read at the strike — and after its own
         * period's flows, so what it puts to work is what it actually has.
         */
        name: 'insurers.allocate',
        spec: 'Insurers B1 Insurers B2 Insurers B2.a Insurers B2.b Fund Shares C1 Fund Shares C2 Private Equity A2.a XI-2',
        anchor: { before: 'funds.strike' },
        reads: [
          { kind: 'event', name: 'fund.called', of: 'thisPeriod' },
          { kind: 'event', name: 'insurer.claim', of: 'anyPeriod' },
        ],
        writes: [],
        run: (ctx: MechanismContext): void => {
          for (const p of ctx.parties.ofKind(INSURANCE)) {
            if (!p.status.alive) continue;
            /**
             * §29 A2.a (item 13.5c): WHAT IT OWES BEFORE WHAT IT WOULD LIKE TO OWN. A call it could
             * not meet is money it must find, and asking for it back comes before putting anything
             * new to work — an investor that allocated while it was in default on a call would be
             * two decisions about one balance (Law 4).
             */
            meetCalls(ctx, p.id);
            allocate(ctx, p.id, ctx.registry.currencyOf(p.region));
          }
        },
      },
      {
        name: 'insurers.cover',
        spec: 'Insurers A4 Insurers A4.a Insurers A4.b Insurers A4.c Clearing C3',
        // Before the goods and paper sessions, because what an insurer writes this period is
        // capacity it then has to stand behind: a session it cannot see cannot be a reason.
        anchor: { before: 'markets' },
        // Law 10, Clearing F1.a: this phase has never RUN — no period of either world has reached
        // it — so what it reads is read off its module's source and not off a measurement, and
        // it is the module's whole read set rather than this phase's. It narrows the first time
        // the phase runs and the check can say which of these it actually wanted.
        reads: [
          { kind: 'event', name: 'fund.struck', of: 'anyPeriod' },
        ],
        writes: [],
        run: (ctx: MechanismContext): void => {
          /**
           * B-2, A-9: THE SECTOR RUNS. `phases: []` and `participants: []` meant nothing in this
           * module was ever asked anything, in any period of any run — the price of cover, the
           * capacity, the venue and the audit family were all reachable only from a test.
           *
           * What is here is the SELL side: every insurer quotes what a unit of cover costs it out
           * of its own claims experience and its own capital (A4.b), and what clears is written.
           * The BUY side — a firm that stands in a physical fact and would rather not (B4) — is
           * item 14's, and until it exists these sessions come back `noDemand`, which is a
           * measured state and not an absence.
           */
          for (const ccy of ctx.registry.currencies.keys()) {
            const bids: Order[] = [];
            for (const p of ctx.parties.ofKind(INSURANCE)) {
              if (!p.status.alive || ctx.registry.currencyOf(p.region) !== ccy) continue;
              bids.push(...quoteCover(ctx.participant(p.id), ccy));
            }
            runCover(ctx, ccy, bids);
          }
        },
      },
    ],
    participants: [],
    families: [promises()],
    seed(ctx: SeedContext): void {
      /**
       * B-2, A1: AN INSURER EXISTS BEFORE ANYBODY BUYS COVER. `insurers()` had no `seed`, so no
       * party of the kind was ever created in any world — the sector was a declaration and nothing
       * else. One per region that has a bank to hold its money, named and banked like any other
       * institution, because an insurer is one: it has a balance sheet, it can fail, and what it
       * writes is a claim on it.
       */
      for (const region of ctx.registry.regions.values()) {
        const bank = [...ctx.parties.all()].find(
          (p) => p.region === region.id && ctx.registry.issuesMoney(p.kind) && p.bank !== p.id,
        );
        if (bank === undefined) continue;
        ctx.parties.add({
          id: insurerIdFor(region.id),
          kind: INSURANCE,
          region: region.id,
          name: `${region.name} Assurance`,
          bank: bank.id,
          representation: 'named',
          status: { alive: true, standing: 'good' },
        });
      }
    },
  };
}

/** Law 9: an insurer is named for where it writes. An id is an id and never a display name. */
export function insurerIdFor(region: RegionId): PartyId {
  return `insurance.${region}` as PartyId;
}

/**
 * B2.a, D2: WHAT A RATE MOVE DOES TO A BOOK OF PROMISES — the number the whole sector turns on,
 * exposed so a test can put a case to it. Falling rates raise the liability; the institution's
 * equity falls by the same amount, because what it owes is what the promise is worth.
 */
export const presentValueOf = (
  schedule: readonly CashFlow[],
  discount: (date: Civil) => Ratio,
): PerPiece =>
  sum(schedule.map((f) => scale(f.perUnit, discount(f.date), 'discounted'))).value;

/** A4.b: the two halves of a cover price, which are the only two things in it. */
export const coverPrice = (experience: PerPiece, requiredOnCapital: Ratio): PerPiece =>
  plus(
    experience,
    // A4.b: what its capital costs is a SHARE and what cover costs is a level, so the share is a
    // level of the same thing before they add — which is what "per unit of cover" means.
    asPerPiece(requiredOnCapital, 'what its capital costs, per unit of cover'),
    'what a unit of cover costs to write',
  );

/** D1: the mismatch, in the one unit that makes it comparable — what each side is worth now. */
export const gapOf = (assets: Cash, liabilities: Cash): Cash =>
  minus(assets, liabilities, 'what it has against what it owes');

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
  /**
   * A-9, Law 5, Clearing D2: A SESSION THAT STRIKES A PRICE AND MOVES NO MONEY IS NOT A SESSION.
   *
   * This cleared the book and then DISCARDED `outcome.fills`, recording only how many there were.
   * Every other market in this world turns a fill into an instruction (D2, D3); this one turned it
   * into a count. So even in a world where somebody bid for cover, no policy was ever registered,
   * no premium was ever paid, and the number in the record was about a trade that did not happen.
   *
   * A fill is a policy: the buyer pays the premium and the insurer ISSUES it the cover, which is a
   * claim on the insurer for the schedule the policy carries (B1). Both legs, one instruction, same
   * period (Law 5).
   */
  let written = 0;
  for (const fill of outcome.fills) {
    if (fill.side !== 'buy') continue;
    const insurer = sellerOf(outcome.fills, fill);
    if (insurer === undefined) continue;
    const qty = downTick(fill.qty);
    if (qty <= 0) continue;
    const premium = ctx.registry.cashFor(valueAt(outcome.price, qty, 'the premium at inception'));
    if (premium <= 0) continue;
    const id = policyIdFor(insurer, fill.party, ctx.period);
    if (!ctx.instruments.has(id)) {
      ctx.issue({
        id,
        kind: POLICY,
        issuer: some(insurer),
        ccy,
        terms: coverTerms(ctx, insurer, ccy),
        market: none(),
      });
    }
    const r = ctx.settle({
      legs: [
        {
          kind: 'asset',
          from: insurer,
          to: fill.party,
          instrument: id,
          qty,
          pricePerUnit: some(outcome.price),
          accruedPerUnit: none(),
          fromCell: none(),
          toCell: none(),
        },
        {
          kind: 'money',
          from: ctx.accountOf(fill.party, ccy),
          to: ctx.accountOf(insurer, ccy),
          // Treasury C1: what this money is to the party getting it — a premium is revenue.
          receipt: { of: 'sale' },
          ccy,
          amount: premium,
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'corporateAction',
      reason: `${String(insurer)} writes cover for ${String(fill.party)}`,
    });
    if (r.outcome === 'settled') written += 1;
  }
  ctx.record('cover.cleared', [], { ccy, price: outcome.price, written }, true);
};

/** Clearing D2: who was on the other side of this fill, from the book's own record of it. */
function sellerOf(fills: readonly Fill[], buy: Fill): PartyId | undefined {
  return fills.find((f) => f.side === 'sell' && f.party !== buy.party)?.party;
}

/**
 * B1, B2, B2.b: WHAT A UNIT OF COVER PROMISES. One payment, at the end of the term the exchange
 * writes cover for, discounted at the sovereign curve of its own money — which is what gives the
 * sector its duration (B2.b: a liability with no schedule is a cash balance).
 *
 * The TERM is a convention of the policy and not a forecast of when a claim arrives: what a claim
 * costs is `claimsSeen`, read off what this insurer has actually paid, and it is the PRICE that
 * carries it (A4.c). Nothing here draws from a distribution anybody stated.
 */
function coverTerms(ctx: MechanismContext, insurer: PartyId, ccy: CurrencyCode): PolicyTerms {
  const family = ctx.sovereignCurveIn(ccy);
  if (!family.some) {
    throw new Missing('Insurers B2', `cover in ${ccy} has no curve to be discounted at`, { ccy });
  }
  return {
    kind: POLICY,
    insurer,
    // B1: one unit of cover promises one unit of its money, at the end of the term.
    schedule: [
      {
        date: ctx.calendar.startOf(period(ctx.period + ctx.params.periods(COVER_TERM))),
        perUnit: asPerPiece(1, 'a unit of cover promises a unit of its money'),
      },
    ],
    discountedAt: family.value.id,
  };
}

/** Law 9: a policy is named for who wrote it, for whom, and when. An id is never a display name. */
export const policyIdFor = (insurer: PartyId, holder: PartyId, at: Period): InstrumentId =>
  instrumentId(`policy:${insurer}:${holder}:${at}`);
