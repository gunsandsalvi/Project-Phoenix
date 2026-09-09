/**
 * The foundation seed: one region, one currency, a central bank, a treasury, two banks, three firms,
 * and household cells with dispersed endowments drawn from the seed's own random stream, holding
 * deposits and one sovereign benchmark line at an opening price.
 *
 * @spec Seed A1 Seed A3 Seed A4 Seed A5 Seed B1 Seed B1.a Seed B2 Seed B3 Seed B4 Seed C2 Seed C3 Seed C4 Seed C4.a Seed C4.b Seed E2 XI-14 XI-15 Money A1 Money D2 Law 2
 *
 * Every deposit is a bank's liability; every bond is the treasury's; nothing exists because
 * something needed it to (A4). Endowments are seed STATE, not parameters. The numbers the seed
 * states that a mechanism should produce are registered: the opening price of the line is a
 * placeholder for the auction (worklist 3); the dispersion of household endowments is a SHAPE, a
 * claim about the answer, counted until the mechanisms that produce wealth (wages, saving, XI-16)
 * replace it (worklist 4). How many cells stand for the population is a RESOLUTION (XI-15).
 *
 * Seeded TERMS (Seed C4.b) are permanent structure and are justified here: the line is a ten-year
 * fixed 2% semi-annual bond on ACT/ACT, chosen as a plain benchmark shape a treasury would issue,
 * with a remaining life so the seed has a maturity ahead of it (C3) rather than a bond at issue.
 */
import { civil } from '../calendar/civil.js';
import {
  cohortId,
  currencyCode,
  currencyUnit,
  instrumentId,
  marketId,
  moneyInstrumentId,
  paramId,
  partyId,
  regionId,
  type PartyId,
} from '../core/ids.js';
import { positiveCount } from '../core/num.js';
import { none, some } from '../core/option.js';
import { ANNUAL, SEMI_ANNUAL, rate } from '../core/rate.js';
import {
  PAR,
  SOVEREIGN_BOND,
  sovereignInstruments,
  type SovereignBondTerms,
} from '../mechanisms/sovereign-instruments/index.js';
import type { CellParty, NamedParty } from '../parties/party.js';
import { BANK, CENTRAL_BANK, FIRM, HOUSEHOLD, MONEY_KIND, TREASURY } from '../registry/profiles.js';
import type { Prng } from '../rng/prng.js';
import type { AssemblySpec } from '../world/assemble.js';
import { assemble, KERNEL_PARAMS } from '../world/assemble.js';
import type { SeedContext } from '../world/context.js';
import type { SystemModule } from '../world/module.js';
import type { World } from '../world/world.js';

export const PHX = currencyCode('PHX');
export const REGION = regionId('north');
export const CB = partyId('cb.north');
export const TREASURY_NORTH = partyId('treasury.north');
export const BANK_A = partyId('bank.a');
export const BANK_B = partyId('bank.b');
export const GOV_LINE = instrumentId('gov.north.2.0.2036-03-15');
export const GOV_MARKET = marketId('mkt.gov.north.2036');

const P = {
  cellsPerKey: paramId('seed.households.cellsPerKey'),
  membersPerKey: paramId('seed.households.membersPerKey'),
  depositPerMember: paramId('seed.households.depositPerMember'),
  bondPerMember: paramId('seed.households.bondPerMember'),
  dispersion: paramId('seed.households.dispersion'),
  openingPrice: paramId('seed.openingPrice.gov.north.2036'),
} as const;

/**
 * A multiplier dispersed around one: the product of `n` uniform draws scaled so the mean stays one.
 * A SHAPE (Law 2): the seed's claim about how unequal endowments are, replaced by mechanisms.
 */
function dispersed(rng: Prng, spread: number): number {
  const u = rng.next();
  return 1 + spread * (2 * u - 1);
}

export const foundationSeed: SystemModule = {
  id: 'seed.foundation',
  spec: 'Seed',
  requires: ['sovereign-instruments'],
  instrumentKinds: [],
  partyKinds: [],
  units: [],
  params: [
    {
      id: P.cellsPerKey,
      value: 2,
      unit: 'count',
      kind: 'resolution',
      owner: 'model',
      why: 'XI-15: how many cells stand for each (region, cohort, bank) population; change it and the answer must not move.',
    },
    {
      id: P.membersPerKey,
      value: 1000,
      unit: 'count',
      kind: 'resolution',
      owner: 'model',
      why: 'The population each key stands for; a read once populations are generated, a stated size until then.',
    },
    {
      id: P.depositPerMember,
      value: 0.3,
      unit: 'PHX',
      kind: 'shape',
      owner: 'model',
      why: 'Mean opening deposit per household; replaced by wages and saving (worklist 4).',
    },
    {
      id: P.bondPerMember,
      value: 0.2,
      unit: 'units of par',
      kind: 'shape',
      owner: 'model',
      why: 'Mean opening holding of the benchmark line per household; replaced by portfolio choice (worklist 4).',
    },
    {
      id: P.dispersion,
      value: 0.5,
      unit: 'ratio',
      kind: 'shape',
      owner: 'model',
      why: 'Seed B4: sizes are dispersed, or a sector of equals never produces a market; the width is a claim about the answer.',
    },
    {
      id: P.openingPrice,
      value: 0.98,
      unit: 'PHX per unit of par',
      kind: 'placeholder',
      owner: 'model',
      why: 'Seed C4: an opening condition the first clearing replaces; there is no auction yet to print one.',
      standsInFor: {
        mechanism: 'Sovereign C (the auction) and D (the secondary market)',
        worklistItem: '3',
      },
    },
  ],
  phases: [],
  participants: [],
  families: [],
  seed(ctx: SeedContext): void {
    const named = (
      id: PartyId,
      kind: NamedParty['kind'],
      name: string,
      bank: PartyId,
    ): NamedParty => ({
      id,
      kind,
      region: REGION,
      name,
      bank,
      representation: 'named',
      status: { alive: true },
    });
    ctx.parties.add(named(CB, CENTRAL_BANK, 'Central Bank of North', CB));
    ctx.parties.add(named(TREASURY_NORTH, TREASURY, 'Treasury of North', CB));
    ctx.parties.add(named(BANK_A, BANK, 'Bank A', CB));
    ctx.parties.add(named(BANK_B, BANK, 'Bank B', CB));
    ctx.parties.add(named(partyId('firm.1'), FIRM, 'Firm One', BANK_A));
    ctx.parties.add(named(partyId('firm.2'), FIRM, 'Firm Two', BANK_A));
    ctx.parties.add(named(partyId('firm.3'), FIRM, 'Firm Three', BANK_B));

    // Money instruments: one per issuer (Money A1, D2).
    for (const issuer of [CB, BANK_A, BANK_B]) {
      ctx.instruments.add({
        id: moneyInstrumentId(issuer, PHX),
        kind: MONEY_KIND,
        issuer,
        ccy: PHX,
        terms: { kind: MONEY_KIND },
        market: none(),
      });
    }

    // The sovereign benchmark line, outstanding with a remaining life (Seed C3).
    const terms: SovereignBondTerms = {
      kind: SOVEREIGN_BOND,
      coupon: rate(0.02, ANNUAL),
      couponPeriodicity: SEMI_ANNUAL,
      dayCount: 'ACT/ACT',
      issueDate: civil(2026, 3, 15),
      maturity: civil(2036, 3, 15),
    };
    ctx.instruments.add({
      id: GOV_LINE,
      kind: SOVEREIGN_BOND,
      issuer: TREASURY_NORTH,
      ccy: PHX,
      terms,
      market: some(GOV_MARKET),
    });
    ctx.openMarket({
      id: GOV_MARKET,
      name: 'North 2% 2036',
      instrument: GOV_LINE,
      ccy: PHX,
      rationing: 'proRata',
    });
    const opening = ctx.params.get(P.openingPrice);
    ctx.prices.write({
      instrument: GOV_LINE,
      market: GOV_MARKET,
      period: ctx.period,
      price: opening,
      ccy: PHX,
      provenance: { kind: 'opening' },
    });

    // Institutions and firms: endowments as state (Seed A3).
    ctx.endowMoney(TREASURY_NORTH, PHX, 500);
    ctx.endowMoney(BANK_A, PHX, 400);
    ctx.endowMoney(BANK_B, PHX, 300);
    ctx.endowMoney(partyId('firm.1'), PHX, 200);
    ctx.endowMoney(partyId('firm.2'), PHX, 150);
    ctx.endowMoney(partyId('firm.3'), PHX, 250);
    ctx.endowUnits(CB, GOV_LINE, 1500, opening);
    ctx.endowUnits(BANK_A, GOV_LINE, 500, opening);
    ctx.endowUnits(BANK_B, GOV_LINE, 500, opening);

    // Households: cells per (region, cohort, bank) key, weights summing to the key's population
    // (Seed B1.a), endowments dispersed around the stated means (Seed B4).
    const cells = positiveCount(ctx.params.get(P.cellsPerKey), 'cellsPerKey');
    const members = positiveCount(ctx.params.get(P.membersPerKey), 'membersPerKey');
    const spread = ctx.params.get(P.dispersion);
    const rng = ctx.rng.derive('households');
    for (const cohort of ctx.registry.cohorts) {
      for (const bank of [BANK_A, BANK_B]) {
        const weights = splitPopulation(members, cells);
        weights.forEach((weight, n) => {
          const cell: CellParty = {
            id: partyId(`hh.${cohort.id}.${bank}.${n}`),
            kind: HOUSEHOLD,
            region: REGION,
            name: `Households ${cohort.name} at ${bank} #${n}`,
            bank,
            representation: 'cell',
            status: { alive: true },
            weight,
            key: { region: REGION, cohort: cohortId(cohort.id), bank },
          };
          ctx.parties.add(cell);
          ctx.endowMoney(cell.id, PHX, ctx.params.get(P.depositPerMember) * dispersed(rng, spread));
          ctx.endowUnits(
            cell.id,
            GOV_LINE,
            ctx.params.get(P.bondPerMember) * dispersed(rng, spread),
            opening,
          );
        });
      }
    }
  },
};

/** Split a population into `cells` whole counts that sum to it exactly (Appendix A: a weight is a count). */
function splitPopulation(population: number, cells: number): number[] {
  const base = Math.floor(population / cells);
  const remainder = population - base * cells;
  const out: number[] = [];
  for (let i = 0; i < cells; i += 1) out.push(base + (i < remainder ? 1 : 0));
  return out;
}

export function foundationSpec(seed: string): AssemblySpec {
  return {
    seed,
    epoch: civil(2026, 1, 5),
    registry: {
      currencies: [{ code: PHX, name: 'Phoenix unit', centralBank: CB }],
      regions: [{ id: REGION, name: 'North', ccy: PHX }],
      units: [{ id: currencyUnit(PHX), name: 'PHX', countable: false }],
      cohorts: [
        { id: cohortId('working'), name: 'working age', fromAge: 18 },
        { id: cohortId('retired'), name: 'retired', fromAge: 65 },
      ],
      cellKey: ['region', 'cohort', 'bank'],
      lotFlow: 'FIFO',
    },
    params: [
      {
        id: KERNEL_PARAMS.periodDays,
        value: 7,
        unit: 'days',
        kind: 'resolution',
        owner: 'model',
        why: 'A period is a week (docs/ARCHITECTURE.md 4.7); coarser cannot place a weekly cycle, finer buys nothing yet.',
      },
      {
        id: KERNEL_PARAMS.cyclesPerPeriod,
        value: 5,
        unit: 'cycles',
        kind: 'resolution',
        owner: 'model',
        why: 'Money G1: a period holds more than one settlement cycle; five stands for business days.',
      },
      {
        id: KERNEL_PARAMS.worstInstances,
        value: 5,
        unit: 'count',
        kind: 'resolution',
        owner: 'model',
        why: 'Audit D2: how many worst instances a family reports; a reporting depth, not a behaviour.',
      },
    ],
    modules: [sovereignInstruments, foundationSeed],
  };
}

/** Build the foundation world. Reproducible from the seed value (Seed A5). */
export function foundationWorld(seed: string): World {
  return assemble(foundationSpec(seed));
}

export { PAR };
