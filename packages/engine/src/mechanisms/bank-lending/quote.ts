/**
 * What a bank will lend at, and whether it will lend at all.
 *
 * @spec Banks Lending B2 Banks Lending B2.a Banks Lending B2.b Banks Lending B2.c Banks Lending C1 Banks Lending C1.a Banks Lending C1.b Banks Lending C1.c Banks Lending C1.d Banks Lending C3 Banks Lending C4 Banks Lending F3 XI-4 Law 2
 *
 * C1: FOUR NAMED TERMS, each from this bank's own state, and the rate is their sum. XI-4 calls this
 * joint one and says what breaks it: a bank with no cost-of-funds term prices every loan as though
 * it funded at the policy rate whatever its own position, and then nothing about its funding can
 * ever reach a borrower. So the cost of funds here is a READ of what this bank actually pays on
 * what it owes — deposits at the rate it set and rows at the rate the session struck — and two
 * banks with different mixes quote differently because of it (Banks Funding B2, B2.a).
 *
 * C1.b is the bank's OWN assessment of the borrower, and C4 says there is one model per borrower:
 * how often it has seen that borrower fail to pay, over the memory it keeps. It is a frequency of
 * events it observed, not a hazard rate that produces them — defaults still come only from payments
 * that did not happen (XI-1, Firm Birth C2.a) — and it is the channel by which a default becomes
 * information about what the next loan costs (Corporate Credit G8).
 */
import type { PartyId } from '../../core/ids.js';
import type { Event } from '../../journal/journal.js';
import { add, div, mul, sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { ParticipantView } from '../../world/context.js';
import { bankParam, type BankDecl } from './data.js';

/** C1: the four terms, kept apart so a reader can see which one moved (B2.d, XI-4). */
export interface Quote {
  readonly bank: PartyId;
  readonly costOfFunds: number;
  readonly expectedLoss: number;
  readonly capitalCharge: number;
  readonly operatingCost: number;
  readonly rate: number;
}

export interface Regulation {
  readonly capitalRatio: number;
  readonly riskWeight: number;
  readonly operatingCost: number;
}

/**
 * Corporate Credit E5, E5.a, E5.c, XI-4: what a BANK requires, per annum, to hold a named issuer's
 * paper. It is built where a bank's own economics live and computed once (Law 4), because a
 * schedule struck against a second copy of these terms would be a bank pricing its book against a
 * belief it does not hold.
 *
 * TWO OF E5's THREE TERMS ARE HERE. E5.a is its blended cost of funds — the same number its loans
 * are priced off (C1.a), so a bank whose funding gets dearer requires more of every asset it holds
 * and not only of the ones it writes. E5.c is the capital the position consumes times the return it
 * needs on that capital, at the weight the standard puts on that kind of claim. What is NOT here is
 * C1.d's operating cost: making and servicing a loan is work and holding a bond is not, which is
 * why a bank will buy an issuer's paper at a level it would not lend to it at.
 *
 * E5.b IS NOT HERE, AND IT IS A SCOPE BOUNDARY RATHER THAN AN OVERSIGHT. Expected loss is "A4's
 * assessment, times a loss given default", and A4 is explicit that an assessment is an OPINION HELD
 * BY SOMEBODY and not a property of the issuer. What exists today is a bank's own model of a
 * BORROWER it lends to (C4, worklist 6): how often it has seen that name fail to pay, times a loss
 * given default of ALL OF IT. Both halves are wrong for an issuer whose paper is marked to market
 * — a treasury that missed one payment has not stopped being able to create the money it promised
 * (Sovereign G1), and a recovery stated at zero is the fixed recovery Appendix B forbids. The
 * assessment those two terms need is the ratings system and the second opinion (§44, XI-13), which
 * is worklist 12; until then this says nothing rather than saying the loan model's answer.
 *
 * WHAT THAT COSTS, MEASURED: wiring the loan model in here anyway was tried, and one missed treasury
 * payment moved every bank's required yield by a twenty-sixth, repriced the whole curve, met a money
 * fund's forced sale (XI-2) at the desks' bids and took a bank's capital with it — a doom loop that
 * this world cannot yet resolve, because a failed bank's depositors have no account to bank at until
 * bank resolution exists (worklist 11). The finding is recorded; the number is not.
 */
export function holderReservation(
  view: ParticipantView,
  decl: BankDecl,
  reg: Regulation,
  /** E5.a: what this bank actually pays for what funds its book, read off the wire (C1.a). */
  funds: number,
): Quote {
  const consumed = mul(reg.riskWeight, reg.capitalRatio, 'capital consumed per unit held');
  const capitalCharge = mul(
    consumed,
    view.params.get(bankParam(decl.bank, 'returnOnCapital')),
    'capital charge',
  );
  return {
    bank: view.self.id,
    costOfFunds: funds,
    // E5.b: see above. Zero is what this bank has to SAY about the issuer, not what it believes.
    expectedLoss: 0,
    capitalCharge,
    operatingCost: 0,
    rate: add(funds, capitalCharge, 'what it requires to hold it'),
  };
}

/**
 * C1.b, C4: this bank's own view of this borrower — how often it has seen it fail to pay, over the
 * memory this bank keeps. One model, used for the price and for the provision alike.
 */
export function probabilityOfDefault(
  view: ParticipantView,
  decl: BankDecl,
  borrower: PartyId,
  /** Every default anybody has published. They are public, so this bank saw all of them (A2). */
  defaults: readonly Event[],
): number {
  const memory = view.params.get(bankParam(decl.bank, 'credit.memory'));
  const from = view.period - memory;
  const seen = defaults.filter((e) => e.period >= from && e.subjects.includes(borrower));
  const periods = view.period < memory ? view.period : memory;
  if (periods <= 0) return 0;
  const failures = new Set(seen.map((e) => e.period)).size;
  return div(failures, periods, 'how often this borrower has failed');
}

/**
 * C1.b: what it would lose if that happened. An unsecured claim on a borrower recovers what the
 * estate realises, and there is no estate yet (worklist 7) — so all of it, which is the honest
 * number and not a stated recovery rate.
 */
export function lossGivenDefault(security: readonly unknown[]): number {
  return security.length > 0 ? 0 : 1;
}

/** C1: the quote, built from this bank's own state and this borrower's own record. */
export function quote(
  view: ParticipantView,
  decl: BankDecl,
  borrower: PartyId,
  reg: Regulation,
  /** C1.a: what this bank actually paid for what it owed, read off the wire by its own module. */
  funds: number,
  defaults: readonly Event[],
): Quote {
  const pd = probabilityOfDefault(view, decl, borrower, defaults);
  const expectedLoss = mul(pd, lossGivenDefault([]), 'expected loss');
  // C1.c: the capital this loan consumes, times what this bank needs to earn on it.
  const consumed = mul(reg.riskWeight, reg.capitalRatio, 'capital consumed per unit lent');
  const capitalCharge = mul(consumed, view.params.get(bankParam(decl.bank, 'returnOnCapital')), 'capital charge');
  return {
    bank: view.self.id,
    costOfFunds: funds,
    expectedLoss,
    capitalCharge,
    operatingCost: reg.operatingCost,
    rate: add(add(funds, expectedLoss, 'funds and loss'), add(capitalCharge, reg.operatingCost, 'capital and running it'), 'the rate it quotes'),
  };
}

/**
 * B2: what stops it. Three constraints, separate because they are separate things, and which one
 * binds is an outcome (B2.d). B2.b — liquidity — is NOT here: the deposit it creates may be spent
 * away and it must fund that, and there is no money market for it to fund in until worklist 11.
 * Its absence is stated rather than approximated, because a liquidity constraint invented out of a
 * ratio would be a bound (Law 6) standing where a market belongs.
 */
export interface Room {
  /** What its capital leaves it, as its own module published it. Missing before it has (B1). */
  readonly capital: Option<number>;
  readonly appetite: number;
  /**
   * Banks Funding D4: what its own funding leaves it able to lend. Missing where no funding market
   * has said anything about this bank yet — a world with none has no funding condition to transmit.
   */
  readonly funding: Option<number>;
  readonly most: number;
  readonly binds: 'capital' | 'appetite' | 'funding' | 'nothing';
}

/**
 * Banks Funding D4, D4.a: THE CREDIT CRUNCH, and it belongs here rather than in a rule about
 * lending. A bank writes a loan by creating a deposit (B1.a), and a deposit walks: the borrower
 * borrowed in order to pay somebody, and paying somebody who banks elsewhere moves liquid assets it
 * has to have. So what it will lend is what it holds beyond what could leave it — its own two
 * published sides (C1 against C2) — and a bank already short of that lends nothing at all.
 *
 * Both numbers are the funding module's (C1, C2.a) and they arrive the only way anything crosses a
 * module boundary: as the public thing that bank published about itself (F4). It is last period's
 * publication, which is what a lag IS — the credit decision is taken before this period's session
 * has told anybody anything.
 */
export function fundingRoom(view: ParticipantView): Option<number> {
  const said = view.lastOwn('bank.liquidity');
  if (!said.some) return none<number>();
  const liquid = said.value.data['liquid'];
  const exposed = said.value.data['couldLeave'];
  if (typeof liquid !== 'number' || typeof exposed !== 'number') return none<number>();
  return some(sub(liquid, exposed, 'what it can lend and still cover what could leave'));
}

/** Which of the three is the constraint: the smallest of them, named so a refusal says why (C3.a). */
function bindingOf(
  capital: Option<number>,
  appetite: number,
  funding: Option<number>,
): Room['binds'] {
  const cap = capital.some ? capital.value : appetite;
  if (funding.some && funding.value <= cap && funding.value <= appetite) return 'funding';
  return capital.some && cap < appetite ? 'capital' : 'appetite';
}

export function room(view: ParticipantView, decl: BankDecl, borrower: PartyId): Room {
  const capital = view.equity();
  // Banks Capital B1, B1.b, B1.c: what its capital leaves it is the position its own module
  // published this period (`bank.capital`) — the weighted rule and the leverage backstop, whichever
  // of the two leaves it less, in units of the asset it is deciding about. It is READ and not
  // recomputed here, because a bank with two answers to how much capital it has would be a bank
  // that lends against one and reports the other (Law 4, Law 19). A bank that has published no
  // position has not decided anything about it (Appendix A) and is constrained by the rest.
  // At the last close (the phase runs after the marks are taken), which is when a bank last knew
  // what its book was worth.
  const said = view.lastOwn('bank.capital');
  const headroom = said.some ? said.value.data['headroom'] : undefined;
  const byCapital =
    typeof headroom === 'number' ? some(headroom) : none<number>();
  // F3, B2.c: the most it will have out to one name, whatever its capital would allow.
  const limit = mul(capital, view.params.get(bankParam(decl.bank, 'limitPerBorrower')), 'its limit for one name');
  const byAppetite = sub(limit, exposureTo(view, borrower), 'room under its limit');
  const byFunding = fundingRoom(view);
  const least = (a: number, b: Option<number>): number => (b.some && b.value < a ? b.value : a);
  const most = least(least(byAppetite, byCapital), byFunding);
  return {
    capital: byCapital,
    appetite: byAppetite,
    funding: byFunding,
    most,
    binds: most <= 0 ? bindingOf(byCapital, byAppetite, byFunding) : 'nothing',
  };
}

/** F3: what it already has out to this name. */
export function exposureTo(view: ParticipantView, borrower: PartyId): number {
  let total = 0;
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    // F3: EVERYTHING THAT NAME OWES IT, not everything of one kind. A limit that counted only loans
    // would be a limit a bank could go round by lending the same name money in another shape — a
    // week of unsecured money, a claim behind every other claim on it, a balance at it — and the
    // limit would bind on the one exposure it happened to be written about (Law 15, Law 19).
    if (!view.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    if (!i.issuer.some || view.parties.resolve(i.issuer.value).id !== borrower) continue;
    const units = view.quantity(h.instrument);
    if (units <= 0) continue;
    const mark = view.mark(h.instrument);
    total = add(total, mark.some ? mul(units, mark.value, 'at its mark') : units, 'exposure to one name');
  }
  return total;
}
