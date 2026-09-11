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

export interface ParamDecl {
  readonly id: ParamId;
  /**
   * The value AS A PERSON DECLARES IT. Where `quantityOf` names a unit this is a NAMED amount of
   * it — four hundred thousand PHX, seven thousand hours — and what the register answers is the
   * count of pieces the state holds (Law 8). Everywhere else it is the number itself.
   */
  readonly value: number;
  /**
   * The unit the value is in (Law 8): 'periods', 'days', 'per annum', 'ratio of the position'.
   * On a `denominated` value it carries what the denomination does not — the basis and the
   * periodicity, 'per member per period' — because WHICH unit that one is, is the reader's to name.
   */
  readonly unit: string;
  /**
   * Law 2, Law 8: the value is an AMOUNT of something, declared the way a person says it — thirty
   * thousand PHX, seven thousand hours, four hundred thousand units of a line. WHICH unit is named
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
      if (d.kind === 'shape' && namesAnItem(d.why)) {
        throw new InvalidRegistry(
          'Law 2',
          `shape ${d.id} names a worklist item in its reason: a shape with a scheduled death IS a placeholder. Declare kind 'placeholder' with standsInFor { mechanism, worklistItem }`,
          { id: d.id },
        );
      }
      map.set(d.id, Object.freeze({ ...d }));
    }
    this.decls = map;
  }

  /** Read a value. A number nobody declared is missing, never zero. */
  get(id: ParamId): number {
    const d = this.decl(id);
    if (d.denominated === true) {
      throw new InvalidRegistry(
        'Law 8',
        `parameter ${id} is an amount of something: read it with amount(), naming the unit`,
      );
    }
    return d.value;
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
