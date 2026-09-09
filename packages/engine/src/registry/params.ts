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
import type { ParamId } from '../core/ids.js';
import { finite } from '../core/num.js';

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
  readonly value: number;
  /** The unit the value is in (Law 8): 'periods', 'days', 'per-annum', 'count', 'ratio', ... */
  readonly unit: string;
  readonly kind: ParamKind;
  readonly owner: ParamOwner;
  /** Why the value is what it is (Law 16: a comment says what the artefact cannot). */
  readonly why: string;
  readonly standsInFor?: PlaceholderDeath;
}

export interface ParamReport {
  readonly counts: Readonly<Record<ParamKind, number>>;
  readonly placeholders: readonly { id: ParamId; mechanism: string; worklistItem: string }[];
  readonly shapes: readonly ParamId[];
}

export class ParamRegister {
  private readonly decls: ReadonlyMap<ParamId, ParamDecl>;

  constructor(decls: readonly ParamDecl[]) {
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
      map.set(d.id, Object.freeze({ ...d }));
    }
    this.decls = map;
  }

  /** Read a value. A number nobody declared is missing, never zero. */
  get(id: ParamId): number {
    const d = this.decls.get(id);
    if (d === undefined) throw new Missing('Law 2', `parameter ${id} is not declared`, { id });
    return d.value;
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
