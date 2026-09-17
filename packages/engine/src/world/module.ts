/**
 * The system module contract: how a system plugs into the kernel.
 *
 * @spec Law 15 Granularity Part XIII Clearing A3 Clearing B2 Audit B8
 *
 * A module owns one system of the specification (or one instrument family, or one seed). It
 * declares what it adds, data and profiles, and the phases, participants and audit contributions it
 * runs. It never imports another module (lint: phoenix/no-cross-module-import) and never holds a
 * reference to a kernel store: everything it reads and writes goes through the contexts in
 * world/context.ts. Replacing a system is replacing its module.
 */
import type { QuestionDecl } from '../registry/questions.js';
import type { Qty } from '../core/tick.js';
import type { Cash, PerPiece } from '../core/measure.js';
import type { Family } from '../audit/audit.js';
import type { IndexDecl } from '../prices/index-read.js';
import type { Order } from '../clearing/solver.js';
import type { ContractMarketDecl, MarketDecl, MarketKind } from '../clearing/market.js';
import type { VenueDecl } from '../clearing/venue.js';
import type { InstrumentId, InstrumentKindId, MarketId, PartyKindId, RegionId } from '../core/ids.js';
import type { CurveFamilyDecl } from '../prices/curve.js';
import type { DerivativeKindId } from '../core/ids.js';
import { partyKindId } from '../core/ids.js';
import type {
  InstrumentKindProfile,
  OverdraftContext,
  OverdraftDecision,
  PartyKindProfile,
} from '../registry/kinds.js';
import type { DerivativeKindProfile, StruckAt } from '../registry/derivatives.js';
import type { ParamDecl, ParamOwner, ParamRegister } from '../registry/params.js';
import type { NounEntry } from '../registry/nouns.js';
import type { EventKind } from '../journal/journal.js';
import type { AgreementKindDecl } from '../register/agreements.js';
import type { UnitDecl } from '../registry/registry.js';
import type { CurrencyCode, PartyId } from '../core/ids.js';
import type { Option } from '../core/option.js';
import type { Period } from '../calendar/calendar.js';
import type { Instrument } from '../register/instruments.js';
import type { Leg } from '../ledger/instruction.js';
import type {
  BorrowNeed,
  DerivativeClassDecl,
  MechanismContext,
  Outlook,
  OutlookVariable,
  ParticipantView,
  SeedContext,
  WorldReads,
} from './context.js';

export type { DerivativeClassDecl } from './context.js';

/** The kernel's own phases, which a module's phase is anchored to (Clearing F1: a stated point). */
export type KernelPhase = 'corporateActions' | 'markets' | 'revaluation';

/**
 * Law 10, Clearing F1.a: WHAT A PHASE NEEDS OF THE PERIOD IT IS IN, and what it puts into it.
 *
 * Measured before it was designed (item 0a): both worlds were stepped with every journal and price
 * read attributed to the running phase. Two kinds of dependency came out of it and no more.
 *
 * A PRINT HAS ONE WRITER. `runOne` is called from the kernel `markets` phase and from nowhere else,
 * so every cleared price in this world is written there and a phase that reads THIS PERIOD'S runs
 * after it. There is no family to name: naming one would be a second answer to a question the
 * source settles. Reading an EARLIER print — which is what most of this world does — needs nothing
 * of this period and is `anyPeriod` like any other look at history.
 *
 * AN EVENT READ CARRIES ITS PERIOD, and that is the difference between an order and a cycle. Eight
 * phases read the kind they themselves write — a bank's last deposit rate, a fund's last strike, a
 * desk's last estimate — and each is a read of HISTORY. `anyPeriod` says so and orders nothing;
 * `thisPeriod` says the writer must already have run, and is the only thing that is an edge.
 */
export type Dependency =
  | { readonly kind: 'event'; readonly name: EventKind; readonly of: When }
  | { readonly kind: 'print'; readonly of: When };

/**
 * WHICH PERIOD A READ IS OF, and it is the difference between an order and a cycle.
 *
 * `anyPeriod` is a read of HISTORY and orders nothing: a bank's last deposit rate, a fund's last
 * strike, yesterday's close. Eight phases read the very kind they write, and every one of them this
 * way — a dataflow order blind to the period would report each as a cycle. `thisPeriod` says the
 * writer must already have run, and is the only thing that is an edge.
 */
export type When = 'thisPeriod' | 'anyPeriod';

/** What a phase puts into the period. A write is always of this period; there is no other kind. */
export interface Produces {
  readonly kind: 'event';
  readonly name: EventKind;
}

export interface PhaseDecl {
  readonly name: string;
  readonly spec: string;
  /**
   * Law 10: WHERE IN THE PERIOD IT RUNS, against a kernel phase or another module's.
   *
   * It is not derivable and item 0a's premise that it would be is wrong, which the measurement
   * showed. `goods.spoilage` runs after the period's trades and before the marking — E4's own
   * words — and no read or write says so: it reads holdings and writes holdings exactly as
   * `markets`, `firms.produce` and forty others do, so a holdings dependency makes every pair of
   * them mutually dependent and the whole graph is one cycle. The three kernel acts are world-wide
   * MOMENTS, and where a module sits against them is a fact only that module has.
   *
   * What IS derived is the settlement cycle (Money G2) and the order among the siblings of one
   * anchor, both in `world/order.ts` and both out of `reads` and `writes` below.
   */
  readonly anchor: { readonly before: string } | { readonly after: string };
  /** What it needs of this period. A `thisPeriod` event or a print orders it; nothing else does. */
  readonly reads: readonly Dependency[];
  /** What it puts into the period, so another phase can say it needs it. */
  readonly writes: readonly Produces[];
  run(ctx: MechanismContext): void;
}

/**
 * A participant's reason to be in a market (Clearing B2), evaluated per party of the kind with only
 * that party's own view (Observer A4, Expectations D1): a schedule cannot be written against
 * something the party may not see.
 */
/**
 * Law 15, Spot FX B1, B2 (16.6): A PARTICIPANT ASKED OF EVERY KIND THERE IS. A reason that any party
 * can have — owing a money it has not got — is not a list of the kinds that have it; a module that
 * declares one with this kind is expanded at assembly into one declaration per kind the registry
 * knows, so a kind added later is asked without anybody editing a list.
 */
export const EVERY_PARTY_KIND: PartyKindId = partyKindId('*');

export interface ParticipantDecl {
  /** The module that declared it, stamped by the kernel so a never-reached one has an owner. */
  readonly owner?: string;
  /** The kind it is asked of, or `EVERY_PARTY_KIND` for all of them (16.6). */
  readonly partyKind: PartyKindId;
  /**
   * Spot FX D1, Clearing B2, Law 15: WHICH SORT OF MARKET THIS PARTICIPANT IS ASKED ABOUT — the
   * same dispatch key `MARKET_KINDS` runs on, and absent means `asset`, which is what every
   * participant declared before pairs existed.
   *
   * A pair market delivers nothing anybody holds, so a desk that trades instruments has no schedule
   * to post in one and is never asked for one; a currency desk has nothing to say about a bond.
   * Asking everybody about everything and letting each one discover it holds no view is how a
   * participant ends up looking up an instrument behind a market that has none.
   */
  readonly in?: MarketKind;
  /**
   * Law 18, Clearing B2: WHICH MARKETS THIS PARTY COULD BE IN AT ALL THIS CYCLE.
   *
   * A session asks every party of a kind whether it has an order in it, and at the real scale that
   * is 261 markets against 3,169 parties — 827,000 questions a period whose answer is almost always
   * no, and four fifths of what a period costs. THE KERNEL CANNOT GUESS THE ANSWER: which books a
   * party is in is its own business and changes period to period, so the only place it can come
   * from is the module that owns the party.
   *
   * It is a READ, never a second copy (Law 19, Law 4): a participant answers out of the same thing
   * its `orders` answers out of — the plan that party published this period — so a market it names
   * here and a market it posts in cannot disagree. Omitted means every market of its declared kind,
   * which is what every participant did before this door existed and is right for a kind with few
   * parties or a party in every book.
   *
   * It may name a market that does not exist; it is a filter and not a claim about the world.
   */
  readonly markets?: (view: ParticipantView) => readonly MarketId[];
  /**
   * Law 18, Clearing B2: A BOOK EVERY PARTY OF THE KIND IS ASKED ABOUT, whatever `markets` said.
   *
   * `markets` is what a party can say about itself, and some books cannot be answered that way: a
   * future on a thing this party holds none of is still one it would take a view in ONCE THE BOOK
   * HAS PRINTED, because then there is a level for its own number to be above or below. That is a
   * fact about the BOOK and not about any party, so it is settled once for the book instead of by
   * every party discovering it (which is how a narrowing that only had the party's side would lose
   * an order somebody would have posted).
   *
   * Absent means no: a book is asked of the parties that named it. It is a TRAVERSAL both ways —
   * what it adds must be what `orders` could answer in, and what `markets` leaves out must be what
   * `orders` returns nothing in.
   */
  readonly everyone?: (m: MarketDecl, reads: WorldReads) => boolean;
  /**
   * XI-13: WHETHER THIS PARTICIPANT IS IN THE BOOK BECAUSE IT HAS A VIEW — it names a level from
   * what it thinks the thing is worth, puts its own money behind that, and takes the loss when it is
   * wrong. It is what `market.noView` counts, and what a second opinion is made of.
   *
   * IT BELONGS TO THE PARTICIPANT AND NOT TO THE PARTY KIND, which is what worklist 12c measured
   * rather than assumed. It was a flag on the kind, and a kind is the wrong owner: a bank's dealing
   * desk has a view and the same bank's treasury funding itself does not, and they are one party of
   * one kind. A household saving had no view in any book while its reason was a rule — and the
   * moment it prices a share off what that company published it owns net of what it owes, it has
   * one, in that book and not in the others. Whether a party is there for a view is a fact about WHY
   * IT IS POSTING, so it is declared where the posting is declared.
   *
   * Absent means it is not: a participant that does not claim a view does not have one counted.
   */
  readonly speculative?: boolean;
  orders(view: ParticipantView, market: MarketDecl): readonly Order[];
}

/**
 * Clearing B2, Observer A4: THE SAME DOOR, FOR A VENUE. A venue is where something is struck that is
 * not the transfer of an instrument — a job at a wage, a week of money at a rate — so the module
 * that opened it clears it itself. That is the CLEARING; the SCHEDULES are still the participants'.
 *
 * Without this door a venue's module builds every party's schedule inside its own phase, out of a
 * `MechanismContext` that can see every party's private state — which is one module deciding for
 * parties it does not own, with a view no participant may have (A4). With it, a party's schedule
 * into a venue comes from the module that owns that party, evaluated with that party's own view,
 * exactly as a market's does; the venue's module asks for them (`gather`) and clears what it gets.
 */
export interface VenueParticipantDecl {
  /** The module that declared it, stamped by the kernel so a never-reached one has an owner. */
  readonly owner?: string;
  readonly partyKind: PartyKindId;
  orders(view: ParticipantView, venue: VenueDecl): readonly Order[];
}

/**
 * Money B3.a, Banks Lending C3: what a module decides about a customer overdrawn at an issuer of
 * its kind. The kernel calls it through the module's own context — the same way the outlook door
 * works — so the decision is taken with the module's own state and the issuer's own view, which is
 * what a credit decision is made of.
 */
export type CreditDecision = (
  ctx: MechanismContext,
  o: OverdraftContext,
) => OverdraftDecision;

/**
 * Banks Funding A1, E1, Observer A4: WHERE A DEPOSITOR BANKS IS THE DEPOSITOR'S DECISION, and the
 * module that owns its kind is the one that takes it, with that party's own view.
 *
 * It is the same door a market's `participants` and a venue's `venueParticipants` are, for the same
 * reason (Clearing B2): the module that runs the deposit market used to walk every party in the
 * world and decide for each of them out of a `MechanismContext` that can see private state no
 * depositor may have. And the reasons are not one reason wearing three names — A1.a's retail money
 * is insured and sticky, A1.b's corporate money banks where it transacts, A1.c's wholesale money is
 * in the market all day and leaves first — so each kind's is written where that kind lives.
 *
 * `none` is staying where it is. A kind whose profile says it is a depositor must be answered by
 * the module that DECLARES it, and `requireBankChoices` refuses that module at assembly if it is
 * not: a depositor nobody asks is a depositor that can never leave, which is A1.d's stickiness made
 * invisible instead of paid for. At assembly and per module, because the kind and the answer are
 * the same module's to give.
 */
export interface BankChoice {
  readonly to: PartyId;
  readonly reason: string;
}

export interface BankChoiceDecl {
  readonly partyKind: PartyKindId;
  chooses(view: ParticipantView): Option<BankChoice>;
}

/**
 * XI-6, Banks Lending D1, D2: what a lot of a kind that has NO MARKET is worth to whoever holds it.
 *
 * Almost everything is worth what a market said (XI-6), and the kernel reads that from the price
 * store. A loan is the exception the spec names: it is not a security, it has no market price, and
 * D1 says it is carried at amortised cost — less what its holder expects to lose on it, which is
 * that holder's own assessment (D2) and cannot be anybody else's. So the module that owns the kind
 * answers, exactly one module per kind, and the kernel books the difference as the provision:
 * charged to income, visible, and never a reserve sitting beside the loan absorbing things (D2.b).
 */
export type Valuer = (
  ctx: MechanismContext,
  instrument: Instrument,
  at: Period,
) => Option<PerPiece>;

export interface OutlookProvider {
  of(ctx: MechanismContext, party: PartyId, variable: OutlookVariable): Option<Outlook>;
  /** A2: the variables this party has actually observed, in the order the module keeps them. */
  variables(ctx: MechanismContext, party: PartyId): readonly OutlookVariable[];
}

/**
 * Derivative Layer E1, E2, E3, D9, C3.a: WHO SAYS HOW MUCH A MEMBER CAN TAKE, and what it posts.
 *
 * Both are the layer's and neither is the kernel's. What a member holds back as a buffer is its own
 * preference; what it has already committed this period is the layer's own record; and what a
 * margin claim IS, is the layer's instrument. So the module that owns the layer answers, exactly
 * one of them per world, and the market cuts a trade to the smaller of the two sides' answers and
 * puts the margin in the same instruction (E2: the cut happens at the strike).
 *
 * A world with a contract book and nobody answering cannot be sealed: a defaulted-to "as much as
 * you like" is E4's limit raised by omission, which is the one thing that clause forbids.
 */
export interface ClearingCapacity {
  /** E1, E3: how much of `wanted` this party can carry, in the kind's own unit. */
  admits(ctx: MechanismContext, party: PartyId, wanted: number, about: ContractAsk): number;
  /**
   * D2, D9, Money Market A2: WHAT THIS PARTY'S OPEN ROWS WILL ASK IT TO POST, in one money.
   *
   * Margin is an asset swap and not an expense (C3.a), but the money still leaves the account — so
   * a treasury that cannot see it coming funds itself for everything except the one call it is
   * about to get. The kernel cannot work it out: what a margin claim IS, is this module's
   * instrument (Law 15). So the module that owns the layer answers, and the party reads it through
   * its own view like everything else it knows about its own book.
   */
  readonly dueNext?: (ctx: MechanismContext, party: PartyId, ccy: CurrencyCode, at: Period) => Cash;
  /** D9, C3.a: the legs that post it — money out, a claim in, never an expense. */
  margin(
    ctx: MechanismContext,
    party: PartyId,
    against: PartyId,
    size: Qty,
    about: ContractAsk,
  ): readonly Leg[];
}

/**
 * What a market is asking about: the book, the level it cleared at, and the kind of contract. The
 * market is a CONTRACT market, because only a contract market asks — so the terms and the kind are
 * on it and the layer has nothing to narrow (item 13b.1).
 */
export interface ContractAsk {
  readonly market: ContractMarketDecl;
  /** Law 8, E-10: the level, tagged with what its kind quotes in — a price, or a rate. */
  readonly struck: StruckAt;
}

/**
 * Trade Credit A1, A3 (13e): THE SALE A SELLER IS BEING ASKED TO SHIP ON TERMS. What is sold is
 * part of it, because terms are what a supplier gives a customer for goods; a firm selling its own
 * paper is raising money and is paid for it (Clearing B2), and the seller's decision says so.
 */
export interface TermsSale {
  readonly seller: PartyId;
  readonly buyer: PartyId;
  readonly ccy: CurrencyCode;
  readonly cash: number;
  readonly sold: InstrumentId;
}

export type TermsDecision = (ctx: MechanismContext, sale: TermsSale) => Option<InstrumentId>;

/**
 * Securities Lending B1: what this party must deliver that it has not got, evaluated with its own
 * view (Observer A4). Nothing is a party with no reason to be short, which is most of them.
 */
export type BorrowNeeds = (view: ParticipantView) => readonly BorrowNeed[];

export interface SystemModule {
  /** Stable id, also the directory name under src/mechanisms or src/seeds. */
  readonly id: string;
  /** The spec system(s) this module implements. */
  readonly spec: string;
  /** Module ids that must be assembled before this one (Part XIII dependency order). */
  readonly requires: readonly string[];
  readonly instrumentKinds: readonly InstrumentKindProfile[];
  /**
   * Derivative X1, Law 15: kinds of CONTRACT this module owns. A derivative is not a holding and
   * not an instrument, so it is declared apart — and it is owned by exactly one module and asked
   * through one profile, like everything else the kernel must not branch on.
   */
  readonly derivativeKinds?: readonly DerivativeKindProfile[];
  /** What the classes this module owns know that the kernel does not (`DerivativeClassDecl`). */
  readonly derivativeClasses?: readonly DerivativeClassDecl[];
  readonly partyKinds: readonly PartyKindProfile[];
  /** Curve families this module owns (Sovereign D3.a: one owner, one convention). */
  readonly curveFamilies: readonly CurveFamilyDecl[];
  readonly units: readonly UnitDecl[];
  readonly params: readonly ParamDecl[];
  /**
   * Law 15, Law 2: EVERY STORE THIS MODULE KEEPS in `ctx.state`, declared.
   *
   * Absent means it keeps none, and that is checked rather than assumed: `ctx.state` refuses a name
   * this list does not carry. A store that is really an economic NOUN the kernel has no home for
   * names the plan item that gives it one, the way a placeholder parameter names the mechanism that
   * deletes it — and for the same reason (`registry/nouns.ts`).
   */
  readonly nouns?: readonly NounEntry[];
  readonly phases: readonly PhaseDecl[];
  readonly participants: readonly ParticipantDecl[];
  /** Clearing B2: the schedules this module's parties post into venues other modules clear. */
  readonly venueParticipants?: readonly VenueParticipantDecl[];
  /** Contributions to the audit families (a module may build or extend a family). */
  readonly families: readonly Family[];
  /**
   * Indices A1, D5: the index rules this module states. An index is a RULE over constituents, and
   * the rule is data (Law 15); what it comes to is a read (`view.index`), applied in one place so
   * two readers cannot get two levels. One system of them across the world (D5), which is what a
   * single registry at assembly gives: a second module declaring the same id is refused.
   */
  indices?(
    params: Pick<ParamRegister, 'periods' | 'days' | 'months' | 'years' | 'count' | 'ratio' | 'perAnnum' | 'price' | 'pricePerUnit' | 'amount'>,
    /** Currency C4 (16.0): the money a place's lines are stated in, so a rule can name the money its level is in. */
    registry: { currencyOf(region: RegionId): CurrencyCode },
  ): readonly IndexDecl[];
  /**
   * Expectations A2, XI-16: what a party expects. Exactly one module may answer this — an
   * expectation is a fact about a party and has one writer (Law 4) — and the kernel asks it
   * through that module's own context, so the shape of what it keeps stays its own.
   *
   * It answers two questions and they are one door: what a party expects OF a variable, and which
   * variables it has an outlook of at all. A surface that could ask the first but not the second
   * would have to guess the names, and a guessed name is a default outlook by another route (A2:
   * a party that never observed a variable has no outlook of it).
   */
  readonly outlooks?: OutlookProvider;
  /**
   * Observer A5, OB5 (0e′.5): A MEASURE THE OBSERVER SHOWS, answered by the module that owns the facts
   * it is made of. The observer imports no module; it asks a world-scoped question through the one
   * door (`registry/questions.ts`) and the module that declares the answer here is the one writer.
   * The function's type is the asker's business, as with every other answer.
   */
  readonly measures?: readonly { readonly question: QuestionDecl; readonly fn: unknown }[];
  /** XI-6: what a lot of a kind with no market is worth. Exactly one module answers per kind. */
  readonly marks?: readonly { readonly instrumentKind: InstrumentKindId; readonly value: Valuer }[];
  /**
   * Money B3.a, Banks Lending C3: what this module decides about a customer of a given party kind
   * overdrawn at its issuer. Exactly one module may answer for a kind, and a kind whose profile
   * says its answer is a credit decision must have one — a world where nobody takes it cannot be
   * sealed, because a defaulted-to refusal looks exactly like a bank with a credit standard.
   */
  readonly creditDecisions?: readonly {
    readonly partyKind: PartyKindId;
    readonly decide: CreditDecision;
  }[];
  /**
   * Banks Funding E1, Observer A4: where a depositor of a kind this module owns banks, and why it
   * would move. Exactly one module may answer for a kind, and a kind whose profile says it chooses
   * its bank must have one.
   */
  readonly bankChoices?: readonly BankChoiceDecl[];
  /**
   * Trade Credit A1, A3, B5 (13e): WHETHER A SELLER OF THIS KIND SHIPS ON TERMS, and what it takes
   * instead of money. Exactly one module may answer for a kind, and a kind nobody answers for sells
   * for cash — which is what every market did before there was any such thing.
   *
   * It is the same door `creditDecisions` and `bankChoices` are, for the same reason (Clearing B2):
   * extending credit to a customer is the SELLER's judgement of that customer, taken with the
   * seller's own view of it, and the kernel has no business guessing at it. What comes back is the
   * ROW to write — the module issues it, because an invoice is its instrument — and the kernel
   * writes the leg, because settlement is the one writer of a movement.
   */
  readonly termsOffered?: readonly {
    readonly partyKind: PartyKindId;
    readonly decide: TermsDecision;
  }[];
  /**
   * Securities Lending B1, A5.a, Observer A4: WHAT A PARTY OF THIS KIND MUST BORROW, and the most
   * it will pay for it. Exactly one module may answer for a kind, and a kind nobody answers for
   * never borrows — which is what every kind did while the borrow book had no way in (`A-67`).
   *
   * It is the same door `termsOffered`, `creditDecisions` and `bankChoices` are, for the same
   * reason: being short is a POSITION a party took for a reason of its own, and the module that
   * clears the fee has no business inventing one. What comes back is the NEED — a line, a size, a
   * money, a reservation and what it will pledge; the lending module strikes the fee against every
   * other need in the same line and writes the loan, because the loan is its instrument.
   */
  /**
   * XI-8, Law 15 (item 9.1): THE KINDS OF COMMITMENT THIS MODULE OWNS — an employment, a lease, an
   * invoice, a stock loan, a covenant, a deal, a mandate, and every arrear a mechanism can leave.
   *
   * It is the same declaration `instrumentKinds` and `partyKinds` are, for the same reason: the
   * kernel holds the row and ranks it in an estate, and only the module that declared the kind
   * knows what its terms mean. Before it, a kind was a free-text `what` — six spellings of
   * "in arrears" across six modules, none of which any reader could dispatch on.
   */
  readonly agreementKinds?: readonly AgreementKindDecl[];
  /**
   * Fund Shares A3, `B-14` (item 9.7): WHAT A PARTY OF THIS KIND MAY TAKE A POSITION IN.
   *
   * Exactly one module may answer for a kind, and a kind nobody answers for may trade anything —
   * which is what every kind did before mandates existed, and is right: the absence of a rule is
   * not a prohibition. What a POOL may hold is its mandate's (`funds.mandate`), and a bank is under
   * no mandate at all.
   *
   * It is asked through `ParticipantView.mayTrade`, by the derivative layer, before the layer
   * speaks for a party in a book — because the layer owns the book and the party's own module owns
   * the party, and neither may answer the other's question (Observer A4, Law 4).
   */
  readonly tradingLimits?: readonly {
    readonly partyKind: PartyKindId;
    readonly mayTrade: (view: ParticipantView, kind: DerivativeKindId) => boolean;
  }[];
  /**
   * Hedge Funds B1, Prime Brokerage B2, Fund Shares F2 (item 13.3): WHETHER A PARTY OF THIS KIND
   * MAY BORROW AT ALL — the same door as `tradingLimits`, for the same reason and with the same
   * rule: exactly one module answers for a kind, and a kind nobody answers for is whatever its
   * PROFILE says, because the absence of a rule is not a prohibition.
   *
   * `PartyKindProfile.borrows` is the CATEGORY's answer — whether a thing of this sort is capable
   * of owing money — and it is the wrong place for a fact about one party. A pool's is its
   * MANDATE's: *"leverage is a fact about a loan, never a property of the fund"* (B1), and which
   * pools may be levered is what their investors agreed, one mandate at a time. Saying it on the
   * kind meant no pool anywhere could ever be levered, which is what 13h was built on.
   *
   * It is asked through `ParticipantView.mayBorrow`, by a lender deciding whether to offer — the
   * lender owns the loan and the party's own module owns the party, and neither may answer the
   * other's question (Observer A4, Law 4).
   */
  readonly leverageLimits?: readonly {
    readonly partyKind: PartyKindId;
    readonly mayBorrow: (view: ParticipantView) => boolean;
  }[];
  /**
   * Hedge Funds C1, Fund Shares A3 (item 13.2b): WHAT A PARTY OF THIS KIND HAS BEHIND A POSITION IT
   * TAKES ON ITS OWN ACCOUNT — the third door of the same shape, and the same rule: exactly one
   * module answers for a kind, and a kind nobody answers for stands behind a position with its own
   * EQUITY ACCOUNT, which is what a loss on it would fall on.
   *
   * That default is right for a bank, a firm and a household and **wrong for a pool, by
   * construction**. A fund's equity is zero (A3: the holders own the assets, so assets minus
   * liabilities is nothing), and every speculative term in every contract class in this world sized
   * itself by `view.equity()` — so a hedge fund, *"the natural home of the speculative side of every
   * derivative book"* (§28 C1), could take a position of exactly nothing in any of them. What stands
   * behind a pool's position is its investors' money, which is its NAV, and only the module that
   * runs pools can say so.
   */
  readonly riskBearing?: readonly {
    readonly partyKind: PartyKindId;
    readonly standsBehind: (view: ParticipantView) => Cash;
  }[];
  readonly borrowNeeds?: readonly {
    readonly partyKind: PartyKindId;
    readonly needs: BorrowNeeds;
  }[];
  /**
   * XI-3, Banks Capital C3.b: party kinds whose FAILURE this module takes charge of itself, so the
   * estate does not open one for them. A bank is the case: its liabilities are the money everybody
   * else pays with, an estate cannot owe them (Money A1), and what happens instead is a resolution
   * — a valuation, a hierarchy, an acquirer and a guarantee. Declaring it here is what lets the
   * estate leave a kind alone without knowing which kind it is (Law 15: nothing branches).
   */
  readonly resolves?: readonly PartyKindId[];
  /**
   * XI-14, §47 D5 (19.1): THE MANDATES THIS MODULE SPEAKS FOR — the policy owners whose numbers it
   * may move through `ctx.setByMandate`. A mandate belongs to one institution, so exactly one
   * module may act for it: the money market speaks for the central bank, the polity will speak for
   * parliament, and a module that claims one somebody else has answered does not assemble.
   */
  readonly mandates?: readonly ParamOwner[];
  /**
   * Derivative Layer E1-E3, D9: what a member may carry and what it posts against it. Exactly one
   * module may answer, and a world with a contract market and no answer cannot be sealed.
   */
  readonly clearingCapacity?: ClearingCapacity;
  /** Opening state this module contributes (Seed A1); runs in assembly order before the seed audit. */
  seed?(ctx: SeedContext): void;
}
