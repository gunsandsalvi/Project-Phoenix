/**
 * The flotation: a private company goes public because equity is the cheaper money.
 *
 * @spec Equity A6 Equity B1 Equity B2 Equity B3 Equity D1 Equity D1.b Equity D1.c Equity E3 Firm E4 Firm E5 Reporting A2 Clearing B2 Clearing C4.a Private Equity D1 Private Equity D2 Law 3 Law 19
 *
 * WHY IT IS A DECISION AND NOT A SIZE. *"Going public is a matter of funding choice, not how large
 * a firm is"* — and D1.b already said it: an issuance is *"a decision with a reason: a funding need
 * it prefers to meet with equity"*. What stood in this world's way was a threshold in the seed that
 * listed a firm because it was big (10f.1), and what replaces it is the comparison a firm actually
 * makes.
 *
 * WHAT IT COMPARES, and both sides are numbers somebody else published (Law 19, Law 3):
 *
 *  - **what debt costs it** is `credit.quoted` — the keenest quote it was given, which is already
 *    the outcome of banks competing for it (Banks Lending C3.a), published with the SIZE that bank
 *    will actually lend;
 *  - **what its own money earns** is its published accounts (Reporting A2): what it made over the
 *    book it made it on, annualised by the span of its own report. There is no second set of
 *    accounts and no forecast.
 *
 * And the comparison is the one a management really makes. **Borrow at more than the business earns
 * and the owners are worse off**; sell part of it and the new owners share what there is. So a firm
 * whose book earns twenty per cent borrows at eight and keeps the difference — it stays private
 * however large it is — and one whose book earns four floats, however small. That is the correction
 * stated as a mechanism: nothing here reads a size.
 *
 * THE OTHER TWO REASONS ARE REFUSALS, which is the honest shape of a funding constraint: a firm
 * NOBODY WILL LEND TO has no alternative at any price, and one whose bank will not lend it ENOUGH
 * has none at that size (Corporate Credit A1). Neither is a preference; both are what the credit
 * market decided about it.
 *
 * WHAT IT DOES NOT DO IS PRICE ITS OWN ISSUE (Law 3, B3). It brings a SIZE and the least it will
 * take — its own book per share, which is its reservation and not a price — and the session strikes
 * the level out of what the bidders posted (B1, B2). **That first print is the first price the line
 * has ever had**, which is also §29 D2 word for word: an exit produces a cleared price, and a
 * flotation and a private-equity exit by flotation are one mechanism.
 *
 * It can fail (D1.c, Clearing C4.a). A book with no bidder in it prints nothing, the firm has sold
 * nothing, and it is a public company whose line has never traded — which is a real state and the
 * one a failed IPO leaves behind.
 */
import { amountOf, asCash, asRatio, over, pricedAt, type Cash, type PerPiece, type Ratio } from '../../core/measure.js';
import { asQty, upTick, type Qty } from '../../core/tick.js';
import { currencyCode, type CurrencyCode, type InstrumentId, type PartyId } from '../../core/ids.js';
import { none, some, type Option } from '../../core/option.js';
import { period as periodOf } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import { displayName } from '../../registry/naming.js';
import { material } from '../../core/num.js';
import type { MechanismContext } from '../../world/context.js';
import { equityLineOf, equityMarketOf, type EquityDecl } from './data.js';

/** What a firm published it is short of for the thing it wants to build, and the money it is in. */
interface Need {
  readonly short: Cash;
  readonly ccy: CurrencyCode;
}

/**
 * Firm E4, Capital Programme B2: the part of its gap that is not due for weeks — the plant, which
 * is what a firm raises equity for. It is the same field the bond channel reads, because it is the
 * same hole and a firm with one hole must not raise twice to fill it (Law 4).
 */
function shortOf(ctx: MechanismContext, firm: PartyId): Option<Need> {
  const said = ctx.journal.lastOf('firms.funding', String(firm));
  if (said?.period !== ctx.period) return none<Need>();
  const short = said.data['shortTerm'];
  const ccy = said.data['ccy'];
  // E4: a negative short is what it has OVER, and a firm with money to spare raises nothing.
  if (typeof short !== 'number' || short <= 0 || typeof ccy !== 'string') return none<Need>();
  return some({ short: asCash(short, 'what it is short of'), ccy: currencyCode(ccy) });
}

/** Banks Lending C3.a: the keenest quote this firm was given, and how much that bank will lend. */
function quotedTo(
  ctx: MechanismContext,
  firm: PartyId,
): Option<{ readonly rate: Ratio; readonly most: Cash }> {
  const said = ctx.journal.lastOf('credit.quoted', String(firm));
  if (said?.period !== ctx.period) return none();
  const rate = said.data['rate'];
  const most = said.data['most'];
  if (typeof rate !== 'number' || typeof most !== 'number') return none();
  return some({
    rate: asRatio(rate, 'what its bank quoted it'),
    most: asCash(most, 'what that bank will lend it'),
  });
}

/**
 * Reporting A2, Equity D1.b: WHAT ITS OWN MONEY EARNS, per annum, as it published it — and its book
 * per share with it, because the two come out of one statement and reading them from two places
 * would be two sets of accounts (Law 4).
 *
 * A company that has published nothing answers NOTHING, and that is the answer rather than a zero
 * (Appendix A): nobody could value it, so there would be no book to bring an offer to.
 */
function published(
  ctx: MechanismContext,
  firm: PartyId,
  shares: Qty,
): Option<{ readonly earns: Ratio; readonly bookPerShare: PerPiece }> {
  const said = ctx.published.lastStatement(firm);
  if (said === undefined || said.earned <= 0 || said.periods <= 0 || shares <= 0) return none();
  const book = said.assets - said.liabilities;
  if (book <= 0) return none();
  // Law 8: the periodicity is part of the number. What it published covers a span of PERIODS and a
  // rate is quoted per YEAR, so the calendar puts the two in one unit — never a factor typed here.
  const ofAYear = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(ctx.period),
    ctx.calendar.startOf(periodOf(ctx.period + said.periods)),
  );
  if (ofAYear <= 0) return none();
  const annual = over(
    said.earned,
    asRatio(ofAYear, 'the fraction of a year that was'),
    'what it earns a year, as it published it',
  );
  return some({
    earns: asRatio(annual / book, 'what a unit of its own money earns it in a year'),
    bookPerShare: pricedAt(
      asCash(book, 'what it published its book comes to'),
      shares,
      'what its books say a share is a claim on',
    ),
  });
}

/** What this firm decided about coming to market, and why — recorded whether it comes or not. */
export interface Flotation {
  readonly firm: PartyId;
  readonly line: InstrumentId;
  readonly ccy: CurrencyCode;
  /** D1: the shares it brings, and the least it will take for one (B3: its own reservation). */
  readonly size: Qty;
  readonly reservation: PerPiece;
  readonly raising: Cash;
  readonly earns: Ratio;
  readonly why: string;
}

/**
 * D1.b, E3: the decision, for ONE firm, out of what it and its bank published this period.
 *
 * Exported because the decision is the item and a test asks it directly: given a firm whose book
 * earns less than its bank charges, it comes; given one that earns more, it does not.
 */
export function wouldFloat(
  ctx: MechanismContext,
  firm: PartyId,
  line: InstrumentId,
  shares: Qty,
): Option<Flotation> {
  const need = shortOf(ctx, firm);
  if (!need.some) return none<Flotation>();
  const { short, ccy } = need.value;
  const said = published(ctx, firm, shares);
  // B1, B3: nobody can put a number on a company that has published nothing, so there would be no
  // book to bring an offer to. A refusal, and not a guess at what it might be worth (Appendix A).
  if (!said.some) return none<Flotation>();
  const { earns, bookPerShare } = said.value;
  const quoted = quotedTo(ctx, firm);
  /**
   * THE THREE REASONS, and only the first is a preference. The other two are what the credit market
   * decided about this firm, which is Corporate Credit A1's *"a firm large enough to reach a market
   * is not at the mercy of one lender"* seen from the borrower's side.
   */
  const dearer = quoted.some && quoted.value.rate > earns;
  const nobody = !quoted.some;
  const notEnough = quoted.some && quoted.value.most < short;
  if (!dearer && !nobody && !notEnough) return none<Flotation>();
  // Law 8: a share is indivisible, so it brings a whole number of them — UP, because the ask is the
  // MONEY and an offer a fraction of a share short of what it needs is short of it.
  const size = upTick(amountOf(short, bookPerShare, 'shares it must sell to raise it'));
  if (size <= 0 || !material(size, 2, size)) return none<Flotation>();
  return some({
    firm,
    line,
    ccy,
    size: asQty(size, 'the shares it brings to its first session'),
    reservation: bookPerShare,
    raising: short,
    earns,
    why: nobody
      ? 'nobody has quoted it at any price, so equity is the only money there is (Corporate Credit A1)'
      : notEnough
        ? 'its bank will not lend it enough at any price, so the rest has to come from its owners'
        : 'borrowing costs more than its own book earns, so debt would take more from its owners than it brings them (D1.b)',
  });
}

/**
 * E3, D1, D1.c, §29 D2: IT LISTS AND IT OFFERS, in one event.
 *
 * The market is seated on the line and opened through the kernel's one door (`ctx.list`), because a
 * line pointing at a book that is not there is the failure a second door would allow. Then the
 * shares go in as the ISSUER's own supply, cleared by the same solver at one level with everybody
 * else's orders in the book — and withdrawn if nobody will pay the least it will take, which is a
 * failed issue and has consequences (Clearing C4.a).
 */
export function float(ctx: MechanismContext, f: Flotation): void {
  const market = equityMarketOf(String(f.firm));
  ctx.list(f.line, {
    id: market,
    name: displayName(ctx.instruments.get(f.line), ctx.parties, ctx.registry),
    instrument: f.line,
    ccy: f.ccy,
    rationing: 'proRata',
  });
  ctx.offer({
    market,
    issuer: f.firm,
    size: f.size,
    reservation: some(f.reservation),
    allotment: 'uniformPrice',
  });
  ctx.record(
    'equity.float',
    [f.firm, f.line],
    {
      line: f.line,
      market: String(market),
      size: f.size,
      reservation: f.reservation,
      raising: f.raising,
      earns: f.earns,
      why: f.why,
    },
    true,
  );
}

/**
 * E3, D1.b: every private company in this world, asked once, in front of the session.
 *
 * It reads the LINE and not the seed's row (Law 19): `EquityDecl.listed` is an opening condition and
 * goes stale the moment a firm floats, and whether a line trades is a fact about the line.
 */
export function floatations(ctx: MechanismContext, rows: readonly EquityDecl[]): void {
  for (const row of rows) {
    const line = ctx.instruments.get(equityLineOf(row.firm));
    if (!line.status.live || line.market.some) continue;
    const firm = row.firm as PartyId;
    if (!ctx.parties.get(firm).status.alive) continue;
    const decided = wouldFloat(ctx, firm, line.id, line.issued);
    if (decided.some) float(ctx, decided.value);
  }
}
