/**
 * THE PHYSICAL CONDITIONS A LINE STANDS IN: the name of the fact, and how a reader takes it off the
 * public event.
 *
 * @spec Commodities Spot B3 Goods B4 Freight B4 Insurers B4 Law 4 Law 15 Observer A3
 *
 * WHY THIS IS KERNEL DATA AND NOT THE ENVIRONMENT MODULE'S. Four modules read the same fact — the
 * firm whose crop stands in it, the carrier whose passage is blocked by it, the insurer whose
 * policies are struck by it, the household that burns more when it is cold — and a module never
 * imports another (ARCHITECTURE 4.9b). So the NAME of the fact and the shape of the event are the
 * kernel's, exactly as a good's name is (`registry/physical.ts`): a module that does not own a
 * thing still has to spell it, and four copies of this parser would be four ways to misread it.
 *
 * The environment module owns the MECHANISM — what the weather is, how wide it runs, how it carries
 * — and re-exports these so there is one spelling.
 */
import type { Period } from '../calendar/calendar.js';
import type { PlaceId } from '../core/ids.js';
import type { Event } from '../journal/journal.js';
import { Missing } from '../core/errors.js';

/** The public event the physical state crosses on, published once per region per period. */
export const ENVIRONMENT_STATE = 'environment.state';

/**
 * THE NAMES OF THE FACTS. A crop's recipe names one, a structure's kind names one, a policy will
 * name one — and none of those modules may import the one that publishes them (4.9b), so the names
 * are here with the event they arrive on. What each fact IS, how wide it runs and how it carries is
 * the environment module's (`mechanisms/environment/data.ts`), and it re-exports these.
 */
export const GROWING = 'growing';
export const WIND = 'wind';
export const WARMTH = 'warmth';

/** What a reader needs of the journal: this period's events of a kind, and nothing else. */
export interface EnvironmentReads {
  readonly period: Period;
  readonly journal: { ofKind(kind: string): readonly Event[] };
}

/**
 * What the world is doing to this region this period, each fact as a multiple of what the region
 * NORMALLY has. `undefined` is a world with no environment module in it — a scale model built to
 * exercise one door — and a reader says what it does about that rather than assuming an ordinary
 * period, because "normal" is an answer and a missing module is not one.
 */
export function conditionsIn(
  reads: EnvironmentReads,
  region: PlaceId,
): ReadonlyMap<string, number> | undefined {
  for (const e of reads.journal.ofKind(ENVIRONMENT_STATE)) {
    if (e.period !== reads.period || e.subjects[0] !== String(region)) continue;
    const facts = e.data['facts'];
    if (typeof facts !== 'object' || facts === null) continue;
    const out = new Map<string, number>();
    for (const [id, value] of Object.entries(facts as Record<string, unknown>)) {
      if (typeof value === 'number') out.set(id, value);
    }
    return out;
  }
  return undefined;
}

/** What a participant's view offers a reader: this period's public event about a region. */
export interface EnvironmentSeen {
  readonly period: Period;
  lastPublicAbout(kind: string, subject: string): { readonly some: true; readonly value: Event } | { readonly some: false };
}

/**
 * Observer A3 (12d.3): THE SAME FACT, READ FROM A PARTICIPANT'S VIEW — a household costing its
 * basket has no journal, and the event is public about its region, so the view's own public read
 * is the door. `undefined` as above: a world with no environment in it, or none published yet.
 */
export function conditionsSeen(view: EnvironmentSeen, region: PlaceId): ReadonlyMap<string, number> | undefined {
  const said = view.lastPublicAbout(ENVIRONMENT_STATE, String(region));
  if (!said.some || said.value.period !== view.period) return undefined;
  const facts = said.value.data['facts'];
  if (typeof facts !== 'object' || facts === null) return undefined;
  const out = new Map<string, number>();
  for (const [id, value] of Object.entries(facts as Record<string, unknown>)) {
    if (typeof value === 'number') out.set(id, value);
  }
  return out;
}

/**
 * Goods B4, Commodities Spot B3: HOW THIS PERIOD STANDS FOR A LINE EXPOSED TO THESE FACTS — the
 * product of the conditions it names, which is 1 for a line exposed to none and for a world with no
 * environment in it. A product because the influences are independent and multiplicative: a crop
 * that had two thirds of its water and two thirds of its warmth did not have two thirds of a
 * harvest, and adding them would say it did.
 *
 * It is never a bound and never a multiplier on a PRICE. What it multiplies is a quantity of a
 * physical thing at the point that thing is made, which is where the loss actually is (Law 3).
 */
export function conditionsFor(
  reads: EnvironmentReads,
  region: PlaceId,
  facts: readonly string[],
): number {
  if (facts.length === 0) return 1;
  return standingOf(conditionsIn(reads, region), region, facts);
}

/** 12d.3: the same product off a view's read — what a member burns against is what a crop stands in. */
export function conditionsStanding(view: EnvironmentSeen, region: PlaceId, facts: readonly string[]): number {
  if (facts.length === 0) return 1;
  return standingOf(conditionsSeen(view, region), region, facts);
}

function standingOf(here: ReadonlyMap<string, number> | undefined, region: PlaceId, facts: readonly string[]): number {
  if (here === undefined) return 1;
  let standing = 1;
  for (const fact of facts) {
    const value = here.get(fact);
    /**
     * A-4, Law 15: A LINE NAMING A FACT THIS WORLD PUBLISHES NOTHING FOR IS A DEFECT, AND IT SAYS SO.
     *
     * This read `if (value !== undefined) standing *= value` under a comment claiming "the world
     * that has the fact is where the check belongs, and assembly is where it fires". It does not
     * fire there: `world/assemble.ts` checks module ids, dependency cycles, money issuance and bank
     * choices, and `exposedTo` appears nowhere in it. So a recipe naming a fact nobody declares
     * multiplied by one for ever, in silence — a crop with no weather in it, looking exactly like a
     * crop having an ordinary year.
     *
     * The check could not have been at assembly and be this one: which facts a region has is a
     * fact about the PERIOD's published conditions, not about the module list, and a fact can be
     * regional. So it is here, at the read that needs it, where the world has already said what it
     * publishes — a world with NO environment at all is the case above and still answers 1, because
     * "this model has no weather" is an answer and "this line's weather went missing" is not.
     */
    if (value === undefined) {
      throw new Missing(
        'Goods B4',
        `a line is exposed to ${fact} and ${String(region)} publishes no such condition`,
        { region: String(region), fact, published: [...here.keys()] },
      );
    }
    standing *= value;
  }
  return standing;
}
