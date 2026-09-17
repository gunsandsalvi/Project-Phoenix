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
import { asRatio, type PerNamedUnit, type PerPiece, type Ratio } from '../core/measure.js';

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
  /**
   * THE ITEM THAT BUILDS IT and deletes this number in the same change — an open row of
   * `docs/WORKLIST.md`, or an item or step of `docs/IMPLEMENTATION.md`, which is the ORDERED PLAN
   * and is where the first open item is taken from.
   *
   * It was `worklistItem`, and the name had gone stale with the project: four placeholders named
   * worklist rows that were closed, one of them a row the worklist's own header says never produced
   * an outcome. `npm run check:deaths` is what now refuses that, and it resolves against both files
   * — so a death can name the list the work is actually queued on (item 9.2b).
   */
  readonly item: string;
}

/**
 * Law 8: WHAT A DECLARED AMOUNT IS AN AMOUNT OF — the half the unit's free text says and nothing
 * can read. `dimension: 'amount'` says only that the reader names the unit; this says which KIND of
 * unit a reader may name, so a number meant as dollars can be measured against this world's dollars.
 */
export type Denomination =
  /** An amount of whatever money the party reading it deals in: a cost, a limit, a transfer. */
  | 'money'
  /** An amount of somebody's time: hours a person has, hours a venue takes. */
  | 'time';

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
  /**
   * Money for one PIECE of something: what the state holds one of, at this world's resolution.
   *
   * `E-8`, item 2: THERE ARE TWO SCALES AND A DECLARED LEVEL HAS TO SAY WHICH. This world states
   * numbers in named units — a dollar a tonne, a wage an hour, a level a share — and holds pieces
   * of both, and `Registry.priceOf` is the door between them. Six declared levels were all read as
   * `price` and four of them are stated per NAMED unit: `USD per unit of <good>`, `USD per hour of
   * work`, `units of the quote money per unit of the base`, `of face per contract`. Reading one as
   * the other multiplies by a subdivision, which is what "a share worth a hundredth of a cent" was.
   */
  | 'price'
  /** Money for one NAMED unit of something: a dollar a tonne, a wage an hour, read through `pricePerUnit`. */
  | 'pricePerUnit'
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
   *
   * WHAT IT IS AN AMOUNT OF is declared here, and it is the half `denominated: true` could not say.
   * A money amount and a time amount are the same field with the same dimension, so nothing could
   * ask whether a number said to be dollars was a plausible number of dollars — and `insuranceLimit`
   * sat at 100 through every run of this world, a deposit guarantee two thousand times smaller than
   * the deposit it guaranteed, with every household's balance uninsured and the wholesale-first run
   * of Banks Funding E4.a inverted. Declared, it is a CHECK (`test/params.test.ts`): every money
   * amount is measured against what this world's seed actually put in an account.
   */
  readonly denominated?: Denomination;
  readonly kind: ParamKind;
  readonly owner: ParamOwner;
  /** Why the value is what it is (Law 16: a comment says what the artefact cannot). */
  readonly why: string;
  readonly standsInFor?: PlaceholderDeath;
}

/**
 * Law 2: whether a reason names the ITEM that ends this number.
 *
 * Only a SHAPE is asked. The eleven policies and preferences that cite a future item cite it for
 * something else — a rate parliament owns from 19, a comparison that becomes real at 11 — and they
 * are still there afterwards; what changes is who sets them. A shape is a claim about the answer,
 * so an item that produces the answer is that claim's death, and Law 2 has a word for a shape with
 * a death in it. The check is on the reason because that is where the death was hiding: both
 * field guards above pass while the prose says 13h.
 *
 * IT MATCHES THE CLAIM AND NOT THE MENTION, which is item 0c's correction to its own step. The
 * regex was `worklist [0-9]`, and the plan is `docs/IMPLEMENTATION.md` now, so the step said to
 * widen the word to `item`. Widened, it fired on `goods.overhead.grain.itServices.machinery`
 * ("...which is what makes it an overhead rather than an input (item 7b)") and on
 * `goods.power.spoilage` ("Goods A3, E4, item 11: NOT DECLARED"). Neither is a death: this codebase
 * cites an item wherever a number came from, and both of those numbers STAY.
 *
 * A death is a sentence about the number's future — an item BUILDS the mechanism, and this number
 * is DELETED when it lands — so that is what is matched. Two rounds of tuning a word was the tell
 * (Law 12): the rule was never about the word.
 */
const SCHEDULED_DEATH =
  /\b(?:worklist|item)\s+[0-9][0-9a-z.]*\s+(?:builds|replaces|deletes|removes|ends|produces|takes it over)|(?:deleted|removed|replaced|dies|goes)\b[^.]{0,80}?\b(?:worklist|item)\s+[0-9]/i;

function namesAnItem(why: string): boolean {
  return SCHEDULED_DEATH.test(why);
}

/** What the register needs of the registry to count a declared amount in pieces (Law 8). */
export interface UnitSource {
  pieces(id: UnitId, named: number): Qty;
}

export interface ParamReport {
  readonly counts: Readonly<Record<ParamKind, number>>;
  readonly placeholders: readonly { id: ParamId; mechanism: string; item: string }[];
  readonly shapes: readonly ParamId[];
  /**
   * XI-14, §47 D5, Observer A1 (19.1): EVERY POLICY NUMBER, ITS OWNER, AND WHETHER ANYBODY HAS
   * MOVED IT — the value beside the mandate it belongs to.
   *
   * A reader of this world's numbers could see WHAT a policy is and not whose it is, so a rate the
   * central bank administers and a tax rate parliament will own read the same. They are different
   * facts about who may change them, and after §47 they are different facts about who is
   * ANSWERABLE for them. `set` is false where a number still stands at what the seed declared —
   * which is what a standing mandate looks like from outside (Polity D5).
   */
  readonly policies: readonly {
    id: ParamId;
    owner: ParamOwner;
    value: number;
    unit: string;
    set: boolean;
  }[];
}

export class ParamRegister {
  private readonly decls: Map<ParamId, ParamDecl>;
  private readonly units: UnitSource | undefined;
  /** 19.1: which policy numbers somebody has actually moved, as against standing where declared. */
  private readonly moved = new Set<ParamId>();

  /**
   * The units are what a value declared as an AMOUNT is counted in. They are optional because the
   * registry that holds them is itself built on a number in here (`resolution.pieceShift`), so
   * assembly reads that one out of a register with no units — and a register with no units refuses
   * every param that is an amount, rather than answering it in the wrong number.
   */
  constructor(decls: readonly ParamDecl[], units?: UnitSource) {
    this.units = units;
    this.decls = new Map<ParamId, ParamDecl>();
    for (const d of decls) this.declare(d);
  }

  /**
   * XI-14, Firm Birth A1 (12.4a): A NUMBER IS DECLARED WHEN ITS OWNER EXISTS. Every one the seed
   * knows is declared at assembly; a firm born after it has three of its own (its hurdle, its
   * horizon, the hours a tonne takes it), drawn under its own name the way a seeded firm's were,
   * and they are declared here the period it is born. The guards are the same guards; what
   * changes is only when they are asked. Twice is still twice (Law 4).
   */
  /**
   * XI-14 (17.7a): WHETHER THIS NUMBER HAS BEEN DECLARED YET. A party that declares its own
   * preference the first time it needs one has to be able to ask whether it already did — and
   * `decl` throws on a number that is not there, which is right for a read and wrong for a
   * question. Declaring twice is still refused; this is how a caller avoids asking.
   */
  has(id: ParamId): boolean {
    return this.decls.has(id);
  }

  declare(d: ParamDecl): void {
    if (this.decls.has(d.id)) throw new InvalidRegistry('Law 4', `parameter ${d.id} declared twice`);
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
        `${d.kind} ${d.id} names a worklist item in its reason: a ${d.kind} with a scheduled death IS a placeholder. Declare kind 'placeholder' with standsInFor { mechanism, item }`,
        { id: d.id },
      );
    }
    this.decls.set(d.id, Object.freeze({ ...d }));
  }

  /**
   * XI-14, Central Bank B1, §47 (18a.1): A POLICY NUMBER IS SET BY WHOEVER OWNS IT.
   *
   * Every number in this register was declared once and never moved, which is right for a
   * technology and a preference and WRONG for a policy: a policy is somebody's decision, and a
   * decision nobody can revisit is not one. A central bank that cannot change its own rate has no
   * policy at all — it has a constant with a mandate written beside it.
   *
   * THREE THINGS IT REFUSES, and between them they are why this is not a door onto every number:
   * a parameter that is not a POLICY cannot be set at all (a technology that moved would be a
   * fact about the world changing because somebody wanted it to); it may be set only by the owner
   * the declaration NAMES, so nobody sets another's; and the value is finite like any other. What
   * it does then is change the value and hand the caller what to record — the kernel publishes it,
   * because a policy decision is public by construction (Observer A1).
   */
  setByMandate(id: ParamId, value: number, by: ParamOwner, why: string): ParamDecl {
    const was = this.decl(id);
    if (was.kind !== 'policy') {
      throw new InvalidRegistry(
        'XI-14',
        `${id} is declared ${was.kind} and only a policy is set by a mandate`,
        { id, kind: was.kind },
      );
    }
    if (was.owner !== by) {
      throw new InvalidRegistry(
        'XI-14',
        `${id} is ${was.owner}'s and ${by} would set it`,
        { id, owner: was.owner, by },
      );
    }
    if (why.length === 0) throw new InvalidRegistry('Law 16', `setting ${id} has no reason`);
    const now: ParamDecl = { ...was, value: finite(value, `parameter ${id}`), why: `${was.why} ${why}` };
    this.decls.set(id, Object.freeze(now));
    this.moved.add(id);
    return now;
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
    if (d.denominated !== undefined) {
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
  /**
   * Item 16, Law 8: A PURE NUMBER, and the type says so. It is where a `Ratio` enters the world —
   * a share, a fraction, a multiple — and the point of the brand is what a `Ratio` CANNOT be: an
   * amount. A spread declared "over what a deposit returns" and used as an absolute rate (A-44),
   * or a leverage ratio called a cost of funds (A-58), are both a ratio put where a level was
   * wanted, and both are now a question the compiler asks at the door that would take it.
   */
  ratio(id: ParamId): Ratio {
    return asRatio(this.read(id, 'ratio'), id);
  }

  /** A rate per year. Dimensionless in the same sense and for the same reason. */
  perAnnum(id: ParamId): Ratio {
    return asRatio(this.read(id, 'perAnnum'), id);
  }

  /** A distance over the ground (13c.1). */
  km(id: ParamId): number {
    return this.read(id, 'km');
  }

  /** How far a loaded carrier gets over ground like this in a day (13c.1). */
  kmPerDay(id: ParamId): number {
    return this.read(id, 'kmPerDay');
  }

  /**
   * Money for one unit of something: a wage per hour, a level, a price per share.
   *
   * Item 16, Law 8: IT IS A PRICE AND THE TYPE SAYS SO, which is the same door `ratio` is. A
   * declared level enters the world here knowing it is money per piece, so it can be put on a grid,
   * multiplied by a quantity and printed — and cannot be added to a balance or read as an amount.
   */
  price(id: ParamId): PerPiece {
    return this.read(id, 'price') as PerPiece;
  }

  /**
   * Law 8, `E-8`: a declared level in the units A PERSON STATES IT IN — a dollar a tonne, a wage an
   * hour — which is not the same number as money per piece and can no longer be mistaken for one.
   * It crosses to the grid at `Registry.priceOf`, the one door between the two scales.
   */
  pricePerUnit(id: ParamId): PerNamedUnit {
    return this.read(id, 'pricePerUnit') as PerNamedUnit;
  }

  /**
   * Law 8: a declared AMOUNT as the state holds one — a count of that unit's indivisible pieces at
   * this world's resolution. The reader names the unit because the declaration is about an amount
   * and not about whose money or paper it is.
   */
  amount(id: ParamId, unit: UnitId): Qty {
    const d = this.decl(id);
    /**
     * Item 16: AND IT IS NOT A RATIO. A parameter declared as a share cannot be read as an amount
     * of anything — `denominated` is the declaration that says which, and the check below is the
     * runtime half of what `Ratio` says in the type. A-44 and A-58 are both this mistake.
     */
    if (d.denominated === undefined) {
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
    const placeholders: { id: ParamId; mechanism: string; item: string }[] = [];
    const shapes: ParamId[] = [];
    const policies: ParamReport['policies'][number][] = [];
    for (const d of this.decls.values()) {
      counts[d.kind] += 1;
      if (d.kind === 'placeholder' && d.standsInFor !== undefined) {
        placeholders.push({
          id: d.id,
          mechanism: d.standsInFor.mechanism,
          item: d.standsInFor.item,
        });
      }
      if (d.kind === 'shape') shapes.push(d.id);
      if (d.kind === 'policy') {
        policies.push({
          id: d.id,
          owner: d.owner,
          value: d.value,
          unit: d.unit,
          set: this.moved.has(d.id),
        });
      }
    }
    return { counts, placeholders, shapes, policies };
  }
}
