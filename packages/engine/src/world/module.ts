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
import type { PartyKindId } from '../core/ids.js';
import type { CurveFamilyDecl } from '../prices/curve.js';
import type { InstrumentKindProfile, PartyKindProfile } from '../registry/kinds.js';
import type { ParamDecl } from '../registry/params.js';
import type { UnitDecl } from '../registry/registry.js';
import type { MechanismContext, ParticipantView, SeedContext } from './context.js';

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
  /** Opening state this module contributes (Seed A1); runs in assembly order before the seed audit. */
  seed?(ctx: SeedContext): void;
}
