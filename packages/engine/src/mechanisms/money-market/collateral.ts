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
import { sumCash } from '../../core/measure.js';
import {
  asRatio,
  type Cash,
  minus,
  type PerPiece,
  type Ratio,
  ratioOf,
  scale,
  valueAt,
} from '../../core/measure.js';
import type { Qty } from '../../core/tick.js';
import type { Civil } from '../../calendar/civil.js';
import type { DayCount } from '../../calendar/daycount.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { none, some, type Option } from '../../core/option.js';
import { priceAt } from '../../prices/curve.js';
import type { Instrument } from '../../register/instruments.js';
import type { ParticipantView } from '../../world/context.js';
import { expectedLossOn, requiredOf as requiredYield } from '../../registry/banking.js';
import { requiredOnClaim } from '../../registry/secured.js';
import { nextPeriod } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';

/** One day count for what a lender advances, stated once by the module that asks the question. */
export const COLLATERAL_DAY_COUNT: DayCount = 'ACT/ACT';

/**
 * B3.a: what a market will take as security — a claim on a name that a market prices.
 *
 * Three things, and each is asked of what the paper IS rather than of a list: somebody owes it (so
 * there is a name to come after), a market clears it (so it can be SOLD if the borrower does not
 * come back for it), and it is alive. A share is not a claim, a loan has no market, a tonne of grain
 * is neither, and no list anywhere names them.
 *
 * §42 D3 (17.7d): A SCHEDULE IS NOT ONE OF THE THREE. It used to be — nothing was eligible unless
 * its own terms said what it would pay and when — and that refused the senior tranche of a
 * securitisation, which is the paper D3 says *"is used as collateral, so its liquidity matters to
 * the funding system"*. A pass-through pays what the pool pays when the pool pays it (XI-11), so it
 * promises no schedule and never will; what makes it good collateral is not a promise but a market,
 * and `valueToLender` prices the two cases apart.
 */
export function eligible(view: ParticipantView, i: Instrument, on: Civil): boolean {
  if (!i.status.live || !i.issuer.some) return false;
  const profile = view.registry.instrumentKind(i.kind);
  if (!profile.liabilityOfIssuer || profile.pricing !== 'cleared') return false;
  if (profile.cashFlows(i, on, view.calendar, view.registry).length > 0) return true;
  // C1.a: no schedule, so what it converts into is what somebody last paid for it. A line nobody
  // has ever printed converts into nothing anybody can name, and is not taken.
  return view.print(i.id).some;
}

/**
 * Corporate Credit E5, XI-4: what THIS lender requires, per annum, of a named issuer's paper. It is
 * read from what the lender itself published (Law 4: the module that owns a bank's economics owns
 * the number), never rebuilt here — a haircut struck against a second copy of a bank's credit view
 * would be the bank securing its lending against a belief it does not hold.
 */
export function requiredOf(view: ParticipantView, issuer: PartyId): Option<number> {
  const said = requiredYield(view, String(issuer));
  return said.some ? some(said.value as number) : none<number>();
}

/**
 * B3.b: what one unit of this paper is worth to this lender, and there are two answers because there
 * are two kinds of paper.
 *
 * A CLAIM THAT PROMISES A SCHEDULE is worth what that schedule is worth at what this lender requires
 * OF THAT CLAIM — its own reservation for the name, less the part of it whatever is pledged behind
 * the paper takes away (17.7c). So a lender that requires more advances less, a long bond is
 * discounted further than a short one at the same yield gap, and a secured line raises more than an
 * unsecured one from the same issuer. None of that is stated anywhere.
 *
 * A CLAIM THAT PROMISES NOTHING (§42 D3, 17.7d) cannot be discounted, and a schedule invented for it
 * would be a forecast with no falsification test (Law 17) — which is exactly why the tranche kind
 * refuses to have one. What a lender will advance against it is what it could REALISE: the market's
 * own last print, less what this lender expects to lose on the name over the time it would be left
 * holding it, which is the term of the loan it is making. Both numbers are published — the print by
 * the market, the expected loss by the lender under its own name — and neither is a haircut anybody
 * wrote down (B3.b).
 *
 * A lender that has published no view of the name will not take its paper either way: no view, no
 * advance.
 */
export function valueToLender(view: ParticipantView, i: Instrument, on: Civil): Option<PerPiece> {
  if (!i.issuer.some) return none<PerPiece>();
  const flows = view.registry.instrumentKind(i.kind).cashFlows(i, on, view.calendar, view.registry);
  if (flows.length > 0) {
    const required = requiredOnClaim(view, i.id);
    if (!required.some) return none<PerPiece>();
    return some(
      priceAt(
        flows,
        asRatio(required.value, 'what this lender requires of this claim'),
        on,
        COLLATERAL_DAY_COUNT,
        `advance against ${i.id}`,
      ),
    );
  }
  const print = view.print(i.id);
  if (!print.some || print.value.price <= 0) return none<PerPiece>();
  const expected = expectedLossOn(view, String(i.issuer.value));
  if (!expected.some) return none<PerPiece>();
  // C4: the loan is struck for a period and re-struck, so the time it is exposed for is a period of
  // a year, read off the calendar's own dates like every other span (Law 8).
  const term = asRatio(
    yearFraction(COLLATERAL_DAY_COUNT, on, view.calendar.startOf(nextPeriod(view.period))),
    'the term it would be left holding it',
  );
  return some(whatItCouldRealise(print.value.price, expected.value, term));
}

/**
 * B3.b, Banks Funding C1.a (17.7d): WHAT A CLAIM WITH NOTHING TO DISCOUNT IS WORTH TO A LENDER.
 *
 * C1.a says liquid assets differ in how fast and how surely they CONVERT, and for paper that
 * promises no schedule that is the only question there is: what it converts into is what the market
 * last paid for it, and what the lender stands to lose in the meantime is what it expects to lose on
 * the name over the term of the loan it is making. Both are somebody's published number.
 *
 * It is named and separate because it is the one piece of arithmetic in the file: everything around
 * it is a read, and this is the sentence those reads are put together into.
 */
export function whatItCouldRealise(print: PerPiece, expectedLoss: Ratio, term: Ratio): PerPiece {
  return scale(
    print,
    minus(
      asRatio(1, 'what the market last paid'),
      scale(expectedLoss, term, 'what it expects to lose over the term'),
      'what it will advance of it',
    ),
    'what it could realise, less what it stands to lose meanwhile',
  );
}

/**
 * B3.b: the haircut, as the READ it is — how far below the market's price this lender's own
 * valuation sits. Nothing prices off it; it is what a reader (and Part XII) measures. None when the
 * market has never printed the paper, because there is nothing to take a haircut from.
 */
export function haircut(view: ParticipantView, i: Instrument, on: Civil): Option<Ratio> {
  const value = valueToLender(view, i, on);
  const print = view.print(i.id);
  if (!value.some || !print.some || print.value.price <= 0) return none<Ratio>();
  return some(
    ratioOf(
      minus(print.value.price, value.value, 'below the market'),
      print.value.price,
      'haircut',
    ),
  );
}

/** A parcel of paper a borrower could put up, and what this lender would advance against it. */
export interface Advance {
  readonly instrument: InstrumentId;
  /** Units free to bind: what is held less what already stands behind something else (B3.c). */
  readonly free: Qty;
  readonly valuePerUnit: PerPiece;
  readonly total: Cash;
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
      total: valueAt(value.value, free, i.ccy, 'what this parcel would raise'),
    });
  }
  return out.sort((a, b) =>
    a.total.pieces === b.total.pieces
      ? a.instrument < b.instrument
        ? -1
        : 1
      : b.total.pieces - a.total.pieces,
  );
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
  policyHaircut: Ratio,
): readonly Advance[] {
  // Currency B4: a central bank lends ITS money against paper in its money; a foreign line is not
  // collateral at this window, however good — the bank buys the money it is short of instead.
  const lends = cb.registry.currencyOf(cb.self.region);
  const out: Advance[] = [];
  for (const h of borrower.holdings()) {
    const i = borrower.instruments.get(h.instrument);
    if (i.ccy !== lends || !eligible(cb, i, on)) continue;
    const print = borrower.print(i.id);
    if (!print.some || print.value.price <= 0) continue;
    const free = borrower.free(h.instrument);
    if (free <= 0) continue;
    const per = scale(
      print.value.price,
      minus(asRatio(1, 'the whole of it'), policyHaircut, 'after the haircut'),
      'window value',
    );
    if (per <= 0) continue;
    out.push({
      instrument: i.id,
      free,
      valuePerUnit: per,
      total: valueAt(per, free, i.ccy, 'what it would raise'),
    });
  }
  return out.sort((a, b) =>
    a.total.pieces === b.total.pieces
      ? a.instrument < b.instrument
        ? -1
        : 1
      : b.total.pieces - a.total.pieces,
  );
}

/** What the borrower could raise from this lender against everything it has free (C4.b). */
export function borrowingPower(advances: readonly Advance[], ccy: CurrencyCode): Cash {
  return sumCash(
    ccy,
    advances.map((a) => a.total),
    'what its parcels would raise',
  ).value;
}
