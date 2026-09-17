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
import { noCash } from '../core/measure.js';
import { asQty, downTick, downToNamed, toTickOf, upTick } from '../core/tick.js';
import { prng } from '../rng/prng.js';
import type { RegionDecl } from '../registry/registry.js';
import { drawMap, type MapSpec } from './map.js';
import {
  ARABLE,
  BANDS,
  RESOURCES,
  SYLLABLES,
  TERRAINS,
  WATER,
  WORLD,
  mapParams,
} from './map-data.js';
import {
  type GeographyDecl,
  type RatioReads,
  type TerrainReads,
  areaKm2,
  groundIn,
  heartOf,
  resourceId,
  terrainId,
} from '../registry/geography.js';
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
  countryId,
  type CountryId,
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
import type { Qty } from '../core/tick.js';
import {
  acrossMembers,
  amountOf,
  asCash,
  asPerMember,
  asAmount,
  asNamed,
  asPerNamedUnit,
  asPerPiece,
  asRatio,
  asStated,
  type Cash,
  heldAsMoney,
  minus,
  type Named,
  over,
  type PerNamedUnit,
  type PerPiece,
  plus,
  ratioOf,
  type Ratio,
  scale,
  type Stated,
  valueAt,
} from '../core/measure.js';
import { add, addTo, div, positiveCount, sum, zeroIfNone } from '../core/num.js';
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
} from '../mechanisms/capital-programme/index.js';
import { SEED_PLANT_AGES, seedVintage } from '../registry/physical.js';
import { centralBankOmo } from '../mechanisms/central-bank-omo/index.js';
import { moneyMarket } from '../mechanisms/money-market/index.js';
import { estate } from '../mechanisms/estate/index.js';
import { creditEvents } from '../mechanisms/credit-events/index.js';
import { commodities, STORAGE_KIND } from '../mechanisms/commodities/index.js';
import { land } from '../mechanisms/land/index.js';
import {
  LANDLORD,
  LEASE_ROW,
  PREMISES,
  PROPERTY_PARAMS,
  landlordIdFor,
  property,
} from '../mechanisms/property/index.js';
import {
  isPlant,
  plantTerms,
  plantUnitId,
  type LeaseTerms,
  type CapitalKindDecl,
} from '../registry/physical.js';
import { drawMerchants, merchants } from '../mechanisms/merchants/index.js';
import { tradeCredit } from '../mechanisms/trade-credit/index.js';
import { securitisation } from '../mechanisms/securitisation/index.js';
import { shortTermDebt } from '../mechanisms/short-term-debt/index.js';
import { securitiesLending } from '../mechanisms/securities-lending/index.js';
import { corporateBondModule } from '../mechanisms/corporate-bond/index.js';
import { control } from '../mechanisms/control/index.js';
import {
  INSURANCE,
  PENSION,
  insurerIdFor,
  insurers,
  pensionFundIdFor,
} from '../mechanisms/insurers/index.js';
import { external } from '../mechanisms/external/index.js';
import { commodityFutures } from '../mechanisms/commodity-futures/index.js';
import { supply } from '../mechanisms/supply/index.js';
import { polity } from '../mechanisms/polity/index.js';
import { housing } from '../mechanisms/housing/index.js';
import { CONSUMPTION } from '../mechanisms/households/data.js';
import { PROBATE, probateId } from '../mechanisms/households/lifecycle.js';
import {
  drawCarriers,
  freight,
  VESSEL,
  VESSEL_KIND,
  type CarrierDecl,
} from '../mechanisms/freight/index.js';
import { environment } from '../mechanisms/environment/index.js';
import { expectations } from '../mechanisms/expectations/index.js';
import { firms } from '../mechanisms/firms/index.js';
import { FIRM_COUNT, drawFirms, type FirmDecl } from '../mechanisms/firms/data.js';
import { goodId, goodMarketId, goods, isGoodTerms, wipId } from '../mechanisms/goods/index.js';
import { lifeParam, spaceFor, STORAGE } from '../registry/physical.js';
import { GOODS, type GoodDecl } from '../mechanisms/goods/data.js';
import { equity } from '../mechanisms/equity/index.js';
import { drawEquity, equityLineOf, type EquityDecl } from '../mechanisms/equity/data.js';
import {
  drawSmallBusiness,
  SMALL_PER_NAMED,
  type SmallFirmDecl,
} from '../mechanisms/small-business/data.js';
import { smallBusiness } from '../mechanisms/small-business/index.js';
import { FUND, funds } from '../mechanisms/funds/index.js';
import {
  drawTrackers,
  drawFunds,
  drawManagers,
  drawPrivateEquity,
  drawStrategies,
  type FundDecl,
  type ManagerDecl,
} from '../mechanisms/funds/data.js';
import { households } from '../mechanisms/households/index.js';
import { HOURS, labour } from '../mechanisms/labour/index.js';
import { fxMarketOf, pairsOf, spotFx } from '../mechanisms/spot-fx/index.js';
import { drawFxDesks } from '../mechanisms/spot-fx/data.js';
import {
  derivativeLayer,
  houseIdFor,
  TRADES_CONTRACTS,
} from '../mechanisms/derivative-layer/index.js';
import { cds } from '../mechanisms/cds/index.js';
import { irs } from '../mechanisms/irs/index.js';
import { fxDerivatives } from '../mechanisms/fx-derivatives/index.js';
import { options } from '../mechanisms/options/index.js';
import { bondFutures } from '../mechanisms/bond-futures/index.js';
import { research } from '../mechanisms/research/index.js';
import { sovereignCurve } from '../mechanisms/sovereign-curve/index.js';
import { indexFutures } from '../mechanisms/index-futures/index.js';
import {
  EQUITY_INDEX,
  GLOBAL_INDEX,
  SIZE_INDEX,
  indices,
  RATED_INDEX,
} from '../mechanisms/indices/index.js';
import { ASSESSOR_COUNT, drawAssessors, ratings } from '../mechanisms/ratings/index.js';
import { reporting } from '../mechanisms/reporting/index.js';
import { treasury } from '../mechanisms/treasury/index.js';
import type { CellParty, NamedParty } from '../parties/party.js';
import { NO_QTY, roundToNamed, splitOnTick } from '../core/tick.js';
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
/**
 * 13c.1: THE HOME COUNTRY'S FIRST PLACE. Regions are drawn now, named `<country>.<n>`, and a
 * country's first is on its largest piece of land — so this is a stable name for somewhere real
 * rather than a region anybody typed.
 */
export const REGION = regionId('us.1');
/** 13c.1: the country the home region is in — one money, one central bank, one treasury. */
export const HOME = countryId('us');
/**
 * RESOLUTION (13c.1): how many places each country is cut into, at least. It opens at one apiece so
 * the map lands inert — every mechanism reads the ground and nothing about the world moves — and
 * rises in its own step, where what it multiplies is the thing being measured (Law 2).
 */
const PLACES_PER_COUNTRY = 1;

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
/** Seed data, Currency C4 (16.1): the money the world's one cross-region line is REPORTED in — the first country's. */
function globalStatedIn(countries: readonly CountrySeed[]): CurrencyCode {
  const first = countries[0];
  if (first === undefined) throw new Missing('Seed B3', 'this world has no countries in it');
  return first.ccy;
}

export interface CountrySeed {
  /** 13c.1: the country this row opens — what has the money, the central bank and the treasury. */
  readonly country: CountryId;
  /**
   * 13j: THE PLACE ITS PARTIES BOOK IN, which is its first. A country has as many places as the map
   * gives it; this is the one a party with nowhere better to be opens in, and the one a country's
   * own index is named for.
   */
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

/**
 * 13j: EVERY COUNTRY THIS WORLD HAS, and the first of them is not special.
 *
 * It used to be: America was written into this file as a handful of constants and the other three
 * were a list of stubs. That is why every mechanism that crosses a border had one real side and
 * three that produced nothing — the external accounts of a country whose only transaction is a
 * reserve manager's coupon, a spread against a place with no firms in it, a currency layer whose
 * "two parties with opposite reasons" was one bank facing one sovereign.
 *
 * Nothing is drawn per country. There is one draw of banks, one of firms, one of listings — the
 * world's — and what a country is, is where those parties book (Law 4: one writer, and ids that do
 * not need a country in them). Nor is a country's SIZE stated anywhere: the map draws the ground,
 * a firm is placed along it, people live where the ground will feed them and a bank is where its
 * depositors are, so a world whose map came out with a bigger Europe has a bigger Europe.
 */
export const COUNTRIES: readonly CountrySeed[] = [
  {
    country: HOME,
    region: REGION,
    name: 'United States',
    ccy: USD,
    quoteTick: PIP,
    ccyName: 'US dollar',
    centralBank: CB,
    centralBankName: 'Federal Reserve',
    treasury: TREASURY_US,
    treasuryName: 'US Treasury',
    paper: 'ust',
  },
  {
    country: countryId('eu'),
    region: regionId('eu.1'),
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
    country: countryId('uk'),
    region: regionId('uk.1'),
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
    country: countryId('jp'),
    region: regionId('jp.1'),
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
 * Currency A1, A2, A3; Spot FX A3, XI-12: THE OTHER MONEYS, read off the same list. Every one of
 * them means another issuer, another sovereign borrowing in it, and a market against each of the
 * others — and, because there are FOUR countries and not two, a cross that is not the dollar's:
 * with three moneys in a triangle there is a round trip to be taken, which is what XI-12 is about
 * and what a two-country world cannot express at all (no third leg, no vehicle, no gap to close).
 */
export const ABROAD: readonly CountrySeed[] = COUNTRIES.filter((c) => c.country !== HOME);

/** The country a region is in, off the same list. A region's money is its country's (Currency A2). */
/**
 * Housing A3, XI-15, Law 8 (15.4): a shop's opening lease — from the landlords of its bank, for the
 * rooms its plant comes to rounded UP to whole rooms a landlord (a cell lets every member alike),
 * at the landlords' cost a room a period on the money's grid, until the lease term's day. Nothing
 * where the place has no landlords or they have no rooms left: the shop then owns its room.
 */
function leaseFromLandlords(
  ctx: SeedContext,
  letAtSeed: Map<PartyId, number>,
  firm: PartyId,
  region: RegionId,
  kind: CapitalKindDecl,
  rooms: number,
  newPrice: number,
  life: number,
): boolean {
  if (!ctx.registry.partyKinds.has(LANDLORD)) return false;
  const landlord = landlordIdFor(ctx.parties.get(firm).bank);
  if (!ctx.parties.has(landlord)) return false;
  const cell = ctx.parties.get(landlord);
  if (cell.representation !== 'cell' || cell.weight <= 0) return false;
  const pieces = rooms * ctx.registry.subdivision(plantUnitId(kind.id));
  const perMember = upTick(pieces / cell.weight);
  const units = asQty(perMember * cell.weight, 'the rooms it leases, whole a landlord');
  let held = 0;
  for (const h of ctx.register.holdingsOf(landlord)) {
    const i = ctx.instruments.get(h.instrument);
    if (i.status.live && isPlant(i) && plantTerms(i).capitalKind === kind.id)
      held += ctx.register.quantity(landlord, h.instrument);
  }
  const let_ = zeroIfNone(letAtSeed.get(landlord));
  if (held - let_ < units) return false;
  const ccy = ctx.registry.currencyOf(region);
  const sub = ctx.registry.subdivision(plantUnitId(kind.id));
  const perRoom = ctx.registry.onQuoteGrid(
    MONEY_KIND,
    ccy,
    asPerPiece(newPrice / life, 'what a room costs its landlord a period'),
  );
  const rentPerUnit = asPerPiece(perRoom / sub, 'the rent a piece of it a period');
  if (rentPerUnit <= 0 || downTick(rentPerUnit * perMember) < 1) return false;
  const terms: LeaseTerms = {
    kind: LEASE_ROW,
    region,
    capitalKind: kind.id,
    units,
    rentPerUnit,
    until: addDays(
      ctx.calendar.epoch,
      ctx.params.periods(PROPERTY_PARAMS.leaseTerm) * ctx.calendar.periodDays,
    ),
  };
  ctx.owes({
    debtor: firm,
    creditor: landlord,
    ccy,
    owed: 0,
    terms,
    why: `${String(firm)} opens holding a lease of ${String(units)} ${kind.id} from ${String(landlord)}`,
  });
  letAtSeed.set(landlord, let_ + units);
  return true;
}

export const countryOfRegion = (region: RegionId): CountrySeed => {
  const c = COUNTRIES.find((row) => String(region).startsWith(`${row.country}.`));
  if (c === undefined) throw new Missing('13c.1', `${region} is in no country this world declares`);
  return c;
};

/** 13j: the one country a world of one has — a scale model opens the first row and nothing else. */
export const ONE_COUNTRY: readonly CountrySeed[] = COUNTRIES.slice(0, 1);
/**
 * 13c.1: THE LEGS ARE READ OFF THE MAP, so there is no route table here any more. How many hulls
 * a carrier owns is still drawn; where it can sail them is the world's answer and not a list.
 */
const CARRIER_COUNT = 6;

/** 13c.2: the line whose firms trade rather than make. Named once, where the seed filters on it. */
const WHOLESALE = 'wholesale';
/** B2: hulls per unit of drawn size. The smallest carrier has one ship, which is what it means. */
const HULLS_PER_UNIT_OF_SIZE = 1;

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
  readonly banks: Named;
  /** Units one member of every household cell holds (Seed B4, XI-15). */
  readonly perMember: Named;
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
/**
 * PAR: a unit of this sovereign's paper is one unit of its money.
 *
 * What is stated is a DEBT — money per head — and what a line carries is UNITS, and the two are the
 * same number only because every line here is issued at par. That was implicit; it is named here,
 * so the crossing goes through `amountOf` like every other money-to-units read (`E-9`'s family).
 */
const PAR_PER_UNIT: PerNamedUnit = asPerNamedUnit(
  1,
  'a unit of sovereign paper is one of its money',
);

function seedLines(
  epoch: Civil,
  members: number,
  perMember: Stated,
  householdShare: Ratio,
  /** Law 9: the stem its market names this issuer's paper by — `ust`, `bund`, `gilt`, `jgb`. */
  paper: string,
): SeedLine[] {
  const bankWeight = sum(SEED_PROFILE.map((t) => t.bankWeight)).value;
  const householdWeight = sum(SEED_PROFILE.map((t) => t.householdWeight)).value;
  const outstanding = scale(
    perMember,
    asRatio(members, 'the people it is owed by'),
    'the debt outstanding',
  );
  const atBanks = scale(
    outstanding,
    minus(asRatio(1, 'all of it'), householdShare, 'the part the households do not hold'),
    'at the banks',
  );
  const perHead = scale(perMember, householdShare, 'what a member holds directly');
  return SEED_PROFILE.map((t): SeedLine => {
    const maturity = onGrid(epoch, t.months);
    const dated = formatCivil(maturity);
    return {
      id: t.paper === 'bond' ? `${paper}.${dated}` : `${paper}.bill.${dated}`,
      paper: t.paper,
      maturity,
      // Law 8: a unit of the paper is indivisible, so what a line comes to is a whole number of
      // them — the NAMED unit, because that is what a unit of a bond is (`downToNamed`).
      banks: roundToNamed(
        amountOf(
          over(
            scale(atBanks, asRatio(t.bankWeight, 'its weight'), 'its weight'),
            asRatio(bankWeight, 'the weight there is'),
            'this line',
          ),
          PAR_PER_UNIT,
          'units of it',
        ),
        'whole units of this line',
      ),
      perMember: roundToNamed(
        amountOf(
          over(
            scale(perHead, asRatio(t.householdWeight, 'its weight'), 'its weight'),
            asRatio(householdWeight, 'the weight there is'),
            'this line',
          ),
          PAR_PER_UNIT,
          'units of it',
        ),
        'whole units of this line, per member',
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
/**
 * Seed C4: THE ONE LEVEL THIS WORLD OPENS AT, and everything else follows from the recipes.
 *
 * It used to be a level per good — four of them, and it would have been thirty-six once the chain
 * got deep. Thirty-six stated levels is thirty-six claims about the answer, and most of them are
 * not independent: what flour opens at is what the grain in it opens at plus the hours of milling.
 * So ONE number is claimed — what an hour of work opens at — and every good's level is walked up
 * its own recipe from there, which turns thirty-five shapes into arithmetic (Law 2).
 */
const OPENING_WAGE = asPerNamedUnit(40, 'what an hour of work opens at');

const P = {
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
  cbOpeningShare: paramId('seed.centralBank.openingHoldingShare'),
  treasuryBufferShare: paramId('seed.treasury.bufferShare'),
  insurerOpeningSurplusPerHead: paramId('seed.insurer.openingSurplusPerHead'),
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
  const level = openingLevels();
  return [
    {
      id: paramId('seed.openingWage'),
      value: OPENING_WAGE,
      unit: 'USD per hour of work',
      dimension: 'pricePerUnit',
      kind: 'shape' as const,
      owner: 'model' as const,
      why: "Seed C4: the ONE level this world opens at. Every good's opening level is walked up its own recipe from here — the hours it takes at this wage, plus the inputs at what they open at, over the yield — so what the seed claims is one number and not one per line. It is the first clearing's input and not a permanent mark: the session in period one prints a price nobody stated and nothing reads it again.",
    },
    ...GOODS.map((d) => ({
      id: openingPrice(d.subUnit),
      value: zeroIfNone(level.get(d.subUnit)),
      unit: `USD per unit of ${d.subUnit}`,
      dimension: 'pricePerUnit' as const,
      kind: 'shape' as const,
      owner: 'model' as const,
      why: `Seed C4: what ${d.name} opens at, walked up its own recipe from the one wage this world states — ${d.labourHoursPerUnit} hours and its inputs, over a yield of ${d.yieldRate}. It is arithmetic on one claim rather than a claim of its own, and the first session replaces it.`,
    })),
  ];
}

/**
 * Law 19: the level walks UP the chain, each good from what its own inputs opened at. Extraction
 * stands on labour alone, so the walk has somewhere to start and cannot go round: a good is priced
 * only once everything it is made of has been.
 */
function openingLevels(): ReadonlyMap<string, number> {
  const out = new Map<string, PerNamedUnit>();
  const left = new Map(GOODS.map((d) => [d.subUnit, d]));
  while (left.size > 0) {
    let moved = false;
    for (const [subUnit, d] of [...left]) {
      if (!d.inputs.every((i) => out.has(i.subUnit))) continue;
      // Item 16: what one unit of a good costs is MONEY FOR ONE NAMED UNIT all the way down —
      // the inputs are that same level scaled by how many of each a unit takes, the work is the
      // wage scaled by the hours, and the yield divides what one takes over what survives.
      const inputs = sum(
        d.inputs.map((i) =>
          scale(
            zeroIfNone(out.get(i.subUnit)),
            asRatio(i.qtyPerUnit, `what one takes of ${i.subUnit}`),
            'the inputs in one',
          ),
        ),
      ).value;
      const work = scale(
        OPENING_WAGE,
        asRatio(d.labourHoursPerUnit, 'the hours one takes'),
        'the work in one',
      );
      out.set(
        subUnit,
        over(
          plus(work, inputs, 'what one takes'),
          asRatio(d.yieldRate, 'what survives the line'),
          'over what survives',
        ),
      );
      left.delete(subUnit);
      moved = true;
    }
    if (!moved) {
      throw new Missing(
        'Goods A2',
        `the recipe chain does not resolve: ${[...left.keys()].join(', ')}`,
      );
    }
  }
  return out;
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
  carrierRows: readonly CarrierDecl[] = [],
  /**
   * 13c.1: where each firm opens, drawn once in `foundationSpec` and handed here. One draw, read by
   * the seed that places the firms and by the assembly that declares the lines — a second call
   * would be a second world (Law 4).
   */
  placed: ReadonlyMap<string, RegionId> = new Map(),
  /**
   * 13j: where each drawn bank books. One draw of the banks, read by the seed that places them and
   * by the assembly that funds them — a second call would be a second banking system (Law 4).
   */
  banked: ReadonlyMap<string, RegionId> = new Map(),
  /** 13j: the countries this world opens, each getting the same construction (Seed B1). */
  countries: readonly CountrySeed[] = COUNTRIES,
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
        id: P.insurerOpeningSurplusPerHead,
        value: 20,
        unit: "money per head of the region's households",
        dimension: 'amount',
        denominated: 'money',
        kind: 'shape',
        owner: 'model',
        why: "Insurers A1, A3, A4.a, Seed C4 (14.1): WHAT AN INSURER OPENS WITH TO STAND BEHIND COVER — its surplus, in cash at its bank, so that equity is a positive read from the first period and it can write anything at all (an insurer with no surplus writes nothing, A4.a). Per head of the region because that is what the book of cover it will be asked for scales with. It is a SHAPE with a scheduled death: what an insurer's capital IS is an OUTCOME of the premiums it took, the claims it paid and the shares it sold, and once the buyer (14.2), the claim (14.3) and the experience (14.4) exist the opening surplus is what 22a says every opening is — not an equilibrium, a stock the mechanisms then act on. The savers hold the line the equity seed floats against this cash at cost, so the surplus is theirs, subscribed, and not money from nowhere (Seed A2).",
      },
      {
        id: P.membersPerCohort,
        value: 15_000_000,
        unit: 'count',
        dimension: 'count',
        kind: 'technology',
        owner: 'model',
        why: "HOW MANY PEOPLE THERE ARE: fifteen million a cohort and two cohorts, thirty million in the region — a country, and the scale every other number in this seed is a ratio against. A real-world primitive imported as one (Law 2): the population of a country is a fact about the world and not a claim about the answer, and from period one the population is what formation, death and promotion make of it, read off the cells (`integrate` over the households) and never off this number again. It was declared a placeholder for a resolution and the ladder measured otherwise: per-member money and the wage per hour move with the count (the ladder's rungs), so the count is not a resolution whose doubling the answer ignores — it is the size of the country, and the ratios that move with it are findings the ladder carries about the venues at scale. It was six thousand once, and six thousand was a test rig wearing a world's name.",
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
        dimension: 'pricePerUnit',
        kind: 'shape',
        owner: 'model',
        why: "Spot FX C1, Seed C4: what one unit of one money costs in another, before any pair has ever traded. A market that has never traded has no price (XI-6) and a world that opens with holdings in four moneys has to say what they are worth in each other, so ONE level is claimed and each pair's own first session replaces it. One, and the same one for every pair, because a level of one asserts less than any other number would: it says the moneys are all the same size, which is what a world with nothing to distinguish them yet has no reason to deny — and it opens the three crosses consistent with the three dollar rates, so the triangle starts with no gap in it rather than with one somebody put there. What it opens at is not what it stays at: America's banks earn euros, sterling and yen they have no use for and the foreign reserve managers earn dollars they have none for, and where those meet is the rate from period one.",
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
        where: RegionId = REGION,
      ): NamedParty => ({
        id,
        kind,
        region: where,
        name,
        bank,
        representation: 'named',
        status: { alive: true, standing: 'good' },
      });
      // Currency A2, A3: each money's issuer and the sovereign that borrows in it. Each books in its
      // own region, which is what makes everything it holds of another country's paper FOREIGN and
      // everything that country holds of its own foreign the other way (D2).
      for (const c of countries) {
        ctx.parties.add(
          named(c.centralBank, CENTRAL_BANK, c.centralBankName, c.centralBank, c.region),
        );
        ctx.parties.add(named(c.treasury, TREASURY, c.treasuryName, c.centralBank, c.region));
      }
      // Seed B1, B4: as many banks as `banks.count`, each with the disposition and the size its own
      // row states. THREE by default, because with two every depositor that answers a rate is the
      // whole of one side of the deposit market, every interbank session is one name facing one name,
      // and a bank in trouble has exactly one place to go — and because the count being load-bearing
      // in a way no mechanism states is exactly what makes it a RESOLUTION to be measured (XI-15).
      const banks = bankRows.map((b) => ({
        id: partyId(b.bank),
        size: b.size,
        // 13j: where it books, and therefore whose money it issues and whose people bank at it.
        region: banked.get(b.bank) ?? REGION,
      }));
      banks.forEach((b, n) => {
        const home = countryOfRegion(b.region);
        ctx.parties.add(
          named(
            b.id,
            BANK,
            `Bank ${String.fromCharCode(65 + n)}, ${home.name}`,
            home.centralBank,
            b.region,
          ),
        );
      });
      /**
       * 13j, Seed B3: THE BANKS OF A PLACE. A firm banks where it is and a household banks where it
       * lives, because a payment between two parties in two countries is a payment across a border
       * and an account is not. A place the draw gave no bank falls back on the world's first, which
       * is the same named-holder rule the split itself uses.
       */
      const first = banks[0];
      if (first === undefined) throw new Missing('Seed B1', 'this world has no banks');
      const banksIn = new Map<string, typeof banks>();
      for (const b of banks) {
        const where = String(countryOfRegion(b.region).country);
        const held = banksIn.get(where);
        if (held === undefined) banksIn.set(where, [b]);
        else held.push(b);
      }
      const banksFor = (region: RegionId): typeof banks =>
        banksIn.get(String(countryOfRegion(region).country)) ?? [first];
      // Seed B1, B3: the firms this world drew, spread across the banks that exist so that no bank's
      // customers all sit on one side of the payment chain — a bank whose do has a structural reserve
      // drain, which is a flow this world would be opening with rather than producing.
      // 13c.1, Firm A2: AND WHERE EACH OF THEM IS. A line stands on ground, so a firm opens in the
      // home place whose ground is best for what it makes — read off the map, not typed. A line made
      // indoors stands on nothing and opens in the place with the most room to work in. It is the
      // seed STATING an opening (Seed C4) and not a decision: choosing where to BUILD is 13c.2's,
      // and that is what turns this literal from a fact about the world into an outcome.
      firmRows.forEach((f, n) => {
        const where = placed.get(f.firm) ?? REGION;
        const here = banksFor(where);
        const bank = here[n % here.length];
        if (bank === undefined) return;
        ctx.parties.add(named(partyId(f.firm), FIRM, f.name, bank.id, where));
      });

      // Money instruments: one per issuer (Money A1, D2). Each central bank issues its own region's
      // money and a bank issues the money of the region it books in — there is no such thing as
      // money without an issuer, and none of it is anybody else's (A1).
      const issuers: readonly { readonly id: PartyId; readonly region: RegionId }[] = [
        ...countries.map((c) => ({ id: c.centralBank, region: c.region })),
        ...banks,
      ];
      for (const issuer of issuers) {
        const ccy = ctx.registry.currencyOf(issuer.region);
        ctx.instruments.add({
          id: moneyInstrumentId(issuer.id, ccy),
          kind: MONEY_KIND,
          issuer: some(issuer.id),
          ccy,
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
      // XI-15, Law 2: THE POPULATION IS A PROPERTY OF THE WORLD AND NOT OF ITS BANKS. What is
      // stated is how many people a cohort stands for; how many banks they are spread over is a
      // resolution (`banks.count`), and it may not change how many people there are. Stating it
      // per (cohort, bank) key meant a world with a fourth bank had a third more people in it,
      // which is the count of banks answering a question about the population.
      //
      // 0f.9: ONE CELL PER KEY. How finely the population is cut is the LATTICE's resolution now
      // (its band edges, 0f.3), not a count of cells per key: two cells that differ on no
      // dimension are two cells for one key, which is what 0d measured as 299 `units` findings.
      // The kernel places every seeded cell on the rest of its lattice at the seal.
      const members = positiveCount(ctx.params.count(P.membersPerCohort), 'membersPerCohort');
      /**
       * 21.97: AND IT IS A COUNT PER COUNTRY, which is what its own declaration says it is —
       * *"fifteen million a cohort and two cohorts, thirty million in the region — A COUNTRY"*.
       * This split it across every bank in the WORLD, so a world of four countries held thirty
       * million people between them: seven and a half million a nation, in a model whose scale is
       * the thing every other number in the seed is a ratio against. The rig has one country and
       * never saw it; the four-country world has had a quarter of its people since 13j.
       */
      const byCountry = new Map<string, typeof banks>();
      for (const b of banks) {
        const where = String(countryOfRegion(b.region).country);
        const held = byCountry.get(where);
        if (held === undefined) byCountry.set(where, [b]);
        else held.push(b);
      }
      for (const cohort of ctx.registry.cohorts) {
        // Seed B4: and they are spread across the banks IN PROPORTION TO SIZE, so a bigger bank has
        // more depositors — which is what makes it bigger. Split exactly: a weight is a count of
        // people and the odd person has a named cell (core/tick.ts), never a fraction anywhere.
        //
        // 13j: AND THIS IS HOW THE POPULATION SPREADS OVER THE COUNTRIES, with nothing stated. A
        // cell lives where its bank books; a bank books where the people are; and how many people a
        // place holds is what its ground came to (`peopleOn`). So a country's population is the
        // sizes of the banks the ground gave it — two draws and no share anybody chose (Law 2).
        for (const [, itsBanks] of byCountry) {
          const perBank = splitOnTick(
            members,
            itsBanks.map((b) => b.size),
          );
          itsBanks.forEach((bank, at) => {
            const weight = zeroIfNone(perBank[at]);
            if (weight <= 0) return;
            const cell: CellParty = {
              id: partyId(`hh.${cohort.id}.${bank.id}`),
              kind: HOUSEHOLD,
              region: bank.region,
              name: `Households ${cohort.name} at ${bank.id}`,
              bank: bank.id,
              representation: 'cell',
              status: { alive: true, standing: 'good' },
              weight,
              key: { region: bank.region, cohort: cohortId(cohort.id), bank: bank.id },
            };
            ctx.parties.add(cell);
          });
        }
      }
      // Households F2 (13d.1): AND ONE PROBATE OFFICE PER PLACE AND BANK, where what the dead held
      // waits until it can be divided. A cell cannot pay a cell — two weights share no whole number
      // of pieces of anything — so the estate of the departed goes to a NAMED party, which can take
      // a thing to the piece and hand it on. It holds nothing at the opening and it owes nobody.
      for (const bank of banks) {
        ctx.parties.add({
          id: probateId(bank.region, bank.id),
          kind: PROBATE,
          region: bank.region,
          name: `Probate, ${bank.id}`,
          bank: bank.id,
          representation: 'named',
          status: { alive: true, standing: 'good' },
        });
      }
      /**
       * Insurers A1, A3, Seed C4 (14.1): ONE INSURER PER PLACE THAT HAS A BANK, created here and not
       * in its own module's seed because the equity seed floats its line against the book it holds
       * when that seed runs, and a party that does not exist yet has no book. It opens holding its
       * surplus in cash — per head of the households of its region, stated once above — and nothing
       * else: no cover written, nothing owed. The insurers module's own seed leaves a party that is
       * already here alone.
       */
      const headsIn = new Map<string, number>();
      for (const c of ctx.parties.ofKind(HOUSEHOLD)) {
        const at = String(c.region);
        addTo(headsIn, at, weightOf(c));
      }
      // A scale model assembled without the insurers module has no such kind (test/rig.ts
      // `withDependencies`), and a party of a kind nobody registered cannot be added.
      const insurersHere = ctx.registry.partyKinds.has(INSURANCE);
      for (const region of insurersHere ? new Set(banks.map((b) => b.region)) : []) {
        const bank = banksFor(region)[0];
        if (bank === undefined) continue;
        const id = insurerIdFor(region);
        if (ctx.parties.has(id)) continue;
        const home = countryOfRegion(region);
        ctx.parties.add(named(id, INSURANCE, `${String(region)} Assurance`, bank.id, region));
        const heads = headsIn.get(String(region));
        if (heads === undefined || heads <= 0) continue;
        ctx.endowMoney(
          id,
          home.ccy,
          asCash(
            heads * ctx.params.amount(P.insurerOpeningSurplusPerHead, currencyUnit(home.ccy)),
            home.ccy,
            'the surplus it opens with',
          ),
        );
      }

      /**
       * Insurers A1, A4, D3 (14.6): AND ONE PENSION FUND PER PLACE, banked like the insurer, holding
       * NOTHING at the opening — no member has paid in and nobody is promised anything yet; what it
       * comes to hold is the contributions the payrolls carry to it from period one, and what it
       * owes is the promise to the members who retire after the opening. No shape: an empty book
       * is what a scheme that has just been founded has.
       */
      const fundsHere = ctx.registry.partyKinds.has(PENSION);
      for (const region of fundsHere ? new Set(banks.map((b) => b.region)) : []) {
        const bank = banksFor(region)[0];
        if (bank === undefined) continue;
        const id = pensionFundIdFor(region);
        if (ctx.parties.has(id)) continue;
        ctx.parties.add(named(id, PENSION, `${String(region)} Pension Fund`, bank.id, region));
      }

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
      // The seed speaks in NAMED units throughout (stage 2): tonnes, machines, hours — never
      // pieces — and crosses to the grid at `registry.pieces`. Every quantity in this build-out is
      // one, and every recipe coefficient is a count over a count, so `scale` and `over` carry the
      // quantity's dimension and the coefficient can never become one.
      const started = new Map<string, Named>();
      // 13j: a firm makes its line WHERE IT IS, and a world of four countries has the same line
      // opening in more than one of them. A firm whose own place does not make its good opens with
      // nothing, which is what `madeHere` has always meant — it just used to mean it of one place.
      const placeOf = (firm: string): RegionId => placed.get(firm) ?? REGION;
      const madeHere = firmRows.filter((f) =>
        ctx.instruments.has(goodId(f.subUnit, placeOf(f.firm))),
      );
      const sizeOfLine = new Map<string, Named>();
      for (const f of madeHere) {
        sizeOfLine.set(
          f.subUnit,
          plus(
            zeroIfNone(sizeOfLine.get(f.subUnit)),
            asNamed(f.size, 'what this firm is'),
            'its line',
          ),
        );
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
      // A4.b: EVERY kind of plant, not just the kernel's. A good that is what a hull or a silo is
      // made of is a capital good, and a world that counted only machinery called a hull a thing
      // this economy makes for its households (Law 4: one list of what plant is made of).
      const plantKinds = [...CAPITAL_KINDS, STORAGE_KIND, VESSEL_KIND];
      const madeInto = new Set(plantKinds.map((k) => k.madeFrom));
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
      // `params.amount` answers in PIECES of an hour, so this total is on the grid — which is the
      // half of the ratio below that made the unit slip worth writing down.
      let hoursOffered = asAmount<'piece'>(0, 'a world with nobody of working age offers no hours');
      for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
        if (cell.representation !== 'cell') continue;
        if (ctx.registry.cohort(cohortId(keyOf(cell, 'cohort'))).fromAge >= retirementAge) continue;
        hoursOffered = plus(
          hoursOffered,
          scale(perWeek, asRatio(weightOf(cell), 'the people it stands for'), 'what it offers'),
          'the hours there are',
        );
      }
      // ------------------------------------------------------------------------------------------
      // 13c.2: WHAT THIS WORLD OPENS MAKING IS WHAT ITS PEOPLE WANT.
      //
      // It used to be ONE UNIT OF EACH final good, which was harmless while there was one of them
      // and a claim about an answer the moment there were fifteen: a world opening with as many
      // vehicles as tonnes of bread is a world whose entire industrial composition was set by the
      // number 1. Since the basket is a preference in physical quantities (`households/data.ts`),
      // the proportions are there to be read — so the bundle is what the cohorts alive in this world
      // take in a period, each at its own rate, summed over the weights (XI-15, never an average).
      //
      // The scale below is unchanged and still does the work: this pass fixes the MIX, the hours
      // fix the SIZE. A world whose people want more than their hours can make opens making the
      // same basket smaller, in proportion, and the shortage is then a price.
      // ------------------------------------------------------------------------------------------
      const wantedInAPeriod = new Map<string, Named>();
      for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
        if (cell.representation !== 'cell') continue;
        const cohort = keyOf(cell, 'cohort');
        for (const row of CONSUMPTION) {
          if (row.cohort !== cohort) continue;
          wantedInAPeriod.set(
            row.subUnit,
            plus(
              zeroIfNone(wantedInAPeriod.get(row.subUnit)),
              scale(
                plus(
                  asNamed(row.neededPerMember, 'what a member needs in a period'),
                  asNamed(row.wantedPerMember, 'and what it wants'),
                  'what a member takes in a period',
                ),
                asRatio(weightOf(cell), 'the people it stands for'),
                'what this cell takes',
              ),
              'what this world wants in a period',
            ),
          );
        }
      }
      const asked = sum(finalGoods.map((g) => zeroIfNone(wantedInAPeriod.get(g)))).value;
      for (const g of finalGoods) {
        // A world whose basket names none of what it makes is a SCALE MODEL (`test/rig.ts`): it has
        // three lines and no shops, and there is nothing to read the mix off. It opens as it always
        // did, one unit-scale of each, and says so here rather than dividing by nothing.
        let want = asNamed(1, 'one unit-scale of it, in a world with nothing to read a mix off');
        if (asked > 0) want = zeroIfNone(wantedInAPeriod.get(g));
        started.set(
          g,
          over(
            want,
            asRatio(recipeOf(g).yieldRate, 'what survives the line'),
            'started for what is wanted of it',
          ),
        );
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
        let drawn = asNamed(0, 'a line nothing draws from draws nothing');
        for (const by of drawnBy.get(next) ?? []) {
          const qty = recipeOf(by).inputs.find((i) => i.subUnit === next);
          if (qty === undefined) continue;
          drawn = plus(
            drawn,
            scale(
              zeroIfNone(started.get(by)),
              asRatio(qty.qtyPerUnit, 'what one unit of it draws'),
              'what it draws',
            ),
            'drawn',
          );
        }
        started.set(
          next,
          over(
            drawn,
            asRatio(recipeOf(next).yieldRate, 'what survives the line'),
            'started for it',
          ),
        );
        settled = new Set([...settled, next]);
      }
      // Capital Programme A2: the plant those lines run on, with the headroom a going concern has.
      const headroom = ctx.params.ratio(P.plantHeadroom);
      const plantOf = (subUnit: string, kind: string): Named => {
        const need = recipeOf(subUnit).plant.find((q) => q.capitalKind === kind);
        if (need === undefined) return asNamed(0, 'a line that needs none of this kind holds none');
        return scale(
          scale(
            zeroIfNone(started.get(subUnit)),
            asRatio(need.unitsPerUnitPerPeriod, 'the plant a unit takes for a period'),
            'the plant it takes',
          ),
          headroom,
          'with the headroom a going concern has',
        );
      };
      // A4.b: and the capital-goods line starts what REPLACES the plant that wears out — one life's
      // worth of it a life, which is what a stock of machines with a life in it demands every period.
      for (const kind of plantKinds) {
        if (!sizeOfLine.has(kind.madeFrom)) continue;
        const inService = sum([...sizeOfLine.keys()].map((g) => plantOf(g, kind.id))).value;
        /**
         * A-7, Law 4, Law 19: THROUGH THE PARAMETER REGISTER, like every other reader of this fact.
         * It read `kind.usefulLifePeriods` straight off the declaration while
         * `capital-programme` and `firms/decide` both read `params.periods(lifeParam(d.id))` —
         * which is where the number is declared with its unit and its owner (XI-14) and where a
         * resolution shift would move it. Two readable homes for one fact agreed only because both
         * happened to read the same table.
         */
        const wearing = over(
          inService,
          asRatio(ctx.params.periods(lifeParam(kind.id)), 'the periods a unit of it serves'),
          'what wears out in a period',
        );
        started.set(
          kind.madeFrom,
          over(
            wearing,
            asRatio(recipeOf(kind.madeFrom).yieldRate, 'what survives the line'),
            'started for it',
          ),
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
          scale(
            zeroIfNone(started.get(g)),
            asRatio(recipeOf(g).labourHoursPerUnit, `the hours one unit of ${g} takes`),
            `the hours ${g} takes`,
          ),
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
      const takes = ctx.registry.pieces(HOURS, asNamed(hoursForOne, 'the hours one takes'));
      // Renamed from `scale` at item 16: the imported `scale` is the dimension algebra's, and a
      // local of that name shadowed it in the middle of this function.
      // Hours over hours: a pure count of how many of the final good the population's time makes.
      const madeByTheHours = ratioOf(
        hoursOffered,
        takes,
        'how many of the final good the hours there are make',
      );
      for (const g of [...started.keys()]) {
        started.set(
          g,
          scale(zeroIfNone(started.get(g)), madeByTheHours, `what ${g} starts in a period`),
        );
      }
      // Seed B4: and a firm's share of its own line is its own size over the line's.
      // Two sizes on the same line, so what comes out is a pure share and never a quantity.
      const shareOf = (f: FirmDecl): Ratio =>
        ratioOf(
          asNamed(f.size, 'what this firm is'),
          zeroIfNone(sizeOfLine.get(f.subUnit)),
          'its share',
        );
      /** What this firm starts in a period. Everything it opens holding is a period of this. */
      const startsOf = (f: FirmDecl): Named =>
        scale(zeroIfNone(started.get(f.subUnit)), shareOf(f), 'its own');
      const cashPeriods = ctx.params.periods(P.firmCashPeriods);
      /**
       * Seed C1: the money it opens with, as periods of its own turnover at what the good opens at.
       * It pays its wage bill and buys its inputs before it is paid for what it sells, so a firm that
       * opens with nothing fails on a timing gap rather than on its economics (Firm D1).
       */
      const cashOf = (f: FirmDecl): Stated =>
        scale(
          valueAt(
            ctx.params.pricePerUnit(openingPrice(f.subUnit)),
            scale(
              startsOf(f),
              asRatio(recipeOf(f.subUnit).yieldRate, 'what survives the line'),
              'what arrives',
            ),
            'what it turns over',
          ),
          asRatio(cashPeriods, 'the periods of it it opens holding'),
          'periods of it',
        );

      // The maturity profile, outstanding with remaining lives (Seed C3, Treasury D4.a). Every bond
      // carries the coupon that makes it par at the opening yield, so nothing but a level is claimed,
      // and how much of it there is, is the population it is owed by (Law 2: one number, not a table).
      const y = ctx.params.perAnnum(P.openingYield);
      const opening = new Map<string, PerNamedUnit>();

      /**
       * 13j: EVERY COUNTRY'S OPENING BALANCE SHEET, BUILT THE SAME WAY.
       *
       * This was written for one country and read as if that were a simplification. It was not: it
       * was the reason three of the four produced nothing. A sovereign with one line of paper has no
       * curve to be a spread against, a banking system with no depositors has nothing to lend, and a
       * country whose only transaction is a reserve manager's coupon has external accounts that
       * measure a coupon. What follows is the same construction, per country, over that country's
       * own banks, its own people, its own firms and its own money — and not one number in it is
       * stated per country: how big each of them is came off the ground the map drew (`peopleOn`).
       */
      // Banks Capital B1.b: the line each bank runs its own book to — its own rule, whatever country
      // it books in, because a leverage line is a fact about a bank and not about a place.
      const lineOfBank = new Map<PartyId, number>(
        bankRows.map((r) => [
          partyId(r.bank),
          plus(
            ctx.params.ratio(P.leverageRatio),
            asRatio(r.capitalBuffer, 'its own buffer'),
            'the line this bank runs to',
          ),
        ]),
      );
      const cellsIn = new Map<string, PartyId[]>();
      const membersIn = new Map<string, number>();
      for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
        const where = String(countryOfRegion(cell.region).country);
        const held = cellsIn.get(where);
        if (held === undefined) cellsIn.set(where, [cell.id]);
        else held.push(cell.id);
        membersIn.set(where, add(zeroIfNone(membersIn.get(where)), weightOf(cell), 'its people'));
      }
      const firmsIn = new Map<string, FirmDecl[]>();
      for (const f of madeHere) {
        const where = String(countryOfRegion(placeOf(f.firm)).country);
        const held = firmsIn.get(where);
        if (held === undefined) firmsIn.set(where, [f]);
        else held.push(f);
      }

      for (const c of countries) {
        const where = String(c.country);
        const banksHere = banksIn.get(where) ?? [];
        const cellsHere = cellsIn.get(where) ?? [];
        const membersHere = zeroIfNone(membersIn.get(where));
        const firmsHere = firmsIn.get(where) ?? [];
        if (banksHere.length === 0 || membersHere <= 0) continue;
        // Seed C3, Treasury D4.a: HOW MUCH SOVEREIGN PAPER THERE IS, as a stock of the economy it is
        // owed by rather than of the heads in it. A treasury that has borrowed is a treasury that
        // spent, and what it spent it on is an economy — so what is stated is how many periods of
        // what THIS country MAKES its sovereign owes, and the amount follows from the same walk
        // everything else here follows from. Stated per head it did not scale with the real economy
        // at all: a world whose people could make forty times as much had the same money in it, and
        // its banks could not carry their own depositors' accounts (Seed D1 refused to open it).
        const turnover = sum(
          firmsHere.map((f) =>
            valueAt(
              ctx.params.pricePerUnit(openingPrice(f.subUnit)),
              scale(
                startsOf(f),
                asRatio(recipeOf(f.subUnit).yieldRate, 'what survives the line'),
                'what it makes in a period',
              ),
              'what that fetches',
            ),
          ),
        ).value;
        const seedLineRows = seedLines(
          ctx.calendar.epoch,
          membersHere,
          over(
            scale(
              turnover,
              asRatio(ctx.params.periods(P.debtPeriods), 'the periods of it its sovereign owes'),
              'the debt outstanding',
            ),
            asRatio(membersHere, 'the people it is owed by'),
            'per member',
          ),
          ctx.params.ratio(P.householdDebtShare),
          c.paper,
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
              : {
                  kind: SOVEREIGN_BILL,
                  issueDate: ctx.calendar.epoch,
                  maturity,
                  // A2.a: the same convention the opening coupon lines were struck on.
                  dayCount: SEED_DAY_COUNT,
                };
          ctx.instruments.add({
            id,
            kind: line.paper === 'bond' ? SOVEREIGN_BOND : SOVEREIGN_BILL,
            issuer: some(c.treasury),
            ccy: c.ccy,
            terms,
            market: some(market),
          });
          ctx.openMarket({
            id: market,
            name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
            instrument: id,
            ccy: c.ccy,
            rationing: 'proRata',
          });
          const flows = ctx.registry
            .instrumentKind(line.paper === 'bond' ? SOVEREIGN_BOND : SOVEREIGN_BILL)
            .cashFlows(ctx.instruments.get(id), ctx.calendar.epoch, ctx.calendar, ctx.registry);
          // Item 16, and it is a finding (E-8): `priceAt` discounts a schedule whose flows the
          // kernel types per PIECE, and what comes out is used here as a level stated per NAMED
          // unit. The two coincide for par-denominated paper and only for it — `PAR` is declared
          // `perUnit: MONEY_PIECES`, the same subdivision the money has, so `priceOf` is the
          // identity — and nothing anywhere says so. The scale is named here rather than left to
          // that coincidence.
          const price = asPerNamedUnit(
            priceAt(flows, y, ctx.calendar.epoch, SEED_DAY_COUNT, `opening ${line.id}`),
            `the opening level of ${line.id}, per unit of par`,
          );
          opening.set(line.id, price);
          ctx.prices.write({
            instrument: id,
            market,
            period: ctx.period,
            price: priced(ctx, id, price),
            ccy: c.ccy,
            quotedAs: 'money',
            provenance: { kind: 'opening' },
          });
        }

        // ----------------------------------------------------------------------------------------
        // THE OPENING BALANCE SHEET (Seed A3, C1, C5, E2; Central Bank A2, C1; Banks Capital B1.b)
        //
        // What is STATED here is what somebody chose: how much paper each bank holds, how much each
        // household member holds, and how much of its liquid buffer a bank keeps as reserves rather
        // than paper. Everything else is DERIVED from a rule that already governs the party's own
        // behaviour, so nothing opens somewhere its own mechanism would immediately move it away
        // from, and no number here was chosen by looking at the answer (the 11.3 record is what that
        // costs).
        // ----------------------------------------------------------------------------------------
        let systemPaper = asStated(0, 'what the banking system holds');
        let centralBankAssets = asStated(0, 'the central bank’s assets');
        for (const line of seedLineRows) {
          const id = instrumentId(line.id);
          const price = openingOf(opening, line.id);
          const par = priced(ctx, id, price);
          // Item 16: a COUNT of units at a LEVEL, which is what a value is — this was a bare `*`
          // of the two, the one shape `valueAt` exists to make unwriteable.
          systemPaper = plus(
            systemPaper,
            valueAt(price, asNamed(line.banks, 'what the banks hold of it'), 'at this level'),
            'what the banking system holds',
          );
          // Seed E2, XI-15: every member of every cell holds the same stated amount, and the cell
          // carries it with its weight. Its own country's paper: a household saving in a money it
          // is not paid in would be a currency position nobody took (Currency D2).
          if (line.perMember > 0) {
            for (const cell of cellsHere) {
              ctx.endowUnits(cell, id, held(ctx, id, asNamed(line.perMember, 'its units')), par);
            }
          }
          // Central Bank C1, Seed E2: WHAT THE CENTRAL BANK OPENS HOLDING, as a share of each line.
          // It is the seed's own endowment and it is deliberately NOT the OMO's target: opening the
          // central bank at the holding its own policy wants would be seeding an outcome (Seed E1)
          // and importing an equilibrium (Law 2), and it would mean the first open-market session
          // had nothing to do — the mechanism would never be seen to run at all.
          //
          // Its holding is therefore not a number in this table either: it is that share of what the
          // line comes to outstanding once everybody else holds theirs, `others × share / (1 −
          // share)`.
          const others = plus(
            asNamed(line.banks, 'what the banks hold of it'),
            scale(
              asNamed(line.perMember, 'what one member holds of it'),
              asRatio(membersHere, 'the people here'),
              'what the households hold of it',
            ),
            'what everybody else holds of it',
          );
          const share = ctx.params.ratio(P.cbOpeningShare);
          const cbUnits = over(
            scale(others, share, 'the share it targets'),
            minus(asRatio(1, 'the whole line'), share, 'what everybody else has'),
            'its holding',
          );
          const cbDrawn = held(ctx, id, cbUnits);
          if (cbDrawn > 0) {
            ctx.endowUnits(c.centralBank, id, cbDrawn, par);
            // Law 19: what it holds, not what the division asked for — the whole pieces it was
            // actually endowed with, read back in the units this price is quoted in.
            centralBankAssets = plus(
              centralBankAssets,
              valueAt(price, inNamedUnits(ctx, id, cbDrawn), 'central bank assets'),
              'its assets',
            );
          }
        }

        // Money A1, Central Bank A2: NO CENTRAL-BANK MONEY EXISTS THAT ITS ISSUER BOUGHT NOTHING
        // WITH. Its money is its liability and the paper above is the asset it bought with it, so
        // THE SIZE OF ITS BALANCE SHEET IS ALREADY DECIDED: what is left to say is who holds it.
        //
        // Treasury D4.b: it opens with a buffer, because the alternative to one is dependence on
        // every single auction clearing — and the buffer is CENTRAL-BANK MONEY, so what is stated
        // about it is ITS SHARE of that balance sheet. The banks hold the rest as reserves.
        const buffer = scale(
          centralBankAssets,
          ctx.params.ratio(P.treasuryBufferShare),
          "the treasury's buffer",
        );
        ctx.endowMoney(c.treasury, c.ccy, cash(ctx, c.ccy, buffer));
        const reserves = minus(centralBankAssets, buffer, 'what the banks hold in reserve');
        // Seed C1, Banks Capital B1.b: A BANK'S BALANCE SHEET FOLLOWS ITS DEPOSITORS, and this is
        // the line of causality the whole sheet turns on.
        //
        // It used to run the other way: the system's paper was split between banks BY THEIR STATED
        // SIZE, a bank's assets were whatever that came to, and its depositors took the residue.
        // That works for three banks and stops working for twenty — a firm's account is a stated
        // amount and a bank's share of the paper shrinks as 1/count, so past a certain number some
        // bank is handed a depositor bigger than the whole book its capital rule lets it fund, and
        // the world refuses to open. The count of banks was silently load-bearing, which is what
        // makes it worth a derivation rather than a table (Seed B1).
        //
        // So: what each bank must FUND is its firms' accounts plus its households' share of what is
        // left, and its assets are what its own leverage line makes of that. One unknown — what a
        // unit of bank size funds in household money — and it is solved rather than chosen:
        //
        //   Σ (firms_i + size_i·H) / (1 − line_i) = assets to go round
        //
        // Nothing here is fitted and nothing is capped: a bank with a large depositor is a large
        // bank because of it, which is what a deposit IS.
        const all = plus(systemPaper, reserves, 'the assets there are to go round');
        const atBank = new Map<PartyId, Stated>(
          banksHere.map((b) => [
            b.id,
            asStated(0, 'a bank nobody banks at holds nothing for them'),
          ]),
        );
        for (const f of firmsHere) {
          const bank = ctx.parties.get(partyId(f.firm)).bank;
          atBank.set(
            bank,
            plus(zeroIfNone(atBank.get(bank)), cashOf(f), 'what its firms hold at it'),
          );
        }
        // Renamed from `over` at item 2: the imported `over` is the dimension algebra's, and a
        // local of that name shadowed it in the middle of this function — the third time this
        // codebase has reached for one of the algebra's names for a local (`scale` twice before).
        const fundedBy = (b: { id: PartyId }): number => {
          const left = 1 - zeroIfNone(lineOfBank.get(b.id));
          // Seed D1, Banks Capital B1.b: a bank that must fund EVERY asset out of its own capital
          // can hold no deposit at all, and this world opens its firms with accounts at it. There is
          // no balance sheet that satisfies both, so the seed says which rule made it impossible
          // rather than dividing by it and handing the world a negative amount of assets to go round.
          forbid(
            left > 0,
            'Seed D1',
            `${b.id} must fund ${zeroIfNone(lineOfBank.get(b.id))} of every asset out of its own capital, so it can take no deposit — and this world opens depositors at it`,
            { bank: b.id, line: zeroIfNone(lineOfBank.get(b.id)) },
          );
          return left;
        };
        const firmsPart = sum(
          banksHere.map((b) =>
            over(
              zeroIfNone(atBank.get(b.id)),
              asRatio(fundedBy(b), 'what a deposit funds'),
              'its firms',
            ),
          ),
        ).value;
        // A count over a count: what a unit of stated size has to fund, summed over the banks here.
        const sizePart = sum(
          banksHere.map((b) =>
            ratioOf(
              asNamed(b.size, 'what this bank is'),
              asNamed(fundedBy(b), 'what a deposit funds'),
              'its households',
            ),
          ),
        ).value;
        forbid(
          sizePart > 0 && all > firmsPart,
          'Seed D1',
          `${c.name}'s banks cannot carry the accounts it opens them with: ${firmsPart} of assets are needed for the firms alone and there are ${all}`,
          { country: c.country, all, firmsPart },
        );
        const perSize = over(
          minus(all, firmsPart, 'what is left for the households'),
          asRatio(sizePart, 'the stated size there is to share it over'),
          'per unit of size',
        );
        const assetsOf = (bank: { id: PartyId; size: number }): Stated =>
          over(
            plus(
              zeroIfNone(atBank.get(bank.id)),
              scale(perSize, asRatio(bank.size, 'what this bank is'), 'its households'),
              'what it funds',
            ),
            asRatio(fundedBy(bank), 'what a deposit funds'),
            'its assets',
          );
        const assets = new Map<PartyId, Stated>(banksHere.map((b) => [b.id, assetsOf(b)]));

        // Its assets are paper and reserves in the proportion the system holds them, because at the
        // opening nothing has yet decided otherwise — the treasury's own liquidity plan does that
        // from period one (Banks Funding C1).
        const paperShare = ratioOf(systemPaper, all, 'the part of a book that is paper');
        for (const b of banksHere) {
          const mine = zeroIfNone(assets.get(b.id));
          ctx.endowMoney(
            b.id,
            c.ccy,
            cash(ctx, c.ccy, minus(mine, scale(mine, paperShare, 'its paper'), 'its reserves')),
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
            banksHere.map((b) => scale(zeroIfNone(assets.get(b.id)), paperShare, 'its paper')),
          );
          banksHere.forEach((b, at) => {
            const units = zeroIfNone(perBank[at]);
            if (units <= 0) return;
            ctx.endowUnits(b.id, id, held(ctx, id, asNamed(units, 'its paper')), par);
          });
        }
        for (const f of firmsHere) {
          ctx.endowMoney(partyId(f.firm), c.ccy, cash(ctx, c.ccy, asStated(cashOf(f), 'its cash')));
        }
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
          /**
           * Law 8, `E-8`: a rate is a price and opens on its pair's own grid — its pip.
           *
           * AND IT CROSSES THE TWO SCALES FIRST. The declaration says *"units of the quote money
           * per unit of the base"* — named on both sides — and a print is money PIECES per piece.
           * `rateTickFor` beside it has always gone through `priceOf` for exactly this reason; the
           * level itself did not, so the two agreed only while base and quote shared a subdivision.
           * Found by splitting `price` from `pricePerUnit`, which is what `E-8` was about.
           */
          price: toTickOf(
            ctx.registry.priceOf(quote, currencyUnit(base), ctx.params.pricePerUnit(P.openingRate)),
            ctx.registry.rateTickFor(base, quote),
          ),
          ccy: quote,
          quotedAs: 'money',
          provenance: { kind: 'opening' },
        });
      }

      // Central Bank F1, F2 (16.7): WHAT A CENTRAL BANK HOLDS ABROAD IS WHAT IT BOUGHT FOR A REASON.
      // The seed stated a share of every central bank's reserves as another country's paper
      // (`seed.crossHoldingShare`, a placeholder that named this item); it is gone, and a central bank
      // opens holding nothing abroad until a mechanism gives it a reason to buy (18a; 21.54).

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
      // 13c.1: every place that makes a line opens ITS market, not one place's. A world with three
      // producing places and one opening print has two markets nobody can bid into.
      for (const row of GOODS) {
        for (const where of ctx.registry.regions.keys()) {
          const id = goodId(row.subUnit, where);
          if (!ctx.instruments.has(id)) continue;
          openedGoods.add(row.subUnit);
          ctx.prices.write({
            instrument: id,
            market: goodMarketId(row.subUnit, where),
            period: ctx.period,
            // Item 16, and it is a finding (E-8): `params.price` cannot say which scale a declared
            // level is in — `equity.openingShare` is declared in PIECES of money and this one in
            // money for a NAMED unit — and only the declaration's free-text `unit` says which. Named
            // at the site that knows, until the declaration itself carries the scale.
            price: priced(
              ctx,
              id,
              asPerNamedUnit(
                ctx.params.pricePerUnit(openingPrice(row.subUnit)),
                `the opening level of ${row.subUnit}`,
              ),
            ),
            ccy: ctx.registry.currencyOf(where),
            quotedAs: 'money',
            provenance: { kind: 'opening' },
          });
        }
      }
      /**
       * Housing A3, Seed C4, XI-15 (15.3, 15.4): THE LANDLORDS OF EACH BANK, before the firms' plant
       * — because a shop opens holding a LEASE of its room from them (15.4), and a lease needs the
       * landlord and its rooms to exist first. A cell of landlords per bank, each member holding its
       * opening premises over three vintages at what is left of a new building's price, and a
       * building's worth of cash. A world assembled without the property module has no such kind.
       */
      if (ctx.registry.partyKinds.has(LANDLORD)) {
        const kind = CAPITAL_KINDS.find((k) => k.id === PREMISES);
        for (const bank of banks) {
          if (kind === undefined || !ctx.registry.instrumentKinds.has(plantKindId(kind.id))) break;
          const region = bank.region;
          const good = goodId(kind.madeFrom, region);
          if (!ctx.instruments.has(good)) continue;
          const count = ctx.params.count(PROPERTY_PARAMS.landlordsPerBank);
          const id = landlordIdFor(bank.id);
          if (count <= 0 || ctx.parties.has(id)) continue;
          ctx.parties.add({
            id,
            kind: LANDLORD,
            representation: 'cell',
            region,
            name: `${String(count)} landlords at ${String(bank.id)}`,
            bank: bank.id,
            weight: count,
            key: { region: String(region), bank: String(bank.id) },
            status: { alive: true, standing: 'good' },
          });
          const newPrice = ctx.params.pricePerUnit(openingPrice(kind.madeFrom));
          const life = ctx.params.periods(paramId(`plant.usefulLife.${kind.id}`));
          const perMember =
            ctx.params.count(PROPERTY_PARAMS.premisesPerLandlord) *
            ctx.registry.subdivision(plantUnitId(kind.id));
          const perVintage = splitOnTick(
            perMember,
            SEED_PLANT_AGES.map(() => 1),
          );
          SEED_PLANT_AGES.forEach((age, at) => {
            const units = perVintage[at];
            if (units === undefined || units <= 0) return;
            const serviceDate = addDays(ctx.calendar.epoch, -age * ctx.calendar.periodDays);
            const vintage = seedVintage(ctx, kind, region, serviceDate);
            ctx.endowUnits(id, vintage, units, (newPrice * (life - age)) / life);
          });
          // Its opening cash is the property module's to give (its seed runs after the banks' sheets
          // are built): money deposited here would come off what the households have to hold.
        }
      }
      // 15.4: what the landlords of each bank have let at the opening, so no room is let twice.
      const letAtSeed = new Map<PartyId, number>();
      for (const row of madeHere) {
        // A firm whose good this world does not make opens with nothing, because there is nothing
        // for it to hold: the seed endows what exists and never brings an instrument into being to
        // have something to endow (Seed A1). `madeHere` is exactly those that do.
        const firm = partyId(row.firm);
        const here = placeOf(row.firm);
        const price = ctx.params.pricePerUnit(openingPrice(row.subUnit));
        const starts = startsOf(row);
        // Seed D1: ONE PERIOD of what it makes, finished and ready to sell; what a batch still in
        // flight comes to, which is a period of starts for every period its recipe keeps it (B3); and
        // what one period of starting draws of each of its inputs. Never a hoard: a firm sitting on a
        // year of stock would produce nothing for a year and the seed would have decided that.
        const finished = scale(
          starts,
          asRatio(recipeOf(row.subUnit).yieldRate, 'what survives the line'),
          'what arrives in a period',
        );
        const onTheLine = scale(
          starts,
          asRatio(recipeOf(row.subUnit).leadTimePeriods, 'the periods its recipe keeps a batch'),
          'what is still in flight',
        );
        // Seed C4: what it cost whoever holds it is the seed's, and it is below what the market
        // opens at — a firm holding stock it could only sell at a loss would never have made it.
        const good = goodId(row.subUnit, here);
        const basis = priced(ctx, good, asPerNamedUnit(price * SEED_STOCK_BASIS, 'what it cost'));
        if (finished > 0) {
          ctx.endowUnits(firm, good, held(ctx, good, asNamed(finished, 'its stock')), basis);
        }
        if (onTheLine > 0) {
          const wip = wipId(row.subUnit, here);
          ctx.endowUnits(
            firm,
            wip,
            held(ctx, wip, asNamed(onTheLine, 'what is on the line')),
            priced(ctx, wip, asPerNamedUnit(price * SEED_STOCK_BASIS, 'what it has cost so far')),
          );
        }
        // Seed D1: what its recipe draws, so its first batch is not waiting on a market session.
        // Law 19: WHAT it draws is read from the good's own terms, never listed a second time here.
        for (const input of recipeOf(row.subUnit).inputs) {
          if (!ctx.instruments.has(goodId(input.subUnit, here))) continue;
          const line = goodId(input.subUnit, here);
          const paid = scale(
            ctx.params.pricePerUnit(openingPrice(input.subUnit)),
            asRatio(SEED_STOCK_BASIS, 'below what the market opens at'),
            'what it cost whoever holds it',
          );
          const drawn = scale(
            starts,
            asRatio(input.qtyPerUnit, 'what one unit draws of it'),
            'what a period of starting draws',
          );
          if (drawn <= 0) continue;
          ctx.endowUnits(
            firm,
            line,
            held(ctx, line, asNamed(drawn, 'what it holds of it')),
            priced(ctx, line, asPerNamedUnit(paid, 'what it paid for one')),
          );
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
          const mine = downToNamed(
            scale(plantOf(row.subUnit, kind.id), shareOf(row), 'its own plant'),
            'its own plant, in whole machines',
          );
          if (mine <= 0) continue;
          const newPrice = ctx.params.pricePerUnit(openingPrice(kind.madeFrom));
          const life = ctx.params.periods(paramId(`plant.usefulLife.${kind.id}`));
          /**
           * Housing A3, Seed C4 (15.4): A SHOP SELLS FROM A LEASE. Plant a recipe says is LEASED is
           * not the firm's: it opens holding a lease of it from the landlords of its bank — a row
           * with a term, at the landlords' cost a room a period (what a new one costs over its
           * life, B1.a's floor), whole rooms a landlord — and the landlords hold the rooms. A place
           * whose landlords cannot cover the lease leaves the shop owning its room, as before, and
           * the seed says so.
           */
          if (
            need.leased === true &&
            leaseFromLandlords(ctx, letAtSeed, firm, here, kind, mine, newPrice, life)
          )
            continue;
          // Law 8: whole machines, and the odd one has a named vintage rather than being lost to a
          // division that does not come out (core/tick.ts).
          const perVintage = splitOnTick(
            mine,
            SEED_PLANT_AGES.map(() => 1),
          );
          SEED_PLANT_AGES.forEach((age, at) => {
            const serviceDate = addDays(ctx.calendar.epoch, -age * ctx.calendar.periodDays);
            const id = seedVintage(ctx, kind, here, serviceDate);
            // A3, A6: what a vintage that has already run for `age` periods is carried at — the
            // straight line it has been on since it went into service, and nothing else.
            ctx.endowUnits(
              ctx.parties.get(firm).id,
              id,
              held(ctx, id, asNamed(zeroIfNone(perVintage[at]), 'this vintage')),
              priced(
                ctx,
                id,
                asPerNamedUnit((newPrice * (life - age)) / life, 'what a unit of it is carried at'),
              ),
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
          spaceOf(
            ctx,
            row.subUnit,
            held(ctx, goodId(row.subUnit, here), asNamed(finished, 'its stock')),
            here,
          ),
          ...recipeOf(row.subUnit).inputs.map((input) => {
            const line = goodId(input.subUnit, here);
            if (!ctx.instruments.has(line)) return NO_QTY;
            const drawn = scale(
              starts,
              asRatio(input.qtyPerUnit, 'what one unit draws of it'),
              'what a period of starting draws',
            );
            return spaceOf(
              ctx,
              input.subUnit,
              held(ctx, line, asNamed(drawn, 'what it draws')),
              here,
            );
          }),
        ].reduce((a, b) => plus(a, b, 'the room everything here takes'), NO_QTY);
        if (space > 0 && ctx.registry.instrumentKinds.has(plantKindId(STORAGE))) {
          const serviceDate = addDays(ctx.calendar.epoch, -ctx.calendar.periodDays);
          const id = seedVintage(ctx, STORAGE_KIND, here, serviceDate);
          const newPrice = ctx.params.pricePerUnit(openingPrice(STORAGE_KIND.madeFrom));
          const life = ctx.params.periods(paramId(`plant.usefulLife.${STORAGE}`));
          ctx.endowUnits(
            ctx.parties.get(firm).id,
            id,
            space,
            priced(ctx, id, asPerNamedUnit((newPrice * (life - 1)) / life, 'a year-old unit')),
          );
        }
      }
      // Freight B1, B2, Seed C4 (13c): THE HULLS THE CARRIERS OPEN WITH. A carrier without a ship
      // is not a carrier, and how many it has is its own size drawn from the fleet's width (Seed
      // B1.a) — never a number typed here. The capacity that makes freight a real limit is this.
      if (ctx.registry.instrumentKinds.has(plantKindId(VESSEL))) {
        const newPrice = ctx.params.pricePerUnit(openingPrice(VESSEL_KIND.madeFrom));
        const life = ctx.params.periods(paramId(`plant.usefulLife.${VESSEL}`));
        for (const c of carrierRows) {
          const who = partyId(c.carrier);
          if (!ctx.parties.has(who)) continue;
          const serviceDate = addDays(ctx.calendar.epoch, -ctx.calendar.periodDays);
          const id = seedVintage(ctx, VESSEL_KIND, c.region, serviceDate);
          const hulls = held(
            ctx,
            id,
            downToNamed(
              scale(
                asNamed(c.size, 'what this carrier is'),
                asRatio(HULLS_PER_UNIT_OF_SIZE, 'hulls a unit of size carries'),
                'its fleet',
              ),
              'its fleet, in whole hulls',
            ),
          );
          if (hulls <= 0) continue;
          ctx.endowUnits(
            who,
            id,
            hulls,
            priced(ctx, id, asPerNamedUnit((newPrice * (life - 1)) / life, 'a year-old hull')),
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
        const leverage = plus(
          minimum,
          asRatio(row.capitalBuffer, 'its own buffer'),
          'the line this bank runs to',
        );
        // 13j, Money A2: WHOSE MONEY THIS BANK ISSUES, which is the money of the place it books in.
        // A deposit is a holding of its issuer's money and there is no such thing as a holding of a
        // money nobody issued (Money A1), so a seed with four banking systems in it has to say
        // whose — and the one writer of that is the registry, off the bank's own region.
        const ccy = ctx.registry.currencyOf(ctx.parties.get(bank).region);
        const own = moneyInstrumentId(bank, ccy);
        let assets = noCash(ccy);
        for (const h of ctx.register.holdingsOf(bank)) {
          if (h.instrument === own) continue;
          // Currency C4: a bank's book is a REPORT in its own money; a foreign line it was drawn to
          // hold is translated at the rate in force, never added across moneys (Money A2.b).
          assets = plus(
            assets,
            ctx.valuation.inOwnMoney(
              bank,
              ctx.valuation.valueOfLots(h.instrument, h.lots, ctx.period),
              ctx.period,
            ),
            "the bank's opening assets",
          );
        }
        const funding = scale(
          assets,
          minus(asRatio(1, 'the whole book'), leverage, 'what its own capital does not fund'),
          'what its own leverage rule leaves it to fund',
        );
        // Law 19, Law 15: what is ALREADY deposited at it, read off the register — not "what its
        // firms hold", which would be this seed asking what kind a depositor is (nothing branches
        // on a kind: the question is what the account holds, and the answer is the same whoever
        // opened it). Everything endowed at this bank before now counts, and the households take
        // what is left of what has to be funded.
        let already = noCash(ccy);
        for (const holder of ctx.register.holdersOf(own)) {
          if (holder === bank) continue;
          const held = ctx.register.quantity(holder, own);
          already = plus(
            already,
            // XI-15: the register holds PER MEMBER, so what a cell has between them is that times
            // how many of them there are. `acrossMembers` is the one crossing and it refuses a
            // weight that is not a count of people.
            asCash(
              acrossMembers(
                asPerMember<'money:piece'>(held, 'what one member of it holds'),
                weightOf(ctx.parties.get(holder)),
                'in total',
              ),
              ccy,
              'what they hold of it between them',
            ),
            'deposits',
          );
        }
        const fromHouseholds = minus(funding, already, 'what the households must hold');
        const cells = ctx.parties.ofKind(HOUSEHOLD).filter((c) => c.bank === bank);
        const members = cells.reduce((t, c) => t + weightOf(c), 0);
        forbid(
          fromHouseholds.pieces > 0 && members > 0,
          'Banks Capital B1.b',
          `${bank} opens with ${assets.pieces} of assets and ${already.pieces} already deposited at it, which leaves nothing for its households to hold`,
          { bank, assets: assets.pieces, already: already.pieces, funding: funding.pieces, ccy },
        );
        // XI-15: per member, and the cell carries it with its weight.
        const perMember = over(
          fromHouseholds,
          asRatio(members, 'the people who have to hold it'),
          'the deposit one member opens with',
        );
        for (const cell of cells) ctx.endowMoney(cell.id, ccy, perMember);
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
 *
 * 13j: AND WHOSE. It used to name the dollar, because there was one money in this world worth
 * holding; a seed with four banking systems in it has to say whose cents it means (Money A2.b),
 * and every money has its own smallest piece (Currency A3).
 */
function cash(ctx: SeedContext, ccy: CurrencyCode, phx: Stated): Cash {
  // Item 16: money in its named unit IS an amount of that currency's named unit, and this is where
  // it becomes the pieces the state holds — the one door between the two scales (Law 8). What comes
  // out is a VALUE in those pieces, which is what money's own price being one means (Money D2).
  return heldAsMoney(
    ctx.registry.pieces(currencyUnit(ccy), asNamed(phx, 'what is stated')),
    ccy,
    'what is stated, in the pieces the state holds',
  );
}

/** Units of an instrument, in whatever its own unit is named in. */
function held(ctx: SeedContext, instrument: InstrumentId, amount: Named): Qty {
  return ctx.registry.pieces(ctx.instruments.get(instrument).unit, amount);
}

/**
 * Law 8: and back the other way — a count of pieces read in the NAMED units a price is quoted in.
 * A price is money for one named unit, so a value is a named count times a price and never a piece
 * count times one: the two differ by the subdivision, which is a RESOLUTION and must not reach a
 * number anybody acts on (Law 2).
 */
function inNamedUnits(ctx: SeedContext, instrument: InstrumentId, pieces: Qty): Named {
  return asNamed(
    div(pieces, ctx.registry.subdivision(ctx.instruments.get(instrument).unit), 'in named units'),
    'in named units',
  );
}

/** And for a price: money for one NAMED unit becomes money pieces for one piece. */
function priced(ctx: SeedContext, instrument: InstrumentId, perNamedUnit: PerNamedUnit): PerPiece {
  const i = ctx.instruments.get(instrument);
  // Law 8, Seed C4: AN OPENING LEVEL IS A PRICE AND SITS ON THE SAME GRID AS ONE. Nobody posted it
  // and nobody promised anything at it, so there is no side to take a direction from and the
  // nearest tick is the honest answer — but a stated level off the grid would be a level this
  // market could never print again, which is the whole defect 12b.1 exists to remove.
  // 16.1: the level is stated in the LINE's own money, whichever region issued it.
  return ctx.registry.onQuoteGrid(i.kind, i.ccy, ctx.registry.priceOf(i.ccy, i.unit, perNamedUnit));
}

/** The opening price the seed computed for a line; a line with none is a defect, never a default. */
function openingOf(opening: ReadonlyMap<string, PerNamedUnit>, id: string): PerNamedUnit {
  const p = opening.get(id);
  if (p === undefined) throw new Missing('Seed C4', `no opening price for ${id}`, { id });
  return p;
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
  /**
   * 10f.1: A ROW FOR EVERY FIRM — its share line, and whether this world opened it to a market.
   * It is not the listed ones: a private company has a residual and named owners too, and which of
   * them are public is one field of the row rather than which rows there are.
   */
  readonly equities: readonly EquityDecl[];
  /**
   * §42 A2, A6 (item 11): THE SMALL-BUSINESS TIER, cut into cells. It is drawn beside the named
   * firms because it is the same draw seen at a different resolution: how many businesses there
   * are, where they are, who they bank with, and how unequal they are in size.
   */
  readonly small: readonly SmallFirmDecl[];
  readonly funds: readonly FundDecl[];
  readonly trackers: readonly FundDecl[];
  /**
   * F3, Seed A3 (item 10e.4): the HOUSES that run them. Drawn from the pools this world opens with
   * — a manager exists because something names it, never the other way round — and each with its
   * own preferences, so two houses compete rather than repeat one number.
   */
  readonly managers: readonly ManagerDecl[];
}

export function foundationDraw(
  seed: string,
  bankRows: readonly BankDecl[] = drawBanks(BANK_COUNT, seed),
  firmRows: readonly FirmDecl[] = drawFirms(FIRM_COUNT, seed, 0),
  /** 13j: the countries this world opens, so a tracker exists for every market it has. */
  countries: readonly CountrySeed[] = COUNTRIES,
): FoundationDraw {
  const names = bankRows.map((b) => b.bank);
  const equities = drawEquity(firmRows, bankRows, seed);
  /**
   * §42 A2, A5, A6, Seed B1.a (item 11): THE SMALL-BUSINESS TIER. Its lines are the ones whose
   * output is made where it is bought — a haircut, a meal, an hour of a plumber's time — because
   * that is what the small tier of a real economy overwhelmingly IS, and it is a READ of the goods
   * table rather than a second list of lines (Law 19). How many of them there are is this world's
   * named firms times a multiple, so the sector scales with the world and not with this file.
   */
  const small = drawSmallBusiness(
    GOODS.filter((g) => g.output === 'capacity').map((g) => g.subUnit),
    bankRows,
    firmRows.length * SMALL_PER_NAMED,
    seed,
  );
  const pools = drawFunds(bankRows, seed);
  /**
   * §28 A1, A4, B1 (item 13.2): THE STRATEGY HOUSE, and it is drawn beside the long-only pools
   * because it is the same object. What makes its three pools hedge funds is four terms of their
   * mandates — a wide blueprint, `mayWrite: 'anything'`, `leverage`, and a performance fee — and
   * there is no hedge-fund party kind anywhere in this world.
   */
  const strategies = drawStrategies(bankRows, seed);
  /**
   * §29 A1, A2, Seed A3 (item 13.5): THE BUYOUT FUND AND WHO RAISED IT. The seed is the only place
   * that may name both — a closed-end fund's investors are parties the insurers module creates, and
   * the funds module may not say their ids (`no-cross-module-import`). It is the same meeting point
   * the tracker's index is named at, and for the same reason.
   */
  const privateEquity = drawPrivateEquity(
    bankRows,
    [...countries].map((c) => String(insurerIdFor(c.region))),
    seed,
  );
  // Indices C2: the tracker tracks THIS world's equity index, named by the one module that
  // declares it. The seed is where the two meet, because it is the only place that may know both.
  const trackers = drawTrackers(
    // Indices A2, A3: ONLY THE PUBLIC ONES. An index is its constituents' own prints and a
    // private line never prints, so a tracker handed one would be following a level it could
    // not read (§29 C5.a).
    equities.filter((r) => r.listed).map((r) => String(equityLineOf(r.firm))),
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
      // 13j: a tracker on every country's own market, because an index with no vehicle following
      // it is a measurement nobody trades and C2's simultaneity has nothing to be simultaneous.
      ...countries.map((c) => EQUITY_INDEX(c.region)),
      SIZE_INDEX(REGION, 'large'),
      SIZE_INDEX(REGION, 'small'),
      GLOBAL_INDEX(globalStatedIn(countries)),
    ],
    // A4: listed equity is what these hold, and the seed may open the first: a broad equity index
    // is its listed constituents, and they are listed at period zero.
    ['residual'] as const,
    'etf',
    true,
  );
  /**
   * Indices C2, Ratings C2 (17.10b): AND A VEHICLE ON EACH SIDE OF THE CREDIT LINE.
   *
   * A credit tracker is the same construction with two words changed — what it may hold is the
   * claims a company issued rather than the residual, and the seed may NOT open it. Its index holds
   * paper that does not exist at period zero and whose ids nobody could name in advance, so it
   * launches the period its own index first answers, in kind, out of what the participants
   * actually hold (`launchInKind`, E3.a).
   *
   * There are two of them because the line has two sides, and that is the point: a name that is
   * downgraded across it leaves one vehicle's index and joins the other's, and both have to trade.
   * A world where every name sits on one side launches one and leaves the other waiting, which is
   * a true statement about that world rather than a gap.
   */
  const creditTrackers = drawTrackers(
    [],
    names,
    seed,
    [...new Set(countries.map((c) => c.ccy))].flatMap((ccy) => [
      RATED_INDEX(ccy, 'investment'),
      RATED_INDEX(ccy, 'speculative'),
    ]),
    ['corporate'] as const,
    // F3: the same index house. A manager that runs the equity trackers runs the credit ones, which
    // is what an index house is; what tells its vehicles apart is the line each of them follows.
    'etf',
    false,
  );
  return {
    banks: bankRows,
    firms: firmRows,
    equities,
    small,
    funds: [...pools, ...strategies, ...privateEquity],
    trackers: [...trackers, ...creditTrackers],
    // F3 (item 10e.4): the houses, drawn from the pools that name them. Trackers included: the
    // index house is a manager like any other and competes for the same people.
    managers: drawManagers(
      [...pools, ...strategies, ...privateEquity, ...trackers, ...creditTrackers],
      seed,
    ),
  };
}

/**
 * 13c.1: WHAT WORLD TO DRAW. The numbers are the map's own declared parameters, read back here
 * because the ground has to exist before the registry that holds it does — so there is still one
 * writer for each of them and no second copy (Law 4).
 */
function mapReads(): TerrainReads {
  const values = new Map<string, number>(mapParams().map((p) => [String(p.id), p.value]));
  const read = (id: ParamId, what: string): number => {
    const v = values.get(String(id));
    if (v === undefined) throw new Missing('XI-14', `${what} ${id} is not declared`);
    return v;
  };
  return {
    ratio: (id) => read(id, 'the ratio'),
    kmPerDay: (id) => read(id, 'the speed over'),
  };
}

function mapSpec(countries: readonly CountrySeed[]): MapSpec {
  return {
    worldWidthKm: WORLD.widthKm,
    worldHeightKm: WORLD.heightKm,
    tileKm: WORLD.tileKm,
    subdivide: WORLD.subdivide,
    oceanShare: WORLD.oceanShare,
    channels: WORLD.channels,
    reliefKm: WORLD.reliefKm,
    landCells: WORLD.landCells,
    octaves: WORLD.octaves,
    seaAreas: WORLD.seaAreas,
    water: WATER,
    bands: BANDS.map((b) => ({ id: terrainId(b.id), share: b.share })),
    countries: [
      ...countries.map((c) => ({ id: c.country, name: c.name, regions: PLACES_PER_COUNTRY })),
    ],
    terrains: TERRAINS,
    resources: RESOURCES,
    syllables: SYLLABLES,
  };
}

/**
 * Dealer Desks A3: WHO MAKES A MARKET IN A LINE — the makers drawn with the listing, found by the
 * line they were drawn for.
 *
 * Law 18: it is the same draw, read by line instead of searched for. A bank asks this every time it
 * is shopped, and walking every listing in the world for the one that names a line is a search that
 * grows with the number of listed companies while the answer does not.
 */
function makersOf(
  equities: readonly EquityDecl[],
): (i: InstrumentId) => readonly string[] | undefined {
  const byLine = new Map<InstrumentId, readonly string[]>();
  /**
   * 10f.2: ONLY THE LINES THIS WORLD OPENED PUBLIC. A private line answers `undefined` and not an
   * empty list, and the difference matters the day it floats: `[]` says nobody may ever quote it,
   * and the makers draw is an OPENING condition about the companies this world already had. A
   * firm that lists afterwards has no drawn makers, so the fallback this function already documents
   * takes over — the bank's own `makes` decides — until 10f.4, where the bank that RAN the
   * flotation is the one that quotes it, which is what an underwriter is.
   *
   * Nothing is lost by leaving a private line out: a desk covers what has a market
   * (`banks/dealing.ts:coveredLines`), and a private line has none.
   */
  for (const row of equities) {
    if (row.listed) byLine.set(equityLineOf(row.firm), row.makers);
  }
  return (instrument) => byLine.get(instrument);
}

/** Which resource a line stands on, read off the one table that says so (Law 4). */
function standsOnOf(subUnit: string): string | null {
  const d = GOODS.find((g) => g.subUnit === subUnit);
  return d === undefined ? null : d.standsOn;
}

/**
 * 13c.1, Seed B1.a: WHERE EACH FIRM OPENS — DRAWN, weighted by how much ground there is for what it
 * makes. Not every farmer is on the best land, and a rule that put them all there would state an
 * equilibrium rather than an opening (Law 2): one place would make everything, there would be no
 * second print of a good anywhere, and the location basis this world exists to have could not be
 * measured because there would be nothing to measure it between.
 *
 * The weight IS the ground, so good land gets more farms — which is the pull, stated as a draw and
 * not as a certainty. A line made indoors is drawn on room to work in instead.
 *
 * It is PURE and takes the seed, because two callers need the same answer: the seed that places the
 * firms and the assembly that declares the lines they will make. A second rule would be a second
 * world (Law 4).
 */
/**
 * 13j, Goods B4: WHERE THE PEOPLE ARE — on the ground that will feed them.
 *
 * A country's SIZE is not stated anywhere in this seed and this is why it does not have to be. The
 * map draws the ground; how many people a place holds is what its ARABLE ground came to; and every
 * other population fact follows from that — a shop is built next to its customers, a bank is where
 * its depositors are, and a country whose draw gave it better ground is a bigger country. Nothing
 * multiplies it by a share anybody chose.
 *
 * ONE WRITER (Law 4): the siting of firms and the making of household cells both ask here, so the
 * shops cannot be built somewhere the people are not.
 */
export function peopleOn(g: GeographyDecl, reads: RatioReads, r: RegionId): number {
  return groundIn(g, reads, r, ARABLE);
}

/**
 * 13j, Seed B3: WHERE EACH DRAWN BANK BOOKS — with its depositors, which is where the people are.
 *
 * A bank is not drawn per country and its id carries no country (Law 4: one draw of the banks this
 * world has). What makes it American or Japanese is where it books: its own bank is that country's
 * central bank, the money it issues is that country's money, and the cells that hold its deposits
 * are the ones living in its places. A country whose ground feeds more people has more of them.
 *
 * `splitOnTick` is the same door the seed already uses to spread depositors over banks by size: a
 * bank is a whole bank, so the count is split into whole parts that sum to exactly what was drawn.
 */
function placeBanks(
  g: GeographyDecl,
  reads: RatioReads,
  regions: readonly RegionDecl[],
  countries: readonly CountrySeed[],
  banks: readonly BankDecl[],
): ReadonlyMap<string, RegionId> {
  const first = countries[0];
  if (first === undefined) throw new Missing('Seed B3', 'this world has no countries in it');
  // How many people a COUNTRY holds is the ground it has to feed them on, summed over its places.
  const ground = countries.map(
    (c) =>
      sum(regions.filter((r) => r.country === c.country).map((r) => peopleOn(g, reads, r.id)))
        .value,
  );
  const perCountry = splitOnTick(banks.length, ground);
  const out = new Map<string, RegionId>();
  let at = 0;
  countries.forEach((c, k) => {
    const here = zeroIfNone(perCountry[k]);
    for (let n = 0; n < here; n += 1) {
      const row = banks[at];
      at += 1;
      if (row === undefined) return;
      // 13j: a country's banks book in its FIRST place. Which place inside a country its people
      // live in is 13d's question and not this item's; what this decides is which COUNTRY they are
      // in, because that is what settles whose money they hold and whose central bank stands behind
      // them (Money A1). Spreading them within a country moves the population with them, which is
      // the same item's work and would be this one doing it by accident.
      out.set(row.bank, c.region);
    }
  });
  // Clearing C3: the odd bank the split could not place has a named home, and it is the first one.
  for (const row of banks) if (!out.has(row.bank)) out.set(row.bank, first.region);
  return out;
}

function placeFirms(
  g: GeographyDecl,
  reads: RatioReads,
  regions: readonly RegionDecl[],
  countries: readonly CountrySeed[],
  firms: readonly FirmDecl[],
  seed: string,
): ReadonlyMap<string, RegionId> {
  const places = regions.map((r) => r.id);
  const first = places[0];
  if (first === undefined) throw new Missing('Seed B3', 'this world has nowhere in it');
  const rng = prng(seed, 'siting');
  /**
   * 13c.2, 13j: WHERE THE PEOPLE ARE, which is where a shop is — and it is now every country's
   * places and not one country's. A shop is built next to its customers and a mine is built on the
   * ore, and neither of those is a rule anybody has to write: both fall out of the weights below.
   *
   * People live where the ground will feed them, so how many of them are in a place is what that
   * place's ground came to — which is what makes a country's SIZE an outcome of the draw rather
   * than a number anybody states (Law 2). A world whose map gave Europe more arable ground has more
   * Europeans in it, more firms serving them and more banks holding their money.
   */
  /**
   * 13c.2, 13j: WHERE THE PEOPLE ARE, which is where a shop is. A country's people live in its first
   * place until 13d spreads them, so a consumer line opens where its customers are and the ground
   * only decides which COUNTRY that is. A mine is still built on the ore, wherever the ore is.
   */
  const homes = new Set(countries.map((c) => String(c.region)));
  const peopleIn = (r: RegionId): number => (homes.has(String(r)) ? peopleOn(g, reads, r) : 0);
  /**
   * Law 15: WHICH LINES GO WHERE THE PEOPLE ARE is read off the basket — the lines a household
   * takes — and never off what kind of line it is. Add a line to the basket and its firms move to
   * where the customers are; nothing here learns the name of an industry.
   */
  const toHouseholds = new Set(CONSUMPTION.map((c) => c.subUnit));
  const weightsFor = (subUnit: string, standsOn: string | null): number[] =>
    places.map((r) => {
      if (toHouseholds.has(subUnit)) return peopleIn(r);
      if (standsOn === null) return areaKm2(g, r);
      return groundIn(g, reads, r, resourceId(standsOn));
    });
  const held = new Map<string, number[]>();
  const out = new Map<string, RegionId>();
  for (const f of firms) {
    const standsOn = standsOnOf(f.subUnit);
    const key = toHouseholds.has(f.subUnit) ? 'people' : (standsOn ?? '');
    const weights = held.get(key) ?? weightsFor(f.subUnit, standsOn);
    held.set(key, weights);
    const total = sum(weights).value;
    // One draw per firm, along the ground: the better a place, the more of a line lands on it.
    let want = rng.next() * total;
    let at = 0;
    for (let k = 0; k < weights.length; k += 1) {
      const w = weights[k];
      if (w === undefined) break;
      want -= w;
      if (want <= 0) {
        at = k;
        break;
      }
    }
    out.set(f.firm, places[at] ?? first);
  }
  return out;
}

/**
 * 13c.1: WHERE A LINE IS MADE, which is where the firms that make it are. Declaring a good in a
 * place with nobody in it is a market that cannot clear paying a full sweep of every party, every
 * period (Law 18).
 */
const settled = (placed: ReadonlyMap<string, RegionId>): RegionId[] => [
  ...new Set(placed.values()),
];

export function foundationSpec(
  seed: string,
  /**
   * Seed B1, B4: how many banks this world has, and what each of them is like — DRAWN from the
   * stated spread and from this world's own seed value, so a world of two hundred banks is one
   * number and not two thousand two hundred (Audit D3: the same seed gives the same banks).
   */
  bankRows: readonly BankDecl[] = drawBanks(BANK_COUNT, seed),
  /** Seed B1.a, B4: and the firms, the same way and for the same reason. */
  firmRows: readonly FirmDecl[] = drawFirms(FIRM_COUNT, seed, 0),
  /**
   * 13j: HOW MANY COUNTRIES THIS WORLD HAS, and which. It is a RESOLUTION like the count of banks
   * and the count of firms (Law 2): a scale model is a smaller world and not a distorted one, so a
   * rig that opens one country is the same construction with one row rather than four — and the
   * `Seed D1` refusal is what says when a world has been asked for more countries than it has
   * people and firms to open them with.
   */
  countries: readonly CountrySeed[] = COUNTRIES,
): AssemblySpec {
  // Seed B1.a: WHAT THIS WORLD IS MADE OF — one draw, reaching every module that needs it.
  const drew = foundationDraw(seed, bankRows, firmRows, countries);
  // Law 4: ONE DRAW OF THE CARRIERS, read by the module that sails them and by the seed that gives
  // them their hulls. A second draw would be a second fleet wearing this one's name.
  // 13c.1: THE GROUND THIS WORLD STANDS ON, drawn from the same seed as everything else and read
  // by the yield, the weather and every journey. Its own stream, so adding a terrain never
  // reshuffles the banks (Seed A5).
  const drawn = drawMap(mapSpec(countries), mapReads(), seed);
  // Seed B1.a: WHERE EACH FIRM OPENS, drawn once along the ground and read by everybody who needs it.
  const placed = placeFirms(drawn.geography, mapReads(), drawn.regions, countries, firmRows, seed);
  // 13j: AND WHERE EACH BANK BOOKS — with its depositors, which is where the people are. It is what
  // makes a bank American or Japanese, and it is drawn once for the same reason the firms are.
  const banked = placeBanks(drawn.geography, mapReads(), drawn.regions, countries, bankRows);
  // Insurers A1, Equity A1 (14.1): AN INSURER HAS A RESIDUAL AND SOMEBODY OWNS IT, like any company.
  // One row per insurer — one per place that has a bank — drawn like a firm's (public or private,
  // its makers, its payout patience) from its own stream, so adding an insurer reshuffles nobody's
  // firms. The equity seed floats each against the surplus the foundation gave it, to the savers.
  const insurerEquities = drawEquity(
    [...new Set(bankRows.map((b) => banked.get(b.bank) ?? REGION))].map((region) => ({
      firm: String(insurerIdFor(region)),
      size: 1,
    })),
    bankRows,
    `${seed}/insurers`,
  );
  // Law 4: ONE DRAW OF THE CARRIERS, read by the module that sails them and by the seed that gives
  // them their hulls. A second draw would be a second fleet wearing this one's name.
  // 13j: a hull is registered somewhere, and a world of four countries has ports in all of them.
  const carrierRows = drawCarriers(
    CARRIER_COUNT,
    drawn.regions.map((r) => ({
      region: r.id,
      // 13j, Money A1: the banks of the country this port is in. A hull registered in one country
      // and banked in another holds an account in a money its own bank does not issue.
      banks: drew.banks
        .filter((b) => countryOfRegion(banked.get(b.bank) ?? REGION).country === r.country)
        .map((b) => b.bank),
    })),
    seed,
  );
  // 13c.2, Law 4: THE MERCHANTS ARE THE FIRMS ALREADY IN THE WHOLESALE LINE. They are drawn, placed,
  // banked and funded by the ordinary firm machinery like everybody else; what this adds is the two
  // preferences that make one of them buy a cargo it will never use and another one not. The filter
  // is the SEED's, so the module never learns what a line is called (Law 15).
  const merchantRows = drawMerchants(
    firmRows
      .filter((f) => f.subUnit === WHOLESALE)
      .map((f) => ({ firm: f.firm, region: placed.get(f.firm) ?? REGION })),
    seed,
  );
  return {
    seed,
    epoch: civil(2026, 1, 5),
    registry: {
      currencies: countries.map((c) => ({
        code: c.ccy,
        name: c.ccyName,
        centralBank: c.centralBank,
        quoteTick: c.quoteTick,
      })),
      countries: countries.map((c) => ({ id: c.country, name: c.name, ccy: c.ccy })),
      // 13c.1: THE PLACES ARE DRAWN. A region is where a thing is and its money is its country's,
      // so this list grows with the map and the currency list does not.
      regions: drawn.regions,
      geography: drawn.geography,
      units: [
        // Money A2, Currency A3, Law 8: each money is divided into its own smallest piece, and each
        // is its OWN unit — a dollar is a hundred cents, like any real money, and nothing below a
        // cent can be paid, lent, owed or left over anywhere. Two currencies are never added (Money
        // A2.b) and this is where that starts: an amount of euros is counted in euro pieces, and
        // what it comes to in dollars is a conversion at a rate somebody traded at (Currency C5).
        ...countries.map((c) => ({
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
      lotFlow: 'FIFO',
      curveFamilies: [],
    },
    params: [
      // 13c.1: the ground, and every number that says what it is like (XI-14).
      ...mapParams(),
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
        id: KERNEL_PARAMS.livenessHorizon,
        value: 4,
        unit: 'periods',
        dimension: 'periods',
        kind: 'resolution',
        owner: 'model',
        why: 'Part XII (0h.3): the finite prefix a bounded liveness property is refutable on — how long a declared capability may produce nothing, a living party be a side of nothing and a breached row stand unworked before the audit says so. It is a RESOLUTION and is tested as one: lengthening it may only remove findings and can never add one, and nothing in the world reads it, because the audit never repairs (Audit C4).',
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
      // 13c.1: EVERY PLACE, sea areas included — a ship has to be somewhere for a gale to reach it.
      environment(drawn.geography.places, seed, {
        g: drawn.geography,
        heart: (p) => heartOf(drawn.geography, p),
      }),
      // The order matters at one anchor: three phases sit before the revaluation, and they must run
      // in this order — a drawing becomes a loan row, then anything that cannot pay dies, then the
      // people it employed are released. Assembly keeps declaration order for modules that do not
      // require each other, and that is what puts them in it.
      expectations,
      creditEvents,
      // Dealer Desks A3: the desks, and WHICH LINES EACH OF THEM MAKES — drawn with the listing
      // (`ListedDecl.makers`) and handed in here, because the bank that quotes and the listing that
      // drew its makers are two systems and one fact (Law 4).
      banks(drew.banks, makersOf([...drew.equities, ...insurerEquities])),
      estate,
      // Goods A1: the goods of THIS world are made in the one region that has firms in it. The
      // three abroad are a central bank, a treasury and a bond line (13i builds their economies),
      // so opening grain markets there would be three books nobody is ever on either side of.
      // 13c.1: A GOOD EXISTS WHERE IT CAN BE MADE OR SOLD, which is every place of the home
      // country — a firm is placed by the ground now, so declaring the lines in one place only
      // would throw the first time a draw put a farm somewhere else. Places with nobody in them
      // print `noDemand` and say so, which is what an empty place is.
      goods(GOODS, settled(placed)),
      // Capital Programme: the kind of thing plant is, and the schedule it wears out on. Before the
      // firms, because a firm decides what to make against the plant it holds (A2) and what to
      // invest against what a machine costs (B1) — and a kind has to be registered to be held.
      // Commodities Spot A3, A4 (13c): covered space is the second kind of capital this world has,
      // and it is what makes holding a thing cost something. The capital programme owns what a kind
      // of plant IS; the commodities module owns the market in the space it provides.
      capitalProgramme([...CAPITAL_KINDS, STORAGE_KIND, VESSEL_KIND]),
      labour(),
      firms(drew.firms),
      households(),
      // 17f: the book in which a buyer and a seller of an INPUT lock a quantity and a price in for
      // a term. After the firms and the goods, because what it locks in is what a line buys to make
      // something else and both sides are firms; before nothing, because a contract is a
      // commitment and what it changes is what each of them takes to the spot session.
      supply(),
      // Commodities Spot A3, D3: the market in covered space. After the firms, because who is short
      // of room and who has spare is read off what they hold (Law 19).
      commodities(),
      // Freight A4, D1 (13c): the routes this world has and the carriers that sail them. THE LEGS
      // ARE REAL AND IDLE until there is more than one place with goods in it: this world makes
      // its goods in the one region that has firms, and the three abroad are a central bank, a
      // treasury and a bond line until 13i builds their economies. A session with nothing to carry
      // says `noDemand` and says so out loud, which is the honest state for it to be in.
      freight(carrierRows),
      // 13c.2, Freight D3: the firms whose business is that a thing is worth more somewhere else.
      // Until they existed the only shippers were producers holding stock they happened to have,
      // which closes a basis by accident; a merchant is the party whose purpose is to close it and
      // the one that loses money when the gap shuts before the cargo lands.
      merchants(merchantRows),
      // 13d, Housing A1-A3: a dwelling is a good that is built, stands where it was built and
      // wears out; a tenancy is a venue, because what changes hands is the right to be in it for a
      // period and not the thing itself. Rent clears between what letting WEARS the owner and what
      // a household can pay rather than have nowhere, and nothing in either is a coefficient.
      housing(),
      // §47, XI-17 (19.2): the body that owns the numbers parliament owns. It is assembled after
      // the households it is elected by and the treasury whose programme it decides.
      polity,
      // 13e, Trade Credit A1-A3: MOST OF THE CREDIT IN AN ECONOMY IS NOT A BANK'S. After the firms
      // and the goods, because what it decides is whether a SALE between two of them settles in
      // money or in a promise; the kernel writes the leg either way, in the one instruction.
      tradeCredit(),
      // Equity and the desks before the funds: this world's exchange-traded fund holds the listed
      // firms and is launched by the desks that make its market, and both have to exist before a
      // basket can be put in (the funds module reads that off its own data, in `needs`).
      equity([...drew.equities, ...insurerEquities], seed),
      funds([...drew.funds, ...drew.trackers], drew.managers),
      // 13f, Securities Lending A1-A3: title passes and the economics do not. After equity and the
      // funds, because what is lent is the paper they hold and the desks that need to deliver it
      // are the ones that make its market.
      securitiesLending(),
      // 13f, Corporate Credit A1, B2: the other credit channel — many holders each pricing the
      // name from its own view, and covenants tested on what the issuer PUBLISHED (Reporting A2).
      corporateBondModule(),
      // 13g, M&A A1, B1: the market for CONTROL. After equity, because a bid is priced off what a
      // share is worth to each side, and a premium is the distance between two valuations rather
      // than a number anybody set.
      control(),
      // 13h, Insurers B2, B2.b: the sector whose liability is a SCHEDULE and therefore has
      // duration. After the curve, because what a promise of money later is worth is read from the
      // market that prices money later, and a rate move is a solvency event for it.
      insurers(),
      // 13i, Cross-Border E1-E3: a region's accounts with the rest of the world, as a WALK over
      // the settled legs rather than a series anybody imported. They sum to zero because every
      // transaction had two sides, and the audit looks for the leg that went out with nothing back.
      external(),
      sovereignInstruments,
      // Sovereign D3.a, Currency A3: EVERY SOVEREIGN THAT BORROWS HAS A CURVE, and it is its own —
      // one issuer, one money, its own prints. A world whose foreign lines had no curve family
      // would have paper anybody may hold and nobody may value at a yield (D4.a throws where it is
      // asked), which is the same line being a bond here and not one there. One module, because
      // the CONVENTION is one thing and four modules would be four places to write it down.
      sovereignCurve(countries.map((c) => ({ issuer: c.treasury, ccy: c.ccy }))),
      treasury,
      centralBankOmo,
      // The money market after the treasury and the curve: a bank funds itself against the paper
      // those two put into the world, and it prices a name off what the lending module published
      // about it. Both reach it as public events and prints, never as imports (Law 15).
      moneyMarket,
      // 13e, Securitisation C1, XI-11: a loan leaves a bank's book and a REAL PARTY takes it. After
      // the money market, because what a bank is short of — and therefore what it would sell — is
      // what it could not fund there (D1).
      securitisation(),
      // 10b, Short-Term Debt A1, B3.b: MONEY BORROWED FOR WEEKS, and the asking-again that is the
      // whole risk of it. After the money market, because a bank's need here is what the overnight
      // books did not fund; after the funds, because a money fund is the cash investor C1 is about
      // and buys it under its own mandate rather than through a participant declared here. Until
      // this module every failure in this world was a solvency failure — there was nowhere an
      // issuer could be unable to find the money on a day.
      shortTermDebt(),
      // Currency, Spot FX: the pairs, after the banks whose desks quote them and the money market
      // whose overnight book they fund a position in.
      // Spot FX D1, D3, C2.a: the desks draw their OWN numbers, from this world's own seed value
      // (`13b.1`). The banks module no longer carries them and this one no longer reads `BankDecl`:
      // the cycle between the two is gone, and what crosses is a bank's NAME, which is public.
      spotFx(
        drawFxDesks(
          drew.banks.map((b) => b.bank),
          seed,
        ),
      ),
      // The derivative layer: after the money market, because a margin call is met out of cash a
      // member funds there, and after the estate, because a default resolves into one (XI-8). It
      // brings no class of contract with it (13b does that): what it brings is the house, the
      // margin, the fund and the waterfall every class then runs on.
      /**
       * XI-3, Clearing B2: the layer, and WHO TRADES CONTRACTS in this world.
       *
       * Its banks, its firms AND its funds — and being on this list is not permission (item 9.7).
       * It says the layer speaks for parties of this kind when a book asks; whether a given pool may
       * take a position is its MANDATE's answer (Fund Shares A3, `ParticipantView.mayTrade`), and
       * every mandate this world draws says it may write nothing. So no fund posts in a contract
       * book today, by a term of its own contract rather than by its kind being left off a list.
       *
       * **THE THING THAT MUST BE TRUE BEFORE ANY MANDATE SAYS OTHERWISE**, and it is why this used
       * to read `NOT its funds`: a fund's equity is zero by construction because its own claim on
       * itself absorbs whatever its book comes to — and the pass that re-marks that claim reads the
       * REGISTER, where a contract is not (Derivative X1). A tracker that took a derivative position
       * would carry a mark its own share value had never been told about, which is a fund with
       * equity: measured at 83,247,864 on `etf.us` the first time funds were let in here.
       *
       * That pass is **item 13.6**, and 13.2's hedge-fund mandate may not be drawn before it opens.
       * 13h was where a fund would hold derivatives on purpose and it closed without either, which
       * is what `B-14` is.
       */
      derivativeLayer([...TRADES_CONTRACTS, FUND]),
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
      indices(
        countries.map((c) => c.region),
        countries.map((c) => c.ccy),
        // Seed data, Currency C4: the money the one line that crosses regions is REPORTED in — the
        // first country's, declared here and nowhere in a mechanism (16.1).
        globalStatedIn(countries),
      ),
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
      // 13c steps 11-13, Commodity Futures A1-A4, C1-C4: a ladder of delivery dates on every grade
      // that can actually be handed over, converging because delivery is possible rather than
      // because anything enforces it. The carry it is measured against is three reads — the room,
      // the spoilage and the money — and there is no convenience yield anywhere.
      commodityFutures(houseIdFor),
      indexFutures(
        houseIdFor,
        countries.map((c) => ({ id: EQUITY_INDEX(c.region), ccy: c.ccy })),
      ),
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
      foundationSeedFor(drew.banks, drew.firms, carrierRows, placed, banked, countries),
      /**
       * Item 12: THE GROUND, and it is declared HERE because it requires the foundation seed — the
       * state has to exist before it can hold what nobody has built on.
       *
       * It was declared beside `commodities`, where it belongs by subject, and that was wrong for a
       * reason worth writing down: assembly sorts by `requires`, so a module needing
       * `seed.foundation` declared in the middle of the list DRAGS THE SORT — `freight` went after
       * the foundation seed, the foundation's hull block ran before the carrier parties existed,
       * and this world lost its entire merchant fleet. A module that requires the seed is declared
       * after the seed.
       */
      // 15.3: the landlords' seed puts premises in the world before the land seed gives them the ground under them.
      property(),
      land(),
      /**
       * §42 A1, A5, A6 (items 11, 0b): THE TIER BELOW THE NAMED FIRMS, and it is DECLARED HERE for
       * the reason `land()` is: its seed reads the BANK each cell is keyed on, and the banks are
       * parties the foundation makes. Requiring `seed.foundation` instead would drag the sort and
       * take every module declared after it along — which is how this world lost its merchant
       * fleet once already (item 0). A module that needs the seed is declared after it.
       */
      smallBusiness(drew.small),
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
function spaceOf(ctx: SeedContext, subUnit: string, pieces: Qty, where: RegionId): Qty {
  const id = goodId(subUnit, where);
  if (pieces <= 0 || !ctx.instruments.has(id)) return NO_QTY;
  const instrument = ctx.instruments.get(id);
  const good = instrument.terms;
  if (!isGoodTerms(good) || good.storagePerUnit === null) return NO_QTY;
  return spaceFor(ctx, instrument.unit, pieces, ctx.params.ratio(good.storagePerUnit));
}
