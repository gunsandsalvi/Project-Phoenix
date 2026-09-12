/**
 * What a kind of derivative contract is: the profile the kernel asks, exactly as it asks an
 * instrument kind's (Law 15).
 *
 * @spec Derivative D1 Derivative D1.a Derivative D1.b Derivative D2 Derivative D2.a Derivative D3 Derivative D3.a Derivative D4 Derivative D5 Derivative D6 Derivative D6.a Derivative D7 Derivative D7.a Derivative D8 Derivative D8.a Derivative D11 Derivative D11.a Derivative D12 Derivative X1 Derivative Layer A3 Derivative Layer D1 Derivative Layer G4 Law 15
 *
 * A derivative is NOT a holding (X1): nobody issued it, it has no issued amount and it enters no
 * ownership check. So it has a profile of its own rather than an `InstrumentKindProfile`, and the
 * differences are the clauses: it has two named sides instead of an issuer and holders (D1), its
 * value is a MARK read from one side and negated for the other (A3), and the thing it settles
 * against is an `Underlying` that this world must already clear or record (D3, G4).
 *
 * THE MARK IS A FUNCTION OF PUBLIC STATE AND NOTHING ELSE. It is one number read from two sides
 * (A3), so a mark that could see either party's own state would answer two different things for one
 * contract and D1.b would be checking a coincidence. `ContractReads` is therefore the kernel's own
 * reads — prints, marks, indices, curves, public events — and no view of anybody.
 */
import type { Calendar, Period } from '../calendar/calendar.js';
import type { Civil } from '../calendar/civil.js';
import type {
  ContractId,
  CurrencyCode,
  CurveFamilyId,
  DerivativeKindId,
  InstrumentId,
  MarketId,
  PartyId,
  UnitId,
} from '../core/ids.js';
import type { Option } from '../core/option.js';
import type { Event, EventKind } from '../journal/journal.js';
import type { CurveRead } from '../prices/curve.js';
import type { IndexRead } from '../prices/index-read.js';
import type { Print } from '../prices/price-store.js';
import type { MarketDecl } from '../clearing/market.js';
import type { Order } from '../clearing/solver.js';
import type { ParticipantView } from '../world/context.js';
import type { Namer } from './naming.js';
import type { ParamRegister } from './params.js';

/**
 * D12, X1: what a contract's terms ARE. The same shape an instrument's terms have and a different
 * type, because a contract is not an instrument: what its `kind` names is a derivative kind, and a
 * type that let one be passed where the other is expected would be the register and the contract
 * store sharing a door they do not share (X1).
 */
export interface ContractTerms {
  readonly kind: DerivativeKindId;
}

/**
 * D3: what the payoff is a function of — and it is always something this world produced somewhere
 * else. D3.a forbids an underlying that exists only inside the derivative, so there are exactly
 * three shapes and each one names a thing another system already publishes: a print a market
 * cleared, an index that is a read of prints, or a public event this world records.
 */
export type Underlying =
  | { readonly kind: 'print'; readonly market: MarketId; readonly instrument: InstrumentId }
  | { readonly kind: 'index'; readonly index: string }
  | { readonly kind: 'event'; readonly party: PartyId; readonly event: EventKind };

/** D4, D6, D6.a: one dated payment the contract's own terms put in this period, either way. */
export interface ContractPayment {
  readonly from: PartyId;
  readonly to: PartyId;
  readonly ccy: CurrencyCode;
  readonly amount: number;
  readonly date: Civil;
  readonly why: string;
}

/**
 * What a mark, a margin and a close-out may read: the kernel's own public state. Not a party's
 * view, because the answer must be the same from both sides (A3, D1.b).
 */
export interface ContractReads {
  readonly period: Period;
  readonly calendar: Calendar;
  readonly params: Pick<ParamRegister, 'get' | 'amount'>;
  /** The last print at or before a period — the same read every holder of the line gets. */
  print(instrument: InstrumentId, at: Period): Option<Print>;
  /** XI-6: what a unit of a line is carried at, for a claim on a book that no session printed. */
  mark(instrument: InstrumentId, at: Period): Option<number>;
  index(id: string): Option<IndexRead>;
  curve(family: CurveFamilyId): CurveRead;
  /**
   * Derivative Layer D1: THE UNDERLYING'S OWN MEASURED MOVE over the last `periods` prints — the
   * standard deviation of what it actually did, in the price's own unit. It is a READ of history
   * and never a rate per class: D1 says initial margin is sized from the risk of the position, and
   * the one honest source of that in this world is the thing's own record. None when the line has
   * not printed enough times to have a record, which is a different answer from zero.
   */
  measuredMove(instrument: InstrumentId, periods: number): Option<number>;
  /** D3: the public event an underlying of that shape names, most recent, or none. */
  lastEvent(kind: EventKind, subject: string): Option<Event>;
}

/** The row itself. Two named sides, and the mark is written from `a`'s (A3). */
export interface Contract {
  readonly id: ContractId;
  readonly kind: DerivativeKindId;
  /** D1: the side the terms are stated from and the mark is written for. */
  readonly a: PartyId;
  readonly b: PartyId;
  readonly terms: ContractTerms;
  /** D5: the money its legs move in. */
  readonly ccy: CurrencyCode;
  /** D2: what scales the payoff, in the kind's own unit. Not the exposure (D2.a). */
  readonly notional: number;
  /** D7: the rate, spread or strike the two sides entered at, as the market cleared it. */
  readonly struckAt: number;
  /**
   * Register D4: what the position COST — what it was worth to `a` when it was written, which is
   * zero for a contract struck at par (D7.b) and the premium for one bought outright. It is the
   * basis, not a mark: what the equity account has recognised until revaluation moves it.
   */
  readonly basis: number;
  readonly opened: Period;
  readonly state: 'open' | 'terminated';
  readonly terminated: Option<Period>;
  /** C2: the house both sides face for a cleared contract; null for a bilateral one. */
  readonly house: PartyId | null;
}

export interface DerivativeKindProfile {
  readonly id: DerivativeKindId;
  /** D2: the unit the notional is counted in (contracts, units of face). */
  readonly unit: UnitId;
  /**
   * Law 8, Clearing C4.c, D7: the smallest increment this kind's book quotes in — a premium per
   * unit of notional, or the rate a contract is struck at. Declared, like an instrument kind's, and
   * how fine it is, is a RESOLUTION (Law 2): `tickShift` divides it with every other.
   */
  readonly priceTick: number;
  /**
   * D7, D7.b, Law 8: WHAT THE CLEARED LEVEL IS — money per unit of notional, or a RATE per annum.
   *
   * A swap and a credit default swap clear on a rate: what the two sides agree is the fixed rate or
   * the running spread that makes the contract worth nothing at inception (D7.b), and the size
   * against it is a notional. A forward or an option clears on money. The solver does not care —
   * a schedule is size against a level either way — but a reader does, because a level of 0.0125
   * is a hundred and twenty-five basis points and not a cent and a quarter.
   *
   * It belongs to the KIND and not to the book: what a credit default swap is quoted in is a fact
   * about credit default swaps, and a second copy of it on every market that opens one would be a
   * fact with two writers (Law 4). Absent means money, which is what every kind before rates was.
   */
  readonly quotedAs?: 'money' | 'rate';
  /** D3, G4: what it settles against. Checked against the world when a contract is opened. */
  readonly underlying: (c: Contract) => Underlying;
  readonly validateTerms: (terms: ContractTerms) => void;
  readonly displayName: (c: Contract, namer: Namer) => string;
  /**
   * D8: what the contract is worth TO `a`, now. `b`'s is the negation and nothing computes it
   * separately — but the profile must be able to STATE the same contract from `b`'s side (`flip`),
   * and the zero-sum family asks for both and requires them to negate exactly (D1.b).
   */
  readonly mark: (c: Contract, at: Period, reads: ContractReads) => number;
  /**
   * A3, D1.b: the same contract as the OTHER side states it. Swapping `a` and `b` is not enough
   * whenever the terms carry a direction (a forward's buy and sell, a swap's payer and receiver), so
   * the kind says how its own terms read from the other side — and the family then has two
   * independent computations of one number instead of a negation it performed itself.
   */
  readonly flip: (terms: ContractTerms) => ContractTerms;
  /**
   * D7.b: WHAT THE BUYER PAYS THE WRITER WHEN THE CONTRACT IS WRITTEN, per unit of notional.
   *
   * Many derivatives are struck at par — zero value at inception — and then the cleared price IS
   * the fixed rate or spread that makes it so, and nothing changes hands (D7.b). Others are bought
   * outright and the cleared price is the premium. Both are one question asked of the kind, and the
   * answer is what the contract is then WORTH to the buyer: a thing is worth what it cost until
   * something re-marks it (Register D4).
   */
  readonly premiumPerUnit: (struckAt: number, terms: ContractTerms) => number;
  /** D4, D6: what falls due this period under the terms, both directions (D5: each in its money). */
  readonly legs: (c: Contract, at: Period, reads: ContractReads) => readonly ContractPayment[];
  /**
   * Derivative Layer D1, G2: WHAT MUST BE POSTED UP FRONT, from the underlying's own measured move.
   *
   * `none` when the underlying has no record to measure — a line that has printed once has not
   * moved yet, and there is no honest number for what a position in it could do. G2 admits an
   * exposure with no margin only with A STATED REASON, and "nobody can say" is not one: the layer
   * refuses the trade and journals the refusal (E2, E4) rather than admitting it at nothing.
   */
  readonly initialMargin: (c: Contract, at: Period, reads: ContractReads) => Option<number>;
  /** D11.a: the stated value an early termination closes out at, to `a`. */
  readonly closeOut: (c: Contract, at: Period, reads: ContractReads) => number;
  /** D6, D11: whether the term has run out at `at`, so the contract expires this period. */
  readonly expires: (c: Contract, at: Period, calendar: Calendar) => boolean;
  /**
   * Clearing B2, Law 4, Law 15: WHY A PARTY IS IN THIS KIND OF BOOK, asked of the kind.
   *
   * Every book in this world needs reasons on both sides of it, and the reasons to be in a credit
   * default swap are not the reasons to be in a bond future. But ONE PARTY SHOWS ONE FACE TO ONE
   * BOOK (Clearing A2: nobody crosses themselves), and a party whose schedule came from six
   * modules would be six opinions wearing one name — so the layer declares the participant once,
   * per party kind, and asks the kind of contract the book carries. The dispatch is the table
   * Law 15 asks for; the reasons stay with the class that has them.
   *
   * Absent means no party of any kind has a reason to be in this kind of book of its own accord —
   * a test-only kind, or one whose rows are written by a mechanism rather than a session.
   */
  readonly orders?: (view: ParticipantView, m: MarketDecl) => readonly Order[];
}

/**
 * D12: counterparties + underlying + term + strike. Two contracts on one underlying at different
 * strikes are two rows, and an offsetting trade with a different counterparty is a third (B3.a) —
 * so this is a READ for a reporter and never a key the store collapses on.
 */
export function identityOf(c: Contract, u: Underlying): string {
  const under =
    u.kind === 'print' ? `${u.market}/${u.instrument}` : u.kind === 'index' ? u.index : `${u.party}:${u.event}`;
  return `${c.kind}|${c.a}|${c.b}|${under}|${c.opened}|${c.struckAt}`;
}

/** The same contract as `b` states it (A3): the sides swap and the terms are read the other way. */
export function mirrored(c: Contract, profile: DerivativeKindProfile): Contract {
  return { ...c, a: c.b, b: c.a, terms: profile.flip(c.terms) };
}
