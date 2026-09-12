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
import { toTickOf } from '../core/tick.js';
import { PIP, PIP_YEN } from '../registry/grid.js';
import {
  addDays,
  addMonths,
  civil,
  compareCivil,
  formatCivil,
  type Civil,
} from '../calendar/civil.js';
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
  type CurrencyCode,
  type ParamId,
  type PartyId,
  type RegionId,
  fxPairId,
} from '../core/ids.js';
import { Missing } from '../core/errors.js';
import type { DayCount } from '../calendar/daycount.js';
import { priceAt } from '../prices/curve.js';
import { forbid } from '../core/assert.js';
import { keyOf, weightOf } from '../parties/party.js';
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
import { BANK_COUNT, drawBanks, type BankDecl } from '../mechanisms/banks/data.js';
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
import { commodities, STORAGE_KIND } from '../mechanisms/commodities/index.js';
import { environment } from '../mechanisms/environment/index.js';
import { expectations } from '../mechanisms/expectations/index.js';
import { firms } from '../mechanisms/firms/index.js';
import { FIRM_COUNT, drawFirms, type FirmDecl } from '../mechanisms/firms/data.js';
import { goodId, goodMarketId, goods, isGoodTerms, wipId } from '../mechanisms/goods/index.js';
import { spaceFor, STORAGE } from '../registry/physical.js';
import { GOODS, type GoodDecl } from '../mechanisms/goods/data.js';
import { equity } from '../mechanisms/equity/index.js';
import { drawListed, equityLineOf, type ListedDecl } from '../mechanisms/equity/data.js';
import { funds } from '../mechanisms/funds/index.js';
import { drawEtfs, drawFunds, type EtfDecl, type FundDecl } from '../mechanisms/funds/data.js';
import { households } from '../mechanisms/households/index.js';
import { HOURS, labour } from '../mechanisms/labour/index.js';
import { fxMarketOf, pairsOf, spotFx } from '../mechanisms/spot-fx/index.js';
import { drawFxDesks } from '../mechanisms/spot-fx/data.js';
import { derivativeLayer, houseIdFor, TRADES_CONTRACTS } from '../mechanisms/derivative-layer/index.js';
import { cds } from '../mechanisms/cds/index.js';
import { irs } from '../mechanisms/irs/index.js';
import { fxDerivatives } from '../mechanisms/fx-derivatives/index.js';
import { indexFutures } from '../mechanisms/index-futures/index.js';
import { options } from '../mechanisms/options/index.js';
import { bondFutures } from '../mechanisms/bond-futures/index.js';
import { EQUITY_INDEX, GLOBAL_INDEX, SIZE_INDEX, indices } from '../mechanisms/indices/index.js';
import { ASSESSOR_COUNT, drawAssessors, ratings } from '../mechanisms/ratings/index.js';
import { reporting } from '../mechanisms/reporting/index.js';
import { research } from '../mechanisms/research/index.js';
import { sovereignCurve } from '../mechanisms/sovereign-curve/index.js';
import { treasury } from '../mechanisms/treasury/index.js';
import type { CellParty, NamedParty } from '../parties/party.js';
import { downTick, splitOnTick } from '../core/tick.js';
import { displayName } from '../registry/naming.js';
import { MONEY_PIECES, SHARE_PIECES } from '../registry/grid.js';
import {
  BANK,
  CENTRAL_BANK,
  FIRM,
  HOUSEHOLD,
  MONEY_KIND,
  SHARES,
  TREASURY,
} from '../registry/profiles.js';
import type { ParamDecl } from '../registry/params.js';
import type { AssemblySpec } from '../world/assemble.js';
import { assemble, KERNEL_PARAMS } from '../world/assemble.js';
import type { SeedContext } from '../world/context.js';
import type { SystemModule } from '../world/module.js';
import type { World } from '../world/world.js';

export const USD = currencyCode('USD');
export const REGION = regionId('us');
export const CB = partyId('fed');
export const TREASURY_US = partyId('treasury.us');

/**
 * Currency A1, A2, A3; Spot FX A3, XI-12: THE OTHER MONEYS, and the central banks whose liabilities
 * they are. A currency is not a label on an amount — it is a named issuer's promise (A2) — so every
 * one of these means another issuer, another sovereign borrowing in it, and a market against each
 * of the others.
 *
 * They are SMALLER PLACES and they say so: a central bank, a treasury and one line of paper each,
 * with no firms, no households and no labour market, because a real economy abroad is 13i's
 * cross-border item and not this one. What they are here for is what the currency layer needs to be
 * reachable at all: a holding in a money that is not its holder's own, a coupon that arrives in it,
 * two parties with opposite reasons to be in a pair (the US banks earning a foreign money they have
 * no use for, the foreign reserve manager earning dollars it has none for) — and, because there are
 * FOUR of them and not two, a cross that is not the dollar's: with three moneys in a triangle there
 * is a round trip to be taken, which is what XI-12 is about and what a two-country world cannot
 * express at all (no third leg, no vehicle, no gap to close).
 *
 * The names are labels for clarity, not claims about the real places (Law 1 is about mechanism).
 */
export interface CountryDecl {
  readonly region: RegionId;
  readonly name: string;
  readonly ccy: CurrencyCode;
  readonly ccyName: string;
  readonly centralBank: PartyId;
  readonly centralBankName: string;
  readonly treasury: PartyId;
  readonly treasuryName: string;
  /** Law 9: the stem of its government paper's id, as that market's own shorthand for it. */
  readonly paper: string;
  /** Law 8: the smallest increment a rate quoted in this money moves by — its pip (12b.1). */
  readonly quoteTick: number;
}

export const ABROAD: readonly CountryDecl[] = [
  {
    region: regionId('eu'),
    name: 'Europe',
    ccy: currencyCode('EUR'),
    quoteTick: PIP,
    ccyName: 'euro',
    centralBank: partyId('ecb'),
    centralBankName: 'European Central Bank',
    treasury: partyId('treasury.eu'),
    treasuryName: 'European Treasury',
    paper: 'bund',
  },
  {
    region: regionId('uk'),
    name: 'United Kingdom',
    ccy: currencyCode('GBP'),
    quoteTick: PIP,
    ccyName: 'pound sterling',
    centralBank: partyId('boe'),
    centralBankName: 'Bank of England',
    treasury: partyId('treasury.uk'),
    treasuryName: 'HM Treasury',
    paper: 'gilt',
  },
  {
    region: regionId('jp'),
    name: 'Japan',
    ccy: currencyCode('JPY'),
    quoteTick: PIP_YEN,
    ccyName: 'yen',
    centralBank: partyId('boj'),
    centralBankName: 'Bank of Japan',
    treasury: partyId('treasury.jp'),
    treasuryName: 'Japanese Treasury',
    paper: 'jgb',
  },
];
/**
 * Seed B1, B4: WHICH BANKS THIS WORLD HAS is the table the banks module declares, and the seed is
 * built from the same one — there is no count beside it and no second list (Law 4). `BANK_A` and
 * `BANK_B` are exported because two other modules' data name them (a fund banks somewhere, a share
 * line opens on somebody's book) and because every world this seed builds has at least two.
 */
export const BANK_A = partyId('bank.a');
export const BANK_B = partyId('bank.b');
/** The ten-year benchmark: the line every other price is quoted against (Sovereign D4). */
export const GOV_LINE = instrumentId('ust.2036-03-15');
export const GOV_MARKET = marketId('mkt.ust.2036-03-15');
/** Sovereign D3.a: one owner of the curve, one convention, declared by the module that owns it. */
export const GOV_CURVE = curveFamilyId('ust');

/** The issuer's own maturity grid: it places every line it brings on one of these days (B3.a). */
export const MATURITY_MONTHS: readonly number[] = [3, 6, 9, 12];
export const MATURITY_DAY = 15;

/**
 * What the seed opens outstanding: a PROFILE, and never a table of amounts (Seed C3, Treasury D4.a).
 *
 * A TENOR, and the weight the stock carries at it — for each of the two reasons anybody in this
 * world owns the paper, because they are different reasons and they sit at different places on the
 * curve. A bank holds it because it is the liquid asset its own rule asks of it, so its weight is
 * at the short end; a household holds it because it is saving, so its weight is at the long end.
 * Neither weight is an amount: an amount has to be restated every time the world changes size, and
 * a number restated to keep a result is a result wearing an endowment's name (Law 2). How much
 * there IS of it is one number per member of the population, below.
 */
interface SeedTenor {
  readonly paper: 'bill' | 'bond';
  /** Months from the epoch. It is PLACED on the issuer's own maturity grid, never stated (B3.a). */
  readonly months: number;
  /** Seed B4: the part of what the banking system holds that sits in this line. */
  readonly bankWeight: number;
  /** The part of what households hold directly that sits in this line. */
  readonly householdWeight: number;
  readonly why: string;
}

const MONTHS_PER_YEAR = 12;

/**
 * Sovereign D4, Currency A3: the one tenor every other country borrows at. Ten years, which is the
 * benchmark America's own curve is quoted against — the same point on two curves is what makes a
 * spread between two countries mean anything (XI-7), and it is the only line each of them needs to
 * be a borrower in its own money.
 */
const ABROAD_TENOR_MONTHS = 10 * MONTHS_PER_YEAR;

const SEED_PROFILE: readonly SeedTenor[] = [
  {
    paper: 'bill',
    months: 3,
    bankWeight: 5,
    householdWeight: 0,
    why: 'The shortest bill on the grid. A bank holds it because it is the asset its own liquidity rule asks of it and nobody saves at three months, so the whole of this line is at the banks.',
  },
  {
    paper: 'bill',
    months: 6,
    bankWeight: 5,
    householdWeight: 0,
    why: 'The same reason a quarter further out. Two bills rather than one so that the short end of the curve has a shape the first auction can move.',
  },
  {
    paper: 'bill',
    months: MONTHS_PER_YEAR,
    bankWeight: 5,
    householdWeight: 1,
    why: 'The year bill: the first line a saver is in at all, and the last one a treasury is. It is where the two reasons to hold the paper overlap, which is what makes it the line a shock passes through.',
  },
  {
    paper: 'bond',
    months: 2 * MONTHS_PER_YEAR,
    bankWeight: 3,
    householdWeight: 2,
    why: 'The first coupon-paying line. A bank still holds it — it is liquid and it is this issuer — but a saver is now the larger part of the demand, which is what makes a bond market different from a bill market.',
  },
  {
    paper: 'bond',
    months: 5 * MONTHS_PER_YEAR,
    bankWeight: 3,
    householdWeight: 3,
    why: 'The belly of the curve, and the line a bank is least glad to be holding when rates move: long enough to lose real money on and short enough that it cannot claim to be holding it to maturity.',
  },
  {
    paper: 'bond',
    months: 10 * MONTHS_PER_YEAR,
    bankWeight: 3,
    householdWeight: 4,
    why: 'Sovereign D4: the benchmark, and the line every other price in this world is quoted against. The largest single household holding, because it is what saving for a life stage actually looks like.',
  },
];

/** One line of the profile, once the grid has placed it and the population has sized it. */
interface SeedLine {
  readonly id: string;
  /** Which of the two sovereign instruments this line is (Sovereign B1: two, not one with a flag). */
  readonly paper: 'bill' | 'bond';
  readonly maturity: Civil;
  /** Units the banking system holds, split across the banks that exist by each one's size. */
  readonly banks: number;
  /** Units one member of every household cell holds (Seed B4, XI-15). */
  readonly perMember: number;
}

/**
 * B3.a: the issuer places every line it brings on its own maturity grid, so a tenor asked for is a
 * date the market already trades rather than a new one. The first grid day at or after the tenor.
 */
function onGrid(epoch: Civil, months: number): Civil {
  const wanted = addMonths(epoch, months);
  for (let ahead = 0; ahead <= MONTHS_PER_YEAR; ahead += 1) {
    const at = addMonths(wanted, ahead);
    if (!MATURITY_MONTHS.includes(at.m)) continue;
    const day = civil(at.y, at.m, MATURITY_DAY);
    if (compareCivil(day, wanted) >= 0) return day;
  }
  throw new Missing('Sovereign B3.a', `no grid date within a year of ${months} months out`, {
    months,
  });
}

/**
 * Seed C3, B4: THE PROFILE, SIZED BY THE POPULATION IT IS OWED TO. Everything outstanding is a
 * weight of one stated number — what a member of this world holds of its sovereign's debt — so a
 * world of thirty million people has thirty million people's worth of it and no line of data is
 * restated. Law 8: whole units, because a unit of the paper is indivisible.
 */
function seedLines(
  epoch: Civil,
  members: number,
  perMember: number,
  householdShare: number,
): SeedLine[] {
  const bankWeight = sum(SEED_PROFILE.map((t) => t.bankWeight)).value;
  const householdWeight = sum(SEED_PROFILE.map((t) => t.householdWeight)).value;
  const atBanks = mul(
    mul(members, perMember, 'the debt outstanding'),
    1 - householdShare,
    'at the banks',
  );
  const perHead = mul(perMember, householdShare, 'what a member holds directly');
  return SEED_PROFILE.map((t): SeedLine => {
    const maturity = onGrid(epoch, t.months);
    const dated = formatCivil(maturity);
    return {
      id: t.paper === 'bond' ? `ust.${dated}` : `ust.bill.${dated}`,
      paper: t.paper,
      maturity,
      banks: Math.round(div(mul(atBanks, t.bankWeight, 'its weight'), bankWeight, 'this line')),
      perMember: Math.round(
        div(mul(perHead, t.householdWeight, 'its weight'), householdWeight, 'this line'),
      ),
    };
  });
}

/** One day count for the seeded paper, so an opening price and its yield use one convention. */
const SEED_DAY_COUNT: DayCount = 'ACT/ACT';

/**
 * What a firm opens with of what it makes, of what its recipe draws, and of the plant that lets it
 * make anything at all (Seed D1: a stock consistent with the flows that will act on it).
 *
 * NONE OF IT IS STATED ONE FIRM AT A TIME, AND NONE OF IT IS AN AMOUNT. It used to be a table of
 * twelve rows and six columns — seventy-two numbers, every one of them a quantity that meant what it
 * meant only for a world of six thousand people and twelve firms. What is stated instead is the
 * SCALE of the world (one number: what a member of the population takes off the end of the chain in
 * a period) and the three RATIOS below; everything a firm opens holding is that, walked up the
 * chain through the good's own recipe (Law 19) and divided by the firm's own size (Seed B4).
 *
 * So a world of thirty million people opens with thirty million people's worth of stock in it, held
 * by however many firms it has, in the proportions their own sizes give — and the seed has not
 * chosen a single quantity anywhere.
 */
const SEED_STOCK_BASIS = 0.8;
/** Seed C4: the level each market opens at, which is the good's and not any one firm's. */
const SEED_MARKETS: readonly {
  readonly subUnit: string;
  readonly opensAt: number;
  readonly why: string;
}[] = [
  {
    subUnit: 'grain',
    opensAt: 400,
    why: 'Grain takes two periods to grow, so a world that opens with an empty field produces nothing for two of them and the mill has nothing to buy. One crop in the barn and one in the ground is what a going concern looks like.',
  },
  {
    subUnit: 'flour',
    opensAt: 600,
    why: 'Milling is inside the period, so there is nothing on the line; what a mill opens with is what it has already milled.',
  },
  {
    subUnit: 'bread',
    opensAt: 1200,
    why: 'A week of bread in the shop. It goes stale at a quarter a period, so what is not sold is a real loss from the first period on.',
  },
  {
    subUnit: 'machine',
    opensAt: 2500,
    why: 'A machine is sixty hours of engineering, and this is what sixty hours of it is worth at the level the rest of this world opens at — so a workshop bids for an hour somewhere between what a mill will pay and what a farm will, and the capital-goods line is neither the best nor the worst employer on the first morning.',
  },
];

/**
 * Capital Programme A6, Seed C3: the vintages the world opens with, as ages in periods. Three of
 * them, evenly spread across a machine's life, so a third of every firm's plant comes up for
 * replacement at a time and the world has a reason to invest before anything has grown.
 */
const SEED_PLANT_AGES: readonly number[] = [26, 78, 130];

const P = {
  cellsPerKey: paramId('seed.households.cellsPerKey'),
  membersPerCohort: paramId('seed.households.membersPerCohort'),
  debtPeriods: paramId('seed.sovereign.debtPeriods'),
  householdDebtShare: paramId('seed.sovereign.householdShare'),
  firmCashPeriods: paramId('seed.firm.cashPeriods'),
  // Declared by the labour module and read here by id: what a member's time comes to in a period,
  // and who is in the workforce at all. A seed may not import a mechanism.
  hoursPerMember: paramId('labour.hoursPerMember'),
  retirementAge: paramId('labour.retirementAge'),
  plantHeadroom: paramId('seed.firm.plantHeadroom'),
  openingYield: paramId('seed.openingYield'),
  openingRate: paramId('seed.openingRate'),
  crossHoldingShare: paramId('seed.crossHoldingShare'),
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
    unit: `USD per unit of ${row.subUnit}`,
    dimension: 'price',
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
export function foundationSeedFor(
  bankRows: readonly BankDecl[],
  firmRows: readonly FirmDecl[],
): SystemModule {
  return {
    id: 'seed.foundation',
    spec: 'Seed',
    // Part XIII, Seed A3: what this seed READS to size the world it opens. The scale is derived, not
    // stated — it is the hours this world's people actually offer, divided into the hours the chain
    // needs — so the modules that own those facts have to be here: `labour` states what a member
    // offers and when it retires, `households` states who the people are, `goods` states the recipes
    // the chain is made of. A world assembled without them cannot be seeded at all, and saying so
    // here is better than the missing parameter it used to fail on.
    // Part XIII: AND `banks`, because this seed READS `regulation.leverageRatio` by id to size a
    // bank's opening balance sheet (Seed C1, Banks Capital B1.b). A seed may not import a mechanism,
    // so it reads the number by name — and a dependency read by name is still a dependency: a world
    // assembled without the module that declares it got `parameter regulation.leverageRatio is not
    // declared` instead of whatever it was actually testing.
    requires: ['sovereign-instruments', 'goods', 'firms', 'households', 'labour', 'banks'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [
      {
        id: P.cbOpeningShare,
        value: 0.2,
        unit: 'ratio of a line outstanding',
        dimension: 'ratio',
        kind: 'policy',
        owner: 'centralBank',
        why: 'Central Bank C1, C1.a: WHAT THE CENTRAL BANK OPENS HOLDING OF EVERY SOVEREIGN LINE, and therefore how big its balance sheet is — every reserve it has issued was issued to buy this paper (A2). C1.a says it plainly: the size is set by POLICY, and that is what this is. It is stated BELOW the open-market target so the central bank opens short of what its policy wants and its first session has something to do: a seed that opened it at its target would be seeding the outcome of the mechanism it is about to run (Seed E1). It was declared a PLACEHOLDER standing for a derivation from the liquidity standard the banks are held to, and item 11.5 measured that and found there is none to make: `liquid ≥ couldLeave` reduces to `paperShare × haircut \u2264 a bank own capital line`, which at a haircut of 0.05 and lines of 0.038 to 0.068 is slack from a paper share of 0 to one of about 0.77 — an inequality over most of the range, not an equation with one root. What actually put banks below the standard was not the mix at all but the equity float sitting on the desks that made its market, which raises nothing at the window; with the float where its holders are, every bank in every seed opens at 1.04 to 1.08 at two banks and at twenty.',
      },
      {
        id: P.treasuryBufferShare,
        value: 0.4,
        unit: "ratio of the central bank's balance sheet",
        dimension: 'ratio',
        kind: 'shape',
        owner: 'model',
        why: "Treasury D4.b, Seed E2: how much of the central bank's money the treasury opens holding, with the banks holding the rest as reserves. It is a share and not an amount because the amount is not free: every unit of central-bank money was issued to buy the paper above, so what is stated is how it is divided and never how much of it there is. The mechanism that replaces it is the treasury's own funding programme, which decides its balance from period one.",
      },
      {
        id: P.cellsPerKey,
        value: 2,
        unit: 'count',
        dimension: 'count',
        kind: 'resolution',
        owner: 'model',
        why: 'XI-15: how many cells stand for each (region, cohort, bank) population; change it and the answer must not move.',
      },
      {
        id: P.membersPerCohort,
        value: 15_000_000,
        unit: 'count',
        dimension: 'count',
        kind: 'placeholder',
        owner: 'model',
        standsInFor: { mechanism: 'Households A5', worklistItem: '13f' },
        why: "HOW MANY PEOPLE THERE ARE. Fifteen million a cohort and two cohorts, so thirty million in the region — which is a country, and the scale every other number in this seed is a ratio against. It was six thousand, and six thousand was a test rig wearing a world's name: a labour venue with a handful of employers in it, a bank sector whose smallest member was the size of one firm, and a bill line a single household cell could have bought outright. Every mechanism that needs somebody else to be there — a second bidder, another lender, a market that does not move when one party leaves — was being tested against a world too small to have one. It is a PLACEHOLDER and not a resolution: the answer MOVES with it, which is the whole reason it had to change, and what ends it is a population with births and deaths in it (worklist 13f) rather than a count anybody states.",
      },
      {
        id: P.debtPeriods,
        value: 21,
        unit: "periods of the economy's own output",
        dimension: 'periods',
        kind: 'shape',
        owner: 'model',
        why: "Seed C3, Treasury D4.a: HOW MUCH SOVEREIGN PAPER IS OUTSTANDING, as periods of what this world TURNS OVER. A stock against a flow, which is the ratio a sovereign's debt is actually spoken of in. It is not per head: a treasury that has borrowed is a treasury that spent, and what it spent it on is an economy rather than a queue of people, so a world whose people can make forty times as much has forty times the debt and the same ratio — stated per head it did not scale with the real economy at all, and the banks could not carry their own depositors' accounts. TWENTY-ONE AND NOT FIFTY-TWO, because turnover is not output: this walk values every stage of the chain, so a tonne of bread is counted again as the flour and again as the grain, and gross turnover comes to something over twice what the world actually makes. A debt of a year of output is therefore a third of a year of turnover, which is what this is. From period one the treasury's own funding programme decides its stock and nothing reads this again (Treasury D4).",
      },
      {
        id: P.householdDebtShare,
        value: 1 / 3,
        unit: 'share of the debt outstanding',
        dimension: 'ratio',
        kind: 'shape',
        owner: 'model',
        why: 'Sovereign E2.f, Seed E2: the part of the sovereign debt households hold DIRECTLY, with the banking system holding the rest. Two holders and two reasons: a bank holds it because its own liquidity rule asks for a liquid asset, a household because it is saving, and a line held by only one of them has one side to its market. What replaces it is who actually bids in the auctions and the secondary sessions, which happens from period one.',
      },
      {
        id: P.firmCashPeriods,
        value: 3,
        unit: 'periods of turnover',
        dimension: 'periods',
        kind: 'shape',
        owner: 'model',
        why: 'Seed C1, D1: how many periods of its own turnover a firm opens holding as money. It pays its wage bill and buys its inputs before it is paid for what it sells, so a firm that opens with nothing fails in its first period on a timing gap and not on its economics — which is a statement about the seed and not about the firm (Firm D1). Three periods, because the chain is three deep and the money has to get from the end of it back to the start. From period one what a firm holds is what it was paid less what it spent, and nothing reads this again.',
      },
      {
        id: P.plantHeadroom,
        value: 1.5,
        unit: 'multiple of what the line currently starts',
        dimension: 'ratio',
        kind: 'shape',
        owner: 'model',
        why: 'Capital Programme A2, Seed D1: how much plant a firm opens with over what its current output needs. A going concern is not running at its ceiling — a firm with no headroom cannot answer a good week at all, and a world of them would show a supply response of exactly zero from the first period. From period one investment decides the stock and nothing reads this again (Capital Programme B1).',
      },
      {
        id: P.openingRate,
        value: 1,
        unit: 'units of the quote money per unit of the base',
        dimension: 'price',
        kind: 'shape',
        owner: 'model',
        why: "Spot FX C1, Seed C4: what one unit of one money costs in another, before any pair has ever traded. A market that has never traded has no price (XI-6) and a world that opens with holdings in four moneys has to say what they are worth in each other, so ONE level is claimed and each pair's own first session replaces it. One, and the same one for every pair, because a level of one asserts less than any other number would: it says the moneys are all the same size, which is what a world with nothing to distinguish them yet has no reason to deny — and it opens the three crosses consistent with the three dollar rates, so the triangle starts with no gap in it rather than with one somebody put there. What it opens at is not what it stays at: America's banks earn euros, sterling and yen they have no use for and the foreign reserve managers earn dollars they have none for, and where those meet is the rate from period one.",
      },
      {
        id: P.crossHoldingShare,
        value: 0.08,
        unit: "share of a holder's paper that is another country's",
        dimension: 'ratio',
        kind: 'placeholder',
        owner: 'model',
        why: "Currency D2, Central Bank F4: how much of every central bank's reserves is another country's paper, split evenly between the countries that issue it. A central bank holds it because that is what reserves ARE (F4), and it is the only holder for whom foreign paper is what it is for — what a COMMERCIAL bank holds abroad is a position it takes with its own capital, which is 13h's decision and not the seed's. Eight per cent, small enough that this is a reserve holding rather than a currency fund and large enough that a week of exchange rates is visible in what a central bank is worth. Evenly, because the seed has nothing to say about which foreign government a reserve manager prefers. It is a PLACEHOLDER: what replaces it is the portfolio decision at 13h, after which what anybody holds abroad is an outcome of what it bought and sold.",
        standsInFor: {
          mechanism: 'Central Bank F4',
          worklistItem: '13h',
        },
      },
      ...openingPrices(),
      {
        id: P.openingYield,
        value: 0.02,
        unit: 'per annum',
        dimension: 'perAnnum',
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
      ctx.parties.add(named(CB, CENTRAL_BANK, 'Federal Reserve', CB));
      ctx.parties.add(named(TREASURY_US, TREASURY, 'US Treasury', CB));
      // Currency A2, A3: each other money's issuer and the sovereign that borrows in it. They book in
      // their own region, which is what makes everything they hold of American paper FOREIGN and
      // everything America holds of theirs foreign the other way (D2).
      for (const c of ABROAD) {
        const abroad = (id: PartyId, kind: NamedParty['kind'], name: string): NamedParty => ({
          id,
          kind,
          region: c.region,
          name,
          bank: c.centralBank,
          representation: 'named',
          status: { alive: true },
        });
        ctx.parties.add(abroad(c.centralBank, CENTRAL_BANK, c.centralBankName));
        ctx.parties.add(abroad(c.treasury, TREASURY, c.treasuryName));
      }
      // Seed B1, B4: as many banks as `banks.count`, each with the disposition and the size its own
      // row states. THREE by default, because with two every depositor that answers a rate is the
      // whole of one side of the deposit market, every interbank session is one name facing one name,
      // and a bank in trouble has exactly one place to go — and because the count being load-bearing
      // in a way no mechanism states is exactly what makes it a RESOLUTION to be measured (XI-15).
      const banks = bankRows.map((b) => ({ id: partyId(b.bank), size: b.size }));
      banks.forEach((b, n) => {
        ctx.parties.add(named(b.id, BANK, `Bank ${String.fromCharCode(65 + n)}`, CB));
      });
      // Seed B1, B3: the firms this world drew, spread across the banks that exist so that no bank's
      // customers all sit on one side of the payment chain — a bank whose do has a structural reserve
      // drain, which is a flow this world would be opening with rather than producing.
      firmRows.forEach((f, n) => {
        const bank = banks[n % banks.length];
        if (bank === undefined) return;
        ctx.parties.add(named(partyId(f.firm), FIRM, f.name, bank.id));
      });

      // Money instruments: one per issuer (Money A1, D2). Each central bank issues its own region's
      // money and a bank issues the money of the region it books in — there is no such thing as
      // money without an issuer, and none of it is anybody else's (A1).
      for (const issuer of [CB, ...banks.map((b) => b.id)]) {
        ctx.instruments.add({
          id: moneyInstrumentId(issuer, USD),
          kind: MONEY_KIND,
          issuer: some(issuer),
          ccy: USD,
          terms: { kind: MONEY_KIND },
          market: none(),
        });
      }
      for (const c of ABROAD) {
        ctx.instruments.add({
          id: moneyInstrumentId(c.centralBank, c.ccy),
          kind: MONEY_KIND,
          issuer: some(c.centralBank),
          ccy: c.ccy,
          terms: { kind: MONEY_KIND },
          market: none(),
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
      const cells = positiveCount(ctx.params.count(P.cellsPerKey), 'cellsPerKey');
      const members = positiveCount(ctx.params.count(P.membersPerCohort), 'membersPerCohort');
      for (const cohort of ctx.registry.cohorts) {
        // Seed B4: and they are spread across the banks IN PROPORTION TO SIZE, so a bigger bank has
        // more depositors — which is what makes it bigger. Split exactly: a weight is a count of
        // people and the odd person has a named cell (core/tick.ts), never a fraction anywhere.
        const perBank = splitOnTick(
          members,
          banks.map((b) => b.size),
        );
        const weights = banks.flatMap((bank, at) =>
          splitPopulation(zeroIfNone(perBank[at]), cells).map((weight, n) => ({ bank, weight, n })),
        );
        weights.forEach(({ bank, weight, n }) => {
          if (weight <= 0) return;
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

      // Law 19: HOW MANY PEOPLE THERE ARE is read off the cells that were just created, never
      // recomputed from the parameter they were cut from — a second derivation of a population is a
      // second population, and it goes wrong the first time a split does not come out.
      const householdMembers = ctx.parties.ofKind(HOUSEHOLD).reduce((t, c) => t + weightOf(c), 0);

      // ------------------------------------------------------------------------------------------
      // WHAT EVERY FIRM OPENS HOLDING (Seed D1, Goods A2.a, Capital Programme A2; Law 2, Law 19)
      //
      // NO STATED QUANTITY AT ALL. What this world makes is what its people's time makes: the chain
      // is walked once at unit scale through each good's own recipe — the recipe says how much of an
      // input a unit started draws, the yield says how many of the started ones arrive, and the hours
      // say what each step costs — and the hours the population offers then divide by the hours one
      // unit of the final good costs all the way down.
      //
      // The seed therefore chooses no amount anywhere. Ask it for thirty million people and it opens
      // with what thirty million people can make of grain, flour, bread, machines and plant, held by
      // however many firms the world has, in the proportions their own draws gave them.
      // ------------------------------------------------------------------------------------------
      const started = new Map<string, number>();
      const madeHere = firmRows.filter((f) => ctx.instruments.has(goodId(f.subUnit, REGION)));
      const sizeOfLine = new Map<string, number>();
      for (const f of madeHere) {
        sizeOfLine.set(f.subUnit, add(zeroIfNone(sizeOfLine.get(f.subUnit)), f.size, 'its line'));
      }
      // Law 19: the recipe is read from the ONE place that writes it — the goods registry, in the
      // units a person says it in. The instrument's own terms carry the same recipe converted to
      // pieces of each good (Law 8), and this seed speaks in named units by contract, so reading the
      // converted copy here would be converting it back.
      const recipeOf = (subUnit: string): GoodDecl => {
        const d = GOODS.find((g) => g.subUnit === subUnit);
        if (d === undefined) throw new Missing('Goods A1', `no recipe for ${subUnit}`, { subUnit });
        return d;
      };
      // What draws what. A good nothing else draws is what this world makes FOR somebody: the
      // population, for a consumption good, and the replacement of worn-out plant, for a capital one.
      const drawnBy = new Map<string, string[]>();
      for (const subUnit of sizeOfLine.keys()) {
        for (const input of recipeOf(subUnit).inputs) {
          drawnBy.set(input.subUnit, [...(drawnBy.get(input.subUnit) ?? []), subUnit]);
        }
      }
      const madeInto = new Set(CAPITAL_KINDS.map((k) => k.madeFrom));
      const finalGoods = [...sizeOfLine.keys()]
        .filter((g) => (drawnBy.get(g) ?? []).length === 0 && !madeInto.has(g))
        .sort();
      // ------------------------------------------------------------------------------------------
      // HOW BIG THIS ECONOMY IS, and it is NOT STATED (Law 2: the fewest primitives).
      //
      // What a world makes is what its people's time makes. Every step of every chain takes hours
      // the recipe names (Goods A2.c), the people offer the hours their own week has in it (Labour
      // A1, B2), and the ones who offer them are the cohorts below the retirement age (B3). Between
      // them those three facts FIX the scale, and there is nothing left for the seed to say about it.
      //
      // It used to be one stated number — what a member takes off the end of the chain in a period —
      // and the number was two orders of magnitude below what the population could produce: it asked
      // 0.4 hours a week of a member who offers 35, so 97% of this world's time had nowhere to go,
      // the wage bill was a fortieth of what its people could earn, and every firm was bound by a
      // demand that could never have paid for what its plant was sized to make (item 12's finding
      // 12-15, now worklist 16's starting number).
      // The seed was not sizing an economy; it was starving one.
      //
      // ONE PASS AT UNIT SCALE and then a multiplication, because the chain is linear in its output:
      // ask what one unit of the final good costs in hours ALL THE WAY DOWN — including the machines
      // that wear out making it — and the answer divides the hours there are.
      // ------------------------------------------------------------------------------------------
      const retirementAge = ctx.params.years(P.retirementAge);
      const perWeek = ctx.params.amount(P.hoursPerMember, HOURS);
      let hoursOffered = 0;
      for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
        if (cell.representation !== 'cell') continue;
        if (ctx.registry.cohort(cohortId(keyOf(cell, 'cohort'))).fromAge >= retirementAge) continue;
        hoursOffered = add(
          hoursOffered,
          mul(weightOf(cell), perWeek, 'what it offers'),
          'the hours there are',
        );
      }
      for (const g of finalGoods) {
        started.set(g, div(1, recipeOf(g).yieldRate, 'started for one of it'));
      }
      // Down the chain, deepest first: a line starts what everything it feeds draws from it.
      const upstream = [...sizeOfLine.keys()].filter(
        (g) => !finalGoods.includes(g) && !madeInto.has(g),
      );
      let settled = new Set(finalGoods);
      while (upstream.some((g) => !settled.has(g))) {
        const next = upstream.find(
          (g) => !settled.has(g) && (drawnBy.get(g) ?? []).every((d) => settled.has(d)),
        );
        if (next === undefined) break;
        let drawn = 0;
        for (const by of drawnBy.get(next) ?? []) {
          const qty = recipeOf(by).inputs.find((i) => i.subUnit === next);
          if (qty === undefined) continue;
          drawn = add(
            drawn,
            mul(zeroIfNone(started.get(by)), qty.qtyPerUnit, 'what it draws'),
            'drawn',
          );
        }
        started.set(next, div(drawn, recipeOf(next).yieldRate, 'started for it'));
        settled = new Set([...settled, next]);
      }
      // Capital Programme A2: the plant those lines run on, with the headroom a going concern has.
      const headroom = ctx.params.ratio(P.plantHeadroom);
      const plantOf = (subUnit: string, kind: string): number => {
        const need = recipeOf(subUnit).plant.find((q) => q.capitalKind === kind);
        if (need === undefined) return 0;
        return mul(
          mul(zeroIfNone(started.get(subUnit)), need.unitsPerUnitPerPeriod, 'the plant it takes'),
          headroom,
          'with the headroom a going concern has',
        );
      };
      // A4.b: and the capital-goods line starts what REPLACES the plant that wears out — one life's
      // worth of it a life, which is what a stock of machines with a life in it demands every period.
      for (const kind of CAPITAL_KINDS) {
        if (!sizeOfLine.has(kind.madeFrom)) continue;
        const inService = sum([...sizeOfLine.keys()].map((g) => plantOf(g, kind.id))).value;
        const wearing = div(inService, kind.usefulLifePeriods, 'what wears out in a period');
        started.set(
          kind.madeFrom,
          div(wearing, recipeOf(kind.madeFrom).yieldRate, 'started for it'),
        );
      }
      // Goods A2.c, Labour A1: AND NOW THE SCALE. Everything above is what ONE unit of the final good
      // costs the world, all the way down — the grain it takes, the milling, the baking, and the
      // share of a machine's life it wears out. What that comes to in HOURS divides the hours the
      // population has, and the answer is how many of the final good this world makes in a period.
      //
      // Law 19: the hours are the recipe's own, read from the goods registry like everything else
      // here, and a firm's own productivity does not enter — that is dispersion WITHIN a line
      // (Firm A3), and what is being sized is the line.
      const hoursForOne = sum(
        [...sizeOfLine.keys()].map((g) =>
          mul(zeroIfNone(started.get(g)), recipeOf(g).labourHoursPerUnit, `the hours ${g} takes`),
        ),
      ).value;
      forbid(
        hoursForOne > 0,
        'Seed D1',
        'this world makes nothing that takes anybody any time, so its people have nothing to do',
        { hoursForOne },
      );
      // Law 8, Law 2: BOTH SIDES OF THIS RATIO ARE COUNTED THE SAME WAY. `hoursOffered` is what the
      // state holds — a count of the indivisible pieces an hour is divided into at this world's
      // resolution (`params.amount`) — and `hoursForOne` came off the recipes, which speak in whole
      // named hours. Dividing one by the other multiplied this world's entire real economy by the
      // subdivision of an hour: declare the same world in finer pieces and it made ninety times as
      // much of everything, its sovereign borrowed ninety times as much against it, and at a
      // hundredth of a cent the money stock went past exact arithmetic and Law 8 refused to open the
      // world at all. That is the resolution invariance (Law 2) failing at its own first step, and
      // the test that reported it was right: `test/tick.test.ts` is what caught it.
      const takes = ctx.registry.pieces(HOURS, hoursForOne);
      const scale = div(hoursOffered, takes, 'how many of the final good the hours there are make');
      for (const g of [...started.keys()]) {
        started.set(g, mul(zeroIfNone(started.get(g)), scale, `what ${g} starts in a period`));
      }
      // Seed B4: and a firm's share of its own line is its own size over the line's.
      const shareOf = (f: FirmDecl): number =>
        div(f.size, zeroIfNone(sizeOfLine.get(f.subUnit)), 'its share');
      /** What this firm starts in a period. Everything it opens holding is a period of this. */
      const startsOf = (f: FirmDecl): number =>
        mul(zeroIfNone(started.get(f.subUnit)), shareOf(f), 'its own');
      const cashPeriods = ctx.params.periods(P.firmCashPeriods);
      /**
       * Seed C1: the money it opens with, as periods of its own turnover at what the good opens at.
       * It pays its wage bill and buys its inputs before it is paid for what it sells, so a firm that
       * opens with nothing fails on a timing gap rather than on its economics (Firm D1).
       */
      const cashOf = (f: FirmDecl): number =>
        mul(
          mul(
            mul(startsOf(f), recipeOf(f.subUnit).yieldRate, 'what arrives'),
            ctx.params.price(openingPrice(f.subUnit)),
            'what it turns over',
          ),
          cashPeriods,
          'periods of it',
        );

      // The maturity profile, outstanding with remaining lives (Seed C3, Treasury D4.a). Every bond
      // carries the coupon that makes it par at the opening yield, so nothing but a level is claimed,
      // and how much of it there is, is the population it is owed by (Law 2: one number, not a table).
      const y = ctx.params.perAnnum(P.openingYield);
      const opening = new Map<string, number>();
      // Seed C3, Treasury D4.a: HOW MUCH SOVEREIGN PAPER THERE IS, as a stock of the economy it is
      // owed by rather than of the heads in it. A treasury that has borrowed is a treasury that
      // spent, and what it spent it on is an economy — so what is stated is how many periods of what
      // this world MAKES the sovereign owes, and the amount follows from the same walk everything
      // else here follows from. Stated per head it did not scale with the real economy at all: a
      // world whose people could make forty times as much had the same money in it, and its banks
      // could not carry their own depositors' accounts (Seed D1 refused to open it).
      const turnover = sum(
        [...sizeOfLine.keys()].map((g) =>
          mul(
            mul(zeroIfNone(started.get(g)), recipeOf(g).yieldRate, `what ${g} makes in a period`),
            ctx.params.price(openingPrice(g)),
            'what that fetches',
          ),
        ),
      ).value;
      const seedLineRows = seedLines(
        ctx.calendar.epoch,
        householdMembers,
        div(
          mul(turnover, ctx.params.periods(P.debtPeriods), 'the debt outstanding'),
          householdMembers,
          'per member',
        ),
        ctx.params.ratio(P.householdDebtShare),
      );
      for (const line of seedLineRows) {
        const id = instrumentId(line.id);
        const market = marketId(`mkt.${line.id}`);
        const maturity = line.maturity;
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
          issuer: some(TREASURY_US),
          ccy: USD,
          terms,
          market: some(market),
        });
        ctx.openMarket({
          id: market,
          name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
          instrument: id,
          ccy: USD,
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
          ccy: USD,
          provenance: { kind: 'opening' },
        });
      }

      // ------------------------------------------------------------------------------------------
      // THE OTHER COUNTRIES' PAPER, AND WHO HOLDS WHOSE (Currency A3, D2; Spot FX A3, B1, B2; Bond N3)
      //
      // One line each, at the benchmark tenor, in that country's own money and promised by its own
      // treasury. The US banks hold some of all three and each foreign reserve manager holds dollars:
      // that is what a cross holding IS, and it is what makes every part of the currency layer
      // reachable at once —
      //
      //   a position in a money that is not its holder's own, so the revaluation has something to
      //   revalue (D2); a COUPON that arrives in a money its receiver does not book in, so the
      //   conversion is a real payment and not a display (C4); two parties with OPPOSITE reasons to
      //   be in the pair market — the US banks earning euros, sterling and yen they have no use for
      //   and the foreign central banks earning dollars they have none for (B1, B2), neither of them
      //   a view of the rate (XI-13); and SIX pairs rather than one, so a cross exists that is not
      //   the dollar and a round trip through three of them either does or does not pay (XI-12).
      //
      // WHAT EVERY RATE OPENS AT is the one stated level, like every other opening price, and each
      // pair's own first session replaces it (Seed C4).
      const abroadLine = new Map<string, InstrumentId>();
      const abroadPrice = new Map<string, number>();
      for (const c of ABROAD) {
        const line = instrumentId(
          `${c.paper}.${formatCivil(onGrid(ctx.calendar.epoch, ABROAD_TENOR_MONTHS))}`,
        );
        const market = marketId(`mkt.${line}`);
        const terms: SovereignBondTerms = {
          kind: SOVEREIGN_BOND,
          coupon: rate(y, ANNUAL),
          couponPeriodicity: SEMI_ANNUAL,
          dayCount: SEED_DAY_COUNT,
          issueDate: ctx.calendar.epoch,
          maturity: onGrid(ctx.calendar.epoch, ABROAD_TENOR_MONTHS),
        };
        ctx.instruments.add({
          id: line,
          kind: SOVEREIGN_BOND,
          issuer: some(c.treasury),
          ccy: c.ccy,
          terms,
          market: some(market),
        });
        ctx.openMarket({
          id: market,
          name: displayName(ctx.instruments.get(line), ctx.parties, ctx.registry),
          instrument: line,
          ccy: c.ccy,
          rationing: 'proRata',
        });
        const flows = ctx.registry
          .instrumentKind(SOVEREIGN_BOND)
          .cashFlows(ctx.instruments.get(line), ctx.calendar.epoch, ctx.calendar);
        const price = priceAt(flows, y, ctx.calendar.epoch, SEED_DAY_COUNT, `opening ${line}`);
        ctx.prices.write({
          instrument: line,
          market,
          period: ctx.period,
          price: priced(ctx, line, price),
          ccy: c.ccy,
          provenance: { kind: 'opening' },
        });
        abroadLine.set(String(c.region), line);
        abroadPrice.set(String(c.region), price);
      }
      // Spot FX C1, Seed C4: what a unit of one money costs in another, before any pair has traded.
      // EVERY pair, including the three that are not the dollar's: a cross with no opening level is a
      // market whose dealers have nothing to quote around, and a world that opens with holdings in
      // four moneys has to say what they are worth in each other. One level for all of them, because
      // one asserts less than any other number would (P.openingRate), and six sessions replace it.
      for (const { base, quote } of pairsOf([...ctx.registry.currencies.keys()])) {
        ctx.prices.write({
          instrument: fxPairId(base, quote),
          market: fxMarketOf(base, quote),
          period: ctx.period,
          // Law 8: a rate is a price and opens on its pair's own grid — its pip.
          price: toTickOf(
            ctx.params.price(P.openingRate),
            ctx.registry.rateTickFor(base, quote),
          ),
          ccy: quote,
          provenance: { kind: 'opening' },
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

      // What a household member holds in PAPER is stated per member; what the banking system holds
      // of each line is stated; everything else on this sheet is derived from those two and from the
      // rule each bank runs its own book to.
      let systemPaper = 0;
      let centralBankAssets = 0;
      const paperIn = new Map<string, number>();

      for (const line of seedLineRows) {
        const id = instrumentId(line.id);
        const price = openingOf(opening, line.id);
        const par = priced(ctx, id, price);
        paperIn.set(line.id, line.banks * price);
        systemPaper = add(systemPaper, line.banks * price, 'what the banking system holds');
        // Seed E2, XI-15: every member of every cell holds the same stated amount, and the cell
        // carries it with its weight.
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
        const share = ctx.params.ratio(P.cbOpeningShare);
        const cbUnits = div(mul(others, share, 'the share it targets'), 1 - share, 'its holding');
        const cbDrawn = held(ctx, id, cbUnits);
        if (cbDrawn > 0) {
          ctx.endowUnits(CB, id, cbDrawn, par);
          // Law 19: what it holds, not what the division asked for — the whole pieces it was actually
          // endowed with, read back in the units this price is quoted in.
          centralBankAssets = add(
            centralBankAssets,
            mul(inNamedUnits(ctx, id, cbDrawn), price, 'central bank assets'),
            'its assets',
          );
        }
      }

      const crossShare = ctx.params.ratio(P.crossHoldingShare);
      const abroadShare = div(
        crossShare,
        ABROAD.length,
        'the part of it that is any ONE country\u2019s',
      );
      // Central Bank F4, Currency D2: AND ITS RESERVES, which are the other countries' paper. It is
      // on this side of the sheet with the domestic paper and for the same reason: it is an asset the
      // central bank bought with money it issued, so it is part of what decides how big its balance
      // sheet is. Holding it anywhere else was the first version of this seed and it is what took the
      // banking system's liquidity abroad (worklist 11.5): a foreign bond raises nothing at a
      // window that is its own system's, so a commercial bank holding one holds an illiquid asset.
      for (const c of ABROAD) {
        const line = instrumentId(String(abroadLine.get(String(c.region))));
        const price = openingOf(abroadPrice, String(c.region));
        // What it holds of one country's paper is the same share of the system's paper that country's
        // own central bank holds of America's: the arrangement is symmetric, because nothing in this
        // world says which of the four is the reserve currency (XI-12).
        const units = div(
          mul(systemPaper, abroadShare, `what it holds of ${c.name}`),
          price,
          'units',
        );
        if (units <= 0) continue;
        const drawn = held(ctx, line, units);
        if (drawn <= 0) continue;
        ctx.endowUnits(CB, line, drawn, priced(ctx, line, price));
        // Law 8: WHAT IT ACTUALLY HOLDS, in the units the price is quoted in. This read multiplied a
        // count of PIECES by a price per NAMED unit, so the reserves the seed thought it had bought
        // were the subdivision of a bond times too big — eight per cent of the system's paper became
        // eight times it, the money issued against it went with it, and every one of those numbers
        // moved again when the pieces were made finer. One unit on both sides, or it is not a value.
        centralBankAssets = add(
          centralBankAssets,
          mul(inNamedUnits(ctx, line, drawn), price, 'its reserves abroad'),
          'central bank assets',
        );
      }

      // Money A1, Central Bank A2: NO CENTRAL-BANK MONEY EXISTS THAT ITS ISSUER BOUGHT NOTHING WITH.
      // Its money is its liability and the paper above is the asset it bought with it, so THE SIZE OF
      // ITS BALANCE SHEET IS ALREADY DECIDED: what is left to say is who holds that money.
      //
      // Treasury D4.b: it opens with a buffer, because the alternative to one is dependence on every
      // single auction clearing — and the buffer is CENTRAL-BANK MONEY, so what is stated about it is
      // ITS SHARE of that balance sheet. The banks hold the rest as reserves.
      const buffer = mul(
        centralBankAssets,
        ctx.params.ratio(P.treasuryBufferShare),
        "the treasury's buffer",
      );
      ctx.endowMoney(TREASURY_US, USD, cash(ctx, buffer));
      const reserves = sub(centralBankAssets, buffer, 'what the banks hold in reserve');
      // Seed C1, Banks Capital B1.b: A BANK'S BALANCE SHEET FOLLOWS ITS DEPOSITORS, and this is the
      // line of causality the whole sheet turns on.
      //
      // It used to run the other way: the system's paper was split between banks BY THEIR STATED
      // SIZE, a bank's assets were whatever that came to, and its depositors took the residue. That
      // works for three banks and stops working for twenty — a firm's account is a stated amount and
      // a bank's share of the paper shrinks as 1/count, so past a certain number some bank is handed
      // a depositor bigger than the whole book its capital rule lets it fund, and the world refuses
      // to open. The count of banks was silently load-bearing, which is what makes it worth a
      // derivation rather than a table (Seed B1).
      //
      // So: what each bank must FUND is its firms' accounts plus its households' share of what is
      // left, and its assets are what its own leverage line makes of that. One unknown — what a unit
      // of bank size funds in household money — and it is solved rather than chosen:
      //
      //   Σ (firms_i + size_i·H) / (1 − line_i) = assets to go round
      //
      // Nothing here is fitted and nothing is capped: a bank with a large depositor is a large bank
      // because of it, which is what a deposit IS.
      const all = systemPaper + reserves;
      const atBank = new Map<PartyId, number>(banks.map((b) => [b.id, 0]));
      for (const f of madeHere) {
        const bank = ctx.parties.get(partyId(f.firm)).bank;
        atBank.set(bank, add(zeroIfNone(atBank.get(bank)), cashOf(f), 'what its firms hold at it'));
      }
      const lineOfBank = new Map<PartyId, number>(
        bankRows.map((r) => [
          partyId(r.bank),
          add(ctx.params.ratio(P.leverageRatio), r.capitalBuffer, 'the line this bank runs to'),
        ]),
      );
      const over = (b: { id: PartyId }): number => {
        const left = 1 - zeroIfNone(lineOfBank.get(b.id));
        // Seed D1, Banks Capital B1.b: a bank that must fund EVERY asset out of its own capital can
        // hold no deposit at all, and this world opens its firms with accounts at it. There is no
        // balance sheet that satisfies both, so the seed says which rule made it impossible rather
        // than dividing by it and handing the world a negative amount of assets to go round.
        forbid(
          left > 0,
          'Seed D1',
          `${b.id} must fund ${zeroIfNone(lineOfBank.get(b.id))} of every asset out of its own capital, so it can take no deposit — and this world opens depositors at it`,
          { bank: b.id, line: zeroIfNone(lineOfBank.get(b.id)) },
        );
        return left;
      };
      const firmsPart = sum(
        banks.map((b) => div(zeroIfNone(atBank.get(b.id)), over(b), 'its firms')),
      ).value;
      const sizePart = sum(banks.map((b) => div(b.size, over(b), 'its households'))).value;
      forbid(
        sizePart > 0 && all > firmsPart,
        'Seed D1',
        `this world's banks cannot carry the accounts it opens them with: ${firmsPart} of assets are needed for the firms alone and there are ${all}`,
        { all, firmsPart },
      );
      const perSize = div(
        sub(all, firmsPart, 'what is left for the households'),
        sizePart,
        'per unit of size',
      );
      const assetsOf = (b: { id: PartyId; size: number }): number =>
        div(
          add(
            zeroIfNone(atBank.get(b.id)),
            mul(b.size, perSize, 'its households'),
            'what it funds',
          ),
          over(b),
          'its assets',
        );
      const assets = new Map<PartyId, number>(banks.map((b) => [b.id, assetsOf(b)]));

      // Its assets are paper and reserves in the proportion the system holds them, because at the
      // opening nothing has yet decided otherwise — the treasury's own liquidity plan does that from
      // period one (Banks Funding C1).
      const paperShare = div(systemPaper, all, 'the part of a book that is paper');
      for (const b of banks) {
        const mine = zeroIfNone(assets.get(b.id));
        ctx.endowMoney(
          b.id,
          USD,
          cash(ctx, sub(mine, mul(mine, paperShare, 'its paper'), 'its reserves')),
        );
      }
      for (const line of seedLineRows) {
        const id = instrumentId(line.id);
        const price = openingOf(opening, line.id);
        const par = priced(ctx, id, price);
        // Law 8: whole units, split so they sum to exactly what the system holds of the line — the
        // odd unit goes to the largest remainder and has a named holder (core/tick.ts).
        const perBank = splitOnTick(
          line.banks,
          banks.map((b) => mul(zeroIfNone(assets.get(b.id)), paperShare, 'its paper')),
        );
        banks.forEach((b, at) => {
          const units = zeroIfNone(perBank[at]);
          if (units <= 0) return;
          ctx.endowUnits(b.id, id, held(ctx, id, units), par);
        });
      }

      for (const f of madeHere) ctx.endowMoney(partyId(f.firm), USD, cash(ctx, cashOf(f)));

      // ------------------------------------------------------------------------------------------
      // THE CROSS HOLDINGS (Currency D2, Spot FX B1, B2; Central Bank F4)
      //
      // EVERY CROSS HOLDING IN THIS WORLD IS A CENTRAL BANK'S, and that is not a convenience — it is
      // the only holder for whom foreign paper is what it is FOR (Central Bank F4: reserves ARE a
      // claim on another country's issuer). Each country's central bank holds the others' paper, and
      // that one fact gives the currency layer everything it needs: a position to revalue every
      // period (Currency D2, and for a central bank it goes to its revaluation account, A2.c), a
      // COUPON that arrives in a money its receiver does not book in (C4), and parties on both sides
      // of a pair holding a money they have no use for (Spot FX B1, B2; XI-13).
      //
      // IT IS NOT THE COMMERCIAL BANKS', and the first version of this seed had it be. A bank's
      // liquid assets are its reserves plus what its own unencumbered paper would raise AT ITS OWN
      // CENTRAL BANK'S WINDOW (Money Market C1, C1.a) — and a foreign government's bond raises
      // nothing there, because the window is its own system's (Currency D4). So eight per cent of the
      // banking system's paper moved abroad took eight per cent of its liquidity with it, every bank
      // in the world went to negative funding room, `publishQuotes` stopped quoting anybody, and
      // lending, investment and the whole real chain stopped with it — 121 credit quotes became 20.
      // That is not a finding about banks; it is a seed that put a position where its own liquidity
      // rule says it cannot be. What a COMMERCIAL bank holds abroad is a decision it takes with its
      // own capital once there is a reason to (13h), and it is not the seed's to state.
      // Central Bank F4: and each of their own reserves, which are a claim on the American issuer. It
      // is the benchmark line, because that is the one a reserve manager holds.
      const reserveLine = instrumentId(
        seedLineRows[seedLineRows.length - 1]?.id ?? String(GOV_LINE),
      );
      const reservePrice = openingOf(opening, String(reserveLine));
      for (const c of ABROAD) {
        const reserveUnits = div(
          mul(systemPaper, abroadShare, `what ${c.name} holds of it`),
          reservePrice,
          `units ${c.name} holds`,
        );
        if (reserveUnits <= 0) continue;
        ctx.endowUnits(
          c.centralBank,
          reserveLine,
          held(ctx, reserveLine, reserveUnits),
          priced(ctx, reserveLine, reservePrice),
        );
        // Treasury D4.b: and each treasury opens with a buffer of its own money, because a treasury
        // that depends on every auction clearing has no way to pay a coupon in week one.
        ctx.endowMoney(
          c.treasury,
          c.ccy,
          ctx.registry.payable(c.ccy, mul(reserveUnits, reservePrice, 'what it holds abroad')),
        );
      }

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
      // runs when every module has handed out what it hands out (item 12's finding 12-1,
      // docs/RECORD.md).

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
          price: priced(
            ctx,
            goodId(row.subUnit, REGION),
            ctx.params.price(openingPrice(row.subUnit)),
          ),
          ccy: USD,
          provenance: { kind: 'opening' },
        });
      }
      for (const row of madeHere) {
        // A firm whose good this world does not make opens with nothing, because there is nothing
        // for it to hold: the seed endows what exists and never brings an instrument into being to
        // have something to endow (Seed A1). `madeHere` is exactly those that do.
        const firm = partyId(row.firm);
        const price = ctx.params.price(openingPrice(row.subUnit));
        const starts = startsOf(row);
        // Seed D1: ONE PERIOD of what it makes, finished and ready to sell; what a batch still in
        // flight comes to, which is a period of starts for every period its recipe keeps it (B3); and
        // what one period of starting draws of each of its inputs. Never a hoard: a firm sitting on a
        // year of stock would produce nothing for a year and the seed would have decided that.
        const finished = mul(starts, recipeOf(row.subUnit).yieldRate, 'what arrives in a period');
        const onTheLine = mul(
          starts,
          recipeOf(row.subUnit).leadTimePeriods,
          'what is still in flight',
        );
        // Seed C4: what it cost whoever holds it is the seed's, and it is below what the market
        // opens at — a firm holding stock it could only sell at a loss would never have made it.
        const good = goodId(row.subUnit, REGION);
        const basis = priced(ctx, good, price * SEED_STOCK_BASIS);
        if (finished > 0) ctx.endowUnits(firm, good, held(ctx, good, finished), basis);
        if (onTheLine > 0) {
          const wip = wipId(row.subUnit, REGION);
          ctx.endowUnits(
            firm,
            wip,
            held(ctx, wip, onTheLine),
            priced(ctx, wip, price * SEED_STOCK_BASIS),
          );
        }
        // Seed D1: what its recipe draws, so its first batch is not waiting on a market session.
        // Law 19: WHAT it draws is read from the good's own terms, never listed a second time here.
        for (const input of recipeOf(row.subUnit).inputs) {
          if (!ctx.instruments.has(goodId(input.subUnit, REGION))) continue;
          const line = goodId(input.subUnit, REGION);
          const paid = ctx.params.price(openingPrice(input.subUnit)) * SEED_STOCK_BASIS;
          const drawn = mul(starts, input.qtyPerUnit, 'what a period of starting draws');
          if (drawn <= 0) continue;
          ctx.endowUnits(firm, line, held(ctx, line, drawn), priced(ctx, line, paid));
        }
        // Capital Programme A2, A6, Seed D1: the plant its line runs on, spread over three vintages
        // of different ages, each carried at what is left of what a new one costs. WHICH kind of
        // plant is read from the good's own recipe (Law 19), and how many units it needs to make what
        // it makes is that recipe's number too — the seed states only how much headroom it opens with.
        for (const need of recipeOf(row.subUnit).plant) {
          const kind = CAPITAL_KINDS.find((k) => k.id === need.capitalKind);
          // A world assembled without the capital programme has no plant to endow, exactly as a
          // world that does not make a good has no stock of it to endow (Seed A1).
          if (kind === undefined) continue;
          if (!ctx.registry.instrumentKinds.has(plantKindId(kind.id))) continue;
          // Law 8: a machine is a whole machine. What a share of a line's plant comes to is a
          // fraction of one, and what the firm HOLDS is the machines that fraction reaches.
          const mine = downTick(mul(plantOf(row.subUnit, kind.id), shareOf(row), 'its own plant'));
          if (mine <= 0) continue;
          const newPrice = ctx.params.price(openingPrice(kind.madeFrom));
          const life = ctx.params.periods(paramId(`plant.usefulLife.${kind.id}`));
          // Law 8: whole machines, and the odd one has a named vintage rather than being lost to a
          // division that does not come out (core/tick.ts).
          const perVintage = splitOnTick(
            mine,
            SEED_PLANT_AGES.map(() => 1),
          );
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
        // Commodities Spot A3, Seed C4 (13c): NOBODY OPENS HOLDING GRAIN WITHOUT A BARN. The space
        // a holder opens with is the space what it opens holding takes up — an opening CONDITION,
        // read off the good's own terms (Law 19) rather than a number stated here. It is exactly
        // what it holds and no headroom: whether a line that produces more than its barn keeps can
        // rent one, build one or must sell is a MARKET question from the first period (D3), and
        // headroom would be the seed answering it.
        // A3: room for EVERYTHING it opens holding that needs room — what it made and what its
        // recipe drew. A mill holds grain it did not grow, and it keeps that under cover too.
        const space = [
          spaceOf(ctx, row.subUnit, held(ctx, goodId(row.subUnit, REGION), finished)),
          ...recipeOf(row.subUnit).inputs.map((input) => {
            const line = goodId(input.subUnit, REGION);
            if (!ctx.instruments.has(line)) return 0;
            const drawn = mul(starts, input.qtyPerUnit, 'what a period of starting draws');
            return spaceOf(ctx, input.subUnit, held(ctx, line, drawn));
          }),
        ].reduce((a, b) => a + b, 0);
        if (space > 0 && ctx.registry.instrumentKinds.has(plantKindId(STORAGE))) {
          const serviceDate = addDays(ctx.calendar.epoch, -ctx.calendar.periodDays);
          const id = seedVintage(ctx, STORAGE_KIND, REGION, serviceDate);
          const newPrice = ctx.params.price(openingPrice(STORAGE_KIND.madeFrom));
          const life = ctx.params.periods(paramId(`plant.usefulLife.${STORAGE}`));
          ctx.endowUnits(
            ctx.parties.get(firm).id,
            id,
            space,
            priced(ctx, id, (newPrice * (life - 1)) / life),
          );
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
 * funded (item 12's finding 12-1, docs/RECORD.md).
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
export function foundationFundingFor(bankRows: readonly BankDecl[]): SystemModule {
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
      const minimum = ctx.params.ratio(P.leverageRatio);
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
        const own = moneyInstrumentId(bank, USD);
        let assets = 0;
        for (const h of ctx.register.holdingsOf(bank)) {
          if (h.instrument === own) continue;
          assets = add(
            assets,
            ctx.valuation.valueOfLots(h.instrument, h.lots, ctx.period),
            "the bank's opening assets",
          );
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
          already = add(
            already,
            mul(held, weightOf(ctx.parties.get(holder)), 'in total'),
            'deposits',
          );
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
        for (const cell of cells) ctx.endowMoney(cell.id, USD, perMember);
      }
    },
  };
}

/**
 * Law 8: THE SEED SPEAKS IN NAMED UNITS AND THE STATE HOLDS PIECES, and this is the one boundary
 * between them. A number here is what a person would say — 100,000 USD, 45 tonnes, 400 USD the
 * tonne — and what goes into the register is the count of indivisible pieces that comes to: cents,
 * grams, whole machines. Nothing downstream converts anything, because everything downstream is
 * already a count.
 */
function cash(ctx: SeedContext, phx: number): number {
  return ctx.registry.pieces(currencyUnit(USD), phx);
}

/** The same for units of an instrument, in whatever its own unit is named in. */
function held(ctx: SeedContext, instrument: InstrumentId, amount: number): number {
  return ctx.registry.pieces(ctx.instruments.get(instrument).unit, amount);
}

/**
 * Law 8: and back the other way — a count of pieces read in the NAMED units a price is quoted in.
 * A price is money for one named unit, so a value is a named count times a price and never a piece
 * count times one: the two differ by the subdivision, which is a RESOLUTION and must not reach a
 * number anybody acts on (Law 2).
 */
function inNamedUnits(ctx: SeedContext, instrument: InstrumentId, pieces: number): number {
  return div(
    pieces,
    ctx.registry.subdivision(ctx.instruments.get(instrument).unit),
    'in named units',
  );
}

/** And for a price: money for one NAMED unit becomes money pieces for one piece. */
function priced(ctx: SeedContext, instrument: InstrumentId, perNamedUnit: number): number {
  const i = ctx.instruments.get(instrument);
  // Law 8, Seed C4: AN OPENING LEVEL IS A PRICE AND SITS ON THE SAME GRID AS ONE. Nobody posted it
  // and nobody promised anything at it, so there is no side to take a direction from and the
  // nearest tick is the honest answer — but a stated level off the grid would be a level this
  // market could never print again, which is the whole defect 12b.1 exists to remove.
  return ctx.registry.onQuoteGrid(i.kind, USD, ctx.registry.priceOf(USD, i.unit, perNamedUnit));
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
/**
 * Seed B1, B1.a, B4, Law 4: WHAT A WORLD IS MADE OF, drawn once from its own seed value.
 *
 * Every module that needs to know which banks, firms, listings or funds this world has reads it
 * from here, and so does anybody asking the question from outside — a test, the observer, a
 * measurement of the count. One draw, one answer: a second one somewhere else would be a second
 * world wearing this one's name, and the first thing to go wrong would be a firm that exists in
 * one of them and not the other.
 */
export interface FoundationDraw {
  readonly banks: readonly BankDecl[];
  readonly firms: readonly FirmDecl[];
  readonly listed: readonly ListedDecl[];
  readonly funds: readonly FundDecl[];
  readonly etfs: readonly EtfDecl[];
}

export function foundationDraw(
  seed: string,
  bankRows: readonly BankDecl[] = drawBanks(BANK_COUNT, seed),
  firmRows: readonly FirmDecl[] = drawFirms(FIRM_COUNT, seed),
): FoundationDraw {
  const names = bankRows.map((b) => b.bank);
  const listed = drawListed(firmRows, bankRows, seed);
  return {
    banks: bankRows,
    firms: firmRows,
    listed,
    funds: drawFunds(bankRows, seed),
    // Indices C2: the tracker tracks THIS world's equity index, named by the one module that
    // declares it. The seed is where the two meet, because it is the only place that may know both.
    etfs: drawEtfs(
      listed.map((r) => String(equityLineOf(r.firm))),
      names,
      seed,
      /**
       * M9, Indices C2, C2.a, Fund Shares E3.a: A VEHICLE ON EVERY INDEX A TRACKER SHOULD FOLLOW,
       * and only the first of them is the SEED's to launch.
       *
       * A tracker on a size segment holds whatever that segment's rule says is in it, and that rule
       * reads the constituents' own prints (A3) — which at period zero do not exist. So the seed
       * declares the vehicles and launches one: the broad line, whose rule answers from the moment
       * the lines are listed. The rest are launched by `funds.etf`'s own phase the period their own
       * index first HAS a level, in kind, out of what the participants actually hold — a transfer,
       * which is why two vehicles cannot own the same float (`12d-4`, where `etf.us` and
       * `etf.equity.large.us` both opened with negative equity because both were endowed with it).
       *
       * C2's simultaneity needs more than one of them: with a single vehicle on a single line,
       * "every tracker rebalances at once" is a market of one, and a firm crossing a size boundary
       * is a rebalance nobody has to trade.
       */
      [
        EQUITY_INDEX(REGION),
        SIZE_INDEX(REGION, 'large'),
        SIZE_INDEX(REGION, 'small'),
        GLOBAL_INDEX(USD),
      ],
    ),
  };
}

export function foundationSpec(
  seed: string,
  /**
   * Seed B1, B4: how many banks this world has, and what each of them is like — DRAWN from the
   * stated spread and from this world's own seed value, so a world of two hundred banks is one
   * number and not two thousand two hundred (Audit D3: the same seed gives the same banks).
   */
  bankRows: readonly BankDecl[] = drawBanks(BANK_COUNT, seed),
  /** Seed B1.a, B4: and the firms, the same way and for the same reason. */
  firmRows: readonly FirmDecl[] = drawFirms(FIRM_COUNT, seed),
): AssemblySpec {
  // Seed B1.a: WHAT THIS WORLD IS MADE OF — one draw, reaching every module that needs it.
  const drew = foundationDraw(seed, bankRows, firmRows);
  return {
    seed,
    epoch: civil(2026, 1, 5),
    registry: {
      currencies: [
        { code: USD, name: 'US dollar', centralBank: CB, quoteTick: PIP },
        ...ABROAD.map((c) => ({
          code: c.ccy,
          name: c.ccyName,
          centralBank: c.centralBank,
          quoteTick: c.quoteTick,
        })),
      ],
      regions: [
        { id: REGION, name: 'United States', ccy: USD },
        ...ABROAD.map((c) => ({ id: c.region, name: c.name, ccy: c.ccy })),
      ],
      units: [
        // Money A2, Law 8: a USD is a hundred cents, like any real money, and the cent is the
        // smallest amount of it that exists. Every balance in this world is a whole number of them,
        // so nothing below a cent can be paid, lent, owed or left over anywhere.
        { id: currencyUnit(USD), name: 'USD', perUnit: MONEY_PIECES },
        // Currency A3, Law 8: each other money is divided into its own smallest piece, and each is
        // its OWN unit. Two currencies are never added (Money A2.b) and this is where that starts:
        // an amount of euros is counted in euro pieces, and what it comes to in dollars is a
        // conversion at a rate somebody traded at (Currency C5) and never an addition.
        ...ABROAD.map((c) => ({
          id: currencyUnit(c.ccy),
          name: String(c.ccy),
          perUnit: MONEY_PIECES,
        })),
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
        dimension: 'days',
        kind: 'resolution',
        owner: 'model',
        why: 'A period is a week (docs/ARCHITECTURE.md 4.7); coarser cannot place a weekly cycle, finer buys nothing yet.',
      },
      {
        id: KERNEL_PARAMS.cyclesPerPeriod,
        value: 5,
        unit: 'cycles',
        dimension: 'periods',
        kind: 'resolution',
        owner: 'model',
        why: 'Money G1: a period holds more than one settlement cycle; five stands for business days.',
      },
      {
        id: KERNEL_PARAMS.worstInstances,
        value: 5,
        unit: 'count',
        dimension: 'count',
        kind: 'resolution',
        owner: 'model',
        why: 'Audit D2: how many worst instances a family reports; a reporting depth, not a behaviour.',
      },
      {
        id: KERNEL_PARAMS.pieceShift,
        value: 1,
        unit: "multiple of every unit's declared subdivision",
        dimension: 'count',
        kind: 'resolution',
        owner: 'model',
        why: 'Law 8, Law 2: how many times finer than declared every unit is divided — 10 makes the piece a tenth of a cent, a tenth of a gram and a tenth of a share. Each unit states its own subdivision; this moves them all together, which is what makes the subdivision a RESOLUTION that can be TESTED: declare the same world in finer pieces and every structural invariant must hold exactly and the path must not move.',
      },
      {
        id: KERNEL_PARAMS.tickShift,
        value: 1,
        unit: "divisor of every kind's declared price tick",
        dimension: 'count',
        kind: 'technology',
        owner: 'model',
        why: "Law 8, Law 2: how many times finer than declared every QUOTED PRICE moves — 10 makes the tick a tenth of a cent a share, a tenth of a basis point of a bond's face and a tenth of a pip. Each kind states its own increment (registry/grid.ts) and this moves them all together. It is a TECHNOLOGY and not a resolution, which is what measuring it said: a coarser tick pulls every bid down and every ask up until books that used to cross no longer do, so it changes what trades and this world's money stock moves three per cent between one grid and another without converging. That is what a tick does in a real venue. What running the world at a finer one tests is that every STRUCTURAL invariant holds exactly, never that the path is unchanged.",
      },
    ],
    modules: [
      // The physical world, first and depending on nothing (13c): it is not an economic outcome
      // and does not wait for one. Every region this world has stands in weather of its own, and
      // the producer, the carrier, the insurer and the household all read the same fact.
      environment([REGION, ...ABROAD.map((c) => c.region)], seed),
      // The order matters at one anchor: three phases sit before the revaluation, and they must run
      // in this order — a drawing becomes a loan row, then anything that cannot pay dies, then the
      // people it employed are released. Assembly keeps declaration order for modules that do not
      // require each other, and that is what puts them in it.
      expectations,
      creditEvents,
      // Dealer Desks A3: the desks, and WHICH LINES EACH OF THEM MAKES — drawn with the listing
      // (`ListedDecl.makers`) and handed in here, because the bank that quotes and the listing that
      // drew its makers are two systems and one fact (Law 4).
      banks(drew.banks, (instrument) => {
        const row = drew.listed.find((l) => equityLineOf(l.firm) === instrument);
        return row?.makers;
      }),
      estate,
      // Goods A1: the goods of THIS world are made in the one region that has firms in it. The
      // three abroad are a central bank, a treasury and a bond line (13i builds their economies),
      // so opening grain markets there would be three books nobody is ever on either side of.
      goods(GOODS, [REGION]),
      // Capital Programme: the kind of thing plant is, and the schedule it wears out on. Before the
      // firms, because a firm decides what to make against the plant it holds (A2) and what to
      // invest against what a machine costs (B1) — and a kind has to be registered to be held.
      // Commodities Spot A3, A4 (13c): covered space is the second kind of capital this world has,
      // and it is what makes holding a thing cost something. The capital programme owns what a kind
      // of plant IS; the commodities module owns the market in the space it provides.
      capitalProgramme([...CAPITAL_KINDS, STORAGE_KIND]),
      labour(),
      firms(drew.firms),
      households(),
      // Commodities Spot A3, D3: the market in covered space. After the firms, because who is short
      // of room and who has spare is read off what they hold (Law 19).
      commodities(),
      // Equity and the desks before the funds: this world's exchange-traded fund holds the listed
      // firms and is launched by the desks that make its market, and both have to exist before a
      // basket can be put in (the funds module reads that off its own data, in `needs`).
      equity(drew.listed),
      funds(drew.funds, drew.etfs),
      sovereignInstruments,
      // Sovereign D3.a, Currency A3: EVERY SOVEREIGN THAT BORROWS HAS A CURVE, and it is its own —
      // one issuer, one money, its own prints. A world whose foreign lines had no curve family
      // would have paper anybody may hold and nobody may value at a yield (D4.a throws where it is
      // asked), which is the same line being a bond here and not one there. One module, because
      // the CONVENTION is one thing and four modules would be four places to write it down.
      sovereignCurve([
        { issuer: TREASURY_US, ccy: USD },
        ...ABROAD.map((c) => ({ issuer: c.treasury, ccy: c.ccy })),
      ]),
      treasury,
      centralBankOmo,
      // The money market after the treasury and the curve: a bank funds itself against the paper
      // those two put into the world, and it prices a name off what the lending module published
      // about it. Both reach it as public events and prints, never as imports (Law 15).
      moneyMarket,
      // Currency, Spot FX: the pairs, after the banks whose desks quote them and the money market
      // whose overnight book they fund a position in.
      // Spot FX D1, D3, C2.a: the desks draw their OWN numbers, from this world's own seed value
      // (`13b.1`). The banks module no longer carries them and this one no longer reads `BankDecl`:
      // the cycle between the two is gone, and what crosses is a bank's NAME, which is public.
      spotFx(drawFxDesks(drew.banks.map((b) => b.bank), seed)),
      // The derivative layer: after the money market, because a margin call is met out of cash a
      // member funds there, and after the estate, because a default resolves into one (XI-8). It
      // brings no class of contract with it (13b does that): what it brings is the house, the
      // margin, the fund and the waterfall every class then runs on.
      /**
       * XI-3, Clearing B2: the layer, and WHO TRADES CONTRACTS in this world.
       *
       * Its banks and its firms. NOT its funds, and the reason is Fund Shares A3: a fund's equity
       * is zero by construction because its own claim on itself absorbs whatever its book comes
       * to — and the pass that re-marks that claim reads the register, where a contract is not
       * (Derivative X1). A tracker that took a derivative position would carry a mark its own
       * share value had never been told about, which is a fund with equity: measured at 83,247,864
       * on `etf.us` the first time funds were let in here.
       *
       * 13h is where a fund holds derivatives on purpose — and where the one pass that re-marks a
       * fund's claim on itself is next opened (`docs/BUGS.md`, finding `13b-2`).
       */
      derivativeLayer([...TRADES_CONTRACTS]),
      // CDS: the first class on the layer (13b). After it, because a book clears through the house
      // it opened; before the indices, because a default index is an index OF these books.
      cds(houseIdFor),
      // Swaps: after the money market, whose overnight book is what a floating leg fixes on
      // (IRS A1.a, E3), and after the indices publish that fixing.
      irs(houseIdFor),
      // FX derivatives: after the pairs, because the forward settles against the spot print, and
      // after the money market, because the carry a bank quotes is what the two moneys cost it.
      fxDerivatives(houseIdFor),
      // Indices: after everything that prints, because an index is what its constituents printed
      // and the benchmark is what the overnight book settled at (Indices D3.a, E1).
      indices([REGION, ...ABROAD.map((c) => c.region)], [USD, ...ABROAD.map((c) => c.ccy)]),
      // Index futures: after the indices, because what this settles against is an index READ
      // (Indices C3), and it is what a dealer's hedge actually is (Dealer Desks E1, E2).
      // Options: after the equity book clears, because D3.a forbids an underlying that exists only
      // inside the derivative — the ladder is written on the listed lines this world already
      // prints. What it gives the world is a price for optionality, which three other systems need
      // and none of them can form (Bond N11.a, Short-Term Debt B4, §46 A3).
      options(houseIdFor, (ctx) =>
        ctx.instruments
          .all()
          // Law 15: WHAT IT IS COUNTED IN, which is a registry row, rather than which kind it is.
          // An option ladder belongs on the lines a person owns a count of.
          .filter((i) => i.status.live && i.unit === SHARES)
          .map((i) => i.id),
      ),
      // Bond futures: Sovereign I1–I3.a, owned here because a specified clause no item names is a
      // hole in the plan (Appendix C). After the curve and the money market, because the carry it
      // is measured against is a coupon and a financing rate both of them already print.
      bondFutures(houseIdFor, TREASURY_US),
      indexFutures(houseIdFor, [
        { id: EQUITY_INDEX(REGION), ccy: USD },
        ...ABROAD.map((c) => ({ id: EQUITY_INDEX(c.region), ccy: c.ccy })),
      ]),
      // Ratings: after everything it has an opinion about, and it reads none of them — it decides
      // from state through a view with the prices closed (Ratings A2.a).
      ratings(
        drawAssessors(
          ASSESSOR_COUNT,
          drew.banks.map((b) => b.bank),
          seed,
        ),
      ),
      // Reporting: after `equity`, because what makes a company public is a share line somebody
      // outside holds, and after everything that moves a company's equity account, because what it
      // publishes is what those moves came to (Reporting A1.a, A2).
      reporting(seed),
      // Research: after `reporting`, because what a bank estimates is the report a company will
      // publish and what settles its estimate is the one it just did (Reporting C1, F1).
      research(seed),
      foundationSeedFor(drew.banks, drew.firms),
      foundationFundingFor(drew.banks),
    ],
  };
}

/** Build the foundation world. Reproducible from the seed value (Seed A5). */
export function foundationWorld(seed: string): World {
  return assemble(foundationSpec(seed));
}

export { PAR };

/**
 * Commodities Spot A3: HOW MUCH COVERED SPACE A HOLDING TAKES UP, computed by the SAME function the
 * commodities module checks with (Law 4). Two spellings of one conversion would differ by whatever
 * their rounding differed by, and a holder short by one unit of space through rounding alone would
 * lose real tonnes at the close for nothing that happened.
 */
function spaceOf(ctx: SeedContext, subUnit: string, pieces: number): number {
  const id = goodId(subUnit, REGION);
  if (pieces <= 0 || !ctx.instruments.has(id)) return 0;
  const instrument = ctx.instruments.get(id);
  const good = instrument.terms;
  if (!isGoodTerms(good) || good.storagePerUnit === null) return 0;
  return spaceFor(ctx, instrument.unit, pieces, ctx.params.ratio(good.storagePerUnit));
}
