/**
 * The corporate bond: a firm borrows from the market instead of from a bank.
 *
 * @spec Corporate Credit A1 Corporate Credit A2 Corporate Credit A2.a Corporate Credit A2.c Corporate Credit A3 Corporate Credit A3.b Corporate Credit B1 Corporate Credit B2 Corporate Credit B2.a Corporate Credit B3 Corporate Credit B4 Corporate Credit C2 Corporate Credit C2.a Corporate Credit C2.b Corporate Credit C3 Corporate Credit C4 Corporate Credit C5 Corporate Credit C6 Corporate Credit C8 Corporate Credit E5 Corporate Credit E5.d Corporate Credit G1 Corporate Credit G2 Bond N4 Bond N5.a Bond N6 Bond N10 Bond N11 Bond N12 Bond N13 Bond N13.a Reporting A2 Law 2 Law 3 Law 4 Law 8 Law 9 Law 19
 *
 * A FIRM WITH ONE LENDER HAS ONE OPINION ABOUT IT. Until now every borrower in this world went to a
 * bank, took the keenest quote it was given, and that was the whole of the credit market: one
 * assessor per borrower, and a funding cost that moved only when a bank's own cost of funds moved.
 * A bond is the other channel — many holders, each pricing the name from its own view, and a price
 * that is struck in a book rather than offered across a desk.
 *
 * WHAT IS SOVEREIGN ABOUT A SOVEREIGN BOND IS NOT THE SCHEDULE. When a coupon falls and how it
 * accrues is the same fact for any dated bond whoever issued it, so both kinds read ONE statement
 * of it (`registry/claims.ts`). What differs is the three things that make corporate credit a
 * different subject: the issuer can FAIL, so a missed payment is a default and the estate has a
 * ranking to read; the line is SENIOR or SUBORDINATED, and the number is on the instrument where
 * the waterfall already looks (N13.a); and there are COVENANTS, which is the half of credit that
 * happens before anybody misses a payment.
 *
 * A COVENANT IS A TERM OF THE ISSUE AND NOT A BOUND (Law 6). It is what this issuer promised when
 * it borrowed — a line its own accounts must stay the right side of — and breaching it is an EVENT
 * with consequences, never a number that gets pushed back inside a range. It is tested on the
 * PUBLISHED accounts (Reporting A2), with the lag publishing already has, because a covenant a
 * lender could test on private books is not a covenant, it is surveillance.
 */
import {
  amountOf,
  asCash,
  asPerNamedUnit,
  asPerPiece,
  asRatio,
  type Cash,
  minus,
  type PerNamedUnit,
  type PerPiece,
  plus,
  type Ratio,
  ratioOf,
  valueAt,
} from '../../core/measure.js';
import { addMonths, compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import type { DayCount } from '../../calendar/daycount.js';
import {
  currencyCode,
  instrumentId,
  instrumentKindId,
  marketId,
  paramId,
  type CurrencyCode,
  type InstrumentId,
  type MarketId,
  type PartyId,
} from '../../core/ids.js';
import { InvalidRegistry, Missing } from '../../core/errors.js';
import { percent } from '../../core/format.js';
import { none, some, type Option } from '../../core/option.js';
import { ANNUAL, SEMI_ANNUAL, rate } from '../../core/rate.js';
import { upTick, type Qty } from '../../core/tick.js';
import { priceAt } from '../../prices/curve.js';
import { issuerOf, type Instrument, type Terms } from '../../register/instruments.js';
import {
  accruedOf,
  ontoTheGrid,
  cashFlowsOf,
  dueOf,
  type CouponSchedule,
} from '../../registry/claims.js';
import { FACE_TICK, MONEY_PIECES } from '../../registry/grid.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import { unitId } from '../../core/ids.js';
import { displayName, issuerName } from '../../registry/naming.js';
import { FIRM } from '../../registry/profiles.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

export const CORPORATE_BOND = instrumentKindId('corporate.bond');

/** N9: quoted as a fraction of its own face, like any other bond. */
export const CORPORATE_PAR = unitId('corporate.par');

/**
 * B2, B2.a: WHAT THIS ISSUER PROMISED ITS LENDERS, struck when it borrowed and stated on the paper.
 *
 * Two lines, because they are the two questions a lender actually asks and they fail in different
 * worlds: how much it owes against what it has (a balance-sheet test, which a fall in asset prices
 * breaks), and what it earns against what falls due (an income test, which a bad year breaks). A
 * firm can pass either while failing the other, and which one goes says what went wrong.
 *
 * Neither is a parameter. They are TERMS — this issuer's own commitment at this issue — and what a
 * given firm promised is an outcome of what it had to promise to be lent to.
 */
export interface Covenants {
  /** B2: the most it may owe against what it holds, as the issuer's own published accounts read. */
  readonly leverage: Ratio;
  /** B2: the least it must earn against what falls due, on the same published accounts. */
  readonly coverage: Ratio;
}

export interface CorporateBondTerms extends Terms, CouponSchedule {
  readonly kind: typeof CORPORATE_BOND;
  readonly issuer: PartyId;
  /**
   * N13.a: WHERE IT RANKS, as a number the waterfall already orders by. Nothing anywhere asks what
   * kind of instrument it is — a claim that ranks behind another is a claim with a bigger number.
   */
  readonly seniority: number;
  readonly covenants: Covenants;
}

export const isCorporateBond = (t: Terms): t is CorporateBondTerms =>
  'covenants' in t && 'seniority' in t && 'coupon' in t;

export function corporateBondTerms(i: Instrument): CorporateBondTerms {
  if (!isCorporateBond(i.terms)) {
    throw new InvalidRegistry('Corporate Credit A1', `${i.id} is not a corporate bond`);
  }
  return i.terms;
}

/**
 * Law 9, Law 4: a market names a bond by its issuer, its coupon and its maturity. Never by an id.
 *
 * ONE LINE PER ISSUER PER MATURITY, and the name is what makes it one: a firm that comes back in a
 * later period taps the line it already has rather than opening a second one at the same date. Two
 * lines with one name are one line written twice.
 *
 * It took a bare `n` until item 10, which named nothing — the maturity is the half of the name that
 * distinguishes one of a firm's lines from another, so it is the half that is in the id.
 */
export const corporateBondId = (issuer: PartyId, maturity: Civil): InstrumentId =>
  instrumentId(`bond:${issuer}:${formatCivil(maturity)}`);

export const corporateBond: InstrumentKindProfile = {
  id: CORPORATE_BOND,
  pricing: 'cleared',
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  // Register B3, XI-3: A BORROWER OWES THE FACE. Its paper falling because the market has lost
  // faith in it is the HOLDER's loss and never the issuer's gain — a firm that booked a profit on
  // the way down would grow more solvent the closer it came to failing, which puts the solvency
  // trigger out of reach exactly when it is supposed to fire.
  owes: 'face',
  unit: () => CORPORATE_PAR,
  ranking: (i) => ({
    seniority: isCorporateBond(i.terms) ? i.terms.seniority : 0,
    secured: [],
    claim: 'an unsecured claim on the estate, ranking where the paper says it ranks',
  }),
  /** N12: a payment fell due and the issuer did not make it. That is the whole definition. */
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment fell due and the issuer did not make it' }
      : undefined,
  /**
   * G2: AND IT IS DUE ON THE OTHERS TOO. A firm that misses one line has missed them all: its
   * lenders do not wait their turn while the estate empties. A sovereign has no cross-default
   * (Sovereign G3) and says so; a firm has one and says so here.
   */
  accelerates: true,
  validateTerms: (t) => {
    if (!isCorporateBond(t)) throw new InvalidRegistry('Corporate Credit A1', 'not bond terms');
    if (compareCivil(t.issueDate, t.maturity) >= 0) {
      throw new InvalidRegistry('Bond N4', 'a bond matures after it is issued');
    }
    // eslint-disable-next-line phoenix/no-kind-branch -- a periodicity tag, not a party or product
    if (t.coupon.per.kind !== 'annual') {
      throw new InvalidRegistry('Law 8', 'a bond coupon is quoted per annum');
    }
    if (t.covenants.leverage <= 0 || t.covenants.coverage <= 0) {
      throw new InvalidRegistry(
        'Corporate Credit B2',
        'a covenant line of nothing is a promise to nothing',
      );
    }
  },
  displayName: (i, namer) => {
    const who = issuerName(namer, i.id);
    if (!isCorporateBond(i.terms)) return `${who} bond`;
    return `${who} ${percent(i.terms.coupon.amount)} ${formatCivil(i.terms.maturity)}`;
  },
  // N6, N9.b, N5.a, N10: the kernel's schedule, because when a coupon falls is not a corporate fact.
  due: (i, period, cal, scale) =>
    isCorporateBond(i.terms) ? dueOf(i.terms, period, cal, ontoTheGrid(scale, i)) : [],
  accrued: (i, on, cal, scale) =>
    isCorporateBond(i.terms) ? accruedOf(i.terms, on, cal, ontoTheGrid(scale, i)) : 0,
  cashFlows: (i, after, cal, scale) =>
    isCorporateBond(i.terms) ? cashFlowsOf(i.terms, after, cal, ontoTheGrid(scale, i)) : [],
};

/* --------------------------------------------------------------------------------------------
 * THE ISSUE
 * ------------------------------------------------------------------------------------------ */

/** N4, C8: how long a firm's paper runs for. A convention of the market, stated with it (Law 2). */
export const CORPORATE_BOND_PARAMS = {
  tenor: paramId('corporateBond.tenor'),
} as const;

/** ACT/365F, as every dated claim in this world is measured (Law 8: the day count is the number). */
const CORPORATE_DAY_COUNT: DayCount = 'ACT/365F';

/**
 * A1, A2, A2.c, E5.d: A FIRM COMPARES TWO PRICES FOR THE SAME MONEY, and comes to market when the
 * market is the cheaper of them — or when its bank will not lend it enough at any price.
 *
 * A2.c is what makes this a mechanism rather than an accounting consequence: the capital structure
 * that results is an OUTCOME of a choice meeting the market's price, and nothing anywhere assigns
 * a firm a mix of debt. What is still missing is A2.b's target — a leverage, a coverage or a rating
 * the management is managing towards, at its own pace — so what this compares is price against
 * price and not price against a plan. That is item 17.
 *
 * THIS IS WHAT THE MODULE WAS MISSING. It declared the kind, the id, the profile, the ranking, the
 * cross-default and the covenant test, and NOTHING ANYWHERE ISSUED ONE: `testCovenants` walked an
 * empty set every period for the life of the world, three MET marks stood on a line that could not
 * exist, and 51 more Corporate Credit clauses stood behind it. The clause the module is FOR — a firm
 * funding itself in a market rather than at a bank — had no mechanism that put a firm in a market.
 *
 * BOTH SIDES OF THE COMPARISON ARE PRICES SOMEBODY ELSE MADE, and neither is invented here:
 *
 *  - what a BANK charges it is `credit.quoted` — the keenest of the quotes it was given, which is
 *    already the outcome of banks competing for it (Banks Lending C3.a), published with the SIZE
 *    that bank will actually lend;
 *  - what the MARKET would charge is what holders have published they require to hold this name
 *    (`bank.reservation`, E5), and the keenest of those is where a book for its paper would start.
 *
 * It does not price its own issue off either (Law 3). Those two decide whether it COMES; what it
 * pays is what the auction strikes, and its reservation is the alternative it already has.
 *
 * A FIRM NOBODY WOULD HOLD DOES NOT ISSUE. If no holder has published what it requires of this
 * name there is no book to come to, and the answer is a refusal rather than a guess (Appendix A).
 */
export function issueBonds(ctx: MechanismContext): void {
  for (const firm of ctx.parties.ofKind(FIRM)) {
    if (!firm.status.alive) continue;
    const need = shortOf(ctx, firm.id);
    if (!need.some) continue;
    const { short, ccy } = need.value;
    // E5, E5.d: the keenest published requirement for this name. Nobody wanting it at any rate is
    // an empty book, and an issuer does not open one to find that out.
    const keenest = wouldHold(ctx, firm.id);
    if (!keenest.some) continue;
    const quoted = quotedTo(ctx, firm.id);
    /**
     * A2, A2.c: THE TWO REASONS TO COME TO MARKET, and they are different reasons.
     *
     * The market is CHEAPER than its bank — the ordinary case, and the one that makes a bond a
     * competing channel rather than a second best. Or its bank will not lend it ENOUGH: a firm
     * short of more than the keenest bank will put behind it has no alternative at any price,
     * which is A1's "a firm large enough to reach a market is not at the mercy of one lender". A
     * firm nobody has quoted at all is in the second case by construction.
     */
    const cheaper = !quoted.some || keenest.value < quoted.value.rate;
    const enough = quoted.some && quoted.value.most >= short;
    if (!cheaper && enough) continue;
    place(ctx, firm.id, ccy, short, keenest.value, quoted);
  }
}

/** Firm E4, E5: what this firm published it is short of, and the money it is short of it in. */
function shortOf(
  ctx: MechanismContext,
  firm: PartyId,
): Option<{ readonly short: Cash; readonly ccy: CurrencyCode }> {
  const said = ctx.journal.lastOf('firms.funding', String(firm));
  if (said?.period !== ctx.period) return none();
  /**
   * E4, Short-Term Debt B1 (item 10b): THE PART OF ITS GAP THAT IS NOT DUE FOR YEARS.
   *
   * This read `short` — the whole gap — until §9 gave this world a short channel, and then one
   * hole had two issuers filling it: a five-year bond and three-month paper both brought against
   * the same published number, and the firm raised twice what it needed. `firms.funding` now
   * splits it where the firm allocates its own money, and each channel funds what it is for: paper
   * covers what falls due within weeks, a bond covers the plant (Capital Programme B2).
   */
  const short = said.data['shortTerm'];
  const ccy = said.data['ccy'];
  // E4: a negative short is what it has OVER, and a firm with money to spare does not borrow.
  if (typeof short !== 'number' || short <= 0 || typeof ccy !== 'string') return none();
  // Item 16: a published number re-enters the type system through its own dimension's door.
  return some({ short: asCash(short, 'what it is short of'), ccy: currencyCode(ccy) });
}

/**
 * E5, E5.d: THE KEENEST HOLDER'S REQUIREMENT for this name, per annum — where a book for its paper
 * would start.
 *
 * Every bank publishes what it requires to hold each obligor's paper and the lowest is the one that
 * would bid first, so this is a read of what holders SAID (Law 19) and never a spread anybody wrote
 * (Law 3). What that published number contains is the publisher's business and not this module's:
 * `holderReservation` says plainly that it is E5.a and E5.c and that E5.b waits on the ratings
 * system, and a second composition assembled here out of the private terms beside it would be this
 * module pricing a bank's book against a belief that bank does not hold (Law 4).
 */
const wouldHold = (ctx: MechanismContext, issuer: PartyId): Option<Ratio> =>
  ctx.requiredOf(issuer);

/** Banks Lending C3.a: the keenest quote this firm was given, and how much that bank will lend. */
function quotedTo(
  ctx: MechanismContext,
  firm: PartyId,
): Option<{ readonly rate: Ratio; readonly most: Cash }> {
  const said = ctx.journal.lastOf('credit.quoted', String(firm));
  if (said?.period !== ctx.period) return none();
  const quoted = said.data['rate'];
  const most = said.data['most'];
  if (typeof quoted !== 'number' || typeof most !== 'number') return none();
  return some({
    rate: asRatio(quoted, 'what its bank quoted it'),
    most: asCash(most, 'what that bank will lend it'),
  });
}

/**
 * N10, E-9: PAR ON THE PIECE GRID. `ontoTheGrid` wants the instrument and a debut has none yet, so
 * the same crossing is written here against the two things it actually reads — the money and the
 * unit — and both are known before the line exists.
 */
const gridOf =
  (ctx: MechanismContext, ccy: CurrencyCode) =>
  (x: PerNamedUnit, what: string): PerPiece =>
    ctx.registry.priceOf(ccy, CORPORATE_PAR, asPerNamedUnit(x, what));

/**
 * A1, C2, C2.a, C3, C4, C8, Clearing C4: THE ISSUE — a line, a market, and the paper brought to it.
 *
 * C2, C3, C5: THE BOOK IS THE KERNEL'S, and it has to be. Holders post schedules — a size at a level,
 * which is what C2.a says an indication is — one solver strikes the one level at which the book
 * fills (C3), and who got how many units comes out of that book and nowhere else (C5). A second
 * book built in here would be a second answer to the question the clearing system exists to answer.
 *
 * THE RESERVATION IS THE WHOLE OF THE DECISION MADE AGAIN WITH MONEY BEHIND IT: the price at which
 * the issue costs it exactly what its bank quoted. Below that the market is dearer than the loan
 * and the paper is withdrawn (C7) — which is not a bound but the alternative it already has, and a
 * firm with no quote at all has no alternative, so its walk-away is the keenest requirement itself,
 * which is the least anybody said they would take. No concession is invented anywhere in here.
 *
 * A TAP PAYS THE LINE'S OWN COUPON. A coupon is locked at issuance (N5.a) and a firm coming back to
 * a line it already has is selling more of the same paper, so what moves is the price it gets for
 * it and never what it promised.
 */
function place(
  ctx: MechanismContext,
  issuer: PartyId,
  ccy: CurrencyCode,
  short: Cash,
  keenest: Ratio,
  quoted: Option<{ readonly rate: Ratio; readonly most: Cash }>,
): void {
  const on = ctx.calendar.startOf(ctx.period);
  const maturity = addMonths(on, ctx.params.months(CORPORATE_BOND_PARAMS.tenor));
  const id = corporateBondId(issuer, maturity);
  const standing = ctx.instruments.has(id) ? ctx.instruments.get(id) : undefined;
  // G1: a line that has ceased is not tapped. What replaced it is a new name at a new maturity.
  if (standing !== undefined && !standing.status.live) return;
  const onto = gridOf(ctx, ccy);
  const schedule: CouponSchedule =
    standing === undefined
      ? {
          // B3.a: the coupon that makes the line roughly par where a book would start, which is the
          // same construction the treasury opens a line with and for the same reason — a coupon is
          // a payment the ISSUER promises, so it is struck against what somebody said they want
          // rather than against what the auction is going to do.
          coupon: rate(keenest, ANNUAL),
          couponPeriodicity: SEMI_ANNUAL,
          dayCount: CORPORATE_DAY_COUNT,
          issueDate: on,
          maturity,
        }
      : corporateBondTerms(standing);
  const flows = cashFlowsOf(schedule, on, ctx.calendar, onto);
  if (flows.length === 0) return;
  const walkAway = priceAt(
    flows,
    quoted.some ? quoted.value.rate : keenest,
    on,
    schedule.dayCount,
    'what the paper is worth at the price of its alternative',
  );
  if (walkAway <= 0) return;
  // Law 8: it needs to raise a sum of MONEY and raises it by selling UNITS, which are indivisible —
  // so it brings the whole units that sum comes to. UP, because the ask is the money: an issue a
  // fraction of a unit short of what it needs is short of it.
  const units = upTick(amountOf(short, walkAway, 'units offered'));
  if (units <= 0) return;
  if (standing === undefined && !openLine(ctx, issuer, ccy, id, schedule, units, onto)) return;
  ctx.offer({
    market: marketOf(ctx, id),
    issuer,
    size: units,
    reservation: some(walkAway),
    allotment: 'uniformPrice',
  });
  // C2.b, Clearing C4: the announcement, and the two prices it compared — so a reader can see which of B1's two
  // reasons brought it, and so a bank reads off the market that this shortfall is being raised
  // there rather than writing a loan against the same published number (Law 4).
  ctx.record(
    'bond.offered',
    [issuer, id],
    {
      issuer,
      line: id,
      size: units,
      reservation: walkAway,
      coupon: schedule.coupon.amount,
      requiredByHolders: keenest,
      quoted: quoted.some ? quoted.value.rate : null,
      lendable: quoted.some ? quoted.value.most : null,
      short,
    },
    true,
  );
}

/**
 * A1, B2, B2.a, C8, N4, N5.a, N13.a, Law 9: A NEW LINE, with what this issuer promised on it.
 *
 * C8: A DEBUT BRINGS A FRESH INSTRUMENT and a return to a line that already exists is a TAP — added
 * face on paper that already prices, cleared in the same solve as its outstanding stock. Which of
 * the two it is falls out of the NAME (Law 9) and needs no flag: there is one line per issuer per
 * maturity, so a firm that comes back to the same date is asking for more of the same paper.
 *
 * THE COVENANTS ARE ITS OWN ACCOUNTS AS THEY WILL STAND ONCE THIS PAPER IS ON THEM. B2 says what a
 * given firm promised is an outcome of what it had to promise to be lent to, and there is no
 * negotiation in this world to produce one — so what it promises is not to get WORSE than this
 * borrowing leaves it: the leverage its published balance sheet reads with this face added to what
 * it owes, and the coverage its published earnings read against what this line costs it a year.
 * Both are the arithmetic `testCovenants` will do on the same two published numbers, so the promise
 * is exactly "no worse than the day I made it" and not one number of it is invented.
 *
 * A COVENANT SET BEFORE THE FACE WAS ADDED WOULD BREACH ON THE NEXT PUBLICATION BY CONSTRUCTION,
 * which is a false breach and worse than no covenant: it is the mechanism reporting its own
 * arithmetic as the issuer's failure.
 *
 * The headroom a real negotiation would add is what a negotiation is, and its absence makes this
 * the TIGHTEST covenant a lender could ask for rather than a loose one — so a breach here is never
 * a false negative, and the firm that deteriorates after borrowing is the one that trips it.
 *
 * A FIRM THAT HAS PUBLISHED NOTHING DOES NOT BORROW IN A MARKET. It has said nothing it could
 * promise about, and terms nobody can test are not terms (Reporting A2, A2.a).
 */
function openLine(
  ctx: MechanismContext,
  issuer: PartyId,
  ccy: CurrencyCode,
  id: InstrumentId,
  schedule: CouponSchedule,
  units: Qty,
  onto: (x: PerNamedUnit, what: string) => PerPiece,
): boolean {
  const said = ctx.published.lastStatement(issuer);
  if (said === undefined || said.assets <= 0) return false;
  // N10: what it will owe is the FACE, which is par on every unit of it (`owes: 'face'`).
  const face = valueAt(onto(asPerNamedUnit(1, 'par'), 'par on the grid'), units, 'the face it takes on');
  const owes = ratioOf(
    plus(said.liabilities, face, 'what it owes with this on it'),
    said.assets,
    'what it owes against what it holds',
  );
  // The same arithmetic `testCovenants` does: this line's cost per unit of face, times the face.
  const annual = valueAt(
    asPerPiece(schedule.coupon.amount, 'what a unit of it costs a year'),
    units,
    'what this line costs it a year',
  );
  // B2, B4: A LINE THAT COSTS NOTHING A YEAR HAS NO COVER TO PROMISE, so this module brings the
  // coupon-bearing senior line and nothing else. A firm whose keenest holder requires nothing of it
  // would be issuing discount paper, which is short-term debt and is item 10b's subject — not a
  // corporate bond with a covenant on it.
  if (owes <= 0 || annual <= 0) return false;
  // A firm that published a loss has no cover to promise, and promising a negative one is not a
  // promise. It says nothing rather than saying something untestable (B2, Law 2).
  if (said.earned <= 0) return false;
  const covers = ratioOf(said.earned, annual, 'what it earns against what this line costs it');
  const terms: CorporateBondTerms = {
    kind: CORPORATE_BOND,
    issuer,
    // N13.a: senior unsecured, which is what a firm's first market borrowing is. A subordinated
    // line is a second decision and ranks behind this one by carrying a bigger number.
    seniority: 1,
    covenants: { leverage: owes, coverage: covers },
    coupon: schedule.coupon,
    couponPeriodicity: schedule.couponPeriodicity,
    dayCount: schedule.dayCount,
    issueDate: schedule.issueDate,
    maturity: schedule.maturity,
  };
  const market = marketId(`mkt.${id}`);
  ctx.issue({ id, kind: CORPORATE_BOND, issuer: some(issuer), ccy, terms, market: some(market) });
  ctx.openMarket({
    id: market,
    name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
    instrument: id,
    ccy,
    rationing: 'proRata',
  });
  return true;
}

function marketOf(ctx: MechanismContext, id: InstrumentId): MarketId {
  const inst = ctx.instruments.get(id);
  if (!inst.market.some) {
    throw new Missing('Clearing D1', `${id} names no market`, { instrument: id });
  }
  return inst.market.value;
}

/* --------------------------------------------------------------------------------------------
 * THE COVENANT
 * ------------------------------------------------------------------------------------------ */

/**
 * B2, B2.a, G1, Reporting A2: WHAT THE PUBLISHED ACCOUNTS SAY ABOUT WHAT WAS PROMISED.
 *
 * Both tests are reads of one published report and of nothing else (Law 19, Law 4). A covenant
 * tested on numbers only the issuer can see is not a covenant; a covenant tested on a second set of
 * accounts computed here would be a second set of accounts (A2.a).
 *
 * A breach is an EVENT and it is all this does. It never adjusts a number, never accelerates by
 * itself and never repairs anything: what happens next is the holders' decision, and a decision is
 * not a rule.
 */
export function testCovenants(ctx: MechanismContext): void {
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !isCorporateBond(i.terms)) continue;
    const t = i.terms;
    // Reporting A2: the last accounts it published, through the kernel's one typed read (item 3).
    const said = ctx.published.lastStatement(t.issuer);
    if (said === undefined) continue;
    // Law 4, Law 19 (item 9.1): ONE BREACH PER LINE PER SET OF ACCOUNTS, and what has already been
    // said is read from where it was said. This module kept a private `tested` memo of which line
    // it had looked at against which quarter — a second copy of a fact the journal already holds,
    // which is the mirror Law 19 is about. The TEST is a read of one published report and of the
    // terms, so it is pure and costs nothing to repeat; the only thing that must not happen twice
    // is the EVENT, and the journal is the one writer of what has been announced.
    if (alreadySaid(ctx, i.id, said.quarter)) continue;
    const broke: string[] = [];
    // B2: how much it owes against what it holds. A firm with no assets has no ratio that means
    // anything and has breached, which is what the worst case IS rather than a number pushed back.
    if (said.assets <= 0 || ratioOf(said.liabilities, said.assets, 'leverage') > t.covenants.leverage) {
      broke.push('leverage');
    }
    // B2: what it earns against what falls due on this line over a year of it.
    const owed = annualCostOf(t);
    const face = ctx.register.heldTotal(i.id).value;
    const annual = valueAt(owed, face, 'what this line costs it a year');
    if (annual > 0 && ratioOf(said.earned, annual, 'coverage') < t.covenants.coverage) {
      broke.push('coverage');
    }
    if (broke.length === 0) continue;
    ctx.record(
      'covenant.breached',
      [t.issuer, i.id],
      {
        issuer: String(t.issuer),
        bond: String(i.id),
        quarter: said.quarter,
        broke: broke.join(' and '),
        leverage: said.assets > 0 ? ratioOf(said.liabilities, said.assets, 'leverage') : null,
        promised: t.covenants.leverage,
        earned: said.earned,
        owedPerYear: annual,
      },
      true,
    );
  }
}

/** Whether this line's breach on these accounts has already been announced (Law 19: read it). */
function alreadySaid(ctx: MechanismContext, bond: InstrumentId, quarter: string): boolean {
  return ctx.journal
    .forSubject('covenant.breached', String(bond))
    .some((e) => e.data['quarter'] === quarter);
}

/**
 * B3, N13.a: every live line says where it ranks and what it promised. A bond whose ranking nobody
 * stated is the one that silently becomes a waterfall the first time somebody needs one, and a
 * covenant nobody can read is a promise nobody can hold the issuer to.
 */
function paper(): Family {
  return {
    name: 'names',
    contributor: 'corporate-bond',
    spec: 'Corporate Credit B2 Corporate Credit B3 Bond N13 Bond N13.a',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live || !isCorporateBond(i.terms)) continue;
        const holders = view.register.holdersOf(i.id);
        if (holders.length > 0) continue;
        out.push({
          family: 'names',
          spec: 'Corporate Credit B3',
          owner: i.id,
          size: view.register.heldTotal(i.id).value,
          unit: i.unit,
          period: view.period,
          message: `${i.id}: paper outstanding and nobody holding it`,
        });
      }
      return out;
    },
  };
}

export function corporateBondModule(): SystemModule {
  return {
    id: 'corporate-bond',
    // XI-8, item 9.1: NO NOUNS, and the `covenants` slot was NOT an `Agreement` waiting for a
    // home. A corporate bond is an INSTRUMENT — it has holders and it trades — and the agreement
    // store is explicitly what holds the owing that is NOT a security, so a bond must not become
    // one. The covenant is a TERM of that instrument and already lives on it; what the slot held
    // was a memo of which line had been tested against which quarter, which the journal says.
    spec: 'Corporate Credit',
    // Item 10: AND THE BANKS, because the decision this module makes is a comparison against what a
    // bank quoted and what holders published they require, and neither exists in a world with no
    // banks in it. A firm that reaches a market reaches it INSTEAD of a lender (A1), so the lender
    // has to be there to be chosen against.
    requires: ['firms', 'reporting', 'banks'],
    instrumentKinds: [corporateBond],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: CORPORATE_PAR, name: 'units of par', perUnit: MONEY_PIECES }],
    params: [
      {
        id: CORPORATE_BOND_PARAMS.tenor,
        value: 60,
        unit: 'months',
        dimension: 'months',
        kind: 'technology',
        owner: 'standardSetter',
        why: 'Bond N4, Corporate Credit C8: how long a firm\u2019s paper runs for. A convention of the market rather than a choice this world makes each time \u2014 five years is the tenor a company issues a first senior unsecured line at \u2014 and it is stated in MONTHS because that is what the calendar takes (Law 8): a tenor in years would be converted somewhere, and the conversion is the place a duration stops being the number it was declared as. It is not a forecast of how long the firm needs the money: what it needs is what it published it is short of, and the term is the market\u2019s.',
      },
    ],
    phases: [
      {
        name: 'bond.issue',
        spec: 'Corporate Credit A1 Corporate Credit A2 Corporate Credit A2.c Corporate Credit B2 Corporate Credit C2 Corporate Credit C2.a Corporate Credit C3 Corporate Credit C4 Corporate Credit C5 Corporate Credit C8 Corporate Credit E5 Corporate Credit E5.d Bond N4 Bond N5.a Reporting A2',
        // Clearing F1: after everything it reads has been published this period — what it is short
        // of (`firms.decide`), what its bank quoted it and what holders require of its name (both
        // in `lending.write`) — and before the session, because an offer that arrives after the
        // book has cleared is not an offer. `before: markets` is the last position in the period
        // that is still in front of the auction, which is exactly where an issuer stands.
        anchor: { before: 'markets' },
        reads: [
          { kind: 'event', name: 'credit.quoted', of: 'thisPeriod' },
          { kind: 'event', name: 'firms.funding', of: 'thisPeriod' },
        ],
        writes: [],
        run: issueBonds,
      },
      {
        name: 'covenant.test',
        spec: 'Corporate Credit B2 Corporate Credit B2.a Corporate Credit G1 Reporting A2',
        // B2.a: on the PUBLISHED accounts, so after whatever published them this period. A covenant
        // a lender could test on private books is not a covenant, it is surveillance.
        anchor: { before: 'revaluation' },
        // Law 10, Clearing F1.a: this phase has never RUN — no period of either world has reached
        // it — so what it reads is read off its module's source and not off a measurement, and
        // it is the module's whole read set rather than this phase's. It narrows the first time
        // the phase runs and the check can say which of these it actually wanted.
        reads: [
          { kind: 'event', name: 'credit.quoted', of: 'anyPeriod' },
          { kind: 'event', name: 'firms.funding', of: 'anyPeriod' },
        ],
        writes: [],
        run: testCovenants,
      },
    ],
    participants: [],
    families: [paper()],
  };
}

/** B1, A3: what a line costs its issuer a year, per unit of face — a read of its own terms. */
export const annualCostOf = (t: CorporateBondTerms): PerPiece =>
  asPerPiece(t.coupon.amount, 'what a unit of it costs a year');

/** B2: how far the published accounts are the right side of what was promised. Negative is a breach. */
export const headroomOn = (said: { assets: Cash; liabilities: Cash }, c: Covenants): Ratio =>
  said.assets <= 0
    ? asRatio(-1, 'a firm with no assets has breached')
    : minus(c.leverage, ratioOf(said.liabilities, said.assets, 'leverage'), 'what is left of the promise');
