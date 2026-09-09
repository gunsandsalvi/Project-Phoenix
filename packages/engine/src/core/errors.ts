/**
 * The engine's error discipline (docs/ARCHITECTURE.md §5).
 *
 * A contract violation — something impossible by construction — throws one of these at the site and
 * is never caught inside the engine. Every error carries the specification clause it enforces, so a
 * stack trace names the law that was broken.
 *
 * Invariant violations (a false statement about the state) are NOT errors: they are audit findings
 * (audit/), reported and never repaired (Audit C4).
 */

export type SpecCitation = string;

export class PhoenixError extends Error {
  readonly spec: SpecCitation;
  readonly details: Readonly<Record<string, unknown>>;

  constructor(spec: SpecCitation, message: string, details: Record<string, unknown> = {}) {
    super(`[${spec}] ${message}`);
    this.name = new.target.name;
    this.spec = spec;
    this.details = Object.freeze({ ...details });
  }
}

/** A number that is not finite tried to enter the state (Law 7: arithmetic must be accountable). */
export class NonFinite extends PhoenixError {}

/** Two things that may not be combined were combined: currencies, units, periods, periodicities. */
export class Mismatch extends PhoenixError {}

/** Something that must exist does not. Missing is missing, never zero (Appendix A). */
export class Missing extends PhoenixError {}

/** An operation the specification forbids by construction was attempted (a FORBID clause). */
export class Forbidden extends PhoenixError {}

/** A phase read a number the period has not yet produced (Clearing F1.a: the fix is the order). */
export class NotYetProduced extends PhoenixError {}

/** A value was asked of an instrument that has no price and is not declared carried-at-cost (XI-6). */
export class Unpriced extends PhoenixError {}

/** An arithmetic impossibility: a negative count, a fractional count, a share above its whole (Law 6). */
export class Impossible extends PhoenixError {}

/** Registry or parameter data failed validation at construction (Law 2, Law 15, XI-14). */
export class InvalidRegistry extends PhoenixError {}
