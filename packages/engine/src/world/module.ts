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
import type { Family } from '../audit/audit.js';
import type { IndexDecl } from '../prices/index-read.js';
import type { Order } from '../clearing/solver.js';
import type { MarketDecl, MarketKind } from '../clearing/market.js';
import type { VenueDecl } from '../clearing/venue.js';
import type { InstrumentKindId, MarketId, PartyKindId } from '../core/ids.js';
import type { CurveFamilyDecl } from '../prices/curve.js';
import type {
  InstrumentKindProfile,
  OverdraftContext,
  OverdraftDecision,
  PartyKindProfile,
} from '../registry/kinds.js';
import type { DerivativeKindProfile } from '../registry/derivatives.js';
import type { ParamDecl, ParamRegister } from '../registry/params.js';
import type { UnitDecl } from '../registry/registry.js';
import type { CurrencyCode, PartyId } from '../core/ids.js';
import type { Option } from '../core/option.js';
import type { Period } from '../calendar/calendar.js';
import type { Instrument } from '../register/instruments.js';
import type { Leg } from '../ledger/instruction.js';
import type {
  MechanismContext,
  Outlook,
  OutlookVariable,
  ParticipantView,
  SeedContext,
} from './context.js';

/** The kernel's own phases, which a module's phase is anchored to (Clearing F1: a stated point). */
export type KernelPhase = 'corporateActions' | 'markets' | 'revaluation';

export interface PhaseDecl {
  readonly name: string;
  readonly spec: string;
  /**
   * The settlement cycle this phase runs in (Money G2); must not run before an earlier phase's
   * cycle. 'anchor' means the same cycle as the phase it is anchored to, which is how a module says
   * "with that one" without knowing how many cycles this world's calendar has.
   */
  readonly cycle: number | 'anchor';
  /** Where in the period it runs, relative to a kernel phase or another module's phase. */
  readonly anchor: { readonly before: string } | { readonly after: string };
  run(ctx: MechanismContext): void;
}

/**
 * A participant's reason to be in a market (Clearing B2), evaluated per party of the kind with only
 * that party's own view (Observer A4, Expectations D1): a schedule cannot be written against
 * something the party may not see.
 */
export interface ParticipantDecl {
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
 * `none` is staying where it is. A kind whose profile says it chooses its bank must have exactly
 * one module answering, or the world cannot be sealed: a depositor nobody asks is a depositor that
 * can never leave, which is A1.d's stickiness made invisible instead of paid for.
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
) => Option<number>;

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
  readonly dueNext?: (ctx: MechanismContext, party: PartyId, ccy: CurrencyCode, at: Period) => number;
  /** D9, C3.a: the legs that post it — money out, a claim in, never an expense. */
  margin(
    ctx: MechanismContext,
    party: PartyId,
    against: PartyId,
    size: number,
    about: ContractAsk,
  ): readonly Leg[];
}

/** What a market is asking about: the book, the level it cleared at, and the kind of contract. */
export interface ContractAsk {
  readonly market: MarketDecl;
  readonly struck: number;
}

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
  readonly partyKinds: readonly PartyKindProfile[];
  /** Curve families this module owns (Sovereign D3.a: one owner, one convention). */
  readonly curveFamilies: readonly CurveFamilyDecl[];
  readonly units: readonly UnitDecl[];
  readonly params: readonly ParamDecl[];
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
  indices?(params: Pick<ParamRegister, 'get' | 'amount'>): readonly IndexDecl[];
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
   * XI-3, Banks Capital C3.b: party kinds whose FAILURE this module takes charge of itself, so the
   * estate does not open one for them. A bank is the case: its liabilities are the money everybody
   * else pays with, an estate cannot owe them (Money A1), and what happens instead is a resolution
   * — a valuation, a hierarchy, an acquirer and a guarantee. Declaring it here is what lets the
   * estate leave a kind alone without knowing which kind it is (Law 15: nothing branches).
   */
  readonly resolves?: readonly PartyKindId[];
  /**
   * Derivative Layer E1-E3, D9: what a member may carry and what it posts against it. Exactly one
   * module may answer, and a world with a contract market and no answer cannot be sealed.
   */
  readonly clearingCapacity?: ClearingCapacity;
  /** Opening state this module contributes (Seed A1); runs in assembly order before the seed audit. */
  seed?(ctx: SeedContext): void;
}
