/**
 * Kinds and their profiles: the dispatch tables behind Law 15.
 *
 * @spec Bond N9.b Law 15 Law 9 XI-6 XI-15 Equity F4 Fund Shares A3 Money B3 Money B3.a Money B3.b Money B3.c Register E1 Register E2 Granularity
 *
 * A kind is registered at assembly by the module that owns it, with the whole of its behaviour in
 * one profile. The kernel never branches on a kind id; it asks the profile. Adding an instrument or
 * a party kind is one profile in one module; replacing a system replaces its profiles. Every profile
 * referenced by the state must exist at assembly (validated by the Registry), so an unknown kind is
 * a construction error and never a runtime surprise.
 */
import type { Calendar, Period } from '../calendar/calendar.js';
import type { Civil } from '../calendar/civil.js';
import type {
  CurrencyCode,
  InstrumentId,
  InstrumentKindId,
  PartyId,
  PartyKindId,
  UnitId,
} from '../core/ids.js';
import type { Failed } from '../ledger/instruction.js';
import type { Instrument, Terms } from '../register/instruments.js';
import type { Holding } from '../register/register.js';
import type { Option } from '../core/option.js';
import type { Namer } from './naming.js';

/** Named individually or represented as cells with a weight (XI-15). */
export type Representation = 'named' | 'cell';

/**
 * Where an instrument's price comes from (XI-6).
 *
 * `derived` is the one that is neither a market nor a cost, and it exists for exactly one shape:
 * a claim ON A BOOK, whose value IS that book read through the share count (Fund Shares B1). It is
 * not an exception to Law 3 — nothing here prices a thing by a formula in place of a market, and
 * everything the book holds is itself marked at what a market cleared — it is the arithmetic that
 * says what a claim on those marks comes to. Read every time, never stored, the same number for
 * every holder, which is what separates it from a valuer's answer: `marks` says what a lot is worth
 * to the party HOLDING it (a lender's own assessment of its own loan, Banks Lending D2), and that
 * is a different question with a different answer per holder.
 */
export type Pricing = 'money' | 'cleared' | 'carriedAtCost' | 'derived';

/**
 * How a holder carries it, which is a different question from where its price comes from (Goods
 * E1, E2). A bond is carried at the mark and revalues with it; inventory is carried at what it cost
 * and is only ever written DOWN to what a market says (E2.c) — and it still clears in a market,
 * because a price and a carrying value are two things.
 */
export type Carry = 'mark' | 'cost';

/**
 * The stated, consistently applied lot-flow assumption (Register D4; Goods E5 forbids LIFO).
 * Weighted average arrives with inventory (Goods E5); until then FIFO is the only flow.
 */
export type LotFlow = 'FIFO';

/** One dated payment per unit the terms promise (Bond N5, N10): the curve reads these. */
export interface CashFlow {
  readonly date: Civil;
  /** Per unit of the instrument, in its currency. */
  readonly perUnit: number;
}

/** What an instrument's terms say falls due in a period (Register E1, E2). */
export type DueAction =
  | { readonly kind: 'coupon'; readonly date: Civil; readonly amountPerUnit: number }
  | { readonly kind: 'maturity'; readonly date: Civil };

/**
 * Bond N12: an instrument's own definition of failure to perform, met by something that happened.
 *
 * The definition belongs to the instrument, not to the kernel: a sovereign bond has no covenants to
 * breach, so nothing but a missed payment can be one; a loan (worklist 6) has both. The kernel knows
 * only that a payment failed, and asks. And it must be OBSERVABLE BY A HOLDER — which is why what
 * comes back is the words the definition uses, journalled publicly for everyone to react to, and not
 * a flag whose meaning lives in the code that set it.
 */
export interface DefaultDefinition {
  readonly met: string;
}

/**
 * Bond N13, N13.a: what a holder is entitled to on failure, and where that claim stands against the
 * issuer's others. Both are stated even when the answers are "nothing seizable" and "all equal" —
 * an unstated ranking is the one that silently becomes a waterfall the first time somebody needs one.
 */
export interface Ranking {
  /** N13.a: lower is more senior; equal numbers are pari passu. */
  readonly seniority: number;
  /** N13: what is pledged against it, which is nothing for an unsecured claim. */
  readonly secured: readonly { readonly instrument: InstrumentId; readonly qty: number }[];
  /** N13: what the holder is entitled to, in words. Never empty. */
  readonly claim: string;
}

export interface InstrumentKindProfile {
  readonly id: InstrumentKindId;
  /** Where the instrument's price comes from (XI-6). */
  readonly pricing: Pricing;
  /** How a holder carries it: at the mark, or at what it cost (Goods E1). */
  readonly carry: Carry;
  /**
   * Whether a holding of it is a liability of the issuer. Money, bonds, loans, fund shares: yes.
   * A share is the residual claim, not a liability (Equity A1).
   */
  readonly liabilityOfIssuer: boolean;
  /** The unit its quantity is counted in (Register A1.c), given its currency. */
  readonly unit: (ccy: CurrencyCode) => UnitId;
  /** Validate kind-specific terms at registration; throw InvalidRegistry otherwise. */
  readonly validateTerms: (terms: Terms) => void;
  /**
   * Law 9: the name a market would use, built from the instrument's own terms and the name of
   * whoever promised it — which is nobody for a physical thing (Goods A1), so the profile is given
   * the issuer's name only when there is an issuer and says how it names itself without one.
   */
  readonly displayName: (i: Instrument, namer: Namer) => string;
  /** The dated actions the terms place in `period` (Money G3.a), in date order. */
  readonly due: (i: Instrument, period: Period, calendar: Calendar) => readonly DueAction[];
  /**
   * Bond N9.b: interest accrued per unit since the last payment date, on a date. Zero for an
   * instrument that pays no coupon: a bill accretes against its own cleared price (Sovereign F2).
   */
  readonly accrued: (i: Instrument, on: Civil, calendar: Calendar) => number;
  /**
   * Every payment the terms promise strictly after a date, in date order (Sovereign D2: what a
   * yield is derived FROM). An instrument that promises nothing dated returns none of them.
   */
  readonly cashFlows: (i: Instrument, after: Civil, calendar: Calendar) => readonly CashFlow[];
  /**
   * Goods E2, Capital Programme A3: what a LOT of this kind is carried at now, per unit.
   *
   * The kernel asks the profile lot by lot, books the difference against the equity account and
   * re-marks the lot to the answer: one number landing in both places, which is what "one schedule,
   * charged against profit and against the stock" means (A3). A rise is refused unless the kind
   * says it marks both ways (E2.c: nobody but a dealer writes inventory up). None means this lot
   * is carried at what it was, which is the answer for a good whose market has not fallen below
   * cost and for a kind that never writes anything down.
   *
   * It is asked PER LOT because the answer is per lot: two vintages of the same plant have
   * different lives left, and a lot bought second-hand carries what its buyer paid, not what the
   * seller's book said (A6: a vintage has its own cost and its own service date). `marked` is what
   * a market last said a unit is worth, when anything did — a good is written down to it (E2), and
   * a thing that wears out on a schedule of its own does not read it at all.
   */
  readonly carriedAt?: (
    i: Instrument,
    lot: { readonly qty: number; readonly basisPerUnit: number; readonly acquired: Period },
    marked: Option<number>,
    at: Period,
    calendar: Calendar,
  ) => Option<number>;
  /**
   * Goods E2.c: whether this kind may be carried above cost. A dealer's book marks both ways; an
   * ordinary holder's inventory does not.
   */
  readonly fairValueThroughIncome?: boolean;
  /**
   * Goods A1, E4: whether units of this kind are physical things that are made and used up, rather
   * than claims that are issued and redeemed. Only such a kind admits a create or a destroy leg;
   * for everything else, a unit that appeared without an issuer would be an invented claim.
   */
  readonly physical?: boolean;
  /**
   * Bond N13, N13.a: what a holder is entitled to on failure and where it ranks. Required, because
   * "stated even when the answer is nothing" is the whole point of the clause.
   */
  readonly ranking: (i: Instrument) => Ranking;
  /**
   * Bond N12: whether a payment that failed is a DEFAULT under this instrument's own definition.
   * The kernel asks after a coupon or a maturity instruction fails, and journals what comes back.
   * A kind that cannot default — money, a tonne of grain — does not answer.
   */
  readonly defaultOn?: (i: Instrument, failed: Failed) => DefaultDefinition | undefined;
  /**
   * Corporate Credit G2: whether a default on one of this issuer's instruments makes this one due
   * as well. Stated, and false is an answer: a sovereign has no cross-default (Sovereign G3).
   */
  readonly accelerates?: boolean;
  /**
   * Register E4, Equity D4: whether the COUNT of a line of this kind can be restated without
   * anything else about it changing. A share can (a split), and it is the whole of what a split is:
   * more units, each of them smaller, the same claim. A bond cannot — its unit is par, and par
   * restated is a different promise — and neither can a good, whose unit is a tonne.
   *
   * A kind that says nothing is a kind that does not split, which is the answer for almost
   * everything; the door refuses a line whose kind has not said it does.
   */
  readonly splits?: boolean;
  /**
   * Fund Shares B1, XI-6: what one unit is worth, for a kind whose `pricing` is `derived`. The
   * kernel hands it the same reads it uses itself, so a share of a fund is valued off the marks
   * every other holder of those assets is valued off — never a second price system beside them
   * (Law 4). Required on a derived kind and meaningless on any other; assembly refuses a derived
   * kind without one, because a derived value nobody derives is an unpriced position pretending.
   */
  readonly derive?: (i: Instrument, at: Period, reads: DerivedReads) => number;
}

/**
 * What a derived value may read: the register, the instruments, and the kernel's own marks. It is
 * the kernel's own reads and nothing else — no party's view, no module state — because a derived
 * value is a fact about a book that anybody may compute and everybody gets the same answer from.
 */
export interface DerivedReads {
  /** Every holding of a party, and every holder of an instrument (Register B2, both directions). */
  holdingsOf(holder: PartyId): readonly Holding[];
  holdersOf(instrument: InstrumentId): readonly PartyId[];
  quantity(holder: PartyId, instrument: InstrumentId): number;
  /**
   * XI-6, Fund Shares B2, B2.a: what a holder's whole position in one instrument is worth at the
   * last mark on or before `at`, and WHICH period that mark came from. A stale mark is neither an
   * error nor a hole — it is a stale value, and the period is how a reader knows it is stale and
   * says so. None when nothing has ever marked it, which is a different answer from zero.
   */
  worthOf(
    holder: PartyId,
    instrument: InstrumentId,
    at: Period,
  ): Option<{ readonly value: number; readonly from: Period }>;
  /** Every instrument, so a book's liabilities can be found by who issued them (Register B3). */
  instruments(): readonly Instrument[];
  /** How many units of a line exist (Register B2): a share count is `issued`, never a stored total. */
  issued(instrument: InstrumentId): number;
  /** A kind's profile, for a book that must ask what its own holdings are (Law 15). */
  kindOf(instrument: InstrumentId): InstrumentKindProfile;
}

/**
 * What a money issuer does when a payment would take a holder's account below zero (Money B3).
 * Somebody lends it at a rate, or somebody refuses and the refusal is recorded (B3.c).
 *
 * There is nothing else to say. Whoever ALLOWED it writes the row that prices it before the period
 * closes — a bank its customer's drawing (B3.a), the central bank the reserve overdraft it stood
 * behind (D3.b) — so by the audit there is a lender, a rate and a date behind every negative, and
 * an account still below zero is a defect in whichever module allowed it.
 */
export type OverdraftDecision =
  | { readonly allow: true }
  | { readonly allow: false };

/**
 * WHO answers Money B3 for an issuer's customers.
 *
 * A kind whose answer is its own states the function — the central bank's is, because lending
 * reserves to the banks that settle in them is what a central bank is (Central Bank D1). A kind
 * whose answer is a CREDIT DECISION says so and answers nothing: B3.a is explicit that a customer
 * overdrawn is borrowing and that it is its bank's decision, and a decision that weighs the room a
 * bank's own capital supports is not something a kind profile could ever know how to take. The
 * module that owns lending registers it at assembly, and assembly refuses a world where a kind says
 * this and nobody answers — a refusal that exists because a defaulted-to "no" would look exactly
 * like a bank with a credit standard.
 */
export type OverdraftPolicy = ((ctx: OverdraftContext) => OverdraftDecision) | 'aCreditDecision';



export interface OverdraftContext {
  readonly holder: PartyId;
  readonly issuer: PartyId;
  readonly ccy: CurrencyCode;
  /** How far below zero the balance would go, per member of the holder. */
  readonly shortfall: number;
  /**
   * Whether the holder issues money itself. A central bank lends reserves to the banks that settle
   * in them (Central Bank D1, D3); everyone else banking with it — the treasury above all — has an
   * account, not a facility (Central Bank E2, Treasury D3).
   */
  readonly holderIssuesMoney: boolean;
}

export interface PartyKindProfile {
  readonly id: PartyKindId;
  readonly representation: Representation;
  /** Present when parties of this kind issue money (Money A1): a bank, a central bank. */
  readonly moneyIssuer: { readonly overdraft: OverdraftPolicy } | null;
  /**
   * XI-3, XI-8: whether a party of this kind may end the chain of successors. An estate that has
   * sold everything and paid it away has nobody left to succeed it, and every other kind must name
   * somebody: a party that succeeded itself would be a reference that never resolves (Register F2).
   */
  readonly terminal?: boolean;
  /**
   * XI-3: what a party of this kind can FAIL on, stated per kind because the answers differ and
   * because "every kind of party can cease" needs its exceptions named rather than left out. A
   * central bank says nothing on either: it can never run out of what it alone issues, and a loss
   * reduces its equity without ending it (§31 A1.a, E4). A treasury in its own money says nothing
   * either: its failure mode is inflation and not default (Sovereign G1).
   */
  readonly fails?: readonly ('cash' | 'solvency')[];
  /**
   * Banks Lending A1, C3, XI-8: whether anybody can lend to a party of this kind at all. A going
   * concern can borrow, and whether it does is a bank's credit decision; a party whose whole
   * business is being wound up cannot, because there is nobody left to sign and nothing to repay
   * out of. It is declared here rather than asked in a mechanism (Law 15): a bank that funded an
   * estate's own interest week after week would be lending to a liquidation for ever, and the
   * refusal it should have made is a real one that gets recorded (Money B3.c).
   */
  readonly borrows: boolean;
  /**
   * Money A1, Money Market E1, Dealer Desks A1: whether a party of this kind CHOOSES where it
   * banks. Almost everybody does — a household or a firm banks where it likes and leaves when
   * somebody pays it more, which is the whole of what stops a bank paying less (E1).
   *
   * Some do not, and for them the account is part of what the party IS rather than a decision it
   * takes: a trading desk is its bank's own arm and an account at a rival would make it a different
   * firm (A1); a bank and the treasury settle at the central bank because that is what settling in
   * central bank money means (Money C2.a, Treasury D3); an estate holds the account of the party it
   * succeeded and is realising it, not running it (XI-8). Declared here because it is a fact about
   * the kind, so the funding market asks the profile instead of naming kinds (Law 15).
   */
  readonly choosesBank: boolean;
  /**
   * Banks Funding A1, A1.d: WHICH KIND OF DEPOSITOR a party of this kind is, or null for one that
   * is nobody's deposit base. It is a fact about the kind — many and small, fewer and operational,
   * few and very large — so it is declared with the kind and the funding market asks the profile
   * (Law 15). A table inside that market mapping kinds to classes was that branch written out: a
   * module added a kind and the market kept a list of them.
   *
   * What the class then MEANS — insured to a limit, what it costs one to move — belongs to the
   * market that holds the taxonomy, because those are facts about a regulation and a cost and not
   * about what the party is.
   */
  readonly depositClass: string | null;
}
