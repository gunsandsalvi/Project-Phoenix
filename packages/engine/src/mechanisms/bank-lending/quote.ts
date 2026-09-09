/**
 * What a bank will lend at, and whether it will lend at all.
 *
 * @spec Banks Lending B2 Banks Lending B2.a Banks Lending B2.b Banks Lending B2.c Banks Lending C1 Banks Lending C1.a Banks Lending C1.b Banks Lending C1.c Banks Lending C1.d Banks Lending C3 Banks Lending C4 Banks Lending F3 XI-4 Law 2
 *
 * C1: FOUR NAMED TERMS, each from this bank's own state, and the rate is their sum. XI-4 calls this
 * joint one and says what breaks it: a bank with no cost-of-funds term prices every loan as though
 * it funded at the policy rate whatever its own position, and then nothing about its funding can
 * ever reach a borrower. So the cost of funds here is a READ of what this bank actually pays on
 * what it owes — which is zero today, because nothing it owes pays interest yet, and which becomes
 * the thing that differs between banks the moment deposits are priced (worklist 11).
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
import type { ParticipantView } from '../../world/context.js';
import { bankParam, type BankDecl } from './data.js';
import { isLoan } from './loan.js';

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
  readonly capital: number;
  readonly appetite: number;
  readonly most: number;
  readonly binds: 'capital' | 'appetite' | 'nothing';
}

export function room(
  view: ParticipantView,
  decl: BankDecl,
  borrower: PartyId,
  reg: Regulation,
): Room {
  const capital = view.equity();
  const required = add(reg.capitalRatio, view.params.get(bankParam(decl.bank, 'capitalBuffer')), 'what it runs at');
  // B2.a: risk-weighted assets it may carry at all, less what it already carries.
  const weighted = riskWeighted(view, reg.riskWeight);
  const allowed = required > 0 ? div(capital, required, 'assets its capital supports') : 0;
  const byCapital = reg.riskWeight > 0 ? div(sub(allowed, weighted, 'room left'), reg.riskWeight, 'more it could lend') : 0;
  // F3, B2.c: the most it will have out to one name, whatever its capital would allow.
  const limit = mul(capital, view.params.get(bankParam(decl.bank, 'limitPerBorrower')), 'its limit for one name');
  const byAppetite = sub(limit, exposureTo(view, borrower), 'room under its limit');
  const most = byCapital < byAppetite ? byCapital : byAppetite;
  return {
    capital: byCapital,
    appetite: byAppetite,
    most,
    binds: most <= 0 ? (byCapital < byAppetite ? 'capital' : 'appetite') : 'nothing',
  };
}

/** B2.a: what this bank's book weighs, which is what its capital has to support. */
function riskWeighted(view: ParticipantView, weight: number): number {
  let total = 0;
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!isLoan(i.terms)) continue;
    total = add(total, mul(view.quantity(h.instrument), weight, 'weighted'), 'risk-weighted assets');
  }
  return total;
}

/** F3: what it already has out to this name. */
export function exposureTo(view: ParticipantView, borrower: PartyId): number {
  let total = 0;
  for (const h of view.holdings()) {
    const i = view.instruments.get(h.instrument);
    if (!isLoan(i.terms) || i.terms.borrower !== borrower) continue;
    total = add(total, view.quantity(h.instrument), 'exposure to one name');
  }
  return total;
}
