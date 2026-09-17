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
import { isCash } from '../core/measure.js';
import { Impossible } from '../core/errors.js';
import { impossible } from '../core/assert.js';
import type { EventId } from '../core/ids.js';
import type { FactDecl, FactFields, Payload } from '../registry/facts.js';

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
  /** Observer A3 (17.0a): one party showed another one of its own events. */
  | 'disclosed'
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
  /**
   * Law 18: HOW THE LEDGER IS TRAVERSED IS FREE, and nothing below changes what any read returns.
   *
   * Every read here used to walk the whole journal: `ofKind` filtered it, `lastOf` scanned backwards
   * to the beginning whenever a party had never said that thing. A world says more each period than
   * the last one, and every party asks what it last said — so the cost of a period was the cost of
   * every period before it, times how many parties there are. Three hundred firms and six periods
   * spent four fifths of the run walking events nobody wanted.
   *
   * These are the same events under a different arrangement: a list per kind, a list per period, and
   * the latest one per (kind, subject) which is what `lastOf` answers. They are written where the
   * event is written, so there is one writer of each and no copy to go stale (Law 4).
   */
  private readonly byKind = new Map<EventKind, Event[]>();
  /**
   * Law 4, Law 18 (0g.19): THERE IS NO `byId`, BECAUSE THE ARRAY IS ONE.
   *
   * Observer A3 has a disclosure name the event it showed and the reader fetch it by that name, and
   * that was a `Map<number, Event>` — one entry per event, for ever, beside the array. But `id` is
   * `next`, `next` rises by one per event and the event is pushed in the same call, so
   * `events[id - 1]` IS that map's answer at every id it has. It was a second representation of one
   * fact (Law 4) that cost a `Map.set` on every write and grew without bound; `get` reads the array
   * and checks the identity it relies on rather than trusting it.
   */
  private readonly byPeriod = new Map<Period, Event[]>();
  /**
   * 0g.19: NESTED, for the reason `bySubject` is — a string key built from the two was made on every
   * write and on every read, and thrown away each time. The events in it are the same events in the
   * same order.
   */
  private readonly byKindPeriod = new Map<EventKind, Map<Period, Event[]>>();
  /**
   * Law 18: nested rather than keyed on a name built from the two, because a party asking what it
   * last said is one of the commonest reads in the engine and every ask was a string made to be
   * thrown away. The events in it are the same events in the same order.
   */
  private readonly bySubject = new Map<EventKind, Map<string, Event[]>>();
  /** 0g.4: the last PUBLIC event of each kind, kept because it cannot be read off `byKind`. */
  private readonly lastPublicByKind = new Map<EventKind, Event>();

  /**
   * 0i: WRITE A DECLARED FACT. The payload is typed by the declaration, so a field left out or
   * misnamed is a compile error and not a silent `undefined` at whoever reads it later.
   *
   * It is a separate door from `record` only while the migration runs. When the last bag is
   * declared, `record` is deleted and this takes its name — one door (Law 4).
   */
  say<F extends FactFields>(
    period: Period,
    cycle: Cycle,
    decl: FactDecl<F>,
    subjects: readonly string[],
    says: Payload<F>,
    isPublic: boolean,
  ): Event {
    return this.record(period, cycle, decl.kind, subjects, says, isPublic);
  }

  record(
    period: Period,
    cycle: Cycle,
    kind: EventKind,
    subjects: readonly string[],
    data: Record<string, unknown>,
    isPublic: boolean,
  ): Event {
    // 16.0, Law 8: money on the record is PIECES beside a NAMED currency, never a value object a
    // reader would stringify — the writer says `amount` and `ccy` as two facts.
    // 0g.19: the same refusal, without building an array of pairs for every event ever written.
    // `Object.entries` allocated one outer array and one two-element array per key, per event, to
    // find the objects among them; `in` walks the keys and the typeof guard reaches `isCash` only
    // for the values that could possibly be one. Measured: 643 ns of a 1,754 ns write.
    for (const key in data) {
      const v = data[key];
      if (typeof v === 'object' && v !== null && isCash(v))
        throw new Impossible(
          'Law 8',
          `event ${kind}: ${key} is money as a value; record its pieces and name its currency`,
          { kind, key },
        );
    }
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
    push(this.byKind, kind, ev);
    if (isPublic) this.lastPublicByKind.set(kind, ev);
    push(this.byPeriod, period, ev);
    let ofKindPeriod = this.byKindPeriod.get(kind);
    if (ofKindPeriod === undefined) {
      ofKindPeriod = new Map<Period, Event[]>();
      this.byKindPeriod.set(kind, ofKindPeriod);
    }
    push(ofKindPeriod, period, ev);
    if (ev.subjects.length > 0) {
      let mine = this.bySubject.get(kind);
      if (mine === undefined) {
        mine = new Map<string, Event[]>();
        this.bySubject.set(kind, mine);
      }
      for (const s of ev.subjects) push(mine, s, ev);
    }
    return ev;
  }

  /** The event with this id, or nothing: a disclosure that names one that was never written shows nothing. */
  get(id: EventId): Event | undefined {
    const at = this.events[id - 1];
    if (at === undefined) return undefined;
    // The one thing the deleted map was doing that the array cannot do by construction: say so if
    // the assumption it rests on ever stops holding, rather than handing back the wrong event.
    impossible(at.id === id, 'Observer A3', `event ${id} is not at its own place: found ${at.id}`);
    return at;
  }

  all(): readonly Event[] {
    return this.events;
  }

  inPeriod(period: Period): readonly Event[] {
    return this.byPeriod.get(period) ?? EMPTY;
  }

  ofKind(kind: EventKind): readonly Event[] {
    return this.byKind.get(kind) ?? EMPTY;
  }

  /**
   * What was said of one kind IN one period. A read about this period sifted it out of everything
   * ever said of that kind, so it grew with the age of the world rather than with what happened.
   */
  ofKindIn(kind: EventKind, period: Period): readonly Event[] {
    return this.byKindPeriod.get(kind)?.get(period) ?? EMPTY;
  }

  /**
   * A4: everything of a kind this party is a subject of — its own record, oldest first. A private
   * event of somebody else is never reachable through here, for the same reason `lastOf` is not.
   */
  forSubject(kind: EventKind, subject: string): readonly Event[] {
    return this.bySubject.get(kind)?.get(subject) ?? EMPTY;
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
    const of = this.byKind.get(kind) ?? EMPTY;
    const out: Event[] = [];
    for (let i = of.length - 1; i >= 0 && out.length < n; i -= 1) {
      const e = of[i];
      if (e !== undefined && sees(e)) out.push(e);
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
    const said = this.bySubject.get(kind)?.get(subject);
    return said === undefined ? undefined : said[said.length - 1];
  }

  /**
   * 0g.4: THE LAST THING SAID OF A KIND, and the last PUBLIC thing — both O(1).
   *
   * Every reader that wanted one of these asked `ofKind(kind)` and walked or filtered the whole
   * history of it. `view.lastPublic` did the worst of it: `ofKind(kind).filter((e) => e.public)`
   * built a copy of every event of that kind ever recorded, to return the last element of it, and
   * it is on the participant view — so it is inside the order-generation loop. Measured on the
   * (24, 96) rung at period 8: **3,286 `ofKind` calls a period walking 66,184 events**, against a
   * journal of 91,539 — a cost that grows with the AGE of the world rather than with what happened
   * in the period (`ofKindIn`'s docstring says the same thing about its own predecessor).
   *
   * `byKind` already holds the events of a kind in writing order, so the last of them is the last
   * element and needs no walk. The public one cannot be read off that array — most events are not
   * public — so it is the one fact here that is KEPT, written where the event is written, which is
   * the same construction `bySubject` and `byKindPeriod` are (Law 4: one writer, no copy to go
   * stale).
   */
  lastOfKind(kind: EventKind): Event | undefined {
    const said = this.byKind.get(kind);
    return said === undefined ? undefined : said[said.length - 1];
  }

  lastPublicOfKind(kind: EventKind): Event | undefined {
    return this.lastPublicByKind.get(kind);
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

const EMPTY: readonly Event[] = Object.freeze([]);


/** One arrangement of the same events; the list is created the first time something lands in it. */
function push<K>(into: Map<K, Event[]>, key: K, e: Event): void {
  const list = into.get(key);
  if (list === undefined) into.set(key, [e]);
  else list.push(e);
}
