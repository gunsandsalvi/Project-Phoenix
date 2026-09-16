/**
 * WHAT A PARTY PUBLISHED IT NEEDS, AND WHAT A POOL PUBLISHED IT IS WORTH.
 *
 * @spec Firm E4 Firm E5 Capital Programme B2 Short-Term Debt B1 Fund Shares B3 Fund Shares C4 Housing D3 Law 4 Law 15 Law 19
 *
 * The third file on the shape 0e′.1 settled, after `registry/wages.ts` and `registry/banking.ts`:
 * the kind names live here and the fetch happens here, so a mechanism asks about a PARTY and never
 * about an event.
 *
 * A firm publishes ONE funding gap and four channels fund it — an equity float, a bond, commercial
 * paper, and a control bidder reading whether it is forced — which is exactly the arrangement that
 * once had a five-year bond and three-month paper both brought against one published `short` and
 * the firm raising twice what it needed. `firms.funding` splits the gap where the firm allocates
 * its own money, and the split is what each channel reads: `shortNow` is what falls due within
 * weeks, `shortTerm` is the plant (Capital Programme B2). Four sites unpacking that split by hand
 * is four chances to read the wrong half of it.
 *
 * A pool publishes what a share of its book is worth, and a manager, an insurer, a saver, a desk
 * and a control bidder all price against that one number (Fund Shares B3).
 */
import { asCash, type Cash, asPerPiece, type PerPiece } from '../core/measure.js';
import { currencyCode, type CurrencyCode } from '../core/ids.js';
import { none, type Option, some } from '../core/option.js';
import type { Event } from '../journal/journal.js';
import type { Period } from '../calendar/calendar.js';
import type { Agreement, AgreementTerms } from '../register/agreements.js';
import { sum } from '../core/num.js';
import { asAmount, valueAt } from '../core/measure.js';
import type { PartyId } from '../core/ids.js';

const FUNDING = 'firms.funding';
const STRUCK = 'fund.struck';
const LISTED_STRUCK = 'fund.listedStruck';
const SHORTFALL = 'housing.shortfall';

/* --- What a tenant owes on its tenancy ------------------------------------------------------- */

/**
 * Housing A2, A3, Law 15: what a tenancy says that the kernel has no business understanding — a
 * rent per dwelling and a number of dwellings. The housing module writes these terms; a household
 * reads them off its own commitments to know what falls due before it buys a loaf (E3, 0f.7c), and
 * neither module may import the other, so the shape lives here with the other public reads.
 */
export interface TenancyTerms extends AgreementTerms {
  /** A2: struck at the letting and moving only by a new letting. Money per dwelling per period. */
  readonly rentPerDwelling: number;
  readonly dwellings: number;
}

/** Law 15: the terms narrow back structurally — a tenancy is what names a rent and a count. */
export const isTenancy = (t: AgreementTerms): t is TenancyTerms =>
  'rentPerDwelling' in t && 'dwellings' in t;

/**
 * Households E3, 0f.7c: THE RENT THAT FALLS DUE ON THIS PARTY this period, off the kernel's own
 * book of commitments: every performing tenancy it is the tenant of, at the rent it was struck at.
 * A cell's tenancy is on the whole cell, so this is the TOTAL its members owe between them.
 */
export function rentOwedBy(commitments: readonly Agreement[], who: PartyId): Cash {
  const terms: Cash[] = [];
  for (const a of commitments) {
    if (a.debtor !== who || a.state !== 'performing' || !isTenancy(a.terms)) continue;
    terms.push(
      valueAt(
        asPerPiece(a.terms.rentPerDwelling, 'the rent a dwelling was let at'),
        asAmount<'piece'>(a.terms.dwellings, 'the dwellings it took'),
        'the rent on this tenancy',
      ),
    );
  }
  return sum(terms).value;
}

/** The phase's door: the wire, asked about a named party. */
export interface WireReads {
  ofKind(kind: string): readonly Event[];
  lastOf(kind: string, subject: string): Event | undefined;
}

/** The participant's door. */
export interface PartyReads {
  lastOwn(kind: string): Option<Event>;
  lastOwnSince(kind: string, since: Period): Option<Event>;
  lastPublicAbout(kind: string, subject: string): Option<Event>;
}

/* --- What a firm published it is short of -------------------------------------------------- */

/** Firm E4: the gap a firm published, split where the firm itself allocates its money. */
export interface PublishedFunding {
  /** The whole gap, before the split. */
  readonly short: number;
  /** Short-Term Debt B1: what it cannot pay of what falls due within weeks. */
  readonly shortNow: number;
  /** Capital Programme B2: the part not due for years — the plant, which equity and bonds fund. */
  readonly shortTerm: number;
  readonly ccy: CurrencyCode;
}

function fundingFrom(e: Event | undefined): Option<PublishedFunding> {
  if (e === undefined) return none<PublishedFunding>();
  const { short, shortNow, shortTerm, ccy } = e.data;
  if (typeof short !== 'number' || typeof shortNow !== 'number') return none<PublishedFunding>();
  if (typeof shortTerm !== 'number' || typeof ccy !== 'string') return none<PublishedFunding>();
  return some({ short, shortNow, shortTerm, ccy: currencyCode(ccy) });
}

/**
 * E4, E5: what a firm published it is short of THIS PERIOD. Older is not its position: a gap it
 * published two periods ago has been funded, or has not, and either way it said so again since.
 */
export function fundingPublishedBy(
  reads: WireReads,
  firm: string,
  at: Period,
): Option<PublishedFunding> {
  const said = reads.lastOf(FUNDING, firm);
  return said?.period === at ? fundingFrom(said) : none<PublishedFunding>();
}

/** The same, as the firm itself sees it. */
export function ownFundingSince(reads: PartyReads, since: Period): Option<PublishedFunding> {
  const said = reads.lastOwnSince(FUNDING, since);
  return said.some ? fundingFrom(said.value) : none<PublishedFunding>();
}

/** The same, asked of the firm's own view without a period condition. */
export function ownFundingThisPeriod(reads: PartyReads, at: Period): Option<PublishedFunding> {
  const said = reads.lastOwn(FUNDING);
  return said.some && said.value.period === at ? fundingFrom(said.value) : none<PublishedFunding>();
}

/* --- What a pool published a share of its book is worth ------------------------------------ */

/** Fund Shares B3: what a pool struck, as it published it. */
export interface PublishedStrike {
  readonly perShare: PerPiece;
  /** C4: what it owes redeemers and could not pay — a pool in this position is a forced seller. */
  readonly shortfall: number;
  /** What a saver requires of it, which is a cost of capital to whatever it holds. */
  readonly requires: Option<number>;
  readonly durationYears: Option<number>;
  readonly offered: unknown;
}

function strikeFrom(e: Event | undefined): Option<PublishedStrike> {
  if (e === undefined) return none<PublishedStrike>();
  const perShare = e.data['perShare'];
  if (typeof perShare !== 'number' || perShare <= 0) return none<PublishedStrike>();
  const shortfall = e.data['shortfall'];
  const requires = e.data['requires'];
  const years = e.data['durationYears'];
  // Item 16: what the pool published a share is worth, re-entering as the level it is.
  return some({
    perShare: asPerPiece(perShare, 'what the fund said a share is worth'),
    shortfall: typeof shortfall === 'number' ? shortfall : 0,
    requires: typeof requires === 'number' ? some(requires) : none<number>(),
    durationYears: typeof years === 'number' ? some(years) : none<number>(),
    offered: e.data['offered'],
  });
}

/** Fund Shares B3: what a named pool last struck at. */
export function strikeOf(reads: WireReads, pool: string): Option<PublishedStrike> {
  return strikeFrom(reads.lastOf(STRUCK, pool));
}

/** The same, as the pool itself sees it, while the strike is still this period's. */
export function ownStrikeSince(reads: PartyReads, since: Period): Option<PublishedStrike> {
  const said = reads.lastOwnSince(STRUCK, since);
  return said.some ? strikeFrom(said.value) : none<PublishedStrike>();
}

/** Every strike this world has published — what a saver holding several pools reads. */
export function strikesPublished(reads: WireReads): readonly Event[] {
  return reads.ofKind(STRUCK);
}

/**
 * Fund Shares F1: what a LISTED pool published a share of its book is worth. It is a different event from
 * `fund.struck` because a listed pool's shares also TRADE, and the gap between the two is what an
 * authorised participant deals on — so a desk reads the book's number and the market's print, and
 * never one in place of the other (Appendix B: no index that inputs to its constituents).
 */
export function listedStrikeOf(reads: PartyReads, pool: string): Option<ListedStrike> {
  const said = reads.lastPublicAbout(LISTED_STRUCK, pool);
  if (!said.some) return none<ListedStrike>();
  const perShare = said.value.data['perShare'];
  if (typeof perShare !== 'number' || perShare <= 0) return none<ListedStrike>();
  // Law 8, Law 16: it is a PRICE over a price, and it is named for that.
  return some({
    perShare: asPerPiece(perShare, 'what one share of the book is worth'),
    basket: said.value.data['basket'],
  });
}

/** What a listed pool struck, and the basket a creation unit is delivered in. */
export interface ListedStrike {
  readonly perShare: PerPiece;
  readonly basket: unknown;
}

/* --- What a household published it cannot house -------------------------------------------- */

/** Housing D3: a cell's own published shortfall — how many dwellings short it is, and of which line. */
export interface PublishedShortfall {
  readonly short: number;
  readonly dwelling: string;
}

/** D3: what this cell published it is short of THIS PERIOD, and nothing from an older one. */
export function shortfallOf(
  reads: PartyReads,
  who: string,
  at: Period,
): Option<PublishedShortfall> {
  const said = reads.lastPublicAbout(SHORTFALL, who);
  if (!said.some || said.value.period !== at) return none<PublishedShortfall>();
  const short = said.value.data['short'];
  const dwelling = said.value.data['dwelling'];
  // Item 16: two published facts re-enter here — a count of dwellings, and the line they are of.
  if (typeof short !== 'number' || short <= 0 || typeof dwelling !== 'string') {
    return none<PublishedShortfall>();
  }
  return some({ short, dwelling });
}

/** A gap that is not positive is not a gap: a party with money to spare raises nothing (E4). */
export function isShort(amount: number, what: string): Option<Cash> {
  return amount > 0 ? some(asCash(amount, what)) : none<Cash>();
}

/* --- What a lettings venue printed ---------------------------------------------------------- */

const RENT_PRINT = 'housing.rent';

/**
 * Housing A2, Households F1.b (12.2): THE RENT A REGION'S LETTINGS VENUE LAST STRUCK, per dwelling
 * per period. The housing module writes it; a household deciding whether its people can afford a
 * roof of their own reads it here, by the registry's name (0e′.3). A region whose venue has never
 * cleared has no rent, which is a refusal and not a zero.
 */
export function rentPrintedIn(reads: WireReads, region: string): Option<PerPiece> {
  let last: PerPiece | undefined;
  for (const e of reads.ofKind(RENT_PRINT)) {
    if (e.data['region'] !== region || e.data['outcome'] !== 'cleared') continue;
    const rent = e.data['rentPerDwelling'];
    if (typeof rent === 'number' && rent > 0) last = asPerPiece(rent, 'what a dwelling let for, per period');
  }
  return last === undefined ? none<PerPiece>() : some(last);
}

/* --- What a pool called of an investor ------------------------------------------------------- */

const CALLED = 'fund.called';

/** §29 A2 (14.7): one call a pool made of an investor — what it asked for, and whether it was paid. */
export interface PublishedCall {
  readonly fund: string;
  readonly called: Cash;
  readonly paid: boolean;
}

/**
 * §29 A2.a, Law 19 (14.7): THE CALLS MADE OF THIS INVESTOR IN ONE PERIOD, off the public record —
 * the funds module writes them; the investor's buffer and its response read them here, by the
 * registry's name (0e′.3), with the period in the ask so a stale call is never read as current.
 */
export function callsOn(
  reads: { ofKindIn(kind: string, period: Period): readonly Event[] },
  investor: PartyId,
  period: Period,
): readonly PublishedCall[] {
  const out: PublishedCall[] = [];
  for (const e of reads.ofKindIn(CALLED, period)) {
    if (e.data['investor'] !== investor) continue;
    const called = e.data['called'];
    if (typeof called !== 'number') continue;
    out.push({ fund: String(e.data['fund']), called: asCash(called, 'what it was called for'), paid: e.data['paid'] === true });
  }
  return out;
}
