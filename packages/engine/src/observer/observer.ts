/**
 * The observer surface: everything shown is derived from the state (Observer D3), a stale mark is
 * visibly stale (A1.a), a missing number is missing (F4), and looking changes nothing (E3): the
 * snapshot is a plain copy the engine never reads back.
 *
 * @spec Sovereign D3 Sovereign D3.b Observer A1 Observer A1.a Observer A2 Observer A3 Observer A4 Observer D1 Observer D3 Observer E1 Observer E3 Observer F1 Observer F2 Observer F4 Law 9
 */
import { formatCivil } from '../calendar/civil.js';
import { asContractMarket, contractOf, pairOf } from '../clearing/market.js';
import { add, div, mul, sub } from '../core/num.js';
import { periodicityLabel } from '../core/rate.js';
import { partyId, type CurrencyCode, type PartyId, type RegionId } from '../core/ids.js';
import { placeAt, tilesOf } from '../registry/geography.js';
import { isGoodTerms } from '../registry/physical.js';
import { isCreateLeg } from '../ledger/instruction.js';
import { OCCUPATION_OF } from '../mechanisms/firms/data.js';
import { OCCUPATIONS } from '../mechanisms/labour/data.js';
import { tileReached } from '../register/voyages.js';
import { weightOf } from '../parties/party.js';
import { struckIn, type Print } from '../prices/price-store.js';
import { contractName, displayName } from '../registry/naming.js';
import type { World } from '../world/world.js';
import type { AuditReport } from '../audit/audit.js';
import type { ParamReport } from '../registry/params.js';
import type { Event, EventKind } from '../journal/journal.js';
import { consensusOf } from '../mechanisms/research/index.js';
import { hedgedResidual } from '../mechanisms/fx-derivatives/index.js';

/** A4: an inspector's full view and a participant's partial view are different products. */
export type Scope =
  { readonly kind: 'inspector' } | { readonly kind: 'party'; readonly party: PartyId };

export interface PrintView {
  readonly instrument: string;
  readonly name: string;
  /**
   * Law 8: money PIECES for one PIECE of the thing, which is what the state holds. What a person
   * reads is money for one NAMED unit, and turning one into the other needs both subdivisions —
   * this unit's and the currency's, both in `subdivisions`.
   */
  readonly price: number;
  readonly ccy: string;
  /** The unit one of these is counted in, so a reader can put the price in a person's terms. */
  readonly unit: string;
  readonly provenance: Print['provenance'];
  /** Periods since the price last actually traded or opened (A1.a). */
  readonly age: number;
  readonly period: number;
}

/** Sovereign D3: a curve as it is read, with what each point is made of (D3.b). */
export interface CurveView {
  readonly family: string;
  readonly name: string;
  readonly compounding: string;
  readonly points: readonly {
    instrument: string;
    name: string;
    tenorYears: number;
    yield: number;
    provenance: string;
  }[];
}

/** Observer F2: fixed income shows the price AND what is derived from it, never one alone. */
export interface YieldView {
  readonly instrument: string;
  /** Missing when this line is on no curve, never zero (F4). */
  readonly yield: number | null;
  readonly tenorYears: number | null;
}

export interface PositionView {
  readonly holder: string;
  readonly instrument: string;
  readonly name: string;
  readonly qtyPerMember: number;
  readonly qtyTotal: number;
  readonly unit: string;
  /** Missing when the instrument has no mark (F4), never zero. */
  readonly valuePerMember: number | null;
  readonly ccy: string;
}

/**
 * Expectations A1, A2, Observer A2: what a party expects, shown beside the party that expects it.
 * There is no sector outlook and no consensus here, because there is none in the model (A2.b), and
 * a viewer that is a party sees its own and nobody else's (Observer A2: no private state).
 */
/**
 * Derivative D1, D8, X1; Observer A4: an open contract, shown to whoever is a side of it.
 *
 * It is not a position and it is not in the register (X1): what a viewer is shown is the row, the
 * two names on it, and what it is worth TO THE SIDE READING IT — an asset to one and a liability to
 * the other (D1). A party scope sees its own rows; the inspector sees every row, once.
 */
export interface ContractView {
  readonly id: string;
  readonly kind: string;
  readonly name: string;
  readonly a: string;
  readonly b: string;
  readonly house: string | null;
  readonly notional: number;
  readonly struckAt: number;
  /**
   * D7, D7.b: WHAT `struckAt` IS. A forward's is money for a unit of the thing; a swap's or a
   * credit default swap's is a RATE PER ANNUM on the notional, and a reader shown 0.01 with no
   * word for which has been shown a number and not a price (Law 8: the unit is part of it).
   */
  readonly struckAs: 'money' | 'rate';
  /** D8: what it is worth to `a`; `b`'s is the negation (Derivative Layer A3). */
  readonly markToA: number;
  /** D1 (layer): what the kind requires against it, or null when the underlying has no record. */
  readonly initialMargin: number | null;
  readonly opened: number;
  readonly ccy: string;
}

/**
 * A contract BOOK's own print: what a session in it agreed, and what that level means.
 *
 * @spec CDS A1.d CDS C3 IRS C1 IRS C1.a FX Forwards C1 Sovereign I1.a Derivative D7 Law 3 Law 8
 *
 * Every curve this item added is the set of these: a reference's protection across tenors, a
 * money's swap rate across tenors, a pair's forward across tenors, an option's premium at a strike.
 * None of them is stored and none of them is fitted — each point is a level somebody paid, and a
 * tenor nobody traded simply has no point on the list.
 */
export interface ContractPrintView {
  readonly market: string;
  readonly name: string;
  readonly kind: string;
  /** The level the session struck. */
  readonly level: number;
  /** Law 8: money per unit, or a RATE per annum — a reader shown 0.01 with no word has a number. */
  readonly quotedAs: 'money' | 'rate';
  readonly ccy: string;
  /** Clearing E1: how real it is — a session this period, or the last one carried. */
  readonly traded: boolean;
}

/**
 * A CURVE, as the set of levels its books cleared — and never a fit.
 *
 * @spec CDS A1.d CDS C1 IRS C1 IRS C1.a FX Forwards C1 Sovereign D3 Law 3 Law 19
 *
 * Every class this world has that clears at more than one tenor has one of these: a reference's
 * protection, a money's swap rate, a pair's forward. A tenor nobody traded is simply not on the
 * list — there is no point between two points, because nobody paid one.
 */
export interface DerivativeCurveView {
  /** What the curve is OF: a reference, a money, a pair — named as the book names it (Law 9). */
  readonly subject: string;
  readonly kind: string;
  readonly quotedAs: 'money' | 'rate';
  readonly ccy: string;
  readonly points: readonly { readonly tenorYears: number; readonly level: number }[];
}

/**
 * A standing MEASUREMENT of a contract book, as the class that owns the book computes it.
 *
 * @spec CDS C3 CDS C3.a CDS E3 IRS C3 Sovereign I1.a Sovereign I2 Derivative D7.a Observer A1 Law 8 Law 15
 *
 * The surface does not know what a basis is for any class: it walks the books and asks each kind
 * (`DerivativeKindProfile.measures`). A world with one more class shows one more measurement
 * without a line here being touched, and every one of these is a read that nothing acts on — that
 * the two prices differ is the thing worth watching, never a discrepancy anything closes.
 */
export interface MeasureView {
  readonly subject: string;
  /** WHICH difference this is. A reader shown a number with no word for it has been shown nothing. */
  readonly measure: string;
  readonly kind: string;
  readonly tenorYears: number | null;
  readonly level: number;
  readonly unit: 'money' | 'rate' | 'notional';
  readonly ccy: string;
}

export interface OutlookView {
  readonly party: string;
  readonly variable: string;
  readonly expected: number;
  readonly unit: string;
  readonly per: string;
  /** B3: how wide this party's own recent surprises have been — a read, never a stated number. */
  readonly confidence: number;
  readonly formed: number;
}

export interface PartyView {
  readonly id: string;
  readonly name: string;
  readonly kind: string;
  readonly representation: string;
  readonly weight: number;
  readonly region: string;
  readonly alive: boolean;
  /**
   * XI-8, Firm Birth D5: where a reference to it goes now that it has ceased — its estate, or
   * itself where the chain of successors ends. Null while it is still here. Without it a dead party
   * is a name that stops, and the estate winding it up is a party nobody can connect it to.
   */
  readonly successor: string | null;
  /** Only in scope (A4): the party's own equity account per member, else null. */
  readonly equityPerMember: number | null;
  /** The money its own region books in, so a reader knows what the equity above is counted in. */
  readonly ccy: string;
}

/**
 * Banks Funding F1, F2, F4, Banks Capital B3.a, Observer A5: WHAT A BANK LOOKS LIKE FROM OUTSIDE.
 *
 * Every number here is one the bank itself published about itself and nothing is recomputed on the
 * way out (Law 4, Law 19, Observer E3) — its deposit lines by class (F1), its reserve balance as a
 * read of its one account (F2), the liquidity metric somebody outside can see (F4), and where its
 * capital stands against the two rules (B3.a). A5 is why `asOf` and `age` travel with them: a
 * published report is what the bank said AT THE CLOSE, and a surface that showed it as though it
 * were now would be inventing a freshness nobody has.
 */
export interface BankView {
  readonly bank: string;
  readonly ccy: string;
  readonly reserves: number;
  readonly liquid: number;
  readonly couldLeave: number;
  readonly metric: number | null;
  readonly deposits: Readonly<Record<string, number>>;
  readonly base: number;
  readonly capital: number;
  readonly weighted: number;
  readonly assets: number;
  readonly weightedRatio: number | null;
  readonly leverageRatio: number | null;
  readonly binds: string | null;
  readonly breach: boolean;
  readonly limitPerName: number | null;
  /**
   * Dealer Desks A1, D5, E4: what its DEALING LINE is carrying and what room it has left. A bank's
   * dealing book is a sub-ledger of the balance sheet above, not a second party's, so it belongs on
   * the bank's own card rather than on one of its own — which is what a surface showing a "desk"
   * used to say and what it no longer can.
   */
  readonly dealingBook: number | null;
  readonly dealingRoom: number | null;
  /** The period each of those was published in, and how many periods ago that was (A5). */
  readonly asOf: number | null;
  readonly age: number | null;
}

/**
 * Currency C5, Spot FX C3, E3, Observer A1: THE RATE BETWEEN TWO MONEYS, and whether the triangle
 * it sits in adds up. Both are reads of prints anybody may see; neither is computed for display and
 * neither is stored (`no display-only number`, Appendix B). The gap is what the direct pair says
 * against what the long way round says, as a share of the direct — a persistent one is a
 * measurement about this world (E3) and never a correction applied to a price.
 */
/**
 * Spot FX D2.a, E4, XI-12: WHAT A HEDGE DOES NOT COVER, per party and per pair, BESIDE what the
 * rate actually did to the position it was hedging.
 *
 * The two do not cancel and they are not meant to. A forward is a notional fixed on a date; the
 * asset it covers is a price that keeps moving, and what the rate does to a balance is measured on
 * what the balance IS at the close. So the hedge's mark and the position's revaluation are two
 * different objects, and the difference between them is the basis and the imperfection. E4 asks
 * for it to be SHOWN rather than netted, which is why the two numbers travel together here and why
 * the residual arrives with its parts.
 *
 * A4: a party's own book is its own. An inspector sees every row; a party sees the rows that name
 * it, and nothing here is reachable about anybody else.
 */
export interface HedgeView {
  readonly party: string;
  readonly pair: string;
  readonly base: string;
  readonly quote: string;
  /** B1, B2: positive is short of the base money and has to buy it; negative is holding it. */
  readonly exposure: number;
  readonly covered: number;
  /** E4: never zeroed. */
  readonly residual: number;
  /**
   * D2.a: what the rate did to what this party holds in the base money this period, in its OWN
   * money — read off the revaluation the engine journalled, never recomputed here (Law 19). Null
   * when the rate did nothing to it, which is a different answer from nothing left over.
   */
  readonly revalued: number | null;
}

export interface RateView {
  readonly pair: string;
  readonly base: string;
  readonly quote: string;
  readonly rate: number | null;
  readonly stale: boolean;
  readonly market: string;
}

export interface TriangleView {
  readonly through: string;
  readonly crossed: number;
  readonly direct: number;
  readonly gap: number;
}

/**
 * Indices A2, E1: WHAT AN INDEX SAYS AND WHAT IT WAS READ FROM. The level is applied where it is
 * asked for, so the surface shows the same number every other reader gets, together with how many
 * periods stand behind it (D5.a: a window longer than that is Missing, never quietly shortened).
 */
export interface IndexView {
  readonly id: string;
  readonly level: number;
  readonly periods: number;
  readonly constituents: number;
  /**
   * A2, A3: WHAT IT WAS READ FROM — each line, what it printed and what it weighs. A size index's
   * boundary is not a number anybody stores: it is where this list stops, and a reader who can see
   * the constituents and their capitalisations can see the firm that is about to cross it.
   */
  readonly from: readonly {
    readonly instrument: string;
    readonly name: string;
    readonly price: number;
    readonly weight: number;
  }[];
  /**
   * B2, C1, Fund Shares E1: THE VEHICLES ON IT — the trackers this world actually launched against
   * this rule, read off the launch each of them published. An index with no vehicle is a
   * measurement; an index with one is a thing that has to be traded when its membership changes.
   */
  readonly vehicles: readonly { readonly fund: string; readonly shares: number }[];
}

/** Ratings A1, E4: WHOSE OPINION, ABOUT WHOM, AND WHAT IT WAS. Three assessors, three answers. */
export interface RatingView {
  readonly assessor: string;
  readonly subject: string;
  readonly grade: string;
  readonly was: string | null;
  readonly period: number;
}

/**
 * Reporting A1, A2, E1; Observer A3, B2.a, D3: WHAT A PUBLIC COMPANY HAS PUBLISHED, and what the
 * banks that cover it have said about it. Every field is a read of a public event; nothing here is
 * stored and looking changes nothing.
 */
export interface StatementsView {
  readonly company: string;
  readonly quarter: string;
  readonly period: number;
  /** G2: the bottom line, and the part of it the marks made rather than anybody paying. */
  readonly earned: number;
  readonly revaluation: number;
  /** A2: the income decomposed, in the words its own writers wrote. */
  readonly income: readonly { readonly cause: string; readonly amount: number }[];
  readonly assets: number;
  readonly liabilities: number;
  readonly shares: number;
  readonly ccy: string;
  /** B1: what management guides to for the coming quarter, if it is guiding. */
  readonly guided: number | null;
  readonly guidedFor: string | null;
  /** C2: the estimates standing, by the bank whose view each is. */
  readonly estimates: readonly {
    readonly bank: string;
    readonly perPeriod: number;
    readonly period: number;
  }[];
  /** E1: the read, with how stale the oldest estimate in it is. Stored nowhere (E3). */
  readonly consensus: {
    readonly count: number;
    readonly mean: number;
    readonly spread: number;
    readonly oldest: number;
  } | null;
  /** F1: what the last report did to the views standing against it. */
  readonly surprises: readonly { readonly bank: string; readonly surprise: number }[];
}

/** What a place is, for a viewer: where it is, what it is made of, and whose it is. */
export interface MapView {
  readonly cols: number;
  readonly rows: number;
  readonly tileKm: number;
  /** Per tile, row-major: the index of the place it is in. Every tile is somewhere. */
  readonly place: readonly number[];
  /** Per tile: how high it stands, in kilometres above the water. */
  readonly elevation: readonly number[];
  /** Per terrain, per tile: the share of the tile that is that. The shares come to one. */
  readonly mix: readonly { readonly terrain: string; readonly share: readonly number[] }[];
  readonly places: readonly {
    readonly id: string;
    readonly name: string;
    /** The country it is in, or null for a stretch of water, which is in nobody's. */
    readonly country: string | null;
    readonly tiles: number;
  }[];
  /** Freight A3: every voyage under way, at the tile it has actually reached. */
  readonly voyages: readonly {
    readonly id: number;
    readonly carrier: string;
    readonly shipper: string;
    readonly cargo: string;
    readonly aboard: number;
    readonly tile: number;
    readonly place: string;
    readonly km: number;
    readonly travelled: number;
  }[];
}

/**
 * 13c.2, §45: WHAT KIND OF ECONOMY THIS IS — output and stock by sector, as a read.
 *
 * Every line employs a trade and every trade is in a sector (Labour A3), so grouping the lines by
 * the sector their trade is in says how much of this world is farming, how much is making things
 * and how much is the two thirds of it that cannot be put in a box. Nothing is stored: the flow is
 * the create legs this period, at what a unit of the line last printed, and the stock is what the
 * register holds at the same print. A reader that wanted either from a stored total would be
 * reading a second copy (Law 19, Appendix B: no stored aggregate).
 */
export interface SectorView {
  readonly sector: string;
  readonly lines: readonly string[];
  /** What its lines MADE this period, at what a unit of each last printed. A flow. */
  readonly made: number;
  /** What is standing in them: units held, at the same prints. A stock, and a different question. */
  readonly held: number;
  /** Whether every line in it can be put in a box. False is a sector made where it is bought. */
  readonly portable: boolean;
}

export interface Snapshot {
  readonly seed: string;
  readonly period: number;
  readonly date: string;
  readonly scope: Scope;
  readonly parties: readonly PartyView[];
  readonly instruments: readonly {
    id: string;
    name: string;
    kind: string;
    issuer: string | null;
    issued: number;
    unit: string;
    ccy: string;
    live: boolean;
    /** Banks Lending E2: false once a payment it promised was missed; null once it has ceased. */
    performing: boolean | null;
  }[];
  /**
   * 13c.1, §45: THE MAP, as a viewer sees it. Reads only, and nothing about it changes because it
   * is being looked at — the grid is drawn once by the seed and written by nothing afterwards, so
   * painting it is a read of a load-bearing thing and never a display-only number (Appendix B).
   */
  readonly map: MapView;
  /** 13c.2: the economy by sector — what each vertical made, what is standing in it, as reads. */
  readonly sectors: readonly SectorView[];
  readonly prints: readonly PrintView[];
  readonly curves: readonly CurveView[];
  readonly yields: readonly YieldView[];
  readonly positions: readonly PositionView[];
  /** Derivative X1, D1: the open contracts a viewer is a side of, with what each is worth to `a`. */
  readonly contracts: readonly ContractView[];
  /** What every contract book last printed — the curves, the premiums and the bases, as reads. */
  readonly contractPrints: readonly ContractPrintView[];
  /** The same prints gathered by what they are OF, in tenor order: the curves (CDS A1.d, IRS C1). */
  readonly derivativeCurves: readonly DerivativeCurveView[];
  /** F1, F2, F4, B3.a: what each bank last published about its funding and its capital. */
  readonly banks: readonly BankView[];
  /** Currency C5: what each pair last printed, and whether that was this period (D1). */
  readonly rates: readonly RateView[];
  /** Spot FX C3, E3: what the triangles say, as a standing measurement. */
  readonly triangles: readonly TriangleView[];
  /** Spot FX E4: what a hedge leaves over, per party and per pair, with its parts. */
  readonly hedges: readonly HedgeView[];
  /** The bases, spreads and implied moves, each computed by the class whose book it is OF. */
  readonly measures: readonly MeasureView[];
  /** Indices E1: every index this world declares, at what its own constituents make it. */
  readonly indices: readonly IndexView[];
  /** Ratings A1, E4: the published opinions, most recent last. */
  readonly ratings: readonly RatingView[];
  /** Reporting A1, E1; Observer A3: what each public company published, and what banks say of it. */
  readonly statements: readonly StatementsView[];
  readonly audit: AuditReport | null;
  readonly params: ParamReport;
  readonly moneyStock: Readonly<Record<string, number>>;
  /**
   * Law 8, Observer F1: HOW MANY PIECES ONE NAMED UNIT IS, by unit name — a hundred cents to the
   * USD, a million grams to the tonne.
   *
   * Every quantity in this snapshot is a COUNT OF PIECES, because that is what the state holds and
   * a surface that quietly rewrote them would be a surface with arithmetic of its own in it. What a
   * person reads is the named unit, so the reader is handed the one number that turns one into the
   * other and nothing is converted on the way out.
   */
  readonly subdivisions: Readonly<Record<string, number>>;
  readonly journal: readonly Event[];
  /**
   * Observer B1, D1: the recent events of each kind the viewer said it follows, at the same depth.
   * A page shows things a party says once a period — an issuer's programme, an auction's result —
   * and the feed above is the last N of EVERYTHING, so what it reaches back to shrinks every time
   * the world finds more to say. These are read by kind, so it reaches them whatever else happened.
   */
  readonly followed: Readonly<Record<string, readonly Event[]>>;
  readonly ledgerLength: number;
  readonly phases: readonly { name: string; cycle: number; spec: string }[];
  /** A2: every party's outlooks for an inspector; only its own for a party (A2: private state). */
  readonly outlooks: readonly OutlookView[];
  /**
   * Observer A4: what the modules keep — employment rows, inventories, the outlook book. It is the
   * INSPECTOR's product and is null for a party, because a module's state is other parties' private
   * state as often as not, and a surface that showed it would be showing it to them.
   */
  readonly state: Readonly<Record<string, unknown>> | null;
}

/**
 * `journalTail`: how many recent events the viewer asked for; the surface adds nothing of its own.
 * `follow`: the kinds it also wants the recent events OF, at that same depth. A viewer that names
 * none follows none.
 */
export function snapshot(
  w: World,
  scope: Scope,
  journalTail: number,
  follow: readonly EventKind[] = [],
): Snapshot {
  const visible = (party: PartyId): boolean => scope.kind === 'inspector' || scope.party === party;
  // A3, A4: an inspector sees the journal whole; a party sees what is public and what names it.
  const sees = (e: Event): boolean =>
    scope.kind === 'inspector' || e.public || e.subjects.includes(scope.party);
  const prints: PrintView[] = [];
  for (const i of w.instruments.all()) {
    if (w.registry.instrumentKind(i.kind).pricing !== 'cleared') continue;
    const latest = w.prices.latest(i.id, w.period);
    if (!latest.some) continue;
    const p = latest.value;
    const from = struckIn(p);
    prints.push({
      instrument: i.id,
      name: displayName(i, w.parties, w.registry),
      price: p.price,
      ccy: p.ccy,
      unit: i.unit,
      provenance: p.provenance,
      age: w.period - from,
      period: p.period,
    });
  }
  // Sovereign D3, Observer F2: the curve as the read it is, and the yield beside every price it
  // came from. Nothing here is stored; asking for the surface builds it and changes nothing (E3).
  const curves: CurveView[] = [];
  const yields: YieldView[] = [];
  for (const family of w.registry.curveFamilies.values()) {
    const read = w.curve(family.id);
    curves.push({
      family: family.id,
      name: family.name,
      compounding: read.compounding,
      points: read.points.map((pt) => ({
        instrument: pt.instrument,
        name: displayName(w.instruments.get(pt.instrument), w.parties, w.registry),
        tenorYears: pt.tenorYears,
        yield: pt.yield,
        provenance: pt.provenance,
      })),
    });
    for (const pt of read.points) {
      yields.push({ instrument: pt.instrument, yield: pt.yield, tenorYears: pt.tenorYears });
    }
  }
  for (const p of prints) {
    if (!yields.some((y) => y.instrument === p.instrument)) {
      yields.push({ instrument: p.instrument, yield: null, tenorYears: null });
    }
  }
  const positions: PositionView[] = [];
  for (const h of w.register.allHoldings()) {
    if (!visible(h.holder)) continue;
    const i = w.instruments.get(h.instrument);
    const qty = h.lots.reduce((s, l) => s + l.qty, 0);
    let value: number | null = null;
    const profile = w.registry.instrumentKind(i.kind);
    const latest = w.prices.latest(i.id, w.period);
    if (profile.pricing === 'money') value = qty;
    // A holding carried at cost is worth what it cost whether or not its market printed (Goods E1);
    // one carried at the mark is worth nothing anyone can state until it has one (XI-6).
    else if (profile.carry === 'cost') value = w.valuation.valueOfLots(i.id, h.lots, w.period);
    else if (latest.some) value = w.valuation.valueOfLots(i.id, h.lots, latest.value.period);
    positions.push({
      holder: h.holder,
      instrument: i.id,
      name: displayName(i, w.parties, w.registry),
      qtyPerMember: qty,
      qtyTotal: qty * weightOf(w.parties.get(h.holder)),
      unit: i.unit,
      valuePerMember: value,
      ccy: i.ccy,
    });
  }
  const contracts: ContractView[] = [];
  for (const c of w.contracts.open_()) {
    // A4: a contract is private between its two sides, so a party scope sees the rows it is on and
    // no others. Nothing here nets anything (G3): every row is its own line.
    if (!visible(c.a) && !visible(c.b)) continue;
    const margin = w.contracts.initialMargin(c, w.period);
    contracts.push({
      id: String(c.id),
      kind: String(c.kind),
      name: contractName(c, w.registry),
      a: String(c.a),
      b: String(c.b),
      house: c.house === null ? null : String(c.house),
      notional: c.notional,
      struckAt: c.struckAt,
      struckAs: w.registry.derivativeKind(c.kind).quotedAs,
      markToA: w.contractMark(c, w.period),
      initialMargin: margin.some ? margin.value : null,
      opened: c.opened,
      ccy: String(c.ccy),
    });
  }
  // Clearing E1, Law 3: every contract book's own level is public, like every other print. It is
  // read off the store rather than rebuilt, and a book that has never traded is simply not here.
  const contractPrints: ContractPrintView[] = [];
  for (const m of w.markets) {
    const decl = contractOf(m);
    if (decl === undefined) continue;
    const print = w.prices.latest(m.instrument, w.period);
    if (!print.some) continue;
    contractPrints.push({
      market: String(m.id),
      name: m.name,
      kind: String(decl.kind),
      level: print.value.price,
      quotedAs: w.registry.derivativeKind(decl.kind).quotedAs,
      ccy: String(m.ccy),
      traded: print.value.period === w.period,
    });
  }
  /**
   * CDS A1.d, IRS C1, FX Forwards C1: THE CURVES, gathered from the prints above and nothing else.
   *
   * A book's own id says what it is of and at what tenor (`cds:<reference>:5y`), because that is
   * how Law 9 says a thing is named — so the curve is a grouping of prints by subject, taken at
   * the read, stored nowhere, and a tenor with no print has no point on it (Law 3, Law 19).
   */
  const bookCurves = new Map<string, { kind: string; quotedAs: 'money' | 'rate'; ccy: string; points: { tenorYears: number; level: number }[] }>();
  for (const p of contractPrints) {
    const parts = p.market.split('.');
    const tail = parts[parts.length - 1] ?? '';
    if (!tail.endsWith('y')) continue;
    const tenorYears = Number(tail.slice(0, -1));
    if (!Number.isFinite(tenorYears) || tenorYears <= 0) continue;
    const subject = parts.slice(0, -1).join('.');
    const held = bookCurves.get(subject) ?? { kind: p.kind, quotedAs: p.quotedAs, ccy: p.ccy, points: [] };
    held.points.push({ tenorYears, level: p.level });
    bookCurves.set(subject, held);
  }
  const derivativeCurves: DerivativeCurveView[] = [...bookCurves.entries()].map(([subject, c]) => ({
    subject,
    kind: c.kind,
    quotedAs: c.quotedAs,
    ccy: c.ccy,
    points: [...c.points].sort((a, b) => a.tenorYears - b.tenorYears),
  }));
  const banks: BankView[] = [];
  for (const p of w.parties.all()) {
    const liquidity = w.journal.lastOf('bank.liquidity', p.id);
    const capital = w.journal.lastOf('bank.capital', p.id);
    const dealing = w.journal.lastOf('bank.dealing', p.id);
    if (liquidity === undefined && capital === undefined) continue;
    const numOf = (e: Event | undefined, key: string): number => {
      const v = e?.data[key];
      return typeof v === 'number' ? v : 0;
    };
    const textOf = (e: Event | undefined, key: string): string | undefined => {
      const v = e?.data[key];
      return typeof v === 'string' ? v : undefined;
    };
    const orNull = (e: Event | undefined, key: string): number | null => {
      const v = e?.data[key];
      return typeof v === 'number' ? v : null;
    };
    const said = liquidity?.period ?? capital?.period ?? null;
    banks.push({
      bank: String(p.id),
      ccy: textOf(liquidity, 'ccy') ?? textOf(capital, 'ccy') ?? '',
      reserves: numOf(liquidity, 'reserves'),
      liquid: numOf(liquidity, 'liquid'),
      couldLeave: numOf(liquidity, 'couldLeave'),
      metric: orNull(liquidity, 'metric'),
      deposits: (liquidity?.data['deposits'] ?? {}) as Readonly<Record<string, number>>,
      base: numOf(liquidity, 'base'),
      capital: numOf(capital, 'capital'),
      weighted: numOf(capital, 'weighted'),
      assets: numOf(capital, 'assets'),
      weightedRatio: orNull(capital, 'weightedRatio'),
      leverageRatio: orNull(capital, 'leverageRatio'),
      binds: typeof capital?.data['binds'] === 'string' ? capital.data['binds'] : null,
      breach: capital?.data['breach'] === true,
      limitPerName: orNull(capital, 'limitPerName'),
      dealingBook: orNull(dealing, 'book'),
      dealingRoom: orNull(dealing, 'roomLeft'),
      asOf: said,
      age: said === null ? null : w.period - said,
    });
  }
  const outlooks: OutlookView[] = [];
  for (const p of w.parties.all()) {
    if (!visible(p.id)) continue;
    for (const variable of w.outlookVariables(p.id)) {
      const o = w.outlookOf(p.id, variable);
      if (!o.some) continue;
      outlooks.push({
        party: p.id,
        variable,
        expected: o.value.expected,
        unit: o.value.unit,
        per: periodicityLabel(o.value.per),
        confidence: o.value.confidence,
        formed: o.value.formed,
      });
    }
  }
  // 13c.1, §45: the map as it stands. It is READ here and nothing writes it, which is why looking
  // at it cannot change the model — the one thing a surface must never do (Appendix B, Observer).
  const g = w.registry.geography;
  const map: MapView = {
    cols: g.cols,
    rows: g.rows,
    tileKm: g.tileKm,
    place: [...g.place],
    elevation: [...g.elevation],
    mix: g.terrains.map((x) => ({ terrain: String(x.id), share: [...(g.mix.get(x.id) ?? [])] })),
    places: g.places.map((id) => ({
      id: String(id),
      name: w.registry.regions.get(id as RegionId)?.name ??
        g.seaAreas.find((s) => s.id === id)?.name ??
        String(id),
      country: w.registry.regions.get(id as RegionId)?.country ?? null,
      tiles: tilesOf(g, id).length,
    })),
    voyages: w.voyages.underWay().map((v) => {
      const tile = tileReached(v);
      return {
        id: v.id,
        carrier: v.carrier,
        shipper: v.shipper,
        cargo: v.cargo,
        aboard: v.aboard,
        tile,
        place: String(placeAt(g, tile)),
        km: v.km,
        travelled: v.kmTravelled,
      };
    }),
  };


  // 13c.2: the economy by sector. One walk of this period's create legs and one of the register,
  // both at the prints the markets made (Law 19: never a stored total, never a re-derived price).
  const sectorOf = new Map(OCCUPATIONS.map((o) => [o.id, o.sector]));
  const sectors = new Map<string, { lines: Set<string>; made: number; held: number; portable: boolean }>();
  const lineOfInstrument = new Map<string, { sector: string; subUnit: string; portable: boolean }>();
  for (const i of w.instruments.all()) {
    if (!isGoodTerms(i.terms)) continue;
    const trade = OCCUPATION_OF[i.terms.subUnit];
    if (trade === undefined) continue;
    const sector = sectorOf.get(trade);
    if (sector === undefined) continue;
    lineOfInstrument.set(String(i.id), { sector, subUnit: i.terms.subUnit, portable: i.terms.portable });
    const row = sectors.get(sector) ?? { lines: new Set<string>(), made: 0, held: 0, portable: true };
    row.lines.add(i.terms.subUnit);
    if (!i.terms.portable) row.portable = false;
    const print = w.prices.latest(i.id, w.period);
    if (print.some) {
      row.held = add(row.held, mul(w.register.heldTotal(i.id).value, print.value.price, 'what is standing in it'), 'the sector’s stock');
    }
    sectors.set(sector, row);
  }
  for (const r of w.ledger.inPeriod(w.period)) {
    if (r.outcome !== 'settled') continue;
    for (const leg of r.instruction.legs) {
      if (!isCreateLeg(leg)) continue;
      const line = lineOfInstrument.get(String(leg.instrument));
      if (line === undefined) continue;
      const print = w.prices.latest(leg.instrument, w.period);
      if (!print.some) continue;
      const row = sectors.get(line.sector);
      if (row === undefined) continue;
      row.made = add(row.made, mul(leg.qty, print.value.price, 'what it made'), 'the sector’s output');
    }
  }
  const sectorViews: SectorView[] = [...sectors.entries()]
    .map(([sector, r]) => ({ sector, lines: [...r.lines].sort(), made: r.made, held: r.held, portable: r.portable }))
    .sort((a, b) => a.sector.localeCompare(b.sector));

  return {
    seed: w.seed,
    period: w.period,
    map,
    sectors: sectorViews,
    date: formatCivil(w.calendar.startOf(w.period)),
    scope,
    parties: w.parties.all().map((p) => ({
      id: p.id,
      name: p.name,
      kind: p.kind,
      representation: p.representation,
      weight: weightOf(p),
      region: p.region,
      alive: p.status.alive,
      successor: p.status.alive ? null : p.status.successor,
      equityPerMember:
        visible(p.id) && w.register.hasEquityAccount(p.id) ? w.register.equity(p.id) : null,
      ccy: w.registry.currencyOf(p.region),
    })),
    instruments: w.instruments.all().map((i) => ({
      id: i.id,
      name: displayName(i, w.parties, w.registry),
      kind: i.kind,
      issuer: i.issuer.some ? i.issuer.value : null,
      issued: i.issued,
      unit: i.unit,
      ccy: i.ccy,
      live: i.status.live,
      // Banks Lending E2: a status is only a status if something shows it. A line that stopped
      // performing looks exactly like one that never missed anything unless the surface says so.
      performing: i.status.live ? i.status.performing : null,
    })),
    prints,
    curves,
    yields,
    positions,
    contracts,
    contractPrints,
    derivativeCurves,
    banks,
    rates: ratesOf(w),
    triangles: trianglesOf(w),
    hedges: hedgesOf(w, visible),
    measures: measuresOf(w),
    indices: indicesOf(w),
    ratings: ratingsOf(w, journalTail, sees),
    statements: statementsOf(w, sees),
    audit: w.last?.audit ?? null,
    params: w.params.report(),
    moneyStock: w.moneyStock(),
    subdivisions: Object.fromEntries(
      [...w.registry.units.values()].map((u) => [u.name, w.registry.subdivision(u.id)]),
    ),
    journal:
      scope.kind === 'inspector'
        ? w.journal.tail(journalTail)
        : w.journal.visibleTo(scope.party, journalTail),
    followed: Object.fromEntries(
      follow.map((kind) => [kind, w.journal.recentOfKind(kind, journalTail, sees)]),
    ),
    ledgerLength: w.ledger.length,
    phases: w.phases.map((p) => ({ name: p.name, cycle: p.cycle, spec: p.spec })),
    outlooks,
    state: scope.kind === 'inspector' ? w.stateSlots() : null,
  };
}

/** Currency C5, D1: every pair's last print, with whether it was struck this period. */
/**
 * Spot FX E4, D2.a, Observer A4: every party's uncovered position in every pair this world quotes,
 * beside what the rate did to it. Read where it is asked for and stored nowhere.
 */
function hedgesOf(w: World, visible: (party: PartyId) => boolean): readonly HedgeView[] {
  const pairs: { base: string; quote: string }[] = [];
  for (const m of w.markets) {
    const pair = pairOf(m);
    if (pair === undefined) continue;
    pairs.push({ base: String(pair.base), quote: String(pair.quote) });
  }
  if (pairs.length === 0) return [];
  // Law 19: what the rate DID is the engine's own record of it, per party and per money, not a
  // second multiplication of a balance by a rate out here.
  const moved = new Map<string, number>();
  for (const e of w.journal.inPeriod(w.period)) {
    if (e.kind !== 'revaluation.fx') continue;
    const ccy = e.data['ccy'];
    const delta = e.data['deltaPerMember'];
    const who = e.subjects[0];
    if (typeof ccy !== 'string' || typeof delta !== 'number' || who === undefined) continue;
    const key = `${who}|${ccy}`;
    const before = moved.get(key);
    moved.set(key, before === undefined ? delta : add(before, delta, 'what the rate did'));
  }
  const out: HedgeView[] = [];
  for (const p of w.parties.alive()) {
    if (!visible(p.id)) continue;
    for (const pair of pairs) {
      const parts = hedgedResidual(
        w.participantView(p.id),
        pair.base as CurrencyCode,
        pair.quote as CurrencyCode,
      );
      // A party with nothing in this pair and nothing fixed in it is not in it: showing a row of
      // zeroes for every party against every pair would bury the ones that mean something.
      if (parts.exposure === 0 && parts.covered === 0) continue;
      const did = moved.get(`${String(p.id)}|${pair.base}`);
      out.push({
        party: String(p.id),
        pair: `${pair.base}/${pair.quote}`,
        base: pair.base,
        quote: pair.quote,
        exposure: parts.exposure,
        covered: parts.covered,
        residual: parts.residual,
        revalued: did ?? null,
      });
    }
  }
  return out;
}

function ratesOf(w: World): readonly RateView[] {
  const out: RateView[] = [];
  for (const m of w.markets) {
    const pair = pairOf(m);
    if (pair === undefined) continue;
    const p = w.prices.latest(m.instrument, w.period);
    out.push({
      pair: `${pair.base}/${pair.quote}`,
      base: pair.base,
      quote: pair.quote,
      rate: p.some ? p.value.price : null,
      stale: p.some ? p.value.provenance.kind === 'stale' : false,
      market: m.id,
    });
  }
  return out;
}

/**
 * Spot FX C3: every triple of moneys this world has markets for, and what the two routes between
 * two of them say. Read from the prints, like everything else here.
 */
function trianglesOf(w: World): readonly TriangleView[] {
  const rate = (base: string, quote: string): number | null => {
    const m = w.markets.find((x) => {
      const p = pairOf(x);
      return p !== undefined && String(p.base) === base && String(p.quote) === quote;
    });
    if (m === undefined) return null;
    const p = w.prices.latest(m.instrument, w.period);
    return p.some ? p.value.price : null;
  };
  const codes = [...w.registry.currencies.keys()].map(String);
  const out: TriangleView[] = [];
  for (let i = 0; i < codes.length; i += 1) {
    for (let j = i + 1; j < codes.length; j += 1) {
      for (let k = j + 1; k < codes.length; k += 1) {
        const [a, b, c] = [codes[i], codes[j], codes[k]];
        if (a === undefined || b === undefined || c === undefined) continue;
        const ab = rate(a, b);
        const bc = rate(b, c);
        const ac = rate(a, c);
        if (ab === null || bc === null || ac === null || ac <= 0) continue;
        const crossed = mul(ab, bc, `${a} through ${b} into ${c}`);
        out.push({
          through: `${a}/${b}/${c}`,
          crossed,
          direct: ac,
          gap: div(sub(crossed, ac, 'what the two routes disagree by'), ac, 'as a share'),
        });
      }
    }
  }
  return out;
}

/** Indices A2, E2: the level applied where it is asked for — the same read every module gets. */
function indicesOf(w: World): readonly IndexView[] {
  // Fund Shares E1, Indices B2: which trackers were launched on which rule, as each of them
  // published it at the moment it was launched (Law 19: never a list this surface maintains).
  const vehicles = new Map<string, { fund: string; shares: number }[]>();
  for (const e of w.journal.ofKind('etf.launched')) {
    const tracks = e.data['tracks'];
    const fund = e.data['fund'];
    const shares = e.data['shares'];
    if (typeof tracks !== 'string' || typeof fund !== 'string' || typeof shares !== 'number') continue;
    const held = vehicles.get(tracks) ?? [];
    held.push({ fund, shares });
    vehicles.set(tracks, held);
  }
  const out: IndexView[] = [];
  for (const decl of w.indexRules()) {
    const read = w.index(decl.id);
    if (!read.some) continue;
    out.push({
      id: decl.id,
      level: read.value.level,
      periods: read.value.periods,
      constituents: read.value.basket.length,
      from: read.value.from.map((c) => ({
        instrument: String(c.instrument),
        name: displayName(w.instruments.get(c.instrument), w.parties, w.registry),
        price: c.price,
        weight: c.weight,
      })),
      vehicles: vehicles.get(decl.id) ?? [],
    });
  }
  return out;
}

/**
 * CDS C3, E3; IRS C3; Sovereign I1.a; Derivative D7.a: THE BASES, asked of the classes that own
 * them (Law 15). The surface walks the contract books and asks each book's own kind what its level
 * says against the rest of the world; it knows what none of them mean, which is why adding a class
 * adds its measurement here without this function changing.
 */
function measuresOf(w: World): readonly MeasureView[] {
  const reads = w.worldReads;
  const out: MeasureView[] = [];
  const seen = new Set<string>();
  for (const m of w.markets) {
    const book = asContractMarket(m);
    if (book === undefined) continue;
    const decl = book.contract;
    for (const x of w.derivativeClass(decl.kind)?.measures?.(book, reads) ?? []) {
      // A measurement of one name at one tenor is one measurement, however many books carry it:
      // net notional on a reference is the same number in every tenor's book (Law 4).
      const key = `${String(decl.kind)}|${x.subject}|${x.measure}|${String(x.tenorYears)}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push({ ...x, kind: String(decl.kind), ccy: String(m.ccy) });
    }
  }
  return out;
}

/** Ratings A1, E4: the actions as they were published, which is the only place a grade lives. */
/**
 * Reporting A1, A2, B1, C2, E1, F1: the three statements a public company published, its guidance,
 * every bank's estimate of it, the consensus read and the last surprise — assembled at the moment
 * of asking out of public events, and stored nowhere (E3, Observer E3).
 *
 * The latest report per company, because a statement is what a company last said; the reader who
 * wants the history has the journal, which is where it is.
 */
function statementsOf(w: World, sees: (e: Event) => boolean): readonly StatementsView[] {
  const text = (v: unknown): string => (typeof v === 'string' ? v : '');
  const nums = (v: unknown): number => (typeof v === 'number' ? v : 0);
  const latest = new Map<string, Event>();
  for (const e of w.journal.ofKind('reporting.report')) {
    if (!sees(e)) continue;
    const who = e.subjects[0];
    if (who !== undefined) latest.set(who, e);
  }
  const out: StatementsView[] = [];
  for (const [company, report] of latest) {
    const guidance = lastOf(w, 'reporting.guidance', company, sees);
    const estimates = new Map<string, { perPeriod: number; period: number }>();
    for (const e of w.journal.ofKind('research.estimate')) {
      if (!sees(e) || e.subjects[1] !== company) continue;
      estimates.set(text(e.data['bank']), {
        perPeriod: nums(e.data['perPeriod']),
        period: e.period,
      });
    }
    const read = consensusOf(w, partyId(company));
    const rows = report.data['income'];
    out.push({
      company,
      quarter: text(report.data['quarter']),
      period: report.period,
      earned: nums(report.data['earned']),
      revaluation: nums(report.data['revaluation']),
      income: Array.isArray(rows)
        ? (rows as unknown[]).map((r) => {
            const row = r as Record<string, unknown>;
            return { cause: text(row['cause']), amount: nums(row['amount']) };
          })
        : [],
      assets: nums(report.data['assets']),
      liabilities: nums(report.data['liabilities']),
      shares: nums(report.data['shares']),
      ccy: text(report.data['ccy']),
      guided: guidance === undefined ? null : nums(guidance.data['guided']),
      guidedFor: guidance === undefined ? null : text(guidance.data['quarter']),
      estimates: [...estimates].map(([bank, v]) => ({
        bank,
        perPeriod: v.perPeriod,
        period: v.period,
      })),
      consensus: read.some ? read.value : null,
      surprises: w.journal
        .ofKind('research.surprise')
        .filter((e) => sees(e) && e.subjects[1] === company && e.period === report.period)
        .map((e) => ({ bank: text(e.data['bank']), surprise: nums(e.data['surprise']) })),
    });
  }
  return out;
}

/** The last thing this party said of a kind, as the observer may see it. */
function lastOf(
  w: World,
  kind: EventKind,
  party: string,
  sees: (e: Event) => boolean,
): Event | undefined {
  let held: Event | undefined;
  for (const e of w.journal.ofKind(kind)) {
    if (!sees(e) || e.subjects[0] !== party) continue;
    held = e;
  }
  return held;
}

function ratingsOf(w: World, depth: number, sees: (e: Event) => boolean): readonly RatingView[] {
  const text = (v: unknown): string => (typeof v === 'string' ? v : '');
  return w.journal.recentOfKind('rating.action', depth, sees).map((e) => ({
    assessor: text(e.data['assessor']),
    subject: text(e.data['subject']) === '' ? text(e.data['instrument']) : text(e.data['subject']),
    grade: text(e.data['grade']),
    was: typeof e.data['was'] === 'string' ? e.data['was'] : null,
    period: e.period,
  }));
}
