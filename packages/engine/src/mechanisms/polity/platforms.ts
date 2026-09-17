/**
 * The parties, and what each of them would do with parliament's numbers.
 *
 * @spec Polity A2 Polity B2 Polity D1 Polity D2 Polity D3 Polity D4 Law 16
 *
 * THREE PLATFORMS, and they differ on the things a polity actually differs on: how much the state
 * takes, how much it hands back, how much it buys, how many people it employs, how hard it holds
 * the currency, and how tightly it regulates. None of them is a caricature and none is the middle
 * of the other two on every number — a party is a bundle of positions, and what makes an election
 * interesting is that a cell can prefer one party's tax rate and another's transfers.
 *
 * The positions are STATED, publicly, in advance (A2), each with its reason, and every one of them
 * is a number parliament owns — `platformPositions` refuses anything else at assembly, so a party
 * cannot promise the central bank's rate or a price (D5, F2).
 */
import type { PlatformDecl } from '../../registry/platforms.js';

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
      { on: 'land.planning.releasePerPeriod', value: 0.9, why: 'It releases land readily: a shortage of houses is a shortage of permissions.' },
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
      { on: 'land.planning.releasePerPeriod', value: 0.5, why: 'Land is released slowly, which is what the people who already own houses prefer.' },
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
      { on: 'land.planning.releasePerPeriod', value: 0.7, why: 'It releases what the last parliament released, and blames the shortage on the one before.' },
    ],
  },
];
