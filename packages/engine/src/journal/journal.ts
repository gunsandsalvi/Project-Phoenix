/**
 * The journal: events generated FROM state transitions by the engine (Observer B2). It is a read of
 * what happened, never an input to anything (B2.a: news never causes anything).
 *
 * @spec Observer B1 Observer B2 Observer B2.a Observer B3 Observer A3 Observer A4 Observer D1 Money G4 Money Market D5.a
 *
 * An event is public or private (Observer A3, A4): a print, a weight event, a cessation, a facility
 * draw and the audit's counts are public; an instruction between two parties is theirs.
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import type { EventId } from '../core/ids.js';

/** Kernel event kinds; modules add their own as `<module>.<event>`. */
export type EventKind =
  | 'instruction.settled'
  | 'instruction.failed'
  | 'reserve.overdraft'
  | 'weight'
  | 'print'
  | 'revaluation'
  | 'party.ceased'
  | 'instrument.ceased'
  | 'audit'
  | `${string}.${string}`;

export interface Event {
  readonly id: EventId;
  readonly period: Period;
  readonly cycle: Cycle;
  readonly kind: EventKind;
  /** B3: named subjects, so the event can be checked against the state. */
  readonly subjects: readonly string[];
  readonly data: Readonly<Record<string, unknown>>;
  /** A3/A4: whether any observer may see it, or only its subjects. */
  readonly public: boolean;
}

export class Journal {
  private readonly events: Event[] = [];
  private next = 1;

  record(
    period: Period,
    cycle: Cycle,
    kind: EventKind,
    subjects: readonly string[],
    data: Record<string, unknown>,
    isPublic: boolean,
  ): Event {
    const ev: Event = Object.freeze({
      id: this.next as EventId,
      period,
      cycle,
      kind,
      subjects: [...subjects],
      data: Object.freeze({ ...data }),
      public: isPublic,
    });
    this.next += 1;
    this.events.push(ev);
    return ev;
  }

  all(): readonly Event[] {
    return this.events;
  }

  inPeriod(period: Period): readonly Event[] {
    return this.events.filter((e) => e.period === period);
  }

  ofKind(kind: EventKind): readonly Event[] {
    return this.events.filter((e) => e.kind === kind);
  }

  tail(n: number): readonly Event[] {
    return this.events.slice(-n);
  }

  /**
   * The most recent events of ONE kind that a viewer may see (Observer B1, D1). A surface showing
   * something said once a period reads it here. Sifting it out of a fixed-depth feed of everything
   * is a read that goes quiet as the world finds more to say each period, and says nothing when it
   * does.
   */
  recentOfKind(kind: EventKind, n: number, sees: (e: Event) => boolean): readonly Event[] {
    const out: Event[] = [];
    for (let i = this.events.length - 1; i >= 0 && out.length < n; i -= 1) {
      const e = this.events[i];
      if (e?.kind === kind && sees(e)) out.push(e);
    }
    return out.reverse();
  }

  /** What a party may see: public events, and private ones it is a subject of (A4). */
  visibleTo(party: string, last: number): readonly Event[] {
    const out: Event[] = [];
    for (let i = this.events.length - 1; i >= 0 && out.length < last; i -= 1) {
      const e = this.events[i];
      if (e !== undefined && (e.public || e.subjects.includes(party))) out.push(e);
    }
    return out.reverse();
  }

  /**
   * A4: the most recent event of a kind this party is a subject of — its own record: what it
   * announced, what it was told, what it was paid. Reading it is a party reading about itself, so
   * a private event of somebody else is never reachable through here.
   */
  lastOf(kind: EventKind, subject: string): Event | undefined {
    for (let i = this.events.length - 1; i >= 0; i -= 1) {
      const e = this.events[i];
      if (e?.kind === kind && e.subjects.includes(subject)) return e;
    }
    return undefined;
  }

  publicTail(last: number): readonly Event[] {
    const out: Event[] = [];
    for (let i = this.events.length - 1; i >= 0 && out.length < last; i -= 1) {
      const e = this.events[i];
      if (e?.public === true) out.push(e);
    }
    return out.reverse();
  }
}
