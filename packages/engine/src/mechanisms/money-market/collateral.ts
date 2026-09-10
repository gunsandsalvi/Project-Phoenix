/**
 * What a lender will take as collateral, and what it will lend against it.
 *
 * @spec Money Market B3 Money Market B3.a Money Market B3.b Money Market B3.c Money Market C4 Money Market C4.b Banks Funding C1 Banks Funding C1.a Central Bank D2 Corporate Credit E5 Law 15 Law 19
 *
 * B3.a: ELIGIBILITY IS DEFINED PER ASSET, and something is ineligible. What makes an asset eligible
 * here is what it IS, asked of the kind's own profile: a claim on somebody (so there is a name
 * behind it), with dated payments (so it can be valued at a yield), that a market prices (so it can
 * be sold if it has to be). A share is not a claim, a loan has no market, a tonne of grain is
 * neither — none of them can be repoed, and no list anywhere names them.
 *
 * B3.b: THE HAIRCUT IS NOT A TABLE. What a lender will advance is what the paper is worth TO IT —
 * its own reservation yield for that issuer, applied to that paper's own remaining cash flows. So
 * the same bond is worth less to a lender that requires more (its credit view and its funding cost
 * are both in that number, Corporate Credit E5), a long bond is discounted further than a short one
 * at the same yield gap, and the haircut against the market price falls out as a READ (Law 19)
 * rather than being stated. A lender that has published no view of an issuer will not take its
 * paper: no view, no advance.
 */
import type { Civil } from '../../calendar/civil.js';
import type { DayCount } from '../../calendar/daycount.js';
import type { InstrumentId, PartyId } from '../../core/ids.js';
import { add, div, mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { priceAt } from '../../prices/curve.js';
import type { Instrument } from '../../register/instruments.js';
import type { ParticipantView } from '../../world/context.js';

/** One day count for what a lender advances, stated once by the module that asks the question. */
export const COLLATERAL_DAY_COUNT: DayCount = 'ACT/ACT';

/** B3.a: what a market will take as security — a priced, dated claim on a name. */
export function eligible(view: ParticipantView, i: Instrument, on: Civil): boolean {
  if (!i.status.live || !i.issuer.some) return false;
  const profile = view.registry.instrumentKind(i.kind);
  if (!profile.liabilityOfIssuer || profile.pricing !== 'cleared') return false;
  return profile.cashFlows(i, on, view.calendar).length > 0;
}

/**
 * Corporate Credit E5, XI-4: what THIS lender requires, per annum, of a named issuer's paper. It is
 * read from what the lender itself published (Law 4: the module that owns a bank's economics owns
 * the number), never rebuilt here — a haircut struck against a second copy of a bank's credit view
 * would be the bank securing its lending against a belief it does not hold.
 */
export function requiredOf(view: ParticipantView, issuer: PartyId): Option<number> {
  return published(view, 'required', issuer);
}

/**
 * Money Market B2, Banks Lending C1.b, C4: what THIS lender believes an unsecured claim on that
 * name costs it — what it expects to lose on the name, and what the capital such a claim consumes
 * costs it. Both are published by the module that owns a bank's credit model, so a bank that has
 * watched a name miss a payment charges it more here without a second belief being wired into the
 * money market (B2.a: and past some point it will not bid at all).
 *
 * What is NOT here is its blended cost of funds, and leaving it out is the point. Placing cash it
 * already has does not fund anything: the alternative to placing it is the floor (B5.a), so what
 * the placement has to beat is the floor plus what the name costs it — which is why an overnight
 * rate sits near the corridor and a year of money to a firm does not.
 */
export function nameCost(view: ParticipantView, borrower: PartyId): Option<number> {
  const loss = published(view, 'expectedLoss', borrower);
  const capital = published(view, 'capitalCost', borrower);
  if (!loss.some || !capital.some) return none<number>();
  return some(add(loss.value, capital.value, 'what the name costs it'));
}

function published(view: ParticipantView, key: string, about: PartyId): Option<number> {
  const own = view.lastOwn('bank.reservation');
  if (!own.some) return none<number>();
  const map = own.value.data[key];
  if (typeof map !== 'object' || map === null) return none<number>();
  const rate = (map as Record<string, unknown>)[about];
  return typeof rate === 'number' ? some(rate) : none<number>();
}

/** B3.b: what one unit of this paper is worth to this lender — its own yield, that paper's flows. */
export function valueToLender(view: ParticipantView, i: Instrument, on: Civil): Option<number> {
  if (!i.issuer.some) return none<number>();
  const required = requiredOf(view, i.issuer.value);
  if (!required.some) return none<number>();
  const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar);
  if (flows.length === 0) return none<number>();
  return some(priceAt(flows, required.value, on, COLLATERAL_DAY_COUNT, `advance against ${i.id}`));
}

/**
 * B3.b: the haircut, as the READ it is — how far below the market's price this lender's own
 * valuation sits. Nothing prices off it; it is what a reader (and Part XII) measures. None when the
 * market has never printed the paper, because there is nothing to take a haircut from.
 */
export function haircut(view: ParticipantView, i: Instrument, on: Civil): Option<number> {
  const value = valueToLender(view, i, on);
  const print = view.print(i.id);
  if (!value.some || !print.some || print.value.price <= 0) return none<number>();
  return some(div(sub(print.value.price, value.value, 'below the market'), print.value.price, 'haircut'));
}

/** A parcel of paper a borrower could put up, and what this lender would advance against it. */
export interface Advance {
  readonly instrument: InstrumentId;
  /** Units free to bind: what is held less what already stands behind something else (B3.c). */
  readonly free: number;
  readonly valuePerUnit: number;
  readonly total: number;
  /** Law 8: the smallest piece of this paper, which is what a lien can be struck in. */
}

/**
 * C4.b, B3.c: everything the borrower has free that this lender would take, most valuable first.
 * What is already pledged is not here, which is the whole of B3.c: running out of unencumbered
 * paper is how a solvent bank stops being able to borrow.
 *
 * The lender reads the BORROWER's holdings, which is a thing a lender may not do in general
 * (Observer A4) — so it is asked with the borrower's own view, by the module, on the borrower's
 * behalf: what a borrower offers up is what the borrower discloses, and this is the disclosure.
 */
export function advances(
  lender: ParticipantView,
  borrower: ParticipantView,
  on: Civil,
): readonly Advance[] {
  const out: Advance[] = [];
  for (const h of borrower.holdings()) {
    const i = borrower.instruments.get(h.instrument);
    if (!eligible(lender, i, on)) continue;
    const value = valueToLender(lender, i, on);
    if (!value.some || value.value <= 0) continue;
    const free = borrower.free(h.instrument);
    if (free <= 0) continue;
    out.push({
      instrument: i.id,
      free,
      valuePerUnit: value.value,
      total: mul(free, value.value, 'what this parcel would raise'),
    });
  }
  return out.sort((a, b) => (a.total === b.total ? (a.instrument < b.instrument ? -1 : 1) : b.total - a.total));
}

/**
 * Central Bank D2: THE WINDOW'S OWN VALUATION, which is a different thing from a market lender's.
 * Eligibility and haircuts are the central bank's CHOICE and a policy instrument in themselves, so
 * what it advances is what the market says the paper is worth less the haircut it has declared —
 * not a yield it requires, because it does not require one: it is the issuer of the money it lends.
 *
 * A6, B3.c: what is already bound is not here either. The window is collateralised like everybody
 * else (C4, C5), which is exactly why a bank out of paper cannot draw it (C4.b).
 */
export function windowAdvances(
  cb: ParticipantView,
  borrower: ParticipantView,
  on: Civil,
  policyHaircut: number,
): readonly Advance[] {
  const out: Advance[] = [];
  for (const h of borrower.holdings()) {
    const i = borrower.instruments.get(h.instrument);
    if (!eligible(cb, i, on)) continue;
    const print = borrower.print(i.id);
    if (!print.some || print.value.price <= 0) continue;
    const free = borrower.free(h.instrument);
    if (free <= 0) continue;
    const per = mul(print.value.price, sub(1, policyHaircut, 'after the haircut'), 'window value');
    if (per <= 0) continue;
    out.push({
      instrument: i.id,
      free,
      valuePerUnit: per,
      total: mul(free, per, 'what it would raise'),
    });
  }
  return out.sort((a, b) => (a.total === b.total ? (a.instrument < b.instrument ? -1 : 1) : b.total - a.total));
}

/**
 * C4.b: what the BORROWER itself reckons it could raise — its own free eligible paper at its own
 * marks. It is a different number from what any particular lender will advance (that is the
 * lender's own view, above), and it is the one the borrower has when it decides how much to ask
 * for: a bank does not know what somebody else's yield will value its paper at until it asks.
 */
export function pledgeable(borrower: ParticipantView, on: Civil): number {
  const terms: number[] = [];
  for (const h of borrower.holdings()) {
    const i = borrower.instruments.get(h.instrument);
    if (!eligible(borrower, i, on)) continue;
    const mark = borrower.mark(i.id);
    const free = borrower.free(h.instrument);
    if (!mark.some || mark.value <= 0 || free <= 0) continue;
    terms.push(mul(free, mark.value, 'what it could put up'));
  }
  return sum(terms).value;
}

/** What the borrower could raise from this lender against everything it has free (C4.b). */
export function borrowingPower(advances: readonly Advance[]): number {
  return sum(advances.map((a) => a.total)).value;
}
