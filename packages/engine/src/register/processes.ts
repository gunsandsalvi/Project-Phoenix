/**
 * A procedure that takes more than one period: what it is, where it has got to, and when it must end.
 *
 * @spec XI-8 Firm Birth D5 Banks Capital C1 Equity D3 Capital Programme C3 Law 4 Law 15 Law 16
 *
 * The single instance of this shape in the codebase was `Winding = {dead, opened, closesAfter,
 * closed}` — four fields in one module's bag. A construction project, an auction cycle, a tender
 * offer period, a rights issue, a resolution and a restructuring are the same shape, and each was
 * going to invent its own version of it: four fields, a different spelling, in a different bag,
 * none of them visible to the kernel and none of them answerable to "what is this party in the
 * middle of".
 *
 * WHAT IT IS. A named procedure about a named subject, with ORDERED STEPS and a period by which it
 * must be over. It holds no money and moves nothing: what happens at each step is the mechanism's
 * own business, and this says only which step that is and how long there is left.
 *
 * WHY THE STEPS ARE DECLARED. A process whose progress is a boolean can say "started" and
 * "finished" and nothing between, which is what made `Winding` unable to distinguish an estate
 * still selling from one paying out — the two states an estate spends all its time in. Steps are
 * strings the owning mechanism names, and NOTHING HERE COMPARES TWO OF THEM (Law 15): this walks
 * them in order and reports where it is.
 */
import type { Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import { Missing } from '../core/errors.js';
import { processId, type PartyId, type ProcessId } from '../core/ids.js';

export type ProcessState = 'running' | 'closed' | 'abandoned';

export interface ProcessDecl {
  /** What sort of procedure this is, in the words its own mechanism uses. */
  readonly what: string;
  /** Whom it is about. A process with no subject is a procedure about nobody. */
  readonly subject: PartyId;
  /** Its steps, in order. At least one, because a procedure with no steps does nothing. */
  readonly steps: readonly string[];
  /** D5: the period by which it must be over, which is what makes it a PROGRAMME and not a wish. */
  readonly closesAfter: Period;
  readonly why: string;
}

export interface Process extends ProcessDecl {
  readonly id: ProcessId;
  readonly opened: Period;
  /** Which step it is on, as an index into `steps`. */
  readonly at: number;
  readonly state: ProcessState;
}

export class Processes {
  private readonly rows = new Map<ProcessId, Process>();
  private readonly bySubject = new Map<PartyId, Set<ProcessId>>();
  private next = 1;

  open(decl: ProcessDecl, at: Period): Process {
    forbid(decl.steps.length > 0, 'Law 16', `${decl.what} is a procedure with no steps`);
    forbid(decl.what.length > 0, 'Law 16', 'a process that does not say what it is');
    forbid(
      decl.closesAfter >= at,
      'Firm Birth D5',
      `${decl.what} for ${decl.subject} closes in ${decl.closesAfter}, before it opened in ${at}`,
    );
    const id = processId(`process.${this.next}`);
    this.next += 1;
    const row: Process = { ...decl, id, opened: at, at: 0, state: 'running' };
    this.rows.set(id, Object.freeze(row));
    const set = this.bySubject.get(decl.subject);
    if (set === undefined) this.bySubject.set(decl.subject, new Set([id]));
    else set.add(id);
    return row;
  }

  get(id: ProcessId): Process {
    const row = this.rows.get(id);
    if (row === undefined) throw new Missing('XI-8', `no process ${id}`);
    return row;
  }

  /** One step on. It refuses to walk past the last one: that is closing, and closing says so. */
  advance(id: ProcessId): Process {
    const row = this.get(id);
    forbid(row.state === 'running', 'Law 15', `${id} is ${row.state} and cannot advance`);
    forbid(
      row.at + 1 < row.steps.length,
      'Law 15',
      `${id} is on its last step (${String(row.steps[row.at])}) and advancing would leave the procedure`,
    );
    const next: Process = { ...row, at: row.at + 1 };
    this.rows.set(id, Object.freeze(next));
    return next;
  }

  /** It finished. What it was for was done, whatever step it got to (D5). */
  close(id: ProcessId): Process {
    return this.settle(id, 'closed');
  }

  /** It stopped without finishing — the subject died, the offer lapsed, the project was dropped. */
  abandon(id: ProcessId): Process {
    return this.settle(id, 'abandoned');
  }

  /** The step it is on, by name. What a reader wants is the word, not the index. */
  step(id: ProcessId): string {
    const row = this.get(id);
    const name = row.steps[row.at];
    if (name === undefined) throw new Missing('Law 15', `${id} is on no step of ${row.what}`);
    return name;
  }

  /** WHAT THIS PARTY IS IN THE MIDDLE OF — the question a module's private bag could not answer. */
  of(subject: PartyId): readonly Process[] {
    const ids = this.bySubject.get(subject);
    if (ids === undefined) return [];
    const out: Process[] = [];
    for (const id of ids) {
      const row = this.rows.get(id);
      if (row !== undefined) out.push(row);
    }
    return out;
  }

  /** Every running procedure of a sort, which is what a phase walks (Law 19: never a second list). */
  running(what: string): readonly Process[] {
    return this.all().filter((p) => p.state === 'running' && p.what === what);
  }

  /** D5: running and out of time. A programme that must be over and is not is the caller's to end. */
  dueBy(at: Period): readonly Process[] {
    return this.all().filter((p) => p.state === 'running' && p.closesAfter <= at);
  }

  all(): readonly Process[] {
    return [...this.rows.values()];
  }

  private settle(id: ProcessId, to: ProcessState): Process {
    const row = this.get(id);
    forbid(row.state === 'running', 'Law 15', `${id} is already ${row.state}`);
    const next: Process = { ...row, state: to };
    this.rows.set(id, Object.freeze(next));
    return next;
  }
}

/** A real read-only facade: no write is reachable through it, at runtime as well as in the types. */
export type ProcessReads = Pick<Processes, 'get' | 'step' | 'of' | 'running' | 'dueBy' | 'all'>;

export function processReads(store: Processes): ProcessReads {
  return Object.freeze({
    get: (id: ProcessId) => store.get(id),
    step: (id: ProcessId) => store.step(id),
    of: (subject: PartyId) => store.of(subject),
    running: (what: string) => store.running(what),
    dueBy: (at: Period) => store.dueBy(at),
    all: () => store.all(),
  });
}
