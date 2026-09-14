/**
 * The ontology register: every THING this world is made of, declared with where it lives.
 *
 * @spec Law 2 Law 4 Law 15 XI-14 Appendix A Appendix C
 *
 * `ParamRegister` forces every declared NUMBER to name its kind, its unit and its owner, and the
 * build fails if one does not. There was no equivalent for CATEGORIES. The specification's systems
 * enumerate BEHAVIOURS and `registry/kinds.ts` enumerates SETTLEMENT OBJECTS; nothing enumerated the
 * things the economy is made of. So "is there a noun for this?" had no lookup — it had a search, and
 * a search finds only what somebody already tried to build. Sixteen missing primitives were found
 * over six passes of reading, each pass finding more only because the method changed.
 *
 * `MechanismContext.state()` is where a module puts something the kernel has no home for. It is a
 * bag: it takes a name and an object and asks nothing. Seventeen modules keep nineteen slots in it,
 * and its own docstring lists the missing primitives without noticing — "a register of employment
 * rows, a book of invoices, a party's outlooks". Each of those is an economic noun with no kernel
 * store, kept privately, invisible to the audit, to the estate's ranking and to every other module.
 *
 * SO A SLOT IS DECLARED OR IT DOES NOT OPEN. A module says what is in it and which of three things
 * it is, and the one that means "this is a noun the kernel should own" must name the plan item that
 * gives it that home — the same construction `ParamRegister` uses for a placeholder, for the same
 * reason: a stand-in with no scheduled death is a permanent one (Law 2).
 *
 * The count of nouns still in a bag is the honest measure of how much ontology is missing, and it is
 * reported rather than hidden (Appendix C).
 */
import { InvalidRegistry } from '../core/errors.js';

/**
 * Law 2: what a module's own store IS, from a closed list.
 *
 * `noun` is a thing this economy HAS, kept in a module's bag because the kernel has no store for it
 * — an employment, a lease, an invoice, a party's outlook. It is a placeholder for a kernel noun and
 * names the plan item that gives it one.
 *
 * `working` is state a phase hands to a later phase WITHIN a period and nothing outside the module
 * has any business reading: a draw the kernel allowed that is waiting to become a row, a line whose
 * wipe has been announced so it is not announced twice. It is not a fact about the world, it is how
 * one module gets from one of its phases to the next.
 *
 * `physics` is the module's own subject matter, which is private by right and wants no kernel home:
 * the weather is the environment module's, and nothing else in this world has an opinion about it.
 */
export type NounKind = 'noun' | 'working' | 'physics';

export const NOUN_KINDS: readonly NounKind[] = ['noun', 'working', 'physics'];

/** Where a noun kept in a module's bag is going, and what ends the arrangement (Law 2). */
export interface NounHome {
  /** The kernel noun it is an instance of, named as `docs/IMPLEMENTATION.md` names it. */
  readonly noun: string;
  /** The plan item that builds that noun and deletes this declaration in the same change. */
  readonly planItem: string;
}

/** What a module declares about one of its own stores. The owner is stamped at assembly (Law 4). */
export interface NounEntry {
  /** The name the module passes to `ctx.state`. */
  readonly name: string;
  readonly kind: NounKind;
  /** What is in it, in the terms a reader of the economy would use. */
  readonly holds: string;
  /** Why it is this kind rather than another (Law 16: a comment says what the artefact cannot). */
  readonly why: string;
  readonly standsInFor?: NounHome;
}

export interface NounDecl extends NounEntry {
  /** The module that keeps it. Stamped by `assemble`, never written by the module itself. */
  readonly owner: string;
}

export interface OntologyReport {
  readonly counts: Readonly<Record<NounKind, number>>;
  /** Every noun still living in a module's bag, and the item that gives it a kernel home. */
  readonly homeless: readonly { owner: string; name: string; noun: string; planItem: string }[];
}

const key = (owner: string, name: string): string => `${owner}/${name}`;

export class OntologyRegister {
  private readonly decls: ReadonlyMap<string, NounDecl>;

  constructor(decls: readonly NounDecl[]) {
    const map = new Map<string, NounDecl>();
    for (const d of decls) {
      const k = key(d.owner, d.name);
      if (map.has(k)) throw new InvalidRegistry('Law 4', `store ${k} is declared twice`);
      if (d.name.length === 0)
        throw new InvalidRegistry('Law 15', `${d.owner} declares a store with no name`);
      if (d.holds.length === 0)
        throw new InvalidRegistry('Law 16', `store ${k} does not say what is in it`);
      if (d.why.length === 0) throw new InvalidRegistry('Law 16', `store ${k} has no reason`);
      if (d.kind === 'noun' && d.standsInFor === undefined) {
        throw new InvalidRegistry(
          'Law 2',
          `${k} is declared a noun and names no plan item that gives it a kernel home; a stand-in with no scheduled death is a permanent one`,
        );
      }
      if (d.kind !== 'noun' && d.standsInFor !== undefined) {
        throw new InvalidRegistry(
          'Law 2',
          `${k} names a kernel home but is declared ${d.kind}; only a noun is going somewhere`,
        );
      }
      map.set(k, d);
    }
    this.decls = map;
  }

  /**
   * The declaration for a store, or a refusal.
   *
   * This is the whole point of the register: a module reaching for a category the kernel has no
   * home for gets a build failure that says so, instead of reaching for the nearest bag — which is
   * the move that produced every missing primitive in `docs/IMPLEMENTATION.md`.
   */
  declared(owner: string, name: string): NounDecl {
    const d = this.decls.get(key(owner, name));
    if (d === undefined) {
      throw new InvalidRegistry(
        'Law 15',
        `${owner} keeps a store called "${name}" that it never declared. Say what is in it and which of ${NOUN_KINDS.join(' | ')} it is; a noun names the plan item that gives it a kernel home (docs/RECORD.md item 0)`,
      );
    }
    return d;
  }

  all(): readonly NounDecl[] {
    return [...this.decls.values()];
  }

  /** Appendix C: recount rather than adjust. What is still in a bag, and what takes it out. */
  report(): OntologyReport {
    const counts: Record<NounKind, number> = { noun: 0, working: 0, physics: 0 };
    const homeless: { owner: string; name: string; noun: string; planItem: string }[] = [];
    for (const d of this.decls.values()) {
      counts[d.kind] += 1;
      const home = d.standsInFor;
      if (home !== undefined) {
        homeless.push({ owner: d.owner, name: d.name, noun: home.noun, planItem: home.planItem });
      }
    }
    return { counts, homeless };
  }
}
