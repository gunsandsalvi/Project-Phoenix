/**
 * Platforms: what each party says it would do with every number parliament owns.
 *
 * @spec Polity A2 Polity B2 Polity C3 Polity D5 Polity F2 Law 2 Law 15 Law 16
 *
 * A2 asks for *a stated position on every POLICY primitive the parliament owns*, and the emphasis is
 * on EVERY: a party that states a tax rate and says nothing about transfers has not given a cell
 * enough to vote on, and whatever it did about the rest would be a number arriving from nowhere
 * after the election. So a platform is COMPLETE by construction, and assembly refuses one that is
 * not — a missing position, a position on something parliament does not own, or two positions on
 * one number.
 *
 * WHAT IS PARLIAMENT'S IS A READ OF THE REGISTER, never a list kept here. Every policy declares its
 * owner (XI-14, D5), so the set a platform must cover is whatever the world's modules declared as
 * parliament's — a world that adds a tax has one more thing every platform must answer, and it
 * fails to assemble until they do. That is the check doing the work a comment used to.
 *
 * A POSITION MAY NAME A FAMILY. Some of parliament's numbers are one per money — the inflation
 * target is per currency — and a party does not have a different view of the target in each: it
 * states *the target* and that is its position in every money this world has. A position whose name
 * ends in a dot covers the family under it, and the coverage check is the same either way.
 */
import type { ParamId } from '../core/ids.js';
import { absolute, asRatio } from '../core/measure.js';
import { InvalidRegistry } from '../core/errors.js';
import type { Dimension, ParamDecl } from './params.js';

export interface PlatformPosition {
  /** A parameter id, or a family prefix ending in a dot — `centralBank.target.` covers every money. */
  readonly on: string;
  readonly value: number;
  /** Law 16: why this party wants this number here. It is what a cell is voting on. */
  readonly why: string;
}

export interface PlatformDecl {
  readonly id: string;
  readonly name: string;
  readonly positions: readonly PlatformPosition[];
}

/**
 * Law 8 (19.6): WHICH NUMBERS ARE COUNTED IN WHOLE THINGS. A position on one of them is a whole
 * one, and so is the mandate a coalition's seats come to — half a hectare of consent, a third of a
 * period of severance and two fifths of a day of reporting lag are not values the unit can hold,
 * and the read that would have to hold one throws where it reads (`asQty`) rather than here.
 */
export const COUNTED: readonly Dimension[] = ['periods', 'days', 'months', 'years', 'count'];

export const isCounted = (d: ParamDecl): boolean => COUNTED.includes(d.dimension);

/** Whether a position covers a parameter: its own name, or the family it names. */
const covers = (on: string, id: ParamId): boolean =>
  on.endsWith('.') ? String(id).startsWith(on) : String(id) === on;

/**
 * A2, D5: WHAT EVERY PLATFORM WOULD SET EVERY ONE OF PARLIAMENT'S NUMBERS TO.
 *
 * It throws rather than reporting, and it throws at ASSEMBLY: a platform is a declaration and an
 * incomplete one is a defect in the declaration, not a finding about the world (Error discipline).
 * The three refusals are the three ways a table of this shape goes wrong, and each says which
 * number and which party.
 */
export function platformPositions(
  rows: readonly PlatformDecl[],
  policies: readonly ParamDecl[],
): Map<string, Map<ParamId, number>> {
  const mine = policies.filter((d) => d.kind === 'policy' && d.owner === 'parliament');
  const out = new Map<string, Map<ParamId, number>>();
  for (const row of rows) {
    if (out.has(row.id)) {
      throw new InvalidRegistry('Polity A2', `two platforms called ${row.id}`, { platform: row.id });
    }
    const said = new Map<ParamId, number>();
    for (const d of mine) {
      const found = row.positions.filter((p) => covers(p.on, d.id));
      if (found.length === 0) {
        throw new InvalidRegistry(
          'Polity A2',
          `${row.id} states no position on ${String(d.id)}, which parliament owns`,
          { platform: row.id, id: String(d.id) },
        );
      }
      if (found.length > 1) {
        throw new InvalidRegistry(
          'Polity A2',
          `${row.id} states ${found.length} positions on ${String(d.id)}`,
          { platform: row.id, id: String(d.id), positions: found.map((p) => p.on).join(', ') },
        );
      }
      const only = found[0];
      if (only === undefined) continue;
      if (only.why.length === 0) {
        throw new InvalidRegistry('Law 16', `${row.id} gives no reason for ${only.on}`, {
          platform: row.id,
        });
      }
      // Law 8: and it is stated in the number's own unit. A position of nine tenths on a number
      // counted in hectares is a party that meant a share and wrote a quantity; the world does not
      // open with it, because what it would produce is a mandate no read of that number can hold.
      if (isCounted(d) && !Number.isInteger(only.value)) {
        throw new InvalidRegistry(
          'Law 8',
          `${row.id} states ${String(only.value)} on ${String(d.id)}, which is counted in whole ${d.dimension === 'count' ? 'things' : d.dimension} ("${d.unit}")`,
          { platform: row.id, id: String(d.id), value: only.value },
        );
      }
      said.set(d.id, only.value);
    }
    // F2, D5: and nothing else. A platform with a position on a number parliament does not own is a
    // party promising something it could not do — the central bank's rate, a price, somebody else's
    // mandate — and the refusal is where the promise is made rather than where it would have failed.
    for (const p of row.positions) {
      if (!mine.some((d) => covers(p.on, d.id))) {
        throw new InvalidRegistry(
          'Polity D5',
          `${row.id} states a position on ${p.on}, which parliament does not own`,
          { platform: row.id, on: p.on },
        );
      }
    }
    out.set(row.id, said);
  }
  return out;
}

/**
 * B2: HOW FAR APART TWO PLATFORMS ARE, over the numbers they both state a position on.
 *
 * It is the ordinary distance between two positions on each number, taken as a share of the spread
 * the platforms themselves cover on that number — so a rate all of them would set within a point of
 * each other is not a chasm, and one they disagree about by a factor of three is. Nothing is
 * normalised against a stated scale, because a scale nobody agreed on would be this file deciding
 * what politics is about: what makes two positions far apart is how far apart the OTHERS are.
 */
export function distanceBetween(
  a: ReadonlyMap<ParamId, number>,
  b: ReadonlyMap<ParamId, number>,
  spread: ReadonlyMap<ParamId, number>,
): number {
  let total = 0;
  let counted = 0;
  for (const [id, mine] of a) {
    const theirs = b.get(id);
    const width = spread.get(id);
    if (theirs === undefined || width === undefined || width <= 0) continue;
    // Law 6: a DISTANCE is unsigned because that is what a distance is, and `absolute` is where
    // this world says so — the same read a balance's size goes through (`core/measure.ts`).
    total += absolute(asRatio(mine - theirs, 'how far apart the two positions are'), 'how far apart they are') / width;
    counted += 1;
  }
  return counted === 0 ? 0 : total / counted;
}

/** B2: how far apart the platforms are on each number, which is what a distance is measured against. */
export function spreadAcross(
  positions: ReadonlyMap<string, ReadonlyMap<ParamId, number>>,
): Map<ParamId, number> {
  const low = new Map<ParamId, number>();
  const high = new Map<ParamId, number>();
  for (const said of positions.values()) {
    for (const [id, value] of said) {
      const l = low.get(id);
      const h = high.get(id);
      if (l === undefined || value < l) low.set(id, value);
      if (h === undefined || value > h) high.set(id, value);
    }
  }
  const out = new Map<ParamId, number>();
  for (const [id, l] of low) {
    const h = high.get(id);
    if (h !== undefined) out.set(id, h - l);
  }
  return out;
}

/**
 * A2, D1–D4: THE PARTIES OF THIS WORLD, and what each would do with parliament's numbers.
 *
 * They are DATA, in the registry, which is where a table of stated numbers belongs (Law 15 and the
 * numeric-literal rule both): three parties that differ on what polities differ on — how much the
 * state takes, hands back, buys and employs, how hard it holds the currency, how wide the deposit
 * guarantee is, when companies must publish, how fast land is released. None of them is the middle
 * of the other two on every number, so a cell can prefer one party's tax rate and another's
 * transfers, which is what makes an election a question rather than a sort.
 */
export const PLATFORMS: readonly PlatformDecl[] = [
  {
    id: 'broad',
    name: 'the broad state',
    positions: [
      { on: 'treasury.tax.profits', value: 0.32, why: 'A company earns where the state built the road it earns on, so it pays for the road.' },
      { on: 'treasury.tax.interestIncome', value: 0.28, why: 'What is earned by holding paper is earned without working, and it is taxed accordingly.' },
      { on: 'treasury.tax.gains', value: 0.28, why: 'A gain and a wage are both income to whoever gets them, and this party taxes them alike.' },
      { on: 'treasury.tax.income', value: 0.2, why: 'Higher than the others, because what it pays for is the transfers below.' },
      { on: 'treasury.tax.consumption', value: 0.08, why: 'Lower than the others: a tax on spending falls hardest on the households that spend all of it.' },
      { on: 'treasury.outlays.transfers.perMember', value: 900, why: 'The largest transfer of the three. It is what this party is FOR, and it is what its rates pay for.' },
      { on: 'treasury.purchases.perPeriod', value: 3_500_000, why: 'A state that builds: the procurement budget buys buildings, care and the advice to plan them.' },
      { on: 'treasury.publicService.hours', value: 240_000, why: 'It employs directly rather than buying the same work from somebody who employs.' },
      { on: 'centralBank.target.', value: 0.025, why: 'A little more inflation is a little less unemployment, and this party would take that trade.' },
      { on: 'labour.severance.periods', value: 8, why: 'A firing costs more here: a job is a household’s whole income and losing one should be dear to whoever ends it.' },
      { on: 'labour.retirementAge', value: 64, why: 'People stop working sooner, and the pension pays for it.' },
      { on: 'pensions.contribution.employeeShare', value: 0.05, why: 'Less from the member.' },
      { on: 'pensions.contribution.employerShare', value: 0.1, why: 'More from the employer, which is where this party thinks the money is.' },
      { on: 'pensions.replacementShare', value: 0.6, why: 'A pension that replaces most of a wage, which is what the contributions above are for.' },
      { on: 'regulation.depositInsurance.limit', value: 150_000, why: 'A wide guarantee: a run is a public harm and the state stands behind more of the deposits.' },
      { on: 'regulation.depositInsurance.premium', value: 0.002, why: 'And the banks pay for the guarantee they are getting.' },
      { on: 'regulation.riskWeight.cds.sold', value: 1.5, why: 'Writing protection is taking a risk, and this party makes it expensive to take.' },
      { on: 'funds.accreditedWealthPerMember', value: 3_000_000, why: 'Fewer households may be sold what a pool sells: it would rather they were protected than free.' },
      { on: 'reporting.lag.days', value: 30, why: 'Companies publish sooner, because what they publish is what everybody else prices on.' },
      { on: 'land.planning.releasePerPeriod', value: 140, why: 'It releases land readily: a shortage of houses is a shortage of permissions, and it grants more of them than either of the others.' },
    ],
  },
  {
    id: 'lean',
    name: 'the lean state',
    positions: [
      { on: 'treasury.tax.profits', value: 0.18, why: 'A company that keeps what it earns invests it, and the investment is what employs people.' },
      { on: 'treasury.tax.interestIncome', value: 0.12, why: 'Saving is deferred spending and this party does not punish it.' },
      { on: 'treasury.tax.gains', value: 0.1, why: 'The lowest rate on the list: risk taken with money already taxed once.' },
      { on: 'treasury.tax.income', value: 0.1, why: 'Low, and it is the point: what a household earns is a household’s.' },
      { on: 'treasury.tax.consumption', value: 0.14, why: 'Higher than the others, because a tax on what is spent is harder to avoid than a tax on what is earned.' },
      { on: 'treasury.outlays.transfers.perMember', value: 300, why: 'A floor and not a living: this party would rather the money stayed where it was earned.' },
      { on: 'treasury.purchases.perPeriod', value: 1_200_000, why: 'It buys what it must and no more.' },
      { on: 'treasury.publicService.hours', value: 80_000, why: 'A small public payroll: what can be bought is bought rather than staffed.' },
      { on: 'centralBank.target.', value: 0.015, why: 'Inflation is a tax nobody voted for, and this party sets the target below the others.' },
      { on: 'labour.severance.periods', value: 2, why: 'A cheap firing is a cheap hiring: a firm that can let somebody go will take somebody on.' },
      { on: 'labour.retirementAge', value: 69, why: 'People live longer and work longer, and the pension costs less because of it.' },
      { on: 'pensions.contribution.employeeShare', value: 0.09, why: 'More from the member, whose pension it is.' },
      { on: 'pensions.contribution.employerShare', value: 0.04, why: 'Less from the employer: a contribution is a wage cost and a wage cost is a job.' },
      { on: 'pensions.replacementShare', value: 0.35, why: 'A pension that keeps somebody rather than one that replaces a wage.' },
      { on: 'regulation.depositInsurance.limit', value: 60_000, why: 'A narrow guarantee: a depositor that is covered whatever happens has no reason to ask who it is banking with.' },
      { on: 'regulation.depositInsurance.premium', value: 0.0005, why: 'And it costs the banks little, because there is little being guaranteed.' },
      { on: 'regulation.riskWeight.cds.sold', value: 0.8, why: 'Capital held against a written position is capital not lent to somebody who wants it.' },
      { on: 'funds.accreditedWealthPerMember', value: 800_000, why: 'More households may buy what a pool sells: it treats them as able to decide for themselves.' },
      { on: 'reporting.lag.days', value: 60, why: 'Companies publish later: an accounting deadline is a cost, and a small company feels it most.' },
      { on: 'land.planning.releasePerPeriod', value: 60, why: 'Land is released slowly, which is what the people who already own houses prefer.' },
    ],
  },
  {
    id: 'steady',
    name: 'the steady hand',
    positions: [
      { on: 'treasury.tax.profits', value: 0.25, why: 'Where the rate has been. This party’s position is that the rates are not the problem.' },
      { on: 'treasury.tax.interestIncome', value: 0.2, why: 'Unchanged, and deliberately so: a rate that moves every parliament is a rate nobody can plan against.' },
      { on: 'treasury.tax.gains', value: 0.18, why: 'Unchanged.' },
      { on: 'treasury.tax.income', value: 0.15, why: 'Unchanged.' },
      { on: 'treasury.tax.consumption', value: 0.1, why: 'Unchanged.' },
      { on: 'treasury.outlays.transfers.perMember', value: 600, why: 'Between the two, and it says so rather than claiming to be neither.' },
      { on: 'treasury.purchases.perPeriod', value: 2_000_000, why: 'It keeps what is there running, which is most of what a state buys.' },
      { on: 'treasury.publicService.hours', value: 150_000, why: 'The payroll it inherited.' },
      { on: 'centralBank.target.', value: 0.02, why: 'The number the institutions this world imports its primitives from actually use.' },
      { on: 'labour.severance.periods', value: 4, why: 'Enough to make a firing a decision and not enough to stop a hiring.' },
      { on: 'labour.retirementAge', value: 66, why: 'A year or two later than it was, which is what demography does to a pension.' },
      { on: 'pensions.contribution.employeeShare', value: 0.07, why: 'Split evenly with the employer, which is the convention it inherited.' },
      { on: 'pensions.contribution.employerShare', value: 0.07, why: 'The other half of the same split.' },
      { on: 'pensions.replacementShare', value: 0.45, why: 'What the contributions above actually fund, which is what it thinks a pension promise should be.' },
      { on: 'regulation.depositInsurance.limit', value: 100_000, why: 'The figure most systems settled on, for the reason most of them settled on it.' },
      { on: 'regulation.depositInsurance.premium', value: 0.001, why: 'Priced to cover what the fund has actually paid out.' },
      { on: 'regulation.riskWeight.cds.sold', value: 1, why: 'A unit of exposure carries a unit of weight: the simplest rule, and the hardest to argue with.' },
      { on: 'funds.accreditedWealthPerMember', value: 1_500_000, why: 'Where the line is. It would rather move it slowly than argue about where it should be.' },
      { on: 'reporting.lag.days', value: 45, why: 'Long enough to close a set of books, short enough that they are still about this quarter.' },
      { on: 'land.planning.releasePerPeriod', value: 100, why: 'It releases what the last parliament released, and blames the shortage on the one before.' },
    ],
  },
];
