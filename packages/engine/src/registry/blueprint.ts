/**
 * The blueprint: one language for what any fund may hold.
 *
 * @spec Fund Shares A4 Fund Shares D1 Fund Shares E1 Hedge Funds A4 Ratings A3 Law 2 Law 4 Law 15 Law 19
 *
 * A4 says a mandate is *"what it may hold, and the mandate is a real constraint on what it buys,
 * not a label"*. This is the constraint, and there is ONE of it for every vehicle this world has or
 * will have: a money fund, a credit fund, an exchange-traded fund, a segregated institutional
 * mandate and a levered strategy are the same object with this set differently.
 *
 * A BLUEPRINT IS A SET OF BANDS, AND A BAND NOT STATED IS NOT A CONSTRAINT. That is what lets one
 * language describe both a money fund — corporate and government paper, under a year, its own money,
 * highest grades — and a macro strategy, which states almost nothing and may hold almost anything.
 * The alternative, a list of instrument kind ids, is what this replaces: it cannot say "three to
 * seven years" at all, it describes a vehicle by enumerating the world, and it goes stale the first
 * time anybody issues a kind it was written before (Law 15).
 *
 * WHAT IT IS BANDED OVER IS ALL READS (`registry/universe.ts`), which is what makes the constraint
 * bite over time rather than at the moment somebody wrote it: a bond ages out of a duration band on
 * its own, and a company falls out of a size band by falling. Nothing here is stored on an asset and
 * nothing maintains it.
 */
import type { CurrencyCode } from '../core/ids.js';
import type { Cash } from '../core/measure.js';
import { rankOf, type Grade } from './grades.js';
import type { AssetClass, Classified } from './universe.js';

/** A band with either end left open, which is how "under a year" and "over a billion" are said. */
export interface Band {
  readonly from?: number;
  readonly to?: number;
}

export interface Blueprint {
  /**
   * What KINDS OF THING it may hold. Empty means it says nothing about class, which is a real
   * blueprint — a macro strategy holds what it likes — and not an absence (App A).
   */
  readonly classes: readonly AssetClass[];
  /**
   * Currency C4, A4: WHICH MONEY. Empty means multi-currency and it is a decision, not a default:
   * a fund that may hold another money has an FX exposure its investors agreed to.
   */
  readonly currencies: readonly CurrencyCode[];
  /** Bond N4: how long a dated claim may still have to run. The money fund's whole character. */
  readonly duration?: Band;
  /** N13.a: how far down the queue it will go. A senior-only mandate says `to: 1`. */
  readonly seniority?: Band;
  /** Whether it may hold what is pledged against something, or only what is not. */
  readonly secured?: boolean;
  /** Equity: public or private. Unstated means it does not care; stated is the real distinction. */
  readonly listed?: boolean;
  /** Equity A2: large, mid or small — banded over what the market says the issuer is worth. */
  readonly size?: Band;
  /**
   * Ratings A3, item 10e.7: THE WORST GRADE IT WILL HOLD, on the kernel's ordered scale. The asset's
   * own grade is the LOWEST any assessor published on the name (`lowestGrade`), which is the
   * conservative convention a mandate is written to — so ONE assessor downgrading is enough to put
   * a name below this line, and the name has to be sold. That is the channel §44 exists for.
   */
  readonly worstGrade?: Grade;
  /**
   * A name nobody has assessed has no grade, which is a different answer from the worst one — so a
   * blueprint states whether it will hold what nobody has looked at. A money fund will not; a
   * strategy whose whole business is names nobody covers will.
   */
  readonly unratedAllowed?: boolean;
}

/**
 * A4, Law 4: DOES THIS BLUEPRINT ADMIT THIS ASSET — the ONE function every vehicle in the world asks,
 * so that two funds cannot come to different answers about the same paper.
 *
 * Every band is a conjunction and every unstated band is silence. A blueprint that states nothing
 * admits everything, which is exactly what an unconstrained strategy is, and a blueprint that states
 * a band the asset cannot answer REFUSES it: a fund that banded on duration is a fund that holds
 * dated claims, and a share has no duration to be inside (App A — missing is missing, and it is not
 * a pass).
 */
export function admits(b: Blueprint, a: Classified, size: (c: Classified) => Cash | undefined): boolean {
  if (b.classes.length > 0 && !b.classes.includes(a.what)) return false;
  if (b.currencies.length > 0 && !b.currencies.includes(a.ccy)) return false;
  if (b.duration !== undefined && !within(b.duration, a.durationYears.some ? a.durationYears.value : undefined)) {
    return false;
  }
  if (b.seniority !== undefined && !within(b.seniority, a.seniority.some ? a.seniority.value : undefined)) {
    return false;
  }
  if (b.secured !== undefined && b.secured !== a.secured) return false;
  if (b.listed !== undefined && b.listed !== a.listed) return false;
  if (b.size !== undefined && !within(b.size, size(a))) return false;
  if (b.worstGrade !== undefined || b.unratedAllowed !== undefined) {
    if (!a.grade.some) {
      // Nobody has looked at this name. Whether that is holdable is the blueprint's to say.
      if (b.unratedAllowed !== true) return false;
    } else if (b.worstGrade !== undefined && rankOf(a.grade.value) > rankOf(b.worstGrade)) {
      // The scale is best-first, so a bigger rank is a worse grade than the line allows.
      return false;
    }
  }
  return true;
}

/**
 * A band contains a number it can see. A band the asset cannot answer is a band it FAILS: the fund
 * stated a constraint about a property this asset does not have, and admitting it anyway would be
 * the `?? 0` this codebase does not do.
 */
function within(band: Band, value: number | undefined): boolean {
  if (value === undefined) return false;
  if (band.from !== undefined && value < band.from) return false;
  if (band.to !== undefined && value > band.to) return false;
  return true;
}
