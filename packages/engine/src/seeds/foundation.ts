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
import { addDays, civil } from '../calendar/civil.js';
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
import { bankLending } from '../mechanisms/bank-lending/index.js';
import {
  CAPITAL_KINDS,
  capitalProgramme,
  plantKindId,
  seedVintage,
} from '../mechanisms/capital-programme/index.js';
import { centralBankOmo } from '../mechanisms/central-bank-omo/index.js';
import { moneyMarket } from '../mechanisms/money-market/index.js';
import { estate } from '../mechanisms/estate/index.js';
import { creditEvents } from '../mechanisms/credit-events/index.js';
import { expectations } from '../mechanisms/expectations/index.js';
import { firms } from '../mechanisms/firms/index.js';
import { goodId, goodMarketId, goodTerms, goods, wipId } from '../mechanisms/goods/index.js';
import { dealers } from '../mechanisms/dealers/index.js';
import { equity } from '../mechanisms/equity/index.js';
import { funds } from '../mechanisms/funds/index.js';
import { households } from '../mechanisms/households/index.js';
import { labour } from '../mechanisms/labour/index.js';
import { sovereignAuction } from '../mechanisms/sovereign-auction/index.js';
import { sovereignCurve } from '../mechanisms/sovereign-curve/index.js';
import { treasury } from '../mechanisms/treasury/index.js';
import type { CellParty, NamedParty } from '../parties/party.js';
import { displayName } from '../registry/naming.js';
import { MONEY_GRID, SHARE_GRID } from '../registry/grid.js';
import { BANK, CENTRAL_BANK, FIRM, HOUSEHOLD, MONEY_KIND, SHARES, TREASURY } from '../registry/profiles.js';
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
interface SeedFirm {
  readonly firm: string;
  readonly name: string;
  /** Where it banks (Money A1): the wage and every invoice leave the account it holds there. */
  readonly bank: PartyId;
  /** Seed C1: the money it opens with, which is some bank's liability like anybody else's. */
  readonly cash: number;
  readonly subUnit: string;
  /** Units finished and ready to sell. */
  readonly finished: number;
  /** Units started and not yet off the line: only a good whose batch takes longer than a period. */
  readonly onTheLine: number;
  /** Seed D1: units of what its recipe draws, so its first batch is not waiting on a market. */
  readonly inputs: number;
  /**
   * Capital Programme A2, Seed D1: the plant it opens with, in units of the kind its line needs.
   * A world whose firms open with no plant produces nothing at all until somebody has built some,
   * which is not an opening condition, it is a different world. It is stated with headroom over
   * what the firm is currently making — a going concern is not running at its ceiling — and it is
   * spread over three vintages so that replacement comes round a third at a time rather than all at
   * once (Seed C3's reason, applied to plant instead of to the maturity profile).
   */
  readonly plant: number;
  readonly why: string;
}

/**
 * Seed B1, B1.a, B4: three firms in every line, no two the same size and no two the same cost.
 *
 * Every line has a firm at each bank (B3). A bank whose customers all sit on one side of the
 * payment chain is a bank with a structural reserve drain, and until the corridor exists (worklist
 * 11) nothing in this world could lend it the difference — so a seed that arranged one would be
 * opening with a flow it has no mechanism for, which is what D1 forbids.
 *
 * The LINE TOTALS are exactly what they were when each line had one firm in it. This item changes
 * the structure of the sector and not the scale of the world: choosing endowments to make some
 * employment number come out would be steering the model, and measuring it is Part XII's job and
 * not this one (Law 11). What each firm is worth is stated with a reason, as the one firm's was.
 */
const SEED_FIRMS: readonly SeedFirm[] = [
  // Grain — 200 of cash, 90 finished and 90 on the line, as the one farm held.
  { firm: 'firm.4', name: 'Broadacre Farm', bank: BANK_B, cash: 100, subUnit: 'grain', finished: 45, onTheLine: 45, inputs: 0, plant: 68,
    why: 'The largest farm in the region and the one that works the best ground.' },
  { firm: 'firm.1', name: 'Middlefield Farm', bank: BANK_A, cash: 65, subUnit: 'grain', finished: 30, onTheLine: 30, inputs: 0, plant: 45,
    why: 'An ordinary farm of ordinary size.' },
  { firm: 'firm.7', name: 'Hollow Farm', bank: BANK_B, cash: 35, subUnit: 'grain', finished: 15, onTheLine: 15, inputs: 0, plant: 23,
    why: 'The smallest, on the poorest ground, with the least cash to carry a bad season.' },
  // Flour — 150 of cash, 75 milled and 110 tonnes of grain to mill, as the one mill held.
  { firm: 'firm.5', name: 'Riverside Mill', bank: BANK_A, cash: 75, subUnit: 'flour', finished: 38, onTheLine: 0, inputs: 55, plant: 46,
    why: 'The big mill, and the one that has already bought most of the grain it will grind.' },
  { firm: 'firm.2', name: 'Town Mill', bank: BANK_B, cash: 50, subUnit: 'flour', finished: 25, onTheLine: 0, inputs: 37, plant: 30,
    why: 'An ordinary mill.' },
  { firm: 'firm.8', name: 'Old Mill', bank: BANK_A, cash: 25, subUnit: 'flour', finished: 12, onTheLine: 0, inputs: 18, plant: 14,
    why: 'The smallest and the oldest.' },
  // Bread — 250 of cash, 105 baked and 75 tonnes of flour, as the one bakery held.
  { firm: 'firm.6', name: 'City Bakery', bank: BANK_A, cash: 125, subUnit: 'bread', finished: 52, onTheLine: 0, inputs: 38, plant: 39,
    why: 'A plant bakery: the biggest oven and the biggest week of bread in the shop.' },
  { firm: 'firm.3', name: 'High Street Bakery', bank: BANK_B, cash: 80, subUnit: 'bread', finished: 35, onTheLine: 0, inputs: 25, plant: 26,
    why: 'An ordinary bakery.' },
  { firm: 'firm.9', name: 'Corner Bakery', bank: BANK_A, cash: 45, subUnit: 'bread', finished: 18, onTheLine: 0, inputs: 12, plant: 14,
    why: 'The smallest, and the one holding the least flour against a week it cannot predict.' },
  // Capital Programme C1, E2: the line that BUILDS the capital. Its output is somebody else's
  // plant, its revenue is somebody else's investment, and the people it employs are employed by
  // the decision to expand. It needs no plant of its own: a workshop is people and a bench, and
  // saying so is a statement about this world's technology rather than a missing constraint.
  { firm: 'firm.10', name: 'North Engineering', bank: BANK_B, cash: 60, subUnit: 'machine', finished: 4, onTheLine: 3, inputs: 0, plant: 0,
    why: 'The best-equipped workshop in the region and the one with machines already on the bench.' },
  { firm: 'firm.11', name: 'Town Works', bank: BANK_A, cash: 40, subUnit: 'machine', finished: 3, onTheLine: 2, inputs: 0, plant: 0,
    why: 'An ordinary workshop.' },
  { firm: 'firm.12', name: 'Lane Workshop', bank: BANK_B, cash: 25, subUnit: 'machine', finished: 2, onTheLine: 1, inputs: 0, plant: 0,
    why: 'The smallest, and the one that will be priced out of engineering labour first.' },
];

/** Seed C4: the level each market opens at, which is the good's and not any one firm's. */
const SEED_MARKETS: readonly { readonly subUnit: string; readonly opensAt: number; readonly why: string }[] = [
  { subUnit: 'grain', opensAt: 0.4,
    why: 'Grain takes two periods to grow, so a world that opens with an empty field produces nothing for two of them and the mill has nothing to buy. One crop in the barn and one in the ground is what a going concern looks like.' },
  { subUnit: 'flour', opensAt: 0.6,
    why: 'Milling is inside the period, so there is nothing on the line; what a mill opens with is what it has already milled.' },
  { subUnit: 'bread', opensAt: 1.2,
    why: 'A week of bread in the shop. It goes stale at a quarter a period, so what is not sold is a real loss from the first period on.' },
  { subUnit: 'machine', opensAt: 2.5,
    why: 'A machine is sixty hours of engineering, and this is what sixty hours of it is worth at the level the rest of this world opens at — so a workshop bids for an hour somewhere between what a mill will pay and what a farm will, and the capital-goods line is neither the best nor the worst employer on the first morning.' },
];

/**
 * Capital Programme A6, Seed C3: the vintages the world opens with, as ages in periods. Three of
 * them, evenly spread across a machine's life, so a third of every firm's plant comes up for
 * replacement at a time and the world has a reason to invest before anything has grown.
 */
const SEED_PLANT_AGES: readonly number[] = [26, 78, 130];

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
  return SEED_MARKETS.map((row) => ({
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
    // Seed B1: three to a line, each a named party with its own bank (B2, B3).
    for (const f of SEED_FIRMS) ctx.parties.add(named(partyId(f.firm), FIRM, f.name, f.bank));

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
    for (const f of SEED_FIRMS) ctx.endowMoney(partyId(f.firm), PHX, f.cash);
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
    for (const row of SEED_MARKETS) {
      if (!ctx.instruments.has(goodId(row.subUnit, REGION))) continue;
      openedGoods.add(row.subUnit);
      ctx.prices.write({
        instrument: goodId(row.subUnit, REGION),
        market: goodMarketId(row.subUnit, REGION),
        period: ctx.period,
        price: ctx.params.get(openingPrice(row.subUnit)),
        ccy: PHX,
        provenance: { kind: 'opening' },
      });
    }
    for (const row of SEED_FIRMS) {
      // A firm whose good this world does not make opens with nothing, because there is nothing
      // for it to hold: the seed endows what exists and never brings an instrument into being to
      // have something to endow (Seed A1).
      if (!ctx.instruments.has(goodId(row.subUnit, REGION))) continue;
      const firm = partyId(row.firm);
      const price = ctx.params.get(openingPrice(row.subUnit));
      // Seed C4: what it cost whoever holds it is the seed's, and it is below what the market
      // opens at — a firm holding stock it could only sell at a loss would never have made it.
      const basis = price * SEED_STOCK_BASIS;
      if (row.finished > 0) ctx.endowUnits(firm, goodId(row.subUnit, REGION), row.finished, basis);
      if (row.onTheLine > 0) {
        ctx.endowUnits(firm, wipId(row.subUnit, REGION), row.onTheLine, basis);
      }
      // Seed D1: what its recipe draws, so its first batch is not waiting on a market session.
      // Law 19: WHAT it draws is read from the good's own terms, never listed a second time here.
      for (const input of goodTerms(ctx.instruments.get(goodId(row.subUnit, REGION))).recipe.inputs) {
        if (row.inputs <= 0 || !ctx.instruments.has(goodId(input.subUnit, REGION))) continue;
        const paid = ctx.params.get(openingPrice(input.subUnit)) * SEED_STOCK_BASIS;
        ctx.endowUnits(firm, goodId(input.subUnit, REGION), row.inputs, paid);
      }
      // Capital Programme A2, A6, Seed D1: the plant its line runs on, spread over three vintages
      // of different ages, each carried at what is left of what a new one costs. WHICH kind of
      // plant is read from the good's own recipe (Law 19), and how many units it needs to make what
      // it makes is that recipe's number too — the seed states only how much headroom it opens with.
      for (const need of goodTerms(ctx.instruments.get(goodId(row.subUnit, REGION))).recipe.plant) {
        const kind = CAPITAL_KINDS.find((k) => k.id === need.capitalKind);
        // A world assembled without the capital programme has no plant to endow, exactly as a
        // world that does not make a good has no stock of it to endow (Seed A1).
        if (kind === undefined || row.plant <= 0) continue;
        if (!ctx.registry.instrumentKinds.has(plantKindId(kind.id))) continue;
        const newPrice = ctx.params.get(openingPrice(kind.madeFrom));
        const life = ctx.params.get(paramId(`plant.usefulLife.${kind.id}`));
        for (const age of SEED_PLANT_AGES) {
          const serviceDate = addDays(ctx.calendar.epoch, -age * ctx.calendar.periodDays);
          const id = seedVintage(ctx, kind, REGION, serviceDate);
          // A3, A6: what a vintage that has already run for `age` periods is carried at — the
          // straight line it has been on since it went into service, and nothing else.
          ctx.endowUnits(ctx.parties.get(firm).id, id, row.plant / SEED_PLANT_AGES.length, (newPrice * (life - age)) / life);
        }
      }
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
      units: [
        // Money A2, Law 8: PHX has a smallest piece, like any real money. A millionth of a unit
        // at this world's scale — a household member holds a tenth of a PHX — so the grid is real
        // arithmetic rather than a rounding, and fine enough that no decision turns on it.
        { id: currencyUnit(PHX), name: 'PHX', tickExponent: MONEY_GRID },
        // Equity A2, Fund Shares A2: a SHARE COUNT, which more than one system counts in and no one
        // of them owns. Its smallest piece is far below one, and deliberately: a share here costs a
        // few PHX and a household member holds a ten-thousandth of one, so whole shares would put
        // equity out of a household's reach altogether and a coarse grid would round its holding
        // away. What the tick buys is that a pro-rata fill, a
        // subscription and a split all land on a grid whose last piece has a named holder
        // (Clearing C3) — which is the residual that dividing for ever was avoiding by never
        // arriving at one.
        { id: SHARES, name: 'shares', tickExponent: SHARE_GRID },
      ],
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
      {
        id: KERNEL_PARAMS.tickShift,
        value: 0,
        unit: 'halvings',
        kind: 'resolution',
        owner: 'model',
        why: 'Law 8, Law 2: how many halvings finer or coarser than declared every unit\'s smallest piece is. Each unit states its own grid; this moves them all together, which is what makes the grid a RESOLUTION that can be TESTED — run the same world one shift finer and one coarser and the path must not move by more than the rounding the grid itself imposes.',
      },
    ],
    modules: [
      // The order matters at one anchor: three phases sit before the revaluation, and they must run
      // in this order — a drawing becomes a loan row, then anything that cannot pay dies, then the
      // people it employed are released. Assembly keeps declaration order for modules that do not
      // require each other, and that is what puts them in it.
      expectations,
      creditEvents,
      bankLending,
      estate,
      goods(),
      // Capital Programme: the kind of thing plant is, and the schedule it wears out on. Before the
      // firms, because a firm decides what to make against the plant it holds (A2) and what to
      // invest against what a machine costs (B1) — and a kind has to be registered to be held.
      capitalProgramme(),
      labour(),
      firms(),
      households(),
      // Equity and the desks before the funds: this world's exchange-traded fund holds the listed
      // firms and is launched by the desks that make its market, and both have to exist before a
      // basket can be put in (the funds module reads that off its own data, in `needs`).
      equity(),
      dealers(),
      funds(),
      sovereignInstruments,
      sovereignCurve(TREASURY_NORTH, PHX),
      sovereignAuction,
      treasury,
      centralBankOmo,
      // The money market after the treasury and the curve: a bank funds itself against the paper
      // those two put into the world, and it prices a name off what the lending module published
      // about it. Both reach it as public events and prints, never as imports (Law 15).
      moneyMarket,
      foundationSeed,
    ],
  };
}

/** Build the foundation world. Reproducible from the seed value (Seed A5). */
export function foundationWorld(seed: string): World {
  return assemble(foundationSpec(seed));
}

export { PAR };
