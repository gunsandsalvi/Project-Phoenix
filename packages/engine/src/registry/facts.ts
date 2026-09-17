/**
 * The fact register: every EVENT this world records, declared with the shape of what it says.
 *
 * @spec Law 2 Law 4 Law 15 Law 16 Law 19 XI-14 Appendix A Appendix C
 *
 * `ParamRegister` forces every declared NUMBER to name its kind, its unit and its owner, and the
 * build fails if one does not. `registry/nouns.ts` does the same for CATEGORIES, and says in its own
 * docstring why it had to exist: *"is there a noun for this?" had no lookup — it had a search, and a
 * search finds only what somebody already tried to build.*
 *
 * THERE WAS NO EQUIVALENT FOR FACTS, and a fact is how every mechanism in this world reaches every
 * other one. A journal payload was `Record<string, unknown>`: 227 event kinds written with a literal
 * name and 49 more written with a computed one, 496 distinct keys written, 122 read back by name,
 * and nothing anywhere matching a reader to a writer. Twenty-two kinds were written at more than one
 * site with more than one set of keys.
 *
 * WHAT THAT COSTS IS NOT AN ERROR, IT IS A `continue`. A reader that cannot parse a payload skips
 * it. `world.ts requestsIn` rebuilt a borrower's ask out of nine string keys and skipped the ask if
 * any one was missing or of the wrong type — so a borrower whose request did not parse was silently
 * not in the lending market, with no throw, no finding and no red test. A world that looks like a
 * world where nobody wanted to borrow. The same silence is 21.100 (a weight event written `into` and
 * read `to`, so every merge in this world was reported as a holding that moved with no leg behind
 * it), 21.102 (a `moved` map only one family knew how to read) and the whole class of a mechanism
 * that never runs — the class the liveness family had to be invented for at 0h.3, when thirteen
 * modules had never run while every safety check passed.
 *
 * SO A FACT IS DECLARED OR IT IS COUNTED. A writer says what its event says, once, in fields with
 * kinds; `record` takes a payload TYPED by that declaration, so a missing or misnamed field is a
 * compile error instead of a silent `undefined`; and a reader asks the declaration rather than the
 * bag. An undeclared kind is not refused — it is counted, the same construction `nouns` uses for a
 * homeless noun, because the honest measure of how many facts are still bags is a number that falls
 * item by item and not a rewrite of five hundred and fifty-six sites in one change (Appendix C).
 *
 * THERE ARE NO OPTIONAL FIELDS. Absent-means-something is the defect with an optional field instead
 * of a string key: `MoneyLeg.receipt` was optional and *"absent is unclassified, never income"*, so
 * 49 of this engine's 79 money legs landed money on a party classified as nothing and the tax base
 * was short by every one of them. A field that may have no value declares `orNone` and its writer
 * must WRITE the absence. Missing is `Missing`, for a fact exactly as for a number (Appendix A).
 */
import { InvalidRegistry } from '../core/errors.js';
// A type-only import, erased at build: the register says what a fact SAYS and the journal says what
// a fact IS, and a kind is the journal's word. `registry/notices.ts` reaches the same way.
import type { EventKind } from '../journal/journal.js';

/**
 * Law 8, Law 9: WHAT A FIELD OF A FACT IS, from a closed list — the same discipline a param's unit
 * is under. A kind is not decoration: it is what lets a reader be checked against a writer without
 * either of them naming the other, and what stops a period being read as a count.
 */
export type FieldKind =
  | 'party'
  | 'instrument'
  | 'market'
  | 'agreement'
  /** Law 8: a count of a money's smallest piece. The currency is its own field (`ccy`), never a value object. */
  | 'money'
  /** Law 8, Law 9: money PER UNIT of the thing. It is not money and never adds to it. */
  | 'price'
  | 'currency'
  | 'count'
  | 'ratio'
  | 'period'
  | 'text'
  | 'flag'
  /** The id of another event in the journal, so a fact can cite the fact it was based on (Law 19). */
  | 'eventRef'
  /** 0f.6: what moved, per instrument — the shape a weight event owes the flows family. */
  | 'byInstrument'
  /** XI-15: a cell's identity on its kind's declared lattice, dimension by band. */
  | 'latticeKey';

export const FIELD_KINDS: readonly FieldKind[] = [
  'party',
  'instrument',
  'market',
  'agreement',
  'money',
  'price',
  'currency',
  'count',
  'ratio',
  'period',
  'text',
  'flag',
  'eventRef',
  'byInstrument',
  'latticeKey',
];

/** What one field of one fact is, and why it is there (Law 16: a comment says what the shape cannot). */
export interface FieldSpec {
  readonly is: FieldKind;
  /** What it says, in the terms a reader of the economy would use. */
  readonly what: string;
  /**
   * Appendix A: THE VALUE MAY BE ABSENT AND THE FIELD MAY NOT. A party that ceased with no successor
   * writes `null` and says so; it does not leave the key out. This is the difference between a fact
   * that states an absence and a fact that is missing a field, and only the first is readable.
   */
  readonly orNone?: true;
}

export type FactFields = Readonly<Record<string, FieldSpec>>;

/** What a declared field's value is. A reader gets this; a writer must produce it. */
type Value<K extends FieldKind> = K extends 'party' | 'instrument' | 'market' | 'agreement' | 'currency' | 'text'
  ? string
  : K extends 'money' | 'price' | 'count' | 'ratio' | 'period' | 'eventRef'
    ? number
    : K extends 'flag'
      ? boolean
      : K extends 'byInstrument'
        ? Readonly<Record<string, number>>
        : K extends 'latticeKey'
          ? Readonly<Record<string, string>>
          : never;

type Field<S extends FieldSpec> = S extends { readonly orNone: true }
  ? Value<S['is']> | null
  : Value<S['is']>;

/** The payload of a declared fact: every field, none of them optional (see the header). */
export type Payload<F extends FactFields> = { readonly [K in keyof F]: Field<F[K]> };

/**
 * One fact, declared. `kind` is the journal's event kind; `says` is its payload; `why` is the
 * sentence that says what the fact is FOR, because a fact nobody can say the purpose of is a fact
 * nobody should be writing (Law 16).
 */
export interface FactDecl<F extends FactFields = FactFields> {
  readonly kind: EventKind;
  readonly says: F;
  readonly why: string;
  /** The module or kernel part that writes it. Stamped at assembly, never written by the module. */
  readonly owner?: string;
}

/**
 * Declare a fact. It is a function rather than an object literal so the field types survive into
 * `Payload` — a declaration widened to `FactFields` types its payload as `unknown` and the whole
 * register becomes decoration.
 */
export function fact<const F extends FactFields>(
  kind: EventKind,
  why: string,
  says: F,
): FactDecl<F> {
  return { kind, why, says };
}

export interface FactsReport {
  readonly declared: number;
  /** Appendix C: the kinds still written as a bag, and who writes them. The measure that must fall. */
  readonly undeclared: readonly { kind: string; owner: string }[];
}

export class FactRegister {
  private readonly decls: ReadonlyMap<string, FactDecl>;
  /** Law 19: what has actually been written with no declaration, counted at the write and not guessed. */
  private readonly bags = new Map<string, string>();

  constructor(decls: readonly FactDecl[]) {
    const map = new Map<string, FactDecl>();
    for (const d of decls) {
      if (d.kind.length === 0) throw new InvalidRegistry('Law 15', 'a fact is declared with no kind');
      if (d.why.length === 0)
        throw new InvalidRegistry('Law 16', `fact ${d.kind} does not say what it is for`);
      const held = map.get(d.kind);
      if (held !== undefined) {
        // Law 4: ONE declaration per fact. Twenty-two kinds were written at more than one site with
        // more than one set of keys, which is how a writer says `into` and a reader asks for `to`.
        throw new InvalidRegistry(
          'Law 4',
          `fact ${d.kind} is declared twice, by ${held.owner ?? 'the kernel'} and by ${d.owner ?? 'the kernel'}; one fact has one shape`,
        );
      }
      for (const [name, spec] of Object.entries(d.says)) {
        if (!FIELD_KINDS.includes(spec.is)) {
          throw new InvalidRegistry(
            'Law 8',
            `fact ${d.kind} field ${name} is a "${spec.is}", which is not one of ${FIELD_KINDS.join(' | ')}`,
          );
        }
        if (spec.what.length === 0) {
          throw new InvalidRegistry('Law 16', `fact ${d.kind} field ${name} does not say what it is`);
        }
      }
      map.set(d.kind, d);
    }
    this.decls = map;
  }

  has(kind: string): boolean {
    return this.decls.has(kind);
  }

  /**
   * The declaration for a kind, or a refusal. This is the point of the register: a reader reaching
   * for a fact nobody declared is told so, instead of getting `undefined` and skipping the event.
   */
  declared(kind: string): FactDecl {
    const d = this.decls.get(kind);
    if (d === undefined) {
      throw new InvalidRegistry(
        'Law 15',
        `nothing declares the fact "${kind}". Say what it says, in fields with kinds, on the phase that writes it (docs/IMPLEMENTATION.md 0i)`,
      );
    }
    return d;
  }

  /** Appendix C: a kind written with no declaration, counted where it happens rather than searched for. */
  countBag(kind: string, owner: string): void {
    if (this.decls.has(kind)) return;
    this.bags.set(kind, owner);
  }

  /** Appendix C: recount rather than adjust. What is declared, and what is still a bag. */
  report(): FactsReport {
    return {
      declared: this.decls.size,
      undeclared: [...this.bags]
        .map(([kind, owner]) => ({ kind, owner }))
        .sort((a, b) => a.kind.localeCompare(b.kind)),
    };
  }

  all(): readonly FactDecl[] {
    return [...this.decls.values()];
  }
}

/**
 * 0i, Law 19: READ A DECLARED FACT, or throw.
 *
 * Every reader of a journal payload in this engine tested each field's type by hand and SKIPPED the
 * event when one did not match — `return none()`, `continue`. That is why a renamed key is not an
 * error but an absence: a broker with no line, a borrower not in the lending market, a merge with
 * no leg behind it. A fact that does not match its own declaration is a contract violation at the
 * writer, so it throws with a citation and the field's name (Error discipline), and the reader gets
 * the payload with its types rather than a bag to interrogate.
 */
export function says<F extends FactFields>(
  event: { readonly kind: string; readonly data: Readonly<Record<string, unknown>> },
  decl: FactDecl<F>,
): Payload<F> {
  if (event.kind !== decl.kind) {
    throw new InvalidRegistry(
      'Law 4',
      `read a "${event.kind}" as a "${decl.kind}"; a fact is read as what it was written as`,
    );
  }
  for (const [name, spec] of Object.entries(decl.says)) {
    const v = event.data[name];
    if (v === null) {
      if (spec.orNone === true) continue;
      throw new InvalidRegistry(
        'Appendix A',
        `${decl.kind}.${name} says nothing and is not declared to be able to; missing is Missing`,
      );
    }
    if (!matches(spec.is, v)) {
      throw new InvalidRegistry(
        'Law 8',
        `${decl.kind}.${name} is declared a ${spec.is} and was written as ${v === undefined ? 'nothing at all' : typeof v}`,
      );
    }
  }
  return event.data as Payload<F>;
}

function matches(is: FieldKind, v: unknown): boolean {
  switch (is) {
    case 'party':
    case 'instrument':
    case 'market':
    case 'agreement':
    case 'currency':
    case 'text':
      return typeof v === 'string';
    case 'money':
    case 'price':
    case 'count':
    case 'ratio':
    case 'period':
    case 'eventRef':
      return typeof v === 'number' && Number.isFinite(v);
    case 'flag':
      return typeof v === 'boolean';
    case 'byInstrument':
      return typeof v === 'object' && v !== null && Object.values(v).every((x) => typeof x === 'number');
    case 'latticeKey':
      return typeof v === 'object' && v !== null && Object.values(v).every((x) => typeof x === 'string');
  }
}
