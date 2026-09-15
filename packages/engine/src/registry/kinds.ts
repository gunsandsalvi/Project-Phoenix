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
  CurveFamilyId,
} from '../core/ids.js';
import type { Failed } from '../ledger/instruction.js';
import type { Instrument, Terms } from '../register/instruments.js';
import type { Holding } from '../register/register.js';
import type { CurveRead } from '../prices/curve.js';
import type { Option } from '../core/option.js';
import type { LatticeDecl } from './lattice.js';
import type { Cash, PerNamedUnit, PerPiece, Ratio } from '../core/measure.js';
import type { Qty } from '../core/tick.js';
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
/**
 * What a kind needs to say what one unit is worth beyond its own promise (`worthTo`).
 *
 * Everything in it is PUBLIC (Observer A3): what an issuer published about itself, and how many
 * units of a line exist. A kind cannot reach a holder's private state through here, and it has no
 * business with one — what it answers is a fact about the INSTRUMENT at a required return.
 */
export interface WorthReads {
  readonly calendar: Calendar;
  readonly period: Period;
  /**
   * Reporting A1, A2: the issuer's last published accounts — what it said it earned and over how
   * many periods. There is no second set of accounts and no forecast (Reporting A2.a).
   */
  lastReport(issuer: PartyId): Option<{ readonly earned: Cash; readonly periods: number }>;
  /** How many units of this line exist, so a per-unit figure is per unit. */
  issued(instrument: InstrumentId): Qty;
}

export interface CashFlow {
  readonly date: Civil;
  /**
   * Per unit of the instrument, in its currency.
   *
   * Item 16: A PRICE, because that is what money per piece is — which is why discounting a schedule
   * gives a price back and never a balance, and why a coupon cannot be added to one.
   */
  readonly perUnit: PerPiece;
}

/** What an instrument's terms say falls due in a period (Register E1, E2). */
export type DueAction =
  | { readonly kind: 'coupon'; readonly date: Civil; readonly amountPerUnit: PerPiece }
  /**
   * Bond F3, Banks Lending F2 (11.2): principal repaid ON A SCHEDULE. The fraction of every
   * holding the issuer redeems at par on the date, so an amortiser is redeemed one slice at a time
   * and the maturity takes what is left. It is a fraction of what is outstanding and not an
   * amount, because the row's outstanding is the one writer of what there is to repay (Law 4).
   */
  | { readonly kind: 'amortisation'; readonly date: Civil; readonly unitsPerUnit: Ratio }
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
  /**
   * Register B3, XI-3, §48: WHAT THE ISSUER OWES WHEN THE PRICE MOVES, and the two answers are
   * genuinely different things rather than two conventions for one thing.
   *
   * `'face'` — THE PROMISE DOES NOT CHANGE. A borrower owes what it agreed to pay. When the market
   * marks its paper down because it is walking towards default, the HOLDER has lost something real
   * and the issuer has gained nothing: it still has to find the whole amount on the day. Debt is
   * not carried at market value on an issuer's balance sheet, and a world that did carry it there
   * would have a firm growing MORE solvent the less anybody trusted it — which would put the
   * solvency trigger out of reach exactly when it is supposed to fire (XI-3: nothing is immortal).
   *
   * `'value'` — THE CLAIM IS THE BOOK. A fund share is a residual claim on a pool: what the issuer
   * owes its holders IS what the pool is worth, so when the pool moves the liability moves with it
   * and the issuer's own equity stays where it belongs, at zero (Fund Shares A3). Leaving this out
   * would have a fund booking a profit on assets that are its holders'.
   *
   * There is no third answer and no default: a kind that is a liability of somebody has to say
   * which of its issuer's two situations it is in.
   */
  readonly owes: 'face' | 'value';
  /** The unit its quantity is counted in (Register A1.c), given its currency. */
  readonly unit: (ccy: CurrencyCode) => UnitId;
  /**
   * Law 1, Law 8: THE SMALLEST INCREMENT THIS KIND IS QUOTED IN, as money per named unit of it —
   * a cent a share, a basis point of a bond's face, a cent a tonne.
   *
   * A price is a quantity like any other and has a smallest piece for the same reason: a market
   * quotes on a grid, and a level finer than its tick is not a level anybody can hit. Without one
   * a print carried sixteen significant figures and every `units x price` read inherited them
   * (worklist 12b.1), so a balance sheet was finer than any money that could ever pay it.
   *
   * It is DECLARED and not derived from the money's own grid, because the two are different
   * conventions: a share moves in cents and a bond in ten-thousandths of its face, and both are paid
   * for in the same cents. Only a kind whose price comes from a market or a derivation has one —
   * money is worth one of itself and a kind carried at cost is never quoted (the registry refuses
   * either mistake at assembly). How fine it is, is a RESOLUTION (Law 2).
   */
  readonly priceTick?: number;
  /** Validate kind-specific terms at registration; throw InvalidRegistry otherwise. */
  readonly validateTerms: (terms: Terms) => void;
  /**
   * Law 9: the name a market would use, built from the instrument's own terms and the name of
   * whoever promised it — which is nobody for a physical thing (Goods A1), so the profile is given
   * the issuer's name only when there is an issuer and says how it names itself without one.
   */
  readonly displayName: (i: Instrument, namer: Namer) => string;
  /** The dated actions the terms place in `period` (Money G3.a), in date order. */
  readonly due: (
    i: Instrument,
    period: Period,
    calendar: Calendar,
    scale: PriceScale,
  ) => readonly DueAction[];
  /**
   * Bond N9.b: interest accrued per unit since the last payment date, on a date. Zero for an
   * instrument that pays no coupon: a bill accretes against its own cleared price (Sovereign F2).
   */
  readonly accrued: (i: Instrument, on: Civil, calendar: Calendar, scale: PriceScale) => number;
  /**
   * Every payment the terms promise strictly after a date, in date order (Sovereign D2: what a
   * yield is derived FROM). An instrument that promises nothing dated returns none of them.
   */
  readonly cashFlows: (
    i: Instrument,
    after: Civil,
    calendar: Calendar,
    scale: PriceScale,
  ) => readonly CashFlow[];
  /**
   * §46, Equity B1, Capital Programme B1: WHAT ONE UNIT IS WORTH TO A HOLDER THAT REQUIRES `required`
   * PER ANNUM — the other half of this profile, and the half that was missing.
   *
   * Every other field here answers *what is this thing legally and how does it settle*. Not one
   * answered *why would anyone hold it*, and `cashFlows` is the ISSUER'S CONTRACTUAL PROMISE, so the
   * only theory of value this world had was "discount the promise". A share has no promise. From
   * that single absence: `funds.eligible` returned false for anything with no cash flows, so no fund
   * could ever hold a share and every fund was a bond fund at the type level; a household therefore
   * valued a share by extrapolating its own price history; and the one earnings-based valuation in
   * the tree sat inside a module that has never produced anything. The equity market was a closed
   * loop of price-extrapolators with no fundamental side (`docs/RECORD.md` item 4).
   *
   * ABSENT IS AN ANSWER AND NOT A DEFAULT: it says THE PROMISE IS THE EXPECTATION, which is true of
   * every contractual instrument, and the kernel then discounts `cashFlows` at what the holder
   * requires. A kind that is worth something for another reason says so here.
   *
   * It is not a price and cannot become one (Law 3). The profile answers the stream, the PARTY
   * brings the required return out of its own circumstances, and what comes out is one participant's
   * bid meeting another's in a book.
   */
  readonly worthTo?: (i: Instrument, required: Ratio, reads: WorthReads) => Option<PerPiece>;
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
    lot: { readonly qty: number; readonly basisPerUnit: PerPiece; readonly acquired: Period },
    marked: Option<PerPiece>,
    at: Period,
    calendar: Calendar,
  ) => Option<PerPiece>;
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
/**
 * Law 8, E-9: THE ONE DOOR BETWEEN THE TWO SCALES, handed to a profile that has to cross it.
 *
 * A coupon and a par are stated per NAMED unit of face — five per hundred of it — and a price in
 * this world is money PIECES per piece of the thing. The three terms functions above all produce
 * prices out of numbers stated the other way, and each of them used to write `asPerPiece(...)`
 * straight onto a named number: right today, and only because every face unit in this world happens
 * to be declared with the money's own subdivision (`PAR` is `MONEY_PIECES`, a loan's unit IS the
 * money's piece), so the crossing factor is exactly one. A world that declared a bond in units of a
 * hundred would have had its coupons, its accruals and its redemption silently out by a hundred.
 *
 * `Registry` satisfies this, so a caller passes the registry it already has.
 */
export interface PriceScale {
  priceOf(ccy: CurrencyCode, unit: UnitId, perNamedUnit: PerNamedUnit): PerPiece;
}

export interface DerivedReads {
  /** Every holding of a party, and every holder of an instrument (Register B2, both directions). */
  holdingsOf(holder: PartyId): readonly Holding[];
  holdersOf(instrument: InstrumentId): readonly PartyId[];
  quantity(holder: PartyId, instrument: InstrumentId): Qty;
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
  ): Option<{ readonly value: Cash; readonly from: Period; readonly ccy: CurrencyCode }>;
  /**
   * Currency C4.a, A-50: what that is worth on this party's OWN book. A book is kept in one money
   * and adding two of them is a defect, so a reader that sums a party's positions converts here.
   */
  inOwnMoney(party: PartyId, value: Cash, from: CurrencyCode, at: Period): Cash;
  /** Every instrument, so a book's liabilities can be found by who issued them (Register B3). */
  instruments(): readonly Instrument[];
  /** How many units of a line exist (Register B2): a share count is `issued`, never a stored total. */
  issued(instrument: InstrumentId): Qty;
  /** A kind's profile, for a book that must ask what its own holdings are (Law 15). */
  kindOf(instrument: InstrumentId): InstrumentKindProfile;
  /**
   * Derivative X1, D1, Fund Shares A3, B1 (item 13.6): WHAT THIS PARTY'S OPEN CONTRACTS ARE WORTH
   * TO IT — because a book read out of holdings alone STOPS AT THE REGISTER'S EDGE.
   *
   * A contract is not a holding and never will be (X1): nobody issued it, nobody holds units of it,
   * and it is on both sides' books at once, so it lives in its own store. Everything else a derived
   * value needs is in the register, which is why this read did not exist — and why a pool with a
   * derivative position would have carried a mark ITS OWN SHARE VALUE HAD NEVER BEEN TOLD ABOUT.
   *
   * The consequence is not a rounding. A fund's equity is zero BY CONSTRUCTION (Fund Shares A3):
   * its claim on itself absorbs whatever its book comes to, because the share kind `owes: 'value'`.
   * The equity ACCOUNT moves with every contract revaluation (`revaluationOfContract`); the share
   * LIABILITY moved with the register only. The two answers diverge by exactly the contract book,
   * and the divergence is a fund with equity — *"a fund with equity has mislaid somebody's money"*.
   * Measured at 83,247,864 on one vehicle the first time funds were let into the contract books.
   *
   * Signed, per contract, in the contract's own money: an asset to one side and a liability to the
   * other at every instant (D1), and the reader splits and converts, because a book is kept in one
   * money and adding two of them is a defect (Currency C4.a).
   */
  contractsOf(
    party: PartyId,
    at: Period,
  ): readonly { readonly worth: Cash; readonly ccy: CurrencyCode }[];
  /**
   * Insurers B2: what money later is worth now, from the market that prices money later. A claim
   * whose value is a SCHEDULE discounted at a rate somebody traded needs this and nothing else —
   * and needing it is what gives the sector duration, which B2.b says it must have.
   */
  curve(family: CurveFamilyId, at: Period): CurveRead;
  /** Money G3.a: the DAY a period starts, because a discount factor is a distance between dates. */
  on(at: Period): Civil;
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
 * module that owns lending registers it at assembly, and the SEAL refuses a world where a kind says
 * this and nobody answers (`World.requireCreditDeciders`) — a refusal that exists because a
 * defaulted-to "no" would look exactly like a bank with a credit standard. At the seal rather than
 * at assembly, and for a reason: the module that answers may be declared after the kind.
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

/**
 * §32, §46, Law 2, item 15: WHAT A PARTY OF THIS KIND IS FOR.
 *
 * `PartyKindProfile` had six fields and every one of them was balance-sheet — how it is
 * represented, whether it issues money, what it can fail on, whether it borrows, what sort of
 * depositor it is. NOTHING SAID WHAT A PARTY IS FOR. A firm maximised nothing, a bank had no
 * franchise to protect, a manager had no career: every participant's reason was hard-coded inside
 * its own module's `orders()`, twenty-one private answers to "why does this party do anything",
 * none declared, none comparable and none checkable.
 *
 * IT IS A DECLARED PREFERENCE AND NOT A UTILITY FUNCTION (Law 2). Nothing here is maximised and
 * nothing takes an argmax over it: it is a fact about the kind, said once, that a mechanism can ASK
 * instead of assuming — and dispatch on through a table, which is what Law 15 asks for and what
 * twenty-one hard-coded reasons were not.
 *
 * There is no `itsOwners` beside `theResidual`, deliberately: the residual IS the owners' claim,
 * and two words for one thing is exactly what Law 4 is about.
 */
export type Objective =
  /** A firm, a dealer's parent: what is left after everybody else has been paid (Equity A1). */
  | 'theResidual'
  /** A household cell, a mutual: the people it stands for, and no residual beyond them (XI-15). */
  | 'itsMembers'
  /** A bank: the business of being a bank tomorrow, which is why it will take a loss today. */
  | 'itsFranchise'
  /** A fund, a central bank, an insurer: what it was set up to do, which somebody else wrote. */
  | 'itsMandate'
  /** A treasury, a probate office, an assessor: a DUTY, and duties are not interests. */
  | 'itsOffice'
  /** A dealer desk, a market maker: the position it is carrying and what it costs to carry. */
  | 'itsBook';

export interface PartyKindProfile {
  readonly id: PartyKindId;
  readonly representation: Representation;
  /**
   * XI-15: WHAT STRATIFIES A POPULATION OF THIS KIND — declared by the kind, because two kinds of
   * cell are two populations and nothing says they are cut the same way.
   *
   * A household is where it lives, when it was born and where it banks; a small firm is where it
   * is, where it banks and what LINE it is in, and has no cohort at all. One list for the whole
   * world made those two mutually exclusive: `cellKeyFaults` refused a household for carrying no
   * `line` and a small firm for carrying one, so this world could hold exactly one population and
   * item 0's sixth stop was every cell the small-business seed tried to add.
   *
   * Present for a cell kind and absent for a named one; the registry refuses either the other way.
   * 0f.3: it is the kind's LATTICE (`registry/lattice.ts`) — its categorical dimensions and its
   * banded ones with their edges — and the key is every dimension of it.
   */
  readonly lattice?: LatticeDecl;
  /**
   * Item 15: what a party of this kind is FOR. Required, so a kind cannot be added without saying
   * — which is the whole of what this buys: the compiler asks the question at every new kind.
   */
  readonly objective: Objective;
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
   * Trade Credit A3, A4, Households C1.d, Law 15 (11.0b): WHETHER A BUYER OF THIS KIND IS SHIPPED
   * ON TERMS AT ALL. A firm pays its supplier in thirty days and a small firm is the tier that
   * lives on that (§36 A4); a household pays for a loaf with money it has, because nobody lends it
   * for one (C1.d), and a fund or an assessor buys nothing that ships. The seller's judgement of
   * the buyer (`shipsOnTerms`) is taken among the kinds this says yes for; before it, the seller
   * judged every buyer, and the only one that ever took terms in the scale model was the state.
   */
  readonly buysOnTerms: boolean;
  /**
   * Banks Funding A1, A1.d: WHICH KIND OF DEPOSITOR a party of this kind is, or null for one that
   * is nobody's deposit base. It is a fact about the kind — many and small, fewer and operational,
   * few and very large — so it is declared with the kind and the funding market asks the profile
   * (Law 15). A table inside that market mapping kinds to classes was that branch written out: a
   * module added a kind and the market kept a list of them.
   *
   * What the class then MEANS TO A BANK — insured to a limit — belongs to the market that holds the
   * taxonomy, because that is a fact about a regulation and not about what the party is. WHETHER a
   * party of this kind moves, and what it costs it, is not here either and is not a flag: it is
   * whether the module that owns the kind declared a `bankChoices` reason for it, which is the same
   * fact and its answer in one place (Law 4).
   */
  readonly depositClass: string | null;
  /**
   * Indices D2, Ratings B2, Sovereign A1: WHOSE CREDIT A PARTY OF THIS KIND BORROWS ON. A state
   * borrows on the state's — it taxes, and its central bank issues the money the debt is in — and
   * everybody else borrows on its own. It is a fact about the kind, declared with the kind (Law 15):
   * a credit index that asked "is the issuer the treasury?" would be that branch written out, and
   * would quietly be wrong the first time a world has two sovereigns or an agency.
   */
  readonly sovereign?: boolean;
}
