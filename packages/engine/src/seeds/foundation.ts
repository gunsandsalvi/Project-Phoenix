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
 * Seeded TERMS (Seed C4.b) are permanent structure and are justified here: the treasury opens with a
 * MATURITY PROFILE (Seed C3, Treasury D4.a) — bills at three, six and twelve months and bonds at two,
 * five and ten years, on the issuer's own quarterly maturity grid, no two redeeming in one period —
 * so there is always a wall ahead of it to fund and a curve with more than one point on it. Each bond
 * carries the coupon that makes it par at the one opening yield, so the seed asserts a LEVEL and no
 * SHAPE: the curve opens flat and the auctions and the secondary market give it whatever shape they
 * find. That single yield is the placeholder, and it dies at the first traded print on each line.
 */
import { civil } from '../calendar/civil.js';
import {
  cohortId,
  currencyCode,
  currencyUnit,
  curveFamilyId,
  instrumentId,
  marketId,
  moneyInstrumentId,
  paramId,
  partyId,
  regionId,
  type PartyId,
} from '../core/ids.js';
import { Missing } from '../core/errors.js';
import type { DayCount } from '../calendar/daycount.js';
import { priceAt } from '../prices/curve.js';
import { positiveCount } from '../core/num.js';
import { none, some } from '../core/option.js';
import { ANNUAL, SEMI_ANNUAL, rate } from '../core/rate.js';
import {
  PAR,
  SOVEREIGN_BILL,
  SOVEREIGN_BOND,
  sovereignInstruments,
  type SovereignBillTerms,
  type SovereignBondTerms,
} from '../mechanisms/sovereign-instruments/index.js';
import { centralBankOmo } from '../mechanisms/central-bank-omo/index.js';
import { expectations } from '../mechanisms/expectations/index.js';
import { goods } from '../mechanisms/goods/index.js';
import { sovereignAuction } from '../mechanisms/sovereign-auction/index.js';
import { sovereignCurve } from '../mechanisms/sovereign-curve/index.js';
import { treasury } from '../mechanisms/treasury/index.js';
import type { CellParty, NamedParty } from '../parties/party.js';
import { displayName } from '../registry/naming.js';
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
/** The ten-year benchmark: the line every other price is quoted against (Sovereign D4). */
export const GOV_LINE = instrumentId('gov.north.2036-03-15');
export const GOV_MARKET = marketId('mkt.gov.north.2036-03-15');
/** Sovereign D3.a: one owner of the curve, one convention, declared by the module that owns it. */
export const GOV_CURVE = curveFamilyId('gov.north');

/** The issuer's own maturity grid: it places every line it brings on one of these days (B3.a). */
export const MATURITY_MONTHS: readonly number[] = [3, 6, 9, 12];
export const MATURITY_DAY = 15;

/** What the seed opens outstanding: a profile, not one line (Seed C3, Treasury D4.a). */
interface SeedLine {
  readonly id: string;
  /** Which of the two sovereign instruments this line is (Sovereign B1: two, not one with a flag). */
  readonly paper: 'bill' | 'bond';
  readonly maturity: { y: number; m: number; d: number };
  /** What the central bank opens holding: near the share its own policy names (Central Bank C1). */
  readonly cb: number;
  readonly bankA: number;
  readonly bankB: number;
  /** Per member of every household cell, before dispersion (Seed B4). */
  readonly perMember: number;
}

const SEED_LINES: readonly SeedLine[] = [
  { id: 'gov.north.bill.2026-06-15', paper: 'bill', maturity: { y: 2026, m: 6, d: 15 }, cb: 0, bankA: 300, bankB: 200, perMember: 0 },
  { id: 'gov.north.bill.2026-09-15', paper: 'bill', maturity: { y: 2026, m: 9, d: 15 }, cb: 0, bankA: 200, bankB: 300, perMember: 0 },
  { id: 'gov.north.bill.2027-03-15', paper: 'bill', maturity: { y: 2027, m: 3, d: 15 }, cb: 0, bankA: 250, bankB: 250, perMember: 0.02 },
  { id: 'gov.north.2028-03-15', paper: 'bond', maturity: { y: 2028, m: 3, d: 15 }, cb: 150, bankA: 200, bankB: 100, perMember: 0.04 },
  { id: 'gov.north.2031-03-15', paper: 'bond', maturity: { y: 2031, m: 3, d: 15 }, cb: 180, bankA: 150, bankB: 150, perMember: 0.06 },
  { id: 'gov.north.2036-03-15', paper: 'bond', maturity: { y: 2036, m: 3, d: 15 }, cb: 205, bankA: 150, bankB: 150, perMember: 0.08 },
];

/** One day count for the seeded paper, so an opening price and its yield use one convention. */
const SEED_DAY_COUNT: DayCount = 'ACT/ACT';

const P = {
  cellsPerKey: paramId('seed.households.cellsPerKey'),
  membersPerKey: paramId('seed.households.membersPerKey'),
  depositPerMember: paramId('seed.households.depositPerMember'),
  bondPerMember: paramId('seed.households.bondPerMember'),
  dispersion: paramId('seed.households.dispersion'),
  openingYield: paramId('seed.openingYield'),
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
  curveFamilies: [],
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
      id: P.openingYield,
      value: 0.02,
      unit: 'per annum',
      kind: 'placeholder',
      owner: 'model',
      why: 'Seed C4: the one level the opening world is priced at, flat across the profile so the seed asserts no shape. Every line opens at the price this yield gives it, and the auction and the secondary market replace it line by line as each one trades. It stays while some lines still carry an opening print: the sovereign secondary market has two sides only for the banks whose buffer moved, and a household that holds a line cannot yet act on it (Sovereign E2.f).',
      standsInFor: {
        mechanism: 'Sovereign E2.f (households and firms holding it directly, so every line has a two-sided market)',
        worklistItem: '4',
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
        issuer: some(issuer),
        ccy: PHX,
        terms: { kind: MONEY_KIND },
        market: none(),
      });
    }

    // The maturity profile, outstanding with remaining lives (Seed C3, Treasury D4.a). Every bond
    // carries the coupon that makes it par at the opening yield, so nothing but a level is claimed.
    const y = ctx.params.get(P.openingYield);
    const opening = new Map<string, number>();
    for (const line of SEED_LINES) {
      const id = instrumentId(line.id);
      const market = marketId(`mkt.${line.id}`);
      const maturity = civil(line.maturity.y, line.maturity.m, line.maturity.d);
      const terms: SovereignBondTerms | SovereignBillTerms =
        line.paper === 'bond'
          ? {
              kind: SOVEREIGN_BOND,
              coupon: rate(y, ANNUAL),
              couponPeriodicity: SEMI_ANNUAL,
              dayCount: SEED_DAY_COUNT,
              issueDate: ctx.calendar.epoch,
              maturity,
            }
          : { kind: SOVEREIGN_BILL, issueDate: ctx.calendar.epoch, maturity };
      ctx.instruments.add({
        id,
        kind: line.paper === 'bond' ? SOVEREIGN_BOND : SOVEREIGN_BILL,
        issuer: some(TREASURY_NORTH),
        ccy: PHX,
        terms,
        market: some(market),
      });
      ctx.openMarket({
        id: market,
        name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
        instrument: id,
        ccy: PHX,
        rationing: 'proRata',
      });
      const flows = ctx.registry
        .instrumentKind(line.paper === 'bond' ? SOVEREIGN_BOND : SOVEREIGN_BILL)
        .cashFlows(ctx.instruments.get(id), ctx.calendar.epoch, ctx.calendar);
      const price = priceAt(flows, y, ctx.calendar.epoch, SEED_DAY_COUNT, `opening ${line.id}`);
      opening.set(line.id, price);
      ctx.prices.write({
        instrument: id,
        market,
        period: ctx.period,
        price,
        ccy: PHX,
        provenance: { kind: 'opening' },
      });
    }

    // Institutions and firms: endowments as state (Seed A3).
    // Treasury D4.b: it opens with a buffer, because the alternative to one is dependence on every
    // single auction clearing. The programme manages it from here.
    ctx.endowMoney(TREASURY_NORTH, PHX, 900);
    ctx.endowMoney(BANK_A, PHX, 400);
    ctx.endowMoney(BANK_B, PHX, 300);
    ctx.endowMoney(partyId('firm.1'), PHX, 200);
    ctx.endowMoney(partyId('firm.2'), PHX, 150);
    ctx.endowMoney(partyId('firm.3'), PHX, 250);
    for (const line of SEED_LINES) {
      const id = instrumentId(line.id);
      const price = openingOf(opening, line.id);
      if (line.cb > 0) ctx.endowUnits(CB, id, line.cb, price);
      if (line.bankA > 0) ctx.endowUnits(BANK_A, id, line.bankA, price);
      if (line.bankB > 0) ctx.endowUnits(BANK_B, id, line.bankB, price);
    }

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
          const scale = ctx.params.get(P.bondPerMember) * dispersed(rng, spread);
          for (const line of SEED_LINES) {
            if (line.perMember === 0) continue;
            ctx.endowUnits(
              cell.id,
              instrumentId(line.id),
              line.perMember * scale,
              openingOf(opening, line.id),
            );
          }
        });
      }
    }
  },
};

/** The opening price the seed computed for a line; a line with none is a defect, never a default. */
function openingOf(opening: ReadonlyMap<string, number>, id: string): number {
  const p = opening.get(id);
  if (p === undefined) throw new Missing('Seed C4', `no opening price for ${id}`, { id });
  return p;
}

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
      curveFamilies: [],
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
    modules: [
      expectations,
      goods(),
      sovereignInstruments,
      sovereignCurve(TREASURY_NORTH, PHX),
      sovereignAuction,
      treasury,
      centralBankOmo,
      foundationSeed,
    ],
  };
}

/** Build the foundation world. Reproducible from the seed value (Seed A5). */
export function foundationWorld(seed: string): World {
  return assemble(foundationSpec(seed));
}

export { PAR };
