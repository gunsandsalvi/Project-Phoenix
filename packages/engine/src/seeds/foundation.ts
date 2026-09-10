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
  type InstrumentId,
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
import { forbid } from '../core/assert.js';
import { weightOf } from '../parties/party.js';
import { add, div, mul, positiveCount, sub, sum, zeroIfNone } from '../core/num.js';
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
import { BANKS, type BankDecl } from '../mechanisms/banks/data.js';
import { banks } from '../mechanisms/banks/index.js';
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
import { equity } from '../mechanisms/equity/index.js';
import { funds } from '../mechanisms/funds/index.js';
import { households } from '../mechanisms/households/index.js';
import { labour } from '../mechanisms/labour/index.js';
import { sovereignCurve } from '../mechanisms/sovereign-curve/index.js';
import { treasury } from '../mechanisms/treasury/index.js';
import type { CellParty, NamedParty } from '../parties/party.js';
import { splitOnTick } from '../core/tick.js';
import { displayName } from '../registry/naming.js';
import { MONEY_PIECES, SHARE_PIECES } from '../registry/grid.js';
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
/**
 * Seed B1, B4: WHICH BANKS THIS WORLD HAS is the table the banks module declares, and the seed is
 * built from the same one — there is no count beside it and no second list (Law 4). `BANK_A` and
 * `BANK_B` are exported because two other modules' data name them (a fund banks somewhere, a share
 * line opens on somebody's book) and because every world this seed builds has at least two.
 */
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
  /**
   * Seed B4: what the BANKING SYSTEM holds of this line, split across the banks that exist in
   * proportion to each one's stated size. It used to be one number per bank per line — eighteen of
   * them, whose spread came to a few per cent — and what those eighteen were saying is here in one
   * number per line and one size per bank. Which lines each bank ends up concentrated in is then an
   * outcome of what it buys and sells from period one, rather than something the seed decided.
   */
  readonly banks: number;
  /** Per member of every household cell, before dispersion (Seed B4). */
  readonly perMember: number;
}

const SEED_LINES: readonly SeedLine[] = [
  { id: 'gov.north.bill.2026-06-15', paper: 'bill', maturity: { y: 2026, m: 6, d: 15 }, banks: 500000, perMember: 0 },
  { id: 'gov.north.bill.2026-09-15', paper: 'bill', maturity: { y: 2026, m: 9, d: 15 }, banks: 500000, perMember: 0 },
  { id: 'gov.north.bill.2027-03-15', paper: 'bill', maturity: { y: 2027, m: 3, d: 15 }, banks: 500000, perMember: 20 },
  { id: 'gov.north.2028-03-15', paper: 'bond', maturity: { y: 2028, m: 3, d: 15 }, banks: 300000, perMember: 40 },
  { id: 'gov.north.2031-03-15', paper: 'bond', maturity: { y: 2031, m: 3, d: 15 }, banks: 300000, perMember: 60 },
  { id: 'gov.north.2036-03-15', paper: 'bond', maturity: { y: 2036, m: 3, d: 15 }, banks: 300000, perMember: 80 },
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
  { firm: 'firm.4', name: 'Broadacre Farm', cash: 100000, subUnit: 'grain', finished: 45, onTheLine: 45, inputs: 0, plant: 68,
    why: 'The largest farm in the region and the one that works the best ground.' },
  { firm: 'firm.1', name: 'Middlefield Farm', cash: 65000, subUnit: 'grain', finished: 30, onTheLine: 30, inputs: 0, plant: 45,
    why: 'An ordinary farm of ordinary size.' },
  { firm: 'firm.7', name: 'Hollow Farm', cash: 35000, subUnit: 'grain', finished: 15, onTheLine: 15, inputs: 0, plant: 23,
    why: 'The smallest, on the poorest ground, with the least cash to carry a bad season.' },
  // Flour — 150 of cash, 75 milled and 110 tonnes of grain to mill, as the one mill held.
  { firm: 'firm.5', name: 'Riverside Mill', cash: 75000, subUnit: 'flour', finished: 38, onTheLine: 0, inputs: 55, plant: 46,
    why: 'The big mill, and the one that has already bought most of the grain it will grind.' },
  { firm: 'firm.2', name: 'Town Mill', cash: 50000, subUnit: 'flour', finished: 25, onTheLine: 0, inputs: 37, plant: 30,
    why: 'An ordinary mill.' },
  { firm: 'firm.8', name: 'Old Mill', cash: 25000, subUnit: 'flour', finished: 12, onTheLine: 0, inputs: 18, plant: 14,
    why: 'The smallest and the oldest.' },
  // Bread — 250 of cash, 105 baked and 75 tonnes of flour, as the one bakery held.
  { firm: 'firm.6', name: 'City Bakery', cash: 125000, subUnit: 'bread', finished: 52, onTheLine: 0, inputs: 38, plant: 39,
    why: 'A plant bakery: the biggest oven and the biggest week of bread in the shop.' },
  { firm: 'firm.3', name: 'High Street Bakery', cash: 80000, subUnit: 'bread', finished: 35, onTheLine: 0, inputs: 25, plant: 26,
    why: 'An ordinary bakery.' },
  { firm: 'firm.9', name: 'Corner Bakery', cash: 45000, subUnit: 'bread', finished: 18, onTheLine: 0, inputs: 12, plant: 14,
    why: 'The smallest, and the one holding the least flour against a week it cannot predict.' },
  // Capital Programme C1, E2: the line that BUILDS the capital. Its output is somebody else's
  // plant, its revenue is somebody else's investment, and the people it employs are employed by
  // the decision to expand. It needs no plant of its own: a workshop is people and a bench, and
  // saying so is a statement about this world's technology rather than a missing constraint.
  { firm: 'firm.10', name: 'North Engineering', cash: 60000, subUnit: 'machine', finished: 4, onTheLine: 3, inputs: 0, plant: 0,
    why: 'The best-equipped workshop in the region and the one with machines already on the bench.' },
  { firm: 'firm.11', name: 'Town Works', cash: 40000, subUnit: 'machine', finished: 3, onTheLine: 2, inputs: 0, plant: 0,
    why: 'An ordinary workshop.' },
  { firm: 'firm.12', name: 'Lane Workshop', cash: 25000, subUnit: 'machine', finished: 2, onTheLine: 1, inputs: 0, plant: 0,
    why: 'The smallest, and the one that will be priced out of engineering labour first.' },
];

/** Seed C4: the level each market opens at, which is the good's and not any one firm's. */
const SEED_MARKETS: readonly { readonly subUnit: string; readonly opensAt: number; readonly why: string }[] = [
  { subUnit: 'grain', opensAt: 400,
    why: 'Grain takes two periods to grow, so a world that opens with an empty field produces nothing for two of them and the mill has nothing to buy. One crop in the barn and one in the ground is what a going concern looks like.' },
  { subUnit: 'flour', opensAt: 600,
    why: 'Milling is inside the period, so there is nothing on the line; what a mill opens with is what it has already milled.' },
  { subUnit: 'bread', opensAt: 1200,
    why: 'A week of bread in the shop. It goes stale at a quarter a period, so what is not sold is a real loss from the first period on.' },
  { subUnit: 'machine', opensAt: 2500,
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
  membersPerCohort: paramId('seed.households.membersPerCohort'),
  openingYield: paramId('seed.openingYield'),
  cbOpeningShare: paramId('seed.centralBank.openingHoldingShare'),
  treasuryBufferShare: paramId('seed.treasury.bufferShare'),
  // Declared by another module and read here by id, because a seed may not import a mechanism
  // (phoenix/no-cross-module-import).
  leverageRatio: paramId('regulation.leverageRatio'),
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

/**
 * The seed, built from the same bank table the banks module is (Law 4). A world with two banks or
 * with four is this world with a different table: the paper the banking system holds is split by
 * each bank's stated size, the firms are spread across whatever banks exist, and the population is
 * spread across them too — so nothing here is restated when the count changes, which is what lets
 * the count be MEASURED (XI-15, `test/resolution/banks.test.ts`).
 */
export function foundationSeedFor(bankRows: readonly BankDecl[] = BANKS): SystemModule {
  return {
  id: 'seed.foundation',
  spec: 'Seed',
  requires: ['sovereign-instruments'],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: P.cbOpeningShare,
      value: 0.2,
      unit: 'ratio of a line outstanding',
      kind: 'shape',
      owner: 'model',
      why: 'Seed E2, Central Bank C1: what the central bank opens holding of every sovereign line, and therefore how big its balance sheet is — every reserve it has issued was issued to buy this paper (Central Bank A2). It is stated BELOW the OMO\'s own target share so that the central bank opens short of what its policy wants and its first open-market session has something to do: a seed that opened it at its target would be seeding the outcome of the mechanism it is about to run (Seed E1). The mechanism that replaces this number is the open-market session itself, which runs from period one.',
    },
    {
      id: P.treasuryBufferShare,
      value: 0.4,
      unit: "ratio of the central bank's balance sheet",
      kind: 'shape',
      owner: 'model',
      why: "Treasury D4.b, Seed E2: how much of the central bank's money the treasury opens holding, with the banks holding the rest as reserves. It is a share and not an amount because the amount is not free: every unit of central-bank money was issued to buy the paper above, so what is stated is how it is divided and never how much of it there is. The mechanism that replaces it is the treasury's own funding programme, which decides its balance from period one.",
    },
    {
      id: P.cellsPerKey,
      value: 2,
      unit: 'count',
      kind: 'resolution',
      owner: 'model',
      why: 'XI-15: how many cells stand for each (region, cohort, bank) population; change it and the answer must not move.',
    },
    {
      id: P.membersPerCohort,
      value: 3000,
      unit: 'count',
      kind: 'resolution',
      owner: 'model',
      why: 'The population each cohort stands for, spread across the banks that exist and cut into `cellsPerKey` cells at each of them. It is per COHORT and not per (cohort, bank) key, because how many banks a world has is a resolution of its own (`banks.count`) and a population that grew with it would be the count of banks answering a question about how many people there are.',
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
    // Seed B1, B4: as many banks as `banks.count`, each with the disposition and the size its own
    // row states. THREE by default, because with two every depositor that answers a rate is the
    // whole of one side of the deposit market, every interbank session is one name facing one name,
    // and a bank in trouble has exactly one place to go — and because the count being load-bearing
    // in a way no mechanism states is exactly what makes it a RESOLUTION to be measured (XI-15).
    const banks = bankRows.map((b) => ({ id: partyId(b.bank), size: b.size }));
    banks.forEach((b, n) => {
      ctx.parties.add(named(b.id, BANK, `Bank ${String.fromCharCode(65 + n)}`, CB));
    });
    // Seed B1, B3: three firms to a line, spread across the banks that exist so that no bank's
    // customers all sit on one side of the payment chain — a bank whose do has a structural reserve
    // drain, which is a flow this world would be opening with rather than producing.
    SEED_FIRMS.forEach((f, n) => {
      const bank = banks[n % banks.length];
      if (bank === undefined) return;
      ctx.parties.add(named(partyId(f.firm), FIRM, f.name, bank.id));
    });

    // Money instruments: one per issuer (Money A1, D2).
    for (const issuer of [CB, ...banks.map((b) => b.id)]) {
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
        price: priced(ctx, id, price),
        ccy: PHX,
        provenance: { kind: 'opening' },
      });
    }

    // Households: cells per (region, cohort, bank) key, weights summing to the key's population
    // (Seed B1.a). They are created BEFORE the endowments because they are the parties the
    // endowments have to add up against: a bank's funding is its depositors' money, and until the
    // depositors exist there is nobody for it to be owed to.
    //
    // THEY OPEN WITH SOMETHING, and the comment that used to stand here said the opposite. It
    // justified a household holding nothing by citing Seed E — which says "no OUTCOME is seeded"
    // (E1) and "the seed sets reasons and ENDOWMENTS" (E2). Opening wealth is an endowment, in
    // exactly the sense E2 names, and every other party in this seed has one stated: the firms get
    // cash and stock, the banks reserves and paper, the treasury a buffer. Households alone held
    // nothing, and what that produced is measurable: no saver funds a bank, so the banks were 63%
    // to 84% their own equity; equity at the return it asks is the dearest money a bank has, so
    // every bank required 5.7% to 8.9% of every issuer; and paper near par yields 2%, so no bid was
    // ever posted. 187 of 312 sovereign sessions cleared with NO DEMAND AT ALL. The seed was not
    // declining to say how rich anybody is — it was saying everybody is poor, which is a statement
    // about the answer with no mechanism behind it.
    //
    // What they are unequal in is still an outcome from the first period on: who was hired, at what
    // wage, and what each of them made of it. What is endowed is the stock they start from.
    // XI-15, Law 2: THE POPULATION IS A PROPERTY OF THE WORLD AND NOT OF ITS BANKS. What is stated
    // is how many people a cohort stands for; how many cells they are cut into is one resolution
    // (`cellsPerKey`) and how many banks they are spread over is another (`banks.count`), and
    // neither may change how many people there are. Stating it per (cohort, bank) key meant a world
    // with a fourth bank had a third more people in it, which is the count of banks answering a
    // question about the population.
    const cells = positiveCount(ctx.params.get(P.cellsPerKey), 'cellsPerKey');
    const members = positiveCount(ctx.params.get(P.membersPerCohort), 'membersPerCohort');
    for (const cohort of ctx.registry.cohorts) {
      const weights = splitPopulation(members, banks.length * cells);
      weights.forEach((weight, at) => {
        const bank = banks[Math.floor(at / cells)];
        const n = at % cells;
        if (bank === undefined) return;
        const cell: CellParty = {
          id: partyId(`hh.${cohort.id}.${bank.id}.${n}`),
          kind: HOUSEHOLD,
          region: REGION,
          name: `Households ${cohort.name} at ${bank.id} #${n}`,
          bank: bank.id,
          representation: 'cell',
          status: { alive: true },
          weight,
          key: { region: REGION, cohort: cohortId(cohort.id), bank: bank.id },
        };
        ctx.parties.add(cell);
      });
    }

    // ------------------------------------------------------------------------------------------
    // THE OPENING BALANCE SHEET (Seed A3, C1, C5, E2; Central Bank A2, C1; Banks Capital B1.b)
    //
    // What is STATED here is what somebody chose: how much paper each bank holds, how much each
    // household member holds, and how much of its liquid buffer a bank keeps as reserves rather
    // than paper. Everything else is DERIVED from a rule that already governs the party's own
    // behaviour, so nothing opens somewhere its own mechanism would immediately move it away from,
    // and no number here was chosen by looking at the answer (the 11.3 record is what that costs).
    // ------------------------------------------------------------------------------------------

    // The paper each bank holds is stated; what a household member holds is stated per member.
    const householdMembers = ctx.parties
      .ofKind(HOUSEHOLD)
      .reduce((t, c) => t + weightOf(c), 0);
    const bankPaper = new Map<PartyId, number>(banks.map((b) => [b.id, 0]));
    let centralBankAssets = 0;

    for (const line of SEED_LINES) {
      const id = instrumentId(line.id);
      const price = openingOf(opening, line.id);
      const par = priced(ctx, id, price);
      // Law 2, Law 8: what the banking system holds of the line, split by each bank's stated size
      // into WHOLE UNITS that sum to exactly what was stated — the odd unit goes to the largest
      // remainder and has a named holder, rather than being lost to a division that does not come
      // out (core/tick.ts).
      const perBank = splitOnTick(line.banks, banks.map((b) => b.size));
      banks.forEach((b, at) => {
        const units = zeroIfNone(perBank[at]);
        if (units <= 0) return;
        ctx.endowUnits(b.id, id, held(ctx, id, units), par);
        bankPaper.set(b.id, add(zeroIfNone(bankPaper.get(b.id)), units * price, 'bank paper'));
      });
      // Seed E2, XI-15: every member of every cell holds the same stated amount, and the cell
      // carries it with its weight. This column has been in this table since the seed was written
      // and nothing has ever read it — which is why the sovereign's book had one side.
      if (line.perMember > 0) {
        for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
          ctx.endowUnits(cell.id, id, held(ctx, id, line.perMember), par);
        }
      }
      // Central Bank C1, Seed E2: WHAT THE CENTRAL BANK OPENS HOLDING, as a share of each line. It
      // is the seed's own endowment and it is deliberately NOT the OMO's target: opening the
      // central bank at the holding its own policy wants would be seeding an outcome (Seed E1) and
      // importing an equilibrium (Law 2), and it would mean the first open-market session had
      // nothing to do — the mechanism would never be seen to run at all.
      //
      // Its holding is therefore not a number in this table either: it is that share of what the
      // line comes to outstanding once everybody else holds theirs, `others × share / (1 − share)`.
      const others = line.banks + line.perMember * householdMembers;
      const share = ctx.params.get(P.cbOpeningShare);
      const cbUnits = div(mul(others, share, 'the share it targets'), 1 - share, 'its holding');
      if (cbUnits > 0) {
        ctx.endowUnits(CB, id, held(ctx, id, cbUnits), par);
        centralBankAssets = add(centralBankAssets, cbUnits * price, 'central bank assets');
      }
    }

    // Money A1, Central Bank A2: NO CENTRAL-BANK MONEY EXISTS THAT ITS ISSUER BOUGHT NOTHING WITH.
    // Its money is its liability and the paper above is the asset it bought with it, so THE SIZE OF
    // ITS BALANCE SHEET IS ALREADY DECIDED: what is left to say is who holds that money. The seed
    // used to hand out 160,000,000 of reserves against 53,531,106 of assets and leave the
    // difference — 106,468,894 — as a hole the central bank then paid the floor rate on for ever;
    // then it stated three reserve figures that had to come to less than assets a share decided,
    // and a world with fewer households did not open. Both were the same defect: a stated number on
    // the side of an identity that is not free.
    //
    // Treasury D4.b: it opens with a buffer, because the alternative to one is dependence on every
    // single auction clearing — and the buffer is CENTRAL-BANK MONEY, so what is stated about it is
    // ITS SHARE of that balance sheet. The banks hold the rest, in proportion to the paper each of
    // them holds: a bigger bank settles bigger payments and carries more against them. Nothing here
    // can fail to add up, because the last holder gets the residue of a subtraction (Law 2).
    const buffer = mul(centralBankAssets, ctx.params.get(P.treasuryBufferShare), "the treasury's buffer");
    ctx.endowMoney(TREASURY_NORTH, PHX, cash(ctx, buffer));
    const reserves = sub(centralBankAssets, buffer, 'what the banks hold in reserve');
    const allPaper = sum([...bankPaper.values()]).value;
    const bankReserves = new Map<PartyId, number>(
      [...bankPaper].map(([bank, paper]) => [
        bank,
        div(mul(reserves, paper, "this bank's share of the paper"), allPaper, 'its reserves'),
      ]),
    );
    for (const [bank, reserve] of bankReserves) ctx.endowMoney(bank, PHX, cash(ctx, reserve));

    for (const f of SEED_FIRMS) ctx.endowMoney(partyId(f.firm), PHX, cash(ctx, f.cash));

    // Banks Capital B1.b, A3: a bank opens where ITS OWN CAPITAL RULE puts it, so it neither has to
    // shrink on the first morning nor opens with headroom nobody gave it. At the opening its assets
    // are reserves and this issuer's paper, both of which the risk weights put at zero
    // (`regulation.riskWeight.sovereign`), so the weighted rule asks for nothing and the UNWEIGHTED
    // one binds — which is the whole reason a leverage ratio exists. Its funding is therefore what
    // is left, and its depositors are the parties that hold it: the firms above, and the households
    // for the rest. `bank-capital.test.ts` asserts the opening capital satisfies BOTH rules as the
    // banks module itself computes them, so this derivation cannot drift from that one (Law 4).
    // WHAT STANDS BEHIND A BANK IS NOT DERIVED HERE, and that is the point: this seed cannot see
    // the shares the equity module hands a bank or the fund it launches, because both of them seed
    // AFTER it (they need the parties it creates). A bank funded against the assets its funder can
    // see opens at whatever share of its own capital the assets it CANNOT see happen to come to —
    // `bank.c` at exactly its 3.0% leverage rule and `bank.a` at 29.7%. It is `seed.funding`, which
    // runs when every module has handed out what it hands out (docs/BUGS.md 12-1).

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
        price: priced(ctx, goodId(row.subUnit, REGION), ctx.params.get(openingPrice(row.subUnit))),
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
      const good = goodId(row.subUnit, REGION);
      const basis = priced(ctx, good, price * SEED_STOCK_BASIS);
      if (row.finished > 0) ctx.endowUnits(firm, good, held(ctx, good, row.finished), basis);
      if (row.onTheLine > 0) {
        const wip = wipId(row.subUnit, REGION);
        ctx.endowUnits(firm, wip, held(ctx, wip, row.onTheLine), priced(ctx, wip, price * SEED_STOCK_BASIS));
      }
      // Seed D1: what its recipe draws, so its first batch is not waiting on a market session.
      // Law 19: WHAT it draws is read from the good's own terms, never listed a second time here.
      for (const input of goodTerms(ctx.instruments.get(goodId(row.subUnit, REGION))).recipe.inputs) {
        if (row.inputs <= 0 || !ctx.instruments.has(goodId(input.subUnit, REGION))) continue;
        const line = goodId(input.subUnit, REGION);
        const paid = ctx.params.get(openingPrice(input.subUnit)) * SEED_STOCK_BASIS;
        ctx.endowUnits(firm, line, held(ctx, line, row.inputs), priced(ctx, line, paid));
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
        // Law 8: whole machines, and the odd one has a named vintage rather than being lost to a
        // division that does not come out (core/tick.ts).
        const perVintage = splitOnTick(row.plant, SEED_PLANT_AGES.map(() => 1));
        SEED_PLANT_AGES.forEach((age, at) => {
          const serviceDate = addDays(ctx.calendar.epoch, -age * ctx.calendar.periodDays);
          const id = seedVintage(ctx, kind, REGION, serviceDate);
          // A3, A6: what a vintage that has already run for `age` periods is carried at — the
          // straight line it has been on since it went into service, and nothing else.
          ctx.endowUnits(
            ctx.parties.get(firm).id,
            id,
            held(ctx, id, zeroIfNone(perVintage[at])),
            priced(ctx, id, (newPrice * (life - age)) / life),
          );
        });
      }
    }

  },
  };
}

/**
 * Seed A4, C1, C5; Banks Capital B1.b: WHAT STANDS BEHIND A BANK AT THE OPENING, derived when every
 * module has handed out what it hands out.
 *
 * It is a second seed module and not a block of the first, because the question needs the whole
 * opening state and the first one cannot see it. `equity` gives a bank the float it makes a market
 * in and `funds` gives it a launch of an exchange-traded fund; both need the parties the foundation
 * creates, so both seed after it, so the foundation funds a bank against the assets IT endowed and
 * against nothing else. What that produced is measurable and was measured: `bank.c` opened at
 * exactly 3.0% equity — its own leverage rule, correctly applied to what the foundation could see —
 * while `bank.a` opened at 29.7% and `bank.b` at 23.8%, the difference being shares nobody had
 * funded (docs/BUGS.md 12-1).
 *
 * ONE DECLARED NUMBER, and everything derived from it (Law 2). A bank opens where its OWN capital
 * rule puts it: at the opening its assets are reserves, this issuer's paper and the float, and its
 * funding is what is left over once its own leverage minimum stands in front of them. Its
 * depositors are the parties that hold that funding — the firms, which the foundation has already
 * given their cash, and the households for the rest. Nothing here is chosen by reading an answer.
 *
 * Law 19: the assets are READ OFF THE REGISTER rather than tallied as they are handed out. A tally
 * is a second copy of the register that goes wrong the moment somebody endows something without
 * adding to it — which is exactly how this defect arrived.
 */
export function foundationFundingFor(bankRows: readonly BankDecl[] = BANKS): SystemModule {
  return {
    id: 'seed.funding',
    spec: 'Seed',
    // Only the seed whose parties it funds. It must run after every module that hands a bank an
    // asset, and it does that by being declared LAST rather than by naming them: naming them would
    // mean a world assembled without `equity` or `funds` could not include this module at all, and
    // a world that opens its banks and then does not fund them is not a smaller world — it is a
    // world where nobody has a deposit (assembly keeps declaration order for modules that do not
    // require each other).
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [],
    families: [],
    seed(ctx: SeedContext): void {
      const minimum = ctx.params.get(P.leverageRatio);
      for (const row of bankRows) {
        const bank = partyId(row.bank);
        if (!ctx.parties.has(bank)) continue;
        // Banks Capital B1.b, B2: THE LINE A BANK RUNS TO IS THE REQUIREMENT PLUS ITS OWN BUFFER,
        // and that is what "where its own capital rule puts it" means. Opened at the bare minimum
        // it is in breach of its own caution on the first morning, has no headroom to lend into,
        // and starts the run shrinking — which is not an opening condition, it is a bank already
        // in trouble. The buffer is its own (B2), so no two banks open at the same share and none
        // of them opens at a number this seed chose.
        const leverage = add(minimum, row.capitalBuffer, 'the line this bank runs to');
        const own = moneyInstrumentId(bank, PHX);
        let assets = 0;
        for (const h of ctx.register.holdingsOf(bank)) {
          if (h.instrument === own) continue;
          assets = add(assets, ctx.valuation.valueOfLots(h.instrument, h.lots, ctx.period), "the bank's opening assets");
        }
        const funding = mul(assets, 1 - leverage, 'what its own leverage rule leaves it to fund');
        // Law 19, Law 15: what is ALREADY deposited at it, read off the register — not "what its
        // firms hold", which would be this seed asking what kind a depositor is (nothing branches
        // on a kind: the question is what the account holds, and the answer is the same whoever
        // opened it). Everything endowed at this bank before now counts, and the households take
        // what is left of what has to be funded.
        let already = 0;
        for (const holder of ctx.register.holdersOf(own)) {
          if (holder === bank) continue;
          const held = ctx.register.quantity(holder, own);
          already = add(already, mul(held, weightOf(ctx.parties.get(holder)), 'in total'), 'deposits');
        }
        const fromHouseholds = sub(funding, already, 'what the households must hold');
        const cells = ctx.parties.ofKind(HOUSEHOLD).filter((c) => c.bank === bank);
        const members = cells.reduce((t, c) => t + weightOf(c), 0);
        forbid(
          fromHouseholds > 0 && members > 0,
          'Banks Capital B1.b',
          `${bank} opens with ${assets} of assets and ${already} already deposited at it, which leaves nothing for its households to hold`,
          { bank, assets, already, funding },
        );
        // XI-15: per member, and the cell carries it with its weight.
        const perMember = div(fromHouseholds, members, 'the deposit one member opens with');
        for (const cell of cells) ctx.endowMoney(cell.id, PHX, perMember);
      }
    },
  };
}

/**
 * Law 8: THE SEED SPEAKS IN NAMED UNITS AND THE STATE HOLDS PIECES, and this is the one boundary
 * between them. A number here is what a person would say — 100,000 PHX, 45 tonnes, 400 PHX the
 * tonne — and what goes into the register is the count of indivisible pieces that comes to: cents,
 * grams, whole machines. Nothing downstream converts anything, because everything downstream is
 * already a count.
 */
function cash(ctx: SeedContext, phx: number): number {
  return ctx.registry.pieces(currencyUnit(PHX), phx);
}

/** The same for units of an instrument, in whatever its own unit is named in. */
function held(ctx: SeedContext, instrument: InstrumentId, named: number): number {
  return ctx.registry.pieces(ctx.instruments.get(instrument).unit, named);
}

/** And for a price: money for one NAMED unit becomes money pieces for one piece. */
function priced(ctx: SeedContext, instrument: InstrumentId, perNamedUnit: number): number {
  return ctx.registry.priceOf(PHX, ctx.instruments.get(instrument).unit, perNamedUnit);
}

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

/**
 * Seed B1, B4: build a world's spec from a seed value and the banks it has. The SAME table reaches
 * the banks module and the seed, so a world with two of them or with four is this world with a
 * different table — which is what a measurement of the count needs and what a count held beside the
 * table would have quietly broken (Law 4).
 */
export function foundationSpec(seed: string, bankRows: readonly BankDecl[] = BANKS): AssemblySpec {
  return {
    seed,
    epoch: civil(2026, 1, 5),
    registry: {
      currencies: [{ code: PHX, name: 'Phoenix unit', centralBank: CB }],
      regions: [{ id: REGION, name: 'North', ccy: PHX }],
      units: [
        // Money A2, Law 8: a PHX is a hundred cents, like any real money, and the cent is the
        // smallest amount of it that exists. Every balance in this world is a whole number of them,
        // so nothing below a cent can be paid, lent, owed or left over anywhere.
        { id: currencyUnit(PHX), name: 'PHX', perUnit: MONEY_PIECES },
        // Equity A2, Fund Shares A2: a SHARE COUNT, which more than one system counts in and no
        // one of them owns. A share is INDIVISIBLE, as a register of members holds it: what one is
        // worth is the seed's own resolution (Equity's opening level), set low enough that a
        // household member holds tens of them rather than a fraction of one. A pro-rata fill, a
        // subscription and a split therefore all land on whole shares whose last one has a named
        // holder (Clearing C3) — which is the residual that dividing for ever never arrived at.
        { id: SHARES, name: 'shares', perUnit: SHARE_PIECES },
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
        id: KERNEL_PARAMS.pieceShift,
        value: 1,
        unit: 'multiple of every unit\'s declared subdivision',
        kind: 'resolution',
        owner: 'model',
        why: 'Law 8, Law 2: how many times finer than declared every unit is divided — 10 makes the piece a tenth of a cent, a tenth of a gram and a tenth of a share. Each unit states its own subdivision; this moves them all together, which is what makes the subdivision a RESOLUTION that can be TESTED: declare the same world in finer pieces and every structural invariant must hold exactly and the path must not move.',
      },
    ],
    modules: [
      // The order matters at one anchor: three phases sit before the revaluation, and they must run
      // in this order — a drawing becomes a loan row, then anything that cannot pay dies, then the
      // people it employed are released. Assembly keeps declaration order for modules that do not
      // require each other, and that is what puts them in it.
      expectations,
      creditEvents,
      banks(bankRows),
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
      funds(),
      sovereignInstruments,
      sovereignCurve(TREASURY_NORTH, PHX),
      treasury,
      centralBankOmo,
      // The money market after the treasury and the curve: a bank funds itself against the paper
      // those two put into the world, and it prices a name off what the lending module published
      // about it. Both reach it as public events and prints, never as imports (Law 15).
      moneyMarket,
      foundationSeedFor(bankRows),
      foundationFundingFor(bankRows),
    ],
  };
}

/** Build the foundation world. Reproducible from the seed value (Seed A5). */
export function foundationWorld(seed: string): World {
  return assemble(foundationSpec(seed));
}

export { PAR };
