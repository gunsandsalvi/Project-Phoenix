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
import type { Order } from '../clearing/solver.js';
import type { MarketDecl } from '../clearing/market.js';
import type { InstrumentKindId, PartyKindId } from '../core/ids.js';
import type { CurveFamilyDecl } from '../prices/curve.js';
import type {
  InstrumentKindProfile,
  OverdraftContext,
  OverdraftDecision,
  PartyKindProfile,
} from '../registry/kinds.js';
import type { ParamDecl } from '../registry/params.js';
import type { UnitDecl } from '../registry/registry.js';
import type { PartyId } from '../core/ids.js';
import type { Option } from '../core/option.js';
import type { Period } from '../calendar/calendar.js';
import type { Instrument } from '../register/instruments.js';
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
  orders(view: ParticipantView, market: MarketDecl): readonly Order[];
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

export interface SystemModule {
  /** Stable id, also the directory name under src/mechanisms or src/seeds. */
  readonly id: string;
  /** The spec system(s) this module implements. */
  readonly spec: string;
  /** Module ids that must be assembled before this one (Part XIII dependency order). */
  readonly requires: readonly string[];
  readonly instrumentKinds: readonly InstrumentKindProfile[];
  readonly partyKinds: readonly PartyKindProfile[];
  /** Curve families this module owns (Sovereign D3.a: one owner, one convention). */
  readonly curveFamilies: readonly CurveFamilyDecl[];
  readonly units: readonly UnitDecl[];
  readonly params: readonly ParamDecl[];
  readonly phases: readonly PhaseDecl[];
  readonly participants: readonly ParticipantDecl[];
  /** Contributions to the audit families (a module may build or extend a family). */
  readonly families: readonly Family[];
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
   * Money B3.a, Banks Lending C3: what this module decides about a customer of a given party kind
   * overdrawn at its issuer. Exactly one module may answer for a kind, and a kind whose profile
   * says its answer is a credit decision must have one — a world where nobody takes it cannot be
   * sealed, because a defaulted-to refusal looks exactly like a bank with a credit standard.
   */
  /** XI-6: what a lot of a kind with no market is worth. Exactly one module answers per kind. */
  readonly marks?: readonly { readonly instrumentKind: InstrumentKindId; readonly value: Valuer }[];
  readonly creditDecisions?: readonly {
    readonly partyKind: PartyKindId;
    readonly decide: CreditDecision;
  }[];
  /**
   * XI-3, Banks Capital C3.b: party kinds whose FAILURE this module takes charge of itself, so the
   * estate does not open one for them. A bank is the case: its liabilities are the money everybody
   * else pays with, an estate cannot owe them (Money A1), and what happens instead is a resolution
   * — a valuation, a hierarchy, an acquirer and a guarantee. Declaring it here is what lets the
   * estate leave a kind alone without knowing which kind it is (Law 15: nothing branches).
   */
  readonly resolves?: readonly PartyKindId[];
  /** Opening state this module contributes (Seed A1); runs in assembly order before the seed audit. */
  seed?(ctx: SeedContext): void;
}
