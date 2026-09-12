/**
 * The parameter register: every number that shapes behaviour, declared with its provenance.
 *
 * @spec Law 2 XI-14 Appendix A
 *
 * A declared number is one of five things: technology, preference, policy (the three primitives),
 * resolution (tested by invariance), or shape (a claim about the answer). A shape with a scheduled
 * death is a placeholder and names the mechanism whose absence it stands in for. The count of shapes
 * and placeholders is the honest measure of how much model is missing and is reported, never hidden.
 *
 * Engine code reads numbers only through this register (lint: phoenix/no-magic-numbers).
 */
import { InvalidRegistry, Missing } from '../core/errors.js';
import type { ParamId, UnitId } from '../core/ids.js';
import { finite } from '../core/num.js';
import type { Qty } from '../core/tick.js';

export type ParamKind =
  'technology' | 'preference' | 'policy' | 'resolution' | 'shape' | 'placeholder';

export const PARAM_KINDS: readonly ParamKind[] = [
  'technology',
  'preference',
  'policy',
  'resolution',
  'shape',
  'placeholder',
];

/** Who sets a POLICY primitive (Polity D5): the register prints the owner beside the value. */
export type ParamOwner = 'parliament' | 'centralBank' | 'standardSetter' | 'constitution' | 'model';

export interface PlaceholderDeath {
  /** The mechanism whose absence this number stands in for, cited by spec system. */
  readonly mechanism: string;
  /** The worklist item that builds it and deletes this number in the same change. */
  readonly worklistItem: string;
}

/** Law 8: the closed vocabulary a declared number's unit belongs to. */
export type Dimension =
  /** A count of periods of this world's calendar. */
  | 'periods'
  /** A count of days, months or years of the civil calendar — never interchangeable with periods. */
  | 'days'
  | 'months'
  | 'years'
  /** A count of things: people, entries, instructions, contracts, machines. */
  | 'count'
  /** A pure share of something, dimensionless: a ratio, a weight, a fraction. */
  | 'ratio'
  /** A rate per year. */
  | 'perAnnum'
  /**
   * 13c.1, Law 8: A DISTANCE OVER THE GROUND, and a speed over it. The map is what brings length
   * into this world, and the two are kept apart for the reason the four durations are: a distance
   * multiplied by a speed is not a distance, and the register is where that is caught.
   */
  | 'km'
  | 'kmPerDay'
  /** Money for one unit of something: a wage per hour, a level, a price per share. */
  | 'price'
  /** A declared AMOUNT of a unit the reader names (`denominated`), read through `amount`. */
  | 'amount';

export interface ParamDecl {
  readonly id: ParamId;
  /**
   * The value AS A PERSON DECLARES IT. Where `quantityOf` names a unit this is a NAMED amount of
   * it — four hundred thousand USD, seven thousand hours — and what the register answers is the
   * count of pieces the state holds (Law 8). Everywhere else it is the number itself.
   */
  readonly value: number;
  /**
   * The unit the value is in (Law 8): 'periods', 'days', 'per annum', 'ratio of the position'.
   * On a `denominated` value it carries what the denomination does not — the basis and the
   * periodicity, 'per member per period' — because WHICH unit that one is, is the reader's to name.
   *
   * It is PROSE, and prose is for a reader. What a reader cannot do with it is check anything,
   * which is what `dimension` is for.
   */
  readonly unit: string;
  /**
   * Law 8: WHAT KIND OF NUMBER THIS IS, from a closed list — the half of `unit` a machine can check.
   *
   * `unit` was a free string that nothing ever read back: the constructor checked it was non-empty
   * and `get(id)` handed out a bare `number`. So the one place in the engine where every
   * behaviour-shaping number is declared with its unit was also the one place the unit could not be
   * checked, and a number declared in 'periods' multiplied by a day count was wrong at no site
   * (item 13b.1).
   *
   * Every read names what it expects — `params.periods(id)`, `params.ratio(id)` — and a read that
   * names the wrong one throws where it is made, with both dimensions in the message. The four
   * durations are kept apart on purpose: periods, days, months and years are the same quantity in
   * four units, and mixing them is exactly the defect this exists to catch.
   */
  readonly dimension: Dimension;
  /**
   * Law 2, Law 8: the value is an AMOUNT of something, declared the way a person says it — thirty
   * thousand USD, seven thousand hours, four hundred thousand units of a line. WHICH unit is named
   * by whoever reads it (`amount`), because one policy or preference is a number for whatever money,
   * time or paper the party reading it deals in; the register turns it into the count of indivisible
   * pieces that unit is counted in. So a declared amount MOVES WITH THE WORLD'S RESOLUTION instead
   * of being restated against it at every site — which is what makes `resolution.pieceShift` an
   * invariance and not a rescaling of half the world.
   */
  readonly denominated?: true;
  readonly kind: ParamKind;
  readonly owner: ParamOwner;
  /** Why the value is what it is (Law 16: a comment says what the artefact cannot). */
  readonly why: string;
  readonly standsInFor?: PlaceholderDeath;
}

/**
 * Law 2: whether a reason names the worklist item that ends this number.
 *
 * Only a SHAPE is asked. The eleven policies and preferences that cite a future item cite it for
 * something else — a rate parliament owns from 14, a comparison that becomes real at 11 — and they
 * are still there afterwards; what changes is who sets them. A shape is a claim about the answer,
 * so an item that produces the answer is that claim's death, and Law 2 has a word for a shape with
 * a death in it. The check is on the reason because that is where the death was hiding: both
 * field guards above pass while the prose says 13h.
 */
function namesAnItem(why: string): boolean {
  return /\bworklist\s+[0-9]/i.test(why);
}

/** What the register needs of the registry to count a declared amount in pieces (Law 8). */
export interface UnitSource {
  pieces(id: UnitId, named: number): Qty;
}

export interface ParamReport {
  readonly counts: Readonly<Record<ParamKind, number>>;
  readonly placeholders: readonly { id: ParamId; mechanism: string; worklistItem: string }[];
  readonly shapes: readonly ParamId[];
}

export class ParamRegister {
  private readonly decls: ReadonlyMap<ParamId, ParamDecl>;
  private readonly units: UnitSource | undefined;

  /**
   * The units are what a value declared as an AMOUNT is counted in. They are optional because the
   * registry that holds them is itself built on a number in here (`resolution.pieceShift`), so
   * assembly reads that one out of a register with no units — and a register with no units refuses
   * every param that is an amount, rather than answering it in the wrong number.
   */
  constructor(decls: readonly ParamDecl[], units?: UnitSource) {
    this.units = units;
    const map = new Map<ParamId, ParamDecl>();
    for (const d of decls) {
      if (map.has(d.id)) throw new InvalidRegistry('Law 4', `parameter ${d.id} declared twice`);
      finite(d.value, `parameter ${d.id}`);
      if (d.unit.length === 0) throw new InvalidRegistry('Law 8', `parameter ${d.id} has no unit`);
      if (d.why.length === 0)
        throw new InvalidRegistry('Law 16', `parameter ${d.id} has no reason`);
      if (d.kind === 'placeholder' && d.standsInFor === undefined) {
        throw new InvalidRegistry(
          'XI-14',
          `placeholder ${d.id} does not name the mechanism it stands in for`,
        );
      }
      if (d.kind !== 'placeholder' && d.standsInFor !== undefined) {
        throw new InvalidRegistry(
          'XI-14',
          `${d.id} names a scheduled death but is declared ${d.kind}`,
        );
      }
      /**
       * Law 2, XI-14: ASKED OF A SHAPE **AND OF A TECHNOLOGY**, and of nothing else.
       *
       * Firing on `shape` alone polices the honest mistake and misses the one that matters: a
       * placeholder mislabelled sails through both field guards above while its own prose says
       * which item kills it. `loan.operatingCost` was exactly that — a wage bill charged to every
       * borrower and paid to nobody, declared a technology, naming 13d in its reason (item 13b.1).
       *
       * It is NOT asked of a policy or a preference, and that is a decision rather than an
       * oversight: the eleven of those that cite a future item cite it for something else — a rate
       * parliament owns from 14, a comparison that becomes real at 11 — and they are still there
       * afterwards, with somebody else setting them. A TECHNOLOGY is different in kind: it is a
       * fact about the world, and a fact about the world does not have a scheduled death. If an
       * item kills it, it was a claim about the answer all along.
       *
       * A placeholder is exempt because naming the item is what a placeholder DOES; it names it in
       * `standsInFor`, which the guard above already requires.
       */
      if ((d.kind === 'shape' || d.kind === 'technology') && namesAnItem(d.why)) {
        throw new InvalidRegistry(
          'Law 2',
          `${d.kind} ${d.id} names a worklist item in its reason: a ${d.kind} with a scheduled death IS a placeholder. Declare kind 'placeholder' with standsInFor { mechanism, worklistItem }`,
          { id: d.id },
        );
      }
      map.set(d.id, Object.freeze({ ...d }));
    }
    this.decls = map;
  }

  /**
   * Read a value, NAMING WHAT YOU EXPECT IT TO BE (Law 8).
   *
   * The dimension is the half of the unit a machine can check, and this is where it is checked: a
   * read that names the wrong one throws here, with both dimensions in the message, instead of
   * quietly handing back a count of periods to something about to multiply it by a day count.
   *
   * There is no unchecked read. `get` used to be one, and every one of its hundred and fifty-eight
   * call sites was a place where the declared unit and the expected unit could differ with nothing
   * to say so (item 13b.1).
   */
  private read(id: ParamId, expected: Dimension): number {
    const d = this.decl(id);
    if (d.denominated === true) {
      throw new InvalidRegistry(
        'Law 8',
        `parameter ${id} is an amount of something: read it with amount(), naming the unit`,
      );
    }
    if (d.dimension !== expected) {
      throw new InvalidRegistry(
        'Law 8',
        `parameter ${id} is declared in ${d.dimension} ("${d.unit}") and was read as ${expected}`,
        { id, declared: d.dimension, read: expected },
      );
    }
    return d.value;
  }

  /** A count of periods of this world's calendar — never days, months or years. */
  periods(id: ParamId): number {
    return this.read(id, 'periods');
  }

  /** A count of days, months or years of the civil calendar (Money G3.a). */
  days(id: ParamId): number {
    return this.read(id, 'days');
  }

  months(id: ParamId): number {
    return this.read(id, 'months');
  }

  years(id: ParamId): number {
    return this.read(id, 'years');
  }

  /** A count of things: people, entries, contracts, machines. */
  count(id: ParamId): number {
    return this.read(id, 'count');
  }

  /** A pure share of something, dimensionless. */
  ratio(id: ParamId): number {
    return this.read(id, 'ratio');
  }

  /** A rate per year. */
  perAnnum(id: ParamId): number {
    return this.read(id, 'perAnnum');
  }

  /** A distance over the ground (13c.1). */
  km(id: ParamId): number {
    return this.read(id, 'km');
  }

  /** How far a loaded carrier gets over ground like this in a day (13c.1). */
  kmPerDay(id: ParamId): number {
    return this.read(id, 'kmPerDay');
  }

  /** Money for one unit of something: a wage per hour, a level, a price per share. */
  price(id: ParamId): number {
    return this.read(id, 'price');
  }

  /**
   * Law 8: a declared AMOUNT as the state holds one — a count of that unit's indivisible pieces at
   * this world's resolution. The reader names the unit because the declaration is about an amount
   * and not about whose money or paper it is.
   */
  amount(id: ParamId, unit: UnitId): Qty {
    const d = this.decl(id);
    if (d.denominated !== true) {
      throw new InvalidRegistry(
        'Law 8',
        `parameter ${id} is declared in "${d.unit}", which is not an amount of ${unit}`,
      );
    }
    if (this.units === undefined) {
      throw new InvalidRegistry(
        'Law 8',
        `parameter ${id} is an amount, and this register has no units to count it in`,
      );
    }
    return this.units.pieces(unit, d.value);
  }

  decl(id: ParamId): ParamDecl {
    const d = this.decls.get(id);
    if (d === undefined) throw new Missing('Law 2', `parameter ${id} is not declared`, { id });
    return d;
  }

  all(): readonly ParamDecl[] {
    return [...this.decls.values()];
  }

  /** The honest measure of how much mechanism is missing (XI-14). */
  report(): ParamReport {
    const counts: Record<ParamKind, number> = {
      technology: 0,
      preference: 0,
      policy: 0,
      resolution: 0,
      shape: 0,
      placeholder: 0,
    };
    const placeholders: { id: ParamId; mechanism: string; worklistItem: string }[] = [];
    const shapes: ParamId[] = [];
    for (const d of this.decls.values()) {
      counts[d.kind] += 1;
      if (d.kind === 'placeholder' && d.standsInFor !== undefined) {
        placeholders.push({
          id: d.id,
          mechanism: d.standsInFor.mechanism,
          worklistItem: d.standsInFor.worklistItem,
        });
      }
      if (d.kind === 'shape') shapes.push(d.id);
    }
    return { counts, placeholders, shapes };
  }
}
