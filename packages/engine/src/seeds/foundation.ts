/**
 * The foundation seed: one region, one currency, a central bank, a treasury, two banks, three firms
 * with stock and a line running, and household cells that open with nothing at all.
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
  type ParamId,
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
import { firms } from '../mechanisms/firms/index.js';
import { goodId, goodMarketId, goods, wipId } from '../mechanisms/goods/index.js';
import { households } from '../mechanisms/households/index.js';
import { labour } from '../mechanisms/labour/index.js';
import { sovereignAuction } from '../mechanisms/sovereign-auction/index.js';
import { sovereignCurve } from '../mechanisms/sovereign-curve/index.js';
import { treasury } from '../mechanisms/treasury/index.js';
import type { CellParty, NamedParty } from '../parties/party.js';
import { displayName } from '../registry/naming.js';
import { BANK, CENTRAL_BANK, FIRM, HOUSEHOLD, MONEY_KIND, TREASURY } from '../registry/profiles.js';
import type { ParamDecl } from '../registry/params.js';
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

/**
 * What each firm opens with of what it makes, and what is already on its line (Seed D1: a stock
 * consistent with the flows that will act on it). It is ONE PERIOD of what the chain downstream of
 * it can take — the households' own income says what that is — and not a hoard: a firm sitting on a
 * year of stock would produce nothing for a year, and the seed would have decided that.
 */
interface SeedStock {
  readonly firm: string;
  readonly subUnit: string;
  /** Units finished and ready to sell. */
  readonly finished: number;
  /** Units started and not yet off the line: only a good whose batch takes longer than a period. */
  readonly onTheLine: number;
  /** Seed C4: what the market it has never traded in opens at, per unit. */
  readonly opensAt: number;
  readonly why: string;
}

const SEED_STOCK: readonly SeedStock[] = [
  {
    firm: 'firm.1',
    subUnit: 'grain',
    finished: 90,
    onTheLine: 90,
    opensAt: 0.4,
    why: 'Grain takes two periods to grow, so a world that opens with an empty field produces nothing for two of them and the mill has nothing to buy. One crop in the barn and one in the ground is what a going concern looks like.',
  },
  {
    firm: 'firm.2',
    subUnit: 'flour',
    finished: 75,
    onTheLine: 0,
    opensAt: 0.6,
    why: 'Milling is inside the period, so there is nothing on the line; what the mill opens with is what it has already milled.',
  },
  {
    firm: 'firm.3',
    subUnit: 'bread',
    finished: 105,
    onTheLine: 0,
    opensAt: 1.2,
    why: 'A week of bread in the shop. It goes stale at a quarter a period, so what is not sold is a real loss from the first period on.',
  },
];

/** The inputs each firm opens with: enough for the batch it will start before it can buy more. */
const SEED_INPUTS: readonly { readonly firm: string; readonly subUnit: string; readonly qty: number }[] = [
  { firm: 'firm.2', subUnit: 'grain', qty: 110 },
  { firm: 'firm.3', subUnit: 'flour', qty: 75 },
];

/**
 * Seed C4: what the stock cost whoever is holding it, as a share of what its market opens at. It is
 * below the opening price because a firm holding stock it could only sell at a loss would not have
 * made it — and it is a cost, which is what a lot carries (Goods E1), never a second price.
 */
const SEED_STOCK_BASIS = 0.8;

const P = {
  cellsPerKey: paramId('seed.households.cellsPerKey'),
  membersPerKey: paramId('seed.households.membersPerKey'),
  openingYield: paramId('seed.openingYield'),
} as const;

/** Seed C4: what a good fetched in the market that has not opened yet, per unit of it. */
const openingPrice = (subUnit: string): ParamId => paramId(`seed.openingPrice.${subUnit}`);

/**
 * Law 2, Seed C4: the level a market opens at, which is a SHAPE and not a placeholder.
 *
 * A placeholder names the worklist item that deletes it, and no item ever will: a market that has
 * never traded has no price (XI-6) and its first buyer has nothing to bid against, so a world that
 * opens with stock in it opens with a level for that stock. The mechanism that replaces the number
 * is the market's own first session, and it already runs — in period one, of every run. What is
 * claimed is a level and it is claimed once; what it is worth from then on is cleared (C4.a).
 *
 * What would retire it is a measurement and not a mechanism (Part XII, worklist 16): run the same
 * world from different opening levels and see whether its path depends on where its markets opened.
 * If it does not, this is a RESOLUTION; if it does, the seed is claiming something load-bearing and
 * needs a way to open a market without stating one.
 */
function openingPrices(): ParamDecl[] {
  return SEED_STOCK.map((row) => ({
    id: openingPrice(row.subUnit),
    value: row.opensAt,
    unit: `PHX per unit of ${row.subUnit}`,
    kind: 'shape' as const,
    owner: 'model' as const,
    why: `Seed C4: ${row.why} It is the first clearing's input and not a permanent mark: the session in period one prints a price nobody stated and nothing reads this number again.`,
  }));
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
    ...openingPrices(),
    {
      id: P.openingYield,
      value: 0.02,
      unit: 'per annum',
      kind: 'shape',
      owner: 'model',
      // It was declared a placeholder dying at this item, on the reasoning that once households
      // held sovereign paper directly (Sovereign E2.f) every line would have a two-sided market and
      // the number would go. They do hold it now, and the number cannot go: a two-sided market from
      // period one says nothing about period zero, which is before any market has run. Seed C3
      // requires a maturity profile outstanding at period zero, and paper outstanding at period
      // zero has to be worth something. So it is a SHAPE — one level, flat across the profile, so
      // the seed claims no curve — retired by the same measurement as the opening prices above.
      why: 'Seed C4, C4.b: the one level the opening world is priced at, flat across the profile so the seed asserts no shape of its own. Every line opens at the price this yield gives it and carries the coupon that makes it par there, so the level is claimed once and the term follows from it rather than from a table. The auction and the secondary market re-price line by line from period one.',
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

    const openedGoods = new Set<string>();
    // Goods (Seed A3, C4, D1): every firm opens with stock of what it makes, with the inputs its
    // recipe draws, and — where a batch takes more than the period it is started in — with one on
    // the line already, so the first period is not the only one that produces nothing. What a
    // market has never traded has no price at all, and somebody must state the one it opens at;
    // that number is a placeholder and the market's own first session replaces it.
    for (const row of SEED_STOCK) {
      // A firm whose good this world does not make opens with nothing, because there is nothing
      // for it to hold: the seed endows what exists and never brings an instrument into being to
      // have something to endow (Seed A1).
      if (!ctx.instruments.has(goodId(row.subUnit, REGION))) continue;
      const firm = partyId(row.firm);
      const price = ctx.params.get(openingPrice(row.subUnit));
      const market = goodMarketId(row.subUnit, REGION);
      if (!openedGoods.has(row.subUnit)) {
        openedGoods.add(row.subUnit);
        ctx.prices.write({
          instrument: goodId(row.subUnit, REGION),
          market,
          period: ctx.period,
          price,
          ccy: PHX,
          provenance: { kind: 'opening' },
        });
      }
      // Seed C4: what it cost whoever holds it is the seed's, and it is below what the market
      // opens at — a firm holding stock it could only sell at a loss would never have made it.
      const basis = price * SEED_STOCK_BASIS;
      if (row.finished > 0) ctx.endowUnits(firm, goodId(row.subUnit, REGION), row.finished, basis);
      if (row.onTheLine > 0) {
        ctx.endowUnits(firm, wipId(row.subUnit, REGION), row.onTheLine, basis);
      }
    }
    for (const row of SEED_INPUTS) {
      if (!ctx.instruments.has(goodId(row.subUnit, REGION))) continue;
      const price = ctx.params.get(openingPrice(row.subUnit));
      ctx.endowUnits(partyId(row.firm), goodId(row.subUnit, REGION), row.qty, price * SEED_STOCK_BASIS);
    }

    // Households: cells per (region, cohort, bank) key, weights summing to the key's population
    // (Seed B1.a). They open with NOTHING — no deposit and no paper — because everything a
    // household has in this world is something it was paid or something it decided to buy, and
    // the seed has no business saying how rich anybody already is (Seed E: what the seed must not
    // decide). What they are unequal in is therefore an outcome from the first period on: who was
    // hired, at what wage, and what each of them made of it.
    const cells = positiveCount(ctx.params.get(P.cellsPerKey), 'cellsPerKey');
    const members = positiveCount(ctx.params.get(P.membersPerKey), 'membersPerKey');
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
      labour(),
      firms(),
      households(),
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
