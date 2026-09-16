/**
 * THE MANDATE: what a pool may hold, how you get in and out, what its manager charges for running
 * it, and whether it is being wound up. The object this whole sector is built on.
 *
 * @spec Fund Shares A4 Fund Shares B3 Fund Shares F3 Fund Shares G1 Fund Shares G1.a Fund Shares G1.b Hedge Funds A3 Hedge Funds A4 Hedge Funds B1 Indices C2 XI-8 Law 2 Law 4 Law 15
 *
 * Item 10e.4: it was inside `index.ts`, which was fine while the module's only reader was the
 * module's own phases. The MANAGER reads it too — its book of business is the mandates it holds,
 * and its decisions are about their terms — and a file that both `index.ts` and `manager.ts` import
 * values from cannot be either of them. So the object lives on its own, and the two files that act
 * on it both read one definition of it (Law 4).
 */
import { agreementKindId, type AgreementId, type CurrencyCode, type PartyId } from '../../core/ids.js';
import type { Period } from '../../calendar/calendar.js';
import { InvalidRegistry } from '../../core/errors.js';
import { none, type Option, some } from '../../core/option.js';
import { asRatio, type Ratio } from '../../core/measure.js';
import type { Agreement, AgreementDecl, AgreementTerms } from '../../register/agreements.js';
import type { Blueprint } from '../../registry/blueprint.js';
import type { FundDecl } from './data.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';

/**
 * Fund Shares A4, F3, XI-8 (item 9.2): A MANDATE IS AN AGREEMENT BETWEEN A POOL AND A MANAGER.
 *
 * It is what splits a fund into the three things it actually is: a POOL that holds and has no
 * opinions, a MANDATE that rules, and a MANAGER that decides. The pool owes the manager its fee and
 * the manager owes the pool its judgement, which is two named parties with dated terms and a state
 * — an agreement, and the eighth kind of one (item 9.1).
 *
 * It was a `FundDecl` row, which is to say a fact about the WORLD'S DATA rather than about these
 * two parties, and nothing outside this module could read it. A separate account is a mandate whose
 * pool is the client's own balance sheet; an ETF and a money fund are pools with different
 * redemption rules; and a HEDGE FUND IS A MANDATE WITH LEVERAGE, which is why `leverage` is here
 * and not on `fundKind` — where it was hard-coded `false` for every pool in every world (item 13).
 */
export const MANDATE = agreementKindId('funds.mandate');

/**
 * §13 G1, G1.a, item 10e: HOW YOU GET IN AND OUT, and it is what decides who can be FORCED TO SELL.
 *
 * G1 already generalises the mechanism — *"a share count, a redemption request, a sale in the same
 * period's books, and the cost of a late sale landing on the holders who stayed"* — so there is ONE
 * subscription and redemption path and these terms say what it does. That is the whole difference
 * between a money fund and a private equity vehicle, and it is a TERM rather than a kind of thing.
 *
 * It is also this sector's contribution to whether a shock travels (XI-2): a redemption reaches a
 * LIQUID fund and becomes a sale into whatever the market gives; it reaches a CLOSED one and stops,
 * because nobody can ask for money that was committed for the life of the fund.
 */
export type Liquidity =
  /** In and out at NAV in any period. The money fund, and what XI-2 door 2 actually runs on. */
  | { readonly how: 'liquid' }
  /**
   * Out at NAV, but only when a window opens, and what does not fit is QUEUED to the next one.
   * C4.a is what the queue costs: the holders who stayed pay for the late sale.
   */
  | { readonly how: 'semiLiquid'; readonly everyPeriods: number }
  /**
   * G1.b: committed for the life of the fund, so there is no redemption at all and it can never be
   * a forced seller — which is the entire reason the structure exists, and what §29 is built on.
   *
   * §29 A4 (17b.9): AND THE LIFE IS A TERM OF IT. *"The fund has a life: it invests, it holds, it
   * exits, and it winds up — and on winding up the investors' claims resolve into cash rather than
   * freezing."* A vintage is a fund raised to run for so long and no longer, and the end of it is
   * not a decision anybody takes when the time comes: it was agreed before the first call. It sits
   * inside this variant rather than beside it because only a closed-end vehicle has one, and a term
   * that is absent for three kinds out of four is not a term of a mandate (Appendix A).
   */
  | { readonly how: 'closed'; readonly lifePeriods: number }
  /**
   * E1, G1.a: you do not subscribe — you buy the share from a HOLDER, in a market, at a cleared
   * price. Creation and redemption are IN KIND against the basket, which is why an exchange-traded
   * fund is not a forced seller and why *"some other vehicle must carry it"*.
   */
  | { readonly how: 'listed' };

/**
 * A3, A4, §28 A4 (item 13.2): WHAT A POOL MAY TAKE A POSITION IN — a list of contract kinds, or the
 * word for a mandate that does not restrict it.
 *
 * IT IS A PERMISSION AND NOT A BAND, which is why `[]` means NOTHING here while an empty band in a
 * blueprint means the blueprint says nothing about that dimension. The two conventions look
 * opposite and are not: a blueprint band CONSTRAINS a universe of things that already exist ("this
 * pool may hold assets of these classes"), so silence is no constraint; this GRANTS an ability
 * ("this pool may write contracts"), so silence is no grant. A money fund does not write
 * derivatives, and `[]` saying so out loud is what lets the derivative layer speak for a pool at
 * all (`B-14`, item 9.7).
 *
 * `'anything'` is §28's WIDE MANDATE — long, short, levered, many markets — and it is a word rather
 * than a list of every class this world happens to have, because a list would be the enumeration of
 * the world that item 10e deleted from `mayHold`: it goes stale the day somebody writes a tenth
 * class, and a mandate its investors agreed was unrestricted would silently stop being one.
 */
export type MayWrite = 'anything' | readonly string[];

/** A3, `B-14`: whether this mandate lets its pool take a position in a contract of this kind. */
export const mayWrite = (m: MayWrite, kind: string): boolean =>
  m === 'anything' || m.includes(kind);

export interface MandateTerms extends AgreementTerms {
  readonly kind: typeof MANDATE;
  /**
   * A4, item 10e: WHAT THIS POOL MAY HOLD, in the one language every vehicle in this world is
   * described by (`registry/blueprint.ts`).
   *
   * It was `mayHold: readonly string[]` — a list of instrument kind ids — and that was three things
   * wrong at once. It could not express *"credit, three to seven years, senior"* at all, so almost
   * every real mandate was inexpressible; it described a vehicle by ENUMERATING THE WORLD, so it
   * went stale the first time anybody issued a kind it was written before; and it was a kind branch
   * wearing a registry's clothes (Law 15). A blueprint bands over READS instead, so a bond ages out
   * of a duration band on its own and a company falls out of a size band by falling.
   */
  readonly blueprint: Blueprint;
  /** G1: how its investors get in and out, which is what decides whether it can be forced to sell. */
  readonly liquidity: Liquidity;
  /**
   * Indices C2, C2.a, item 10e: WHETHER IT CHOOSES, and this is the other half of what a mandate is.
   *
   * The blueprint says what it MAY hold. This says whether it picks within that on its own view —
   * ACTIVE — or holds whatever an index says is in it at whatever the index weighs it — PASSIVE.
   * They are different businesses and the difference is a term the investors agreed to, not a
   * property of a vehicle type: an exchange-traded fund is usually passive but need not be, an
   * index mutual fund is passive and not listed, and a segregated institutional mandate can be
   * either. That is why this is here and not on a declaration row for one kind of vehicle.
   *
   * A TRACKER IS NOT AN INVESTOR, which is the whole reason the distinction earns its place: a
   * rebalance is the index answering differently — a line listed, a line gone, a weight moved — and
   * the fund then HAS to trade, in the same session, at whatever the book gives it (C2.a). It is
   * not choosing, and that is what makes it a transmission channel rather than a buyer.
   *
   * The id is a plain string because a fund may not import the module that declares the index
   * (`no-cross-module-import`); which index a tracker tracks is data about this world.
   */
  readonly tracks: Option<string>;
  /**
   * A3, A4, `B-14` (item 9.7): THE CONTRACT KINDS IT MAY TAKE A POSITION IN, and an empty list is a
   * real term and not an absence — a money fund does not write derivatives, and saying so is what
   * lets the derivative layer speak for a pool at all.
   *
   * It is a separate list from the blueprint because a contract is not an instrument: nobody
   * issued it, nobody holds units of it, and it is on both sides' books at once (Derivative D1). A
   * WIDE mandate — long, short, levered, many markets — is §28's hedge fund, and this is the term
   * that makes it one.
   */
  readonly mayWrite: MayWrite;
  /**
   * B1, F2, XI-3: whether this pool may be levered — and `false` is a real term of a mandate and
   * not an absence. A levered pool borrows from a NAMED lender, which is what makes its leverage a
   * fact about a loan rather than a property of the pool (item 13's B1.a).
   */
  readonly leverage: boolean;
  /**
   * B3, F3, item 10e.4: WHAT THE MANAGER CHARGES, per annum on net assets, STRUCK WHEN THE MANDATE
   * WAS AGREED — and this is where a fee belongs, because a fee is what these two parties agreed
   * and not a number the world declared about a fund.
   *
   * IT WAS A PARAMETER AND IT WAS A PLACEHOLDER, whose own reason said so: *"nothing in this world
   * produces it: no manager competes for the mandate, so the number stands where a competition
   * should be... the missing mechanism is a manager with a cost base"*. That mechanism is
   * `manager.ts`, and this is the number it produces. A pool opened by the SEED carries what the
   * draw gave it — an opening condition (Seed A3), the way a bank opens with a balance sheet — and
   * a pool a manager LAUNCHES carries what that manager decided to charge, which is what it takes
   * to undercut whoever is already running the product.
   *
   * A parameter could not have been either: parameters are declared at assembly, so a fund that did
   * not exist when the world was built could never have had one, and the roster of funds could
   * never have been an outcome.
   */
  readonly feePerAnnum: Ratio;
  /**
   * C2.a: the share of its net assets the pool keeps in cash, so an ordinary redemption needs no
   * sale. Its own caution, and the whole of what decides which fund becomes a forced seller first.
   */
  readonly buffer: Ratio;
  /**
   * D2, D2.a: what its investors require of it over what a deposit pays them, per annum — which is
   * what it will pay for paper. What a saver demands of the fund is what the fund demands of what
   * it holds, and that is a term of the product rather than a fact about the world.
   */
  readonly requiredYieldPerAnnum: Ratio;
  /**
   * F3, G1, item 10e.4: THE MANAGER HAS GIVEN NOTICE, and the pool is being wound up.
   *
   * A wind-down is not a fund failing and it is not a mandate that has already ended: a pool with
   * no mandate has nobody deciding for it, and the assets and the holders would both still be
   * there. So it is a STATE the mandate is in — the manager still runs it, still charges for
   * running it, and what has changed is that it takes nobody new and every holder is redeemed at
   * the NAV whether they asked or not.
   *
   * Everything after that is the machinery that was already here: the redemptions queue (C2.b), the
   * fund sells what it must to pay them (C2.a), what it cannot pay stays queued, and when the last
   * share is gone and the book is empty the mandate ends and the pool ceases (Register F2). A
   * wind-down is therefore a FORCED SALE OF A WHOLE BOOK into whatever the market gives, which is
   * the channel this sector adds to XI-2 and the reason competition between managers is not a
   * bookkeeping detail.
   */
  readonly windingUp: boolean;
  /**
   * §13 A1, item 10e.6: IS THIS OFFERED TO THE PUBLIC, or only to an entrant that clears the line?
   *
   * The owner's ladder — *"retail able to access ETF and MMF, rich retail able to access funds,
   * institutional being also able to do mandates"* — is not three kinds of vehicle and not three
   * kinds of investor. It is ONE question a vehicle asks at its door and one answer an entrant
   * gives, and what separates the two rungs this world has is a POLICY: the accredited-investor
   * threshold (`FUND_PARAMS.accreditedWealth`), a number a regulator sets and changes, owned by
   * `parliament` (Law 2) — which is also item 19's first real channel into this sector.
   *
   * It is a BOOLEAN and not a tier, because a tier would be a taxonomy of vehicles and this item
   * exists to delete those. What the vehicle states is whether it is publicly offered; what the LINE
   * is, is nobody's business but the regulator's, and it can move under a fund that has already
   * been sold.
   *
   * Access says WHERE a cell's money may go and never HOW MUCH: what actually goes in stays a
   * consequence of the cell's own budget and of what it requires of anything it holds (D5). And it
   * never bars the way OUT — a holder that stops clearing the line still owns what it bought.
   */
  readonly offeredPublicly: boolean;
  /**
   * §28 A3 (item 13.2): THE SECOND FEE, and the asymmetric one — the share of a GAIN the manager
   * takes, over the highest value a share of this pool has ever been worth at a charge.
   *
   * *"The asymmetry of that second fee is a reason for risk-taking"*, and the asymmetry is not a
   * rule written anywhere: it falls out of the high-water mark. A gain is shared; a loss is not,
   * and it is not refunded either — so the manager earns nothing at all until the pool is back
   * above where it last charged, and a manager that has just lost money has a reason to take more
   * risk rather than less. That is the mechanism the clause is about.
   *
   * Zero for every long-only pool this world opens with, which is a real term and not an absence.
   */
  readonly performanceFee: Ratio;
  /**
   * Hedge Funds B1, B5, Prime Brokerage B3 (item 13.3): HOW LEVERED THIS POOL MEANS TO BE — its
   * securities book as a multiple of what its investors have in it.
   *
   * It is a PREFERENCE (Law 2) and it is the CLIENT's half of B1's division: *"leverage is a fact
   * about a loan, never a property of the fund"* — so the pool states what it wants and a named
   * lender decides what it gets, and the two are different numbers on purpose. A broker's line can
   * be well under this, and then what the pool runs at is the LENDER's decision, which is B3.
   *
   * `1` for a pool whose mandate does not permit leverage at all, which is a real term and not an
   * absence: unlevered is exactly a target of one.
   */
  readonly targetLeverage: Ratio;
}

/**
 * Law 15: the module that declared the kind narrows a row back to it, structurally — what makes
 * these terms a mandate is that they say what the pool may hold and whether it may be levered.
 */
export const isMandate = (t: AgreementTerms): t is MandateTerms =>
  'blueprint' in t && 'mayWrite' in t && 'leverage' in t && 'feePerAnnum' in t;

/** One mandate as this module reads it: the pool, its manager, and what it may do. */
export interface Mandate extends MandateTerms {
  readonly id: AgreementId;
  readonly pool: PartyId;
  readonly manager: PartyId;
  /**
   * Item 10e.4: THE PERIOD IT WAS OPENED, which is what makes a young pool distinguishable from a
   * failing one. A manager judging a product it launched last week would close every fund in this
   * world the period after it opened (`noticeToGive`), and the agreement already knows when it was
   * written — so nothing is stored to find it out (Law 19).
   */
  readonly since: Period;
}

/**
 * A4, F2, XI-8: WRITE THE MANDATE. One door for the seed and for a launch mid-run, so a pool set up
 * at period zero and one set up in period forty are the same thing (Law 4).
 *
 * `leverage` is `false` for every pool this world draws, and that is a TERM and not an absence: none
 * of the mandates in this world permits borrowing, and a hedge fund is the mandate that does
 * (item 13). It used to be `borrows: false` on the party KIND, which said no pool anywhere may ever
 * be levered — a fact about the world stated as a fact about a category.
 */
export interface Product {
  readonly blueprint: Blueprint;
  readonly liquidity: Liquidity;
  readonly tracks: Option<string>;
  readonly feePerAnnum: Ratio;
  readonly buffer: Ratio;
  readonly requiredYieldPerAnnum: Ratio;
  readonly offeredPublicly: boolean;
  readonly mayWrite: MayWrite;
  readonly leverage: boolean;
  readonly performanceFee: Ratio;
  readonly targetLeverage: Ratio;
}

export function openMandate(
  ctx: { owes: (d: AgreementDecl) => Agreement },
  pool: PartyId,
  manager: PartyId,
  ccy: CurrencyCode,
  product: Product,
): void {
  // A3, B-14: every mandate this world draws writes NO derivatives — a money fund and a commodity
  // fund do not, and an index tracker does not. It is a term, and §28's hedge fund is the mandate
  // that says otherwise (item 13.2).
  const { liquidity, tracks } = product;
  const terms: MandateTerms = {
    kind: MANDATE,
    ...product,
    // A pool is opened to be run, not to be closed. Notice is something a manager gives later.
    windingUp: false,
  };
  ctx.owes({
    debtor: pool,
    creditor: manager,
    ccy,
    owed: 0,
    terms,
    why: `${manager} runs ${pool} under a ${liquidity.how} ${tracks.some ? `mandate tracking ${tracks.value}` : 'mandate on its own view'}`,
  });
}

/**
 * Seed A3, item 10e.4: A DECLARATION IS AN OPENING CONDITION, and this is where it becomes terms.
 *
 * `drawFunds` says what pools this world OPENS with — the way the bank draw says what balance
 * sheets it opens with — and from the moment the mandate is written the declaration says nothing
 * more: every later read is of the agreement, because the agreement is what the two parties are
 * bound by and the row is only what they started from (Law 19).
 */
export function productOf(d: FundDecl, ccy: CurrencyCode): Product {
  return {
    // Item 10e: SINGLE OR MULTI CURRENCY is a term of the mandate, and a single-currency one names
    // ITS OWN money — which is where the fund is, not a field declared beside it.
    blueprint: d.ownCurrencyOnly ? { ...d.blueprint, currencies: [ccy] } : d.blueprint,
    liquidity: d.liquidity,
    offeredPublicly: d.offeredPublicly,
    // C2: a pool that names no index is ACTIVE — it picks within its blueprint on its own view,
    // which is why two of them bid different levels for the same paper.
    tracks: d.tracks === undefined ? none<string>() : some(d.tracks),
    feePerAnnum: asRatio(d.fee, 'what its manager charges it per annum'),
    buffer: asRatio(d.buffer, 'the share of its book it keeps in cash'),
    requiredYieldPerAnnum: asRatio(d.requiredYield, 'what its investors require of it per annum'),
    // A3, §28 A4 (item 13.2): what it may take a position in, and whether it may be levered. Both
    // used to be written HERE, the same for every mandate in every world — a fact about the world's
    // data stated as a fact about the shape of an agreement.
    mayWrite: d.mayWrite,
    leverage: d.leverage,
    performanceFee: asRatio(d.performanceFee, 'the share of a gain its manager takes'),
    targetLeverage: asRatio(d.targetLeverage, 'the multiple of its equity it means to run'),
  };
}

export function mandateOf(a: Agreement): Mandate {
  if (!isMandate(a.terms)) {
    throw new InvalidRegistry('Fund Shares A4', `${a.id} is not a mandate`);
  }
  return { ...a.terms, id: a.id, pool: a.debtor, manager: a.creditor, since: a.since };
}

/**
 * A4: THE MANDATE A POOL IS RUN UNDER, asked of the pool's own commitments. A pool with none is not
 * a fund — it is a party holding things — and nothing here may decide for it.
 */
export function mandateFor(view: ParticipantView): Option<Mandate> {
  for (const a of view.commitments()) {
    if (a.state === 'performing' && a.debtor === view.self.id && isMandate(a.terms)) {
      return some(mandateOf(a));
    }
  }
  return none<Mandate>();
}

/** G1, G1.a, G1.b: whether this vehicle's investors may ask for their money back AT ALL. */
export const redeemable = (l: Liquidity): boolean => l.how === 'liquid' || l.how === 'semiLiquid';

/**
 * G1: whether a window is open this period. A liquid fund's is every period, which is what liquid
 * MEANS; a semi-liquid one's is every so many, placed on the calendar by the calendar (Money G3.a).
 */
export function payingThisPeriod(l: Liquidity, period: number): boolean {
  if (l.how === 'liquid') return true;
  if (l.how !== 'semiLiquid') return false;
  return l.everyPeriods <= 1 || period % l.everyPeriods === 0;
}

/**
 * Seed A3, Law 19, item 10e.4: THE POOLS THIS WORLD ACTUALLY HAS, and it is a READ.
 *
 * Every phase in this module used to walk `decls` — the roster `drawFunds` returned at assembly —
 * which said that the set of funds in this world is fixed for ever and decided before anybody in it
 * has done anything. That is a declared equilibrium of an industry's structure (Law 2), and it is
 * what made a manager's launch and wind-down decisions unbuildable: a pool that did not exist when
 * the module was assembled could never have been struck, never have placed its cash, never have
 * paid anybody.
 *
 * A pool is a POOL BECAUSE SOMEBODY RUNS IT. The mandate is the fact — two named parties, dated
 * terms, a state — so the set of funds is the set of performing mandates, and it is different in
 * period fifty from period zero because managers opened and ended some in between.
 */
export function livingPools(ctx: MechanismContext): readonly Mandate[] {
  const out: Mandate[] = [];
  for (const a of ctx.agreements.ofKind(MANDATE)) {
    if (a.state !== 'performing' || !isMandate(a.terms)) continue;
    if (!ctx.parties.has(a.debtor) || !ctx.parties.get(a.debtor).status.alive) continue;
    out.push(mandateOf(a));
  }
  return out;
}

