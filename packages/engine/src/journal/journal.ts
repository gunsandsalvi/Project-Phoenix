/**
 * The journal: events generated FROM state transitions by the engine (Observer B2). It is a read of
 * what happened, never an input to anything (B2.a: news never causes anything).
 *
 * @spec Observer B1 Observer B2 Observer B2.a Observer B3 Observer D1 Money G4
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import type { EventId } from '../core/ids.js';

export type EventKind =
  | 'instruction.settled'
  | 'instruction.failed'
  | 'reserve.overdraft'
  | 'weight'
  | 'print'
  | 'revaluation'
  | 'party.ceased'
  | 'instrument.ceased'
  | 'audit';

export interface Event {
  readonly id: EventId;
  readonly period: Period;
  readonly cycle: Cycle;
  readonly kind: EventKind;
  /** B3: named subjects, so the event can be checked against the state. */
  readonly subjects: readonly string[];
  readonly data: Readonly<Record<string, unknown>>;
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
  ): Event {
    const ev: Event = Object.freeze({
      id: this.next as EventId,
      period,
      cycle,
      kind,
      subjects: [...subjects],
      data: Object.freeze({ ...data }),
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
}
