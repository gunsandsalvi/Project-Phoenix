/**
 * Reach: what this world DECLARED it could do, and what has ever come of it.
 *
 * @spec Audit E1 Audit E2 Audit E3 Part XII Law 15 Appendix C
 *
 * The world already records every refusal it makes — `market.noView`, `noDemand`, `noSupply`,
 * `noOverlap` are journalled and stored — and nothing read them. Meanwhile a participant that
 * declines returns `[]`, which is the same value as one whose code has never executed. So a sector
 * could be assembled, compiled, wired in, cited, marked `MET` and green for a year without ever
 * producing an outcome, and the only way to find out was to read the source: no corporate bond was
 * ever issued, no insurance party ever created, no bank ever quoted, no order ever reached the
 * tenancy venue, eight of nine derivative books never printed.
 *
 * IT IS NOT AN AUDIT FAMILY, AND THAT IS THE SPEC'S POINT. Audit E1: the audit "cannot find an
 * absence — no invariant fires because credit has no price or because a currency market does not
 * exist; there is nothing to be inconsistent with." E2 separates the two jobs: the audit measures
 * CONSISTENCY and the requirement document measures COMPLETENESS, and neither substitutes for the
 * other. A never-reached capability is an absence, so it is a READ — one of the standing
 * measurements about the model the report already carries (Part XII) — and it feeds
 * `docs/COVERAGE.md`, where completeness lives.
 *
 * Two of the seven kinds are TALLIED, because nothing else records them: what a participant posted,
 * and whether a module's own store was ever opened. The other five are DERIVED at read time from
 * the stores that already hold the answer (Law 19: read the source, never keep a second copy of it).
 */
import type { Period } from '../calendar/calendar.js';
import { none, type Option, some } from '../core/option.js';

/** The kinds of thing a world declares it can do. */
export type CapabilityKind =
  | 'participant'
  | 'venueParticipant'
  | 'borrowNeeds'
  | 'market'
  | 'instrumentKind'
  | 'partyKind'
  | 'derivativeKind'
  | 'store';

export interface Capability {
  readonly kind: CapabilityKind;
  readonly id: string;
  /** The module that declared it, so a never-reached one has somebody to answer for it. */
  readonly owner: string;
  /** How many outcomes it has produced since the world opened. */
  readonly produced: number;
  readonly lastAt: Option<Period>;
}

/** The counts, from a list of capabilities — the shape the audit's `Reads` carries. */
export function reachOf(all: readonly Capability[]): ReachSummary {
  const never = all.filter((c) => c.produced === 0).length;
  return { declared: all.length, reached: all.length - never, never };
}

export interface ReachSummary {
  readonly declared: number;
  readonly reached: number;
  /** Declared and never once produced anything. The number this exists to publish. */
  readonly never: number;
}

const key = (kind: CapabilityKind, id: string): string => `${kind}:${id}`;

interface Row {
  readonly kind: CapabilityKind;
  readonly id: string;
  readonly owner: string;
  produced: number;
  lastAt: Option<Period>;
}

export class Reach {
  private readonly rows = new Map<string, Row>();

  /**
   * A capability exists from the moment it is declared, whether or not it ever runs. Declaring it
   * here is what makes "never" a measurable state rather than an absence of evidence.
   */
  declare(kind: CapabilityKind, id: string, owner: string): void {
    const k = key(kind, id);
    if (this.rows.has(k)) return;
    this.rows.set(k, { kind, id, owner, produced: 0, lastAt: none() });
  }

  /** `n` outcomes came out of it in `at`. Zero is a real answer and moves nothing. */
  produced(kind: CapabilityKind, id: string, n: number, at: Period): void {
    if (n <= 0) return;
    const row = this.rows.get(key(kind, id));
    if (row === undefined) return;
    row.produced += n;
    row.lastAt = some(at);
  }

  /** Whether this capability has ever produced anything, for a derived kind to fold in. */
  fold(kind: CapabilityKind, reached: ReadonlySet<string>, at: Period): void {
    for (const row of this.rows.values()) {
      if (row.kind !== kind || !reached.has(row.id)) continue;
      row.produced += 1;
      row.lastAt = some(at);
    }
  }

  all(): readonly Capability[] {
    return [...this.rows.values()].map((r) => ({ ...r }));
  }

  /** Declared, and nothing has ever come of it. Ordered by owner so a module reads as a block. */
  never(): readonly Capability[] {
    return this.all()
      .filter((c) => c.produced === 0)
      .sort((a, b) => (a.owner === b.owner ? a.id.localeCompare(b.id) : a.owner.localeCompare(b.owner)));
  }

  summary(): ReachSummary {
    const all = this.all();
    const never = all.filter((c) => c.produced === 0).length;
    return { declared: all.length, reached: all.length - never, never };
  }
}
