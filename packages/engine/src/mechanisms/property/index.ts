/**
 * Commercial property: space built to LET, a lease with a term, a landlord sector, and the loan
 * secured on the building.
 *
 * @spec Housing A3 Housing A4 Housing B5 Capital Programme A2 Capital Programme C1 Corporate Credit A1 Clearing C3 Clearing C4.b XI-15 Law 1 Law 3 Law 8 Law 15 Law 19
 *
 * A WAREHOUSE WAS `STORAGE` PLANT A FIRM BUILT FOR ITS OWN STOCK, and nobody built space to let: no
 * landlord, no commercial rent, no lease with a term, no lending secured on a building, and a
 * retail firm sold from nowhere (worklist 13m). The owner and the occupier of a premises can be
 * different parties, and then there is RENT — a real payment between them (Housing A3, which says
 * it of a house and is as true of a shop).
 *
 * THE LANDLORD IS A MASS SECTOR (XI-15): cells of landlords with a lattice, the third profile of
 * a kind that is a population after households and small firms. A landlord holds premises — the
 * plant kind a shop, a surgery or an office is (Capital Programme A2) — and lets what it is not
 * letting already, in a book per place, at its own reservation: the rent the book last struck
 * where it has struck one, and never below what a unit of its premises costs it to hold (the
 * wear, B1.a's "never below what it costs"). A TENANT is a firm whose own plan said its capacity
 * is bound by premises: it bids for the units that would lift the bound to the rate it wants to
 * run at, at what a unit earns it a period — its margin on a unit of output over the units of
 * premises that output takes — which is the tenant's own reason and never a valuation of the
 * building (Clearing A2). What clears is a LEASE: a row from the tenant to the landlord, so many
 * units at the struck rent per period until a day, and the tenant runs on the leased units as on
 * its own (`registry/physical.ts rentedRoom`). The rent is collected every period; a rent not paid
 * is the kernel's arrear; a lease ends when its day passes or a side of it ceases.
 *
 * A BUILDING IS BUILT TO LET: a landlord whose premises are all let bids for buildings at what the
 * rent it sees is worth to it over its own horizon at its own cost of capital, ground first as a
 * firm's project does (15.1), and the capital programme commissions what it bought into premises
 * on the ground it holds. What it cannot pay for it asks its bank for, SECURED ON THE PREMISES IT
 * HOLDS (Corporate Credit A1: the one door every borrower uses) — CRE lending as a secured row,
 * the bank's to grant or refuse out of its own view.
 */
import type { Order } from '../../clearing/solver.js';
import { clear, isCleared } from '../../clearing/solver.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { period } from '../../calendar/calendar.js';
import { addDays, compareCivil } from '../../calendar/civil.js';
import { yearFraction } from '../../calendar/daycount.js';
import { moneyInstrumentId, partyId, type CurrencyCode, type InstrumentId, type PartyId, type RegionId } from '../../core/ids.js';
import {
  amountOf,
  asPerPiece,
  asRatio,
  heldAsMoney,
  minus,
  over,
  type Cash,
  type PerPiece,
  scale,
  valueAt,
} from '../../core/measure.js';
import { atMost, div, largest, raised } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { asQty, downTick, splitOnTick, subQty, upTick, type Qty } from '../../core/tick.js';
import type { AgreementKindDecl } from '../../register/agreements.js';
import { gridPerMember, weightOf } from '../../parties/party.js';
import { costOfCapital } from '../../registry/capital.js';
import { expectedPriceOf } from '../../registry/expectation.js';
import { banksAwayFromTrouble } from '../../registry/switching.js';
import type { LatticeDecl, LatticeReads } from '../../registry/lattice.js';
import type { ParamDecl } from '../../registry/params.js';
import type { PartyKindProfile } from '../../registry/kinds.js';
import { hectaresOf, hectaresUnder, landId } from '../../registry/land.js';
import {
  CAPITAL_KINDS,
  SEED_PLANT_AGES,
  capitalKindOf,
  goodId,
  goodTerms,
  groundUnderPlant,
  lifeParam,
  plantParam,
  plantUnitId,
  seedVintage,
  vintagesHeld,
  wearPerPlantUnit,
} from '../../registry/physical.js';
import { FIRM, MONEY_KIND } from '../../registry/profiles.js';
import {
  LANDLORD,
  LEASE_ROW,
  PREMISES,
  PROPERTY_EDGES,
  PROPERTY_PARAMS,
  RENT_PRINT,
  isLeaseTerms,
  lettingsVenue,
  planOf,
  rentStruckIn,
  unitsLetBy,
  type LeaseTerms,
} from '../../registry/property.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

export { LANDLORD, LEASE_ROW, PREMISES, PROPERTY_PARAMS, RENT_PRINT as PREMISES_RENT_PRINT, lettingsVenue } from '../../registry/property.js';

/** Law 9: a place's landlords are named for the bank they book at, as a pool of small firms is. */
export const landlordIdFor = (bank: PartyId): PartyId => partyId(`landlord.${String(bank)}`);
export const LEASE_SIGNED = 'property.leased';
export const RENT_PAID = 'property.rentPaid';
export const LANDLORD_PLAN = 'property.plan';

/**
 * XI-15: THE THIRD LATTICE. A landlord is somewhere and banks somewhere, and its size is what it
 * holds per member — the edge a RESOLUTION, tested by invariance like every edge (0f.10).
 */
export const LANDLORD_LATTICE: LatticeDecl = {
  kind: LANDLORD,
  categorical: [
    { dim: 'region', movedBy: 'entry', why: 'a building is somewhere, and a landlord lets where its buildings are' },
    { dim: 'bank', movedBy: 'bank.choice', why: 'a landlord borrows against its buildings from the bank it books at' },
  ],
  banded: [
    {
      dim: 'size',
      quantity: (reads: LatticeReads, cell: PartyId): Option<number> => some(reads.cashPerMember(cell, reads.homeCurrency(cell))),
      edges: PROPERTY_EDGES.size,
      why: 'a landlord with cash builds and one without borrows or waits; the size distribution is the state',
    },
  ],
};

export const landlordKind: PartyKindProfile = {
  objective: 'theResidual',
  id: LANDLORD,
  representation: 'cell',
  lattice: LANDLORD_LATTICE,
  moneyIssuer: null,
  fails: ['cash', 'solvency'],
  borrows: true,
  buysOnTerms: false,
  depositClass: 'corporate',
};

export const leaseRowKind: AgreementKindDecl = {
  id: LEASE_ROW,
  what: 'a named tenant holding a named landlord’s premises, so many units at a rent per period, until a day',
  // XI-8: a lease runs with the building; a landlord's successor that holds it is the landlord now.
  binds: 'aGoingConcern',
};

/* --------------------------------------------------------------------------------------------
 * READS
 * ------------------------------------------------------------------------------------------ */

/** What this landlord holds of premises, in pieces, off its own vintages (Law 19). */
function premisesHeld(view: ParticipantView): Qty {
  let out = 0;
  for (const v of vintagesHeld(view, view.calendar.startOf(view.period))) {
    if (v.capitalKind === PREMISES && v.periodsLeft > 0) out += v.units;
  }
  return asQty(out, 'the premises it holds');
}

/** What it has to let: what it holds less what it has let already. */
export function spareOf(view: ParticipantView): Qty {
  const held = premisesHeld(view);
  const out = unitsLetBy(view.commitments(), view.self.id, PREMISES);
  return held > out ? subQty(held, asQty(out, 'what it has let'), 'what it has to let') : asQty(0, 'nothing to let');
}

/**
 * B1.a, Law 3: THE LANDLORD'S RESERVATION — the rent the book last struck where it has struck one,
 * and never below what a unit of its premises costs it a period to hold (the wear). A landlord
 * with buildings and no print asks its cost; one that has seen the book asks the book.
 */
export function reservationOf(view: ParticipantView, region: RegionId): Option<PerPiece> {
  const wear = wearPerPlantUnit(vintagesHeld(view, view.calendar.startOf(view.period)), PREMISES);
  const struck = rentStruckIn(view, region);
  if (!wear.some && !struck.some) return none<PerPiece>();
  const levels = [wear.some ? wear.value : 0, struck.some ? struck.value : 0];
  const level = largest(levels, 'the higher of its cost and the book');
  return level > 0 ? some(asPerPiece(level, 'what it asks a unit')) : none<PerPiece>();
}

/**
 * A3, Capital Programme A2, Clearing A2: THE TENANT'S BID. A firm whose own plan said its capacity
 * binds on premises wants the units that lift it to the rate it wants to run at, and a unit is
 * worth to it a period what a unit of its output earns over the premises that output takes.
 */
export function tenantOrders(view: ParticipantView, venue: VenueDecl): readonly Order[] {
  if (venue.clearedBy !== 'property') return [];
  const region = venue.key['region'] as RegionId | undefined;
  if (region === undefined || view.self.region !== region) return [];
  const plan = planOf(view, view.period);
  if (!plan.some || plan.value.bound !== PREMISES) return [];
  const output = view.instruments.get(plan.value.output as InstrumentId);
  const terms = goodTerms(output);
  const need = terms.recipe.plant.find((p) => p.capitalKind === PREMISES);
  if (need === undefined) return [];
  const perUnit = view.params.ratio(plantParam(terms.subUnit, PREMISES));
  if (perUnit <= 0) return [];
  const gap = plan.value.runRate - plan.value.capacity;
  if (gap <= 0) return [];
  // Law 8: whole pieces of premises, and UP — a room short of what the gap needs does not close it.
  const units = upTick(scale(asQty(gap, 'the output it is short of'), asRatio(perUnit, 'the premises a unit takes'), 'the premises it is short of'));
  if (units <= 0) return [];
  const margin = plan.value.expectedPrice - plan.value.unitCost;
  if (margin <= 0) return [];
  const bid = rentOnGrid(view, venue.ccy, asPerPiece(div(margin, perUnit, 'what a piece of premises earns it a period'), 'its bid a piece a period'));
  if (bid <= 0) return [];
  return [{ party: view.self.id, side: 'buy', price: bid, qty: units }];
}

export function landlordOrders(view: ParticipantView, venue: VenueDecl): readonly Order[] {
  // Clearing B2: its book and no other — a venue of the place is not thereby a lettings book.
  if (venue.clearedBy !== 'property') return [];
  const region = venue.key['region'] as RegionId | undefined;
  if (region === undefined || view.self.region !== region) return [];
  const spare = spareOf(view);
  if (spare <= 0) return [];
  const ask = reservationOf(view, region);
  if (!ask.some) return [];
  const price = rentOnGrid(view, venue.ccy, ask.value);
  if (price <= 0) return [];
  return [{ party: view.self.id, side: 'sell', price, qty: spare }];
}

/**
 * Law 8: A RENT IS QUOTED PER NAMED UNIT OF PREMISES ON THE MONEY'S OWN GRID — a room lets for so
 * many whole cents a period — and the book counts pieces of a room, so the level it clears at is
 * that rent over the pieces a room is. A piece of a room rents for less than a piece of money a
 * period; quoting it on the plant's whole-piece grid rounded every ask to nothing.
 */
function rentOnGrid(view: ParticipantView, ccy: CurrencyCode, perPiece: PerPiece): PerPiece {
  const pieces = view.registry.subdivision(plantUnitId(PREMISES));
  const perUnit = view.registry.onQuoteGrid(MONEY_KIND, ccy, asPerPiece(perPiece * pieces, 'the rent of a whole unit, a period'));
  return asPerPiece(div(perUnit, pieces, 'the rent of a piece of it'), 'the rent a piece a period');
}

/* --------------------------------------------------------------------------------------------
 * THE LETTINGS BOOK
 * ------------------------------------------------------------------------------------------ */

/** The last rent this book struck and when, off its own record — carried onto a print that struck none. */
function lastStruck(ctx: MechanismContext, venue: VenueDecl): { readonly rentPerUnit?: number; readonly struckIn?: number } {
  const said = ctx.journal.forSubject(RENT_PRINT, String(venue.id));
  for (let n = said.length - 1; n >= 0; n -= 1) {
    const e = said[n];
    const rent = e?.data['rentPerUnit'];
    const at = e?.data['struckIn'];
    if (typeof rent === 'number' && rent > 0 && typeof at === 'number') return { rentPerUnit: rent, struckIn: at };
  }
  return {};
}

function letIn(ctx: MechanismContext, venue: VenueDecl): void {
  const region = venue.key['region'] as RegionId | undefined;
  if (region === undefined) return;
  const posted = ctx.posted(venue.id);
  const orders = posted.filter((o) => o.price !== 'market');
  const outcome = clear(orders, 'proRata', 'marginalBid');
  // Law 8, XI-6: a session that does not clear leaves the LAST rent struck on the record with the
  // period it was struck in, so a reader sees the level and how stale it is, never a gap.
  const carried = lastStruck(ctx, venue);
  if (!isCleared(outcome)) {
    ctx.record(RENT_PRINT, [venue.id], { venue: venue.id, region, outcome: outcome.kind, ...carried, bids: orders.filter((o) => o.side === 'buy').length, asks: orders.filter((o) => o.side === 'sell').length }, true);
    return;
  }
  let struck: PerPiece | undefined;
  for (const f of outcome.fills) {
    if (f.side !== 'buy' || f.qty <= 0) continue;
    if (struck === undefined || f.at < struck) struck = f.at;
  }
  if (struck === undefined) {
    ctx.record(RENT_PRINT, [venue.id], { venue: venue.id, region, outcome: 'noDemand', ...carried }, true);
    return;
  }
  // Clearing C3: landlords with the least to ask are let first, tenants with the most to pay first.
  const owners = outcome.fills.filter((f) => f.side === 'sell' && f.qty > 0).sort((a, b) => a.at - b.at);
  const tenants = outcome.fills.filter((f) => f.side === 'buy' && f.qty > 0).sort((a, b) => b.at - a.at);
  const until = addDays(ctx.calendar.startOf(ctx.period), ctx.params.periods(PROPERTY_PARAMS.leaseTerm) * ctx.calendar.periodDays);
  let at = 0;
  let left: Qty = owners[0]?.qty ?? asQty(0);
  let signed = 0;
  for (const t of tenants) {
    let want = t.qty;
    while (want > 0 && at < owners.length) {
      const owner = owners[at];
      if (owner === undefined) break;
      if (left <= 0) {
        at += 1;
        left = owners[at]?.qty ?? asQty(0);
        continue;
      }
      const offered = asQty(atMost(want, left, 'there is no more of it to let than there is'));
      /**
       * XI-15, Law 8: A LANDLORD IS A CELL, and a lease with it is a lease with every member alike —
       * so the units let are a whole number a member, and the rent a member is a whole number of
       * pieces of money a period or it cannot be paid (`collect`). A fill too small for that is
       * turned away and said (`property.unlet`): nobody lets a fortieth of a room for a quarter of
       * a cent.
       */
      const owner_ = ctx.parties.get(owner.party);
      const perMember = downTick(over(asQty(offered, 'the units offered'), asRatio(weightOf(owner_), 'the landlords it stands for'), 'units a landlord'));
      const rentPerMember = downTick(valueAt(struck, perMember, 'the rent a landlord a period'));
      if (perMember <= 0 || rentPerMember <= 0) {
        ctx.record('property.unlet', [String(t.party), String(owner.party)], { tenant: t.party, landlord: owner.party, region, offered, why: 'below a piece a landlord' }, true);
        at += 1;
        left = owners[at]?.qty ?? asQty(0);
        continue;
      }
      const units = asQty(perMember * weightOf(owner_), 'the units let, whole a member');
      const terms: LeaseTerms = { kind: LEASE_ROW, region, capitalKind: PREMISES, units, rentPerUnit: struck, until };
      ctx.owes({
        debtor: t.party,
        creditor: owner.party,
        ccy: venue.ccy,
        owed: 0,
        terms,
        why: `${String(t.party)} leases ${String(units)} of premises from ${String(owner.party)} in ${String(region)}`,
      });
      ctx.record(LEASE_SIGNED, [String(t.party), String(owner.party)], { tenant: t.party, landlord: owner.party, region, units, rentPerUnit: struck, until }, true);
      signed += 1;
      want = subQty(want, units, 'what it is still short of');
      left = subQty(left, units, 'what this owner has left');
    }
  }
  ctx.record(
    RENT_PRINT,
    [venue.id],
    { venue: venue.id, region, outcome: 'cleared', rentPerUnit: struck, struckIn: ctx.period, units: outcome.volume, leases: signed, asks: orders.filter((o) => o.side === 'sell').map((o) => o.price) },
    true,
  );
}

/**
 * A3, Law 5, Law 8: THE RENT IS COLLECTED, tenant to landlord, every period, and a lease whose day
 * has passed or whose side has ceased ends. A landlord is a cell, so what reaches it is a whole
 * number of pieces a member (XI-15): the rent struck is per unit, and what the grid leaves below a
 * piece a member is not paid — there is no such coin.
 */
function collect(ctx: MechanismContext): void {
  const today = ctx.calendar.startOf(ctx.period);
  for (const row of ctx.agreements.ofKind(LEASE_ROW)) {
    if (row.state !== 'performing' || !isLeaseTerms(row.terms)) continue;
    const tenant = ctx.parties.resolve(row.debtor);
    const landlord = ctx.parties.resolve(row.creditor);
    if (!tenant.status.alive || !landlord.status.alive) {
      ctx.endAgreement(row.id, 'a side of it has ceased');
      continue;
    }
    if (compareCivil(row.terms.until, today) <= 0) {
      ctx.endAgreement(row.id, 'the term ended');
      continue;
    }
    const whole = valueAt(row.terms.rentPerUnit, row.terms.units, 'the rent on this lease');
    const perMember = over(whole, asRatio(weightOf(landlord), 'the landlords it stands for'), 'per landlord');
    const share = gridPerMember(ctx.registry, landlord, perMember);
    if (share.total <= 0) {
      // Law 8: a rent below a piece a landlord — a cell that has merged since the lease was
      // signed — cannot be paid, and the row says so rather than nothing.
      ctx.record(RENT_PAID, [String(tenant.id), String(landlord.id)], { lease: row.id, tenant: tenant.id, landlord: landlord.id, amount: 0, paid: false, why: 'below a piece a landlord' }, false);
      continue;
    }
    const r = ctx.settle({
      legs: [{ kind: 'money', from: ctx.accountOf(tenant.id, row.ccy), to: ctx.accountOf(landlord.id, row.ccy), receipt: { of: 'rent' }, ccy: row.ccy, amount: share.total }],
      cause: 'transfer',
      reason: `rent on ${String(row.id)}`,
    });
    ctx.record(RENT_PAID, [String(tenant.id), String(landlord.id)], { lease: row.id, tenant: tenant.id, landlord: landlord.id, amount: share.total, paid: r.outcome === 'settled' }, false);
  }
}

/* --------------------------------------------------------------------------------------------
 * BUILDING TO LET, AND THE LOAN SECURED ON IT
 * ------------------------------------------------------------------------------------------ */

export interface Build {
  readonly good: InstrumentId;
  readonly asking: PerPiece;
  readonly bid: PerPiece;
  readonly wanted: Qty;
  readonly hectaresShort: Qty;
  readonly outlay: Cash;
  /** What a building a member would cost over what it holds — what it asks its bank for. */
  readonly short: Cash;
}

/**
 * A4, B5, Capital Programme B1, XI-4: WHETHER TO BUILD, AND WHAT A BUILDING IS WORTH TO IT. A
 * landlord whose premises are all let — the reason to build — prices a building at the rent its
 * place's book struck, a period, over its own horizon at its own cost of capital: a schedule of
 * rents discounted, which is what a building is worth to whoever will let it and never what the
 * market says it is worth (Clearing A2). It bids when that exceeds what a building is asking, for
 * the buildings its money reaches at the asking, on the ground it holds — and for the ground first
 * where it holds too little (15.1).
 */
export function buildOf(view: ParticipantView, ccy: CurrencyCode): Option<Build> {
  if (spareOf(view) > 0) return none<Build>();
  const rent = rentStruckIn(view, view.self.region);
  if (!rent.some) return none<Build>();
  const kind = capitalKindOf(CAPITAL_KINDS, PREMISES);
  if (kind === undefined) return none<Build>();
  const good = goodId(kind.madeFrom, view.self.region);
  if (!view.instruments.has(good)) return none<Build>();
  const asking = expectedPriceOf(view, good);
  if (!asking.some || asking.value <= 0) return none<Build>();
  const horizon = view.params.periods(PROPERTY_PARAMS.horizon);
  const cost = costOfCapital(view, period(view.period - 1), horizon);
  if (!cost.some) return none<Build>();
  const year = yearFraction('ACT/365F', view.calendar.startOf(view.period), view.calendar.startOf(period(view.period + 1)));
  const life = view.params.periods(lifeParam(PREMISES));
  // A building does not earn past its life: the periods it counts are arithmetic impossibility beyond that.
  const counted = atMost(horizon, life, 'a building earns no rent after it is worn out');
  // What a period of rent is worth today, summed to its horizon: the annuity at its own cost.
  const r = cost.value.perAnnum * year;
  const annuity = r > 0 ? div(1 - raised(1 + r, -counted, 'what the last period of rent is worth today'), r, 'the rents to its horizon, discounted') : counted;
  const bid = view.registry.onQuoteGrid(view.instruments.get(good).kind, ccy, asPerPiece(rent.value * annuity, 'what a building is worth to it'));
  if (bid <= asking.value) return none<Build>();
  const cash = heldAsMoney(view.cash(ccy), 'what it holds');
  // XI-15, Law 8: a landlord builds a building at a time — one a member a period, the pace of a
  // sector that adds to its stock rather than doubling it — and buys what its money reaches of that.
  const pace = asQty(weightOf(view.self), 'a building a member');
  const affordable = downTick(amountOf(cash, asking.value, 'the buildings its money reaches'));
  const wanted = atMost(affordable, pace, 'no more than its pace');
  const short = minus(valueAt(asking.value, pace, 'what its pace costs'), cash, 'what it cannot pay for');
  if (wanted <= 0 && short <= 0) return none<Build>();
  const reads = { registry: view.registry, params: view.params };
  const standing = hectaresOf(groundUnderPlant(reads, vintagesHeld(view, view.calendar.startOf(view.period))));
  const land = landId(view.self.region);
  const held = view.instruments.has(land) ? view.quantity(land) : asQty(0, 'a world with no ground line');
  const free = held > standing ? subQty(held, standing, 'the ground it holds beyond its buildings') : asQty(0, 'no ground beyond its buildings');
  const needed = view.instruments.has(land) ? hectaresUnder(reads, PREMISES, wanted) : asQty(0, 'no ground line');
  const hectaresShort = needed > free ? subQty(needed, free, 'the hectares it is short of') : asQty(0, 'ground enough');
  return some({ good, asking: asking.value, bid, wanted, hectaresShort, outlay: valueAt(asking.value, wanted, 'what it expects to pay'), short });
}

function buildOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  const ccy = view.registry.currencyOf(view.self.region);
  const build = buildOf(view, ccy);
  if (!build.some) return [];
  const b = build.value;
  if (m.instrument === b.good) {
    // Ground first: a building it has nowhere to stand is a building it does not buy yet.
    if (b.hectaresShort > 0 || b.wanted <= 0) return [];
    return [{ party: view.self.id, side: 'buy', price: b.bid, qty: b.wanted }];
  }
  if (m.instrument === landId(view.self.region) && b.hectaresShort > 0) {
    // 15.1: the residual — what the buildings are worth over what they ask, over the ground they take.
    const surplus = minus(valueAt(b.bid, b.wanted, 'what the buildings are worth to it'), b.outlay, 'the surplus over the asking');
    const needed = hectaresUnder({ registry: view.registry, params: view.params }, PREMISES, b.wanted);
    if (surplus <= 0 || needed <= 0) return [];
    const level = view.registry.onQuoteGrid(view.instruments.get(m.instrument).kind, m.ccy, asPerPiece(div(surplus, needed, 'what a hectare is worth to it'), 'its bid a hectare'));
    if (level <= 0) return [];
    return [{ party: view.self.id, side: 'buy', price: level, qty: b.hectaresShort }];
  }
  return [];
}

/**
 * Corporate Credit A1, Housing C1 (15.3): CRE LENDING AS A SECURED ROW. What a landlord cannot pay
 * for out of what it holds it asks its bank for, secured on the premises it holds — the one door
 * every borrower publishes through, and the bank's own view decides. Paid down on a schedule, as a
 * mortgage is.
 */
function askForLoans(ctx: MechanismContext): void {
  for (const cell of ctx.parties.ofKind(LANDLORD)) {
    if (!cell.status.alive) continue;
    const view = ctx.participant(cell.id);
    const ccy = ctx.registry.currencyOf(cell.region);
    const build = buildOf(view, ccy);
    if (!build.some || build.value.short <= 0) continue;
    const security = vintagesHeld(view, ctx.calendar.startOf(ctx.period))
      .filter((v) => v.capitalKind === PREMISES)
      .map((v) => ({ instrument: v.instrument as InstrumentId, qty: v.units }));
    if (security.length === 0) continue;
    ctx.request(cell.id, { ccy, short: build.value.short, security, repays: 'onSchedule' });
    ctx.record(LANDLORD_PLAN, [cell.id], { landlord: cell.id, region: cell.region, asking: build.value.asking, bid: build.value.bid, wanted: build.value.wanted, hectaresShort: build.value.hectaresShort, short: build.value.short, secured: security.length }, true);
  }
}

/* --------------------------------------------------------------------------------------------
 * THE MODULE
 * ------------------------------------------------------------------------------------------ */

function paramsOf(): ParamDecl[] {
  return [
    {
      id: PROPERTY_PARAMS.landlordsPerBank,
      value: 200,
      unit: 'landlords',
      dimension: 'count',
      kind: 'placeholder',
      owner: 'model',
      standsInFor: { mechanism: 'Seed C4, XI-15: the sector’s opening population is a stock the opening states', item: '22a' },
      why: 'Housing A3, XI-15 (15.3): how many landlords bank at each bank at the opening. A SHAPE with its death at 22a: who owns the space a town lets is an outcome of who built it and who bought it, and the opening states the stock the mechanisms then act on.',
    },
    {
      id: PROPERTY_PARAMS.premisesPerLandlord,
      value: 25,
      unit: 'units of premises a landlord',
      dimension: 'count',
      kind: 'placeholder',
      owner: 'model',
      standsInFor: { mechanism: 'Seed C4: the premises a landlord opens holding is a stock the opening states', item: '22a' },
      why: 'Housing A3, A4 (15.3): what a landlord opens holding of premises — rooms, built to let. A SHAPE with its death at 22a: the stock is a register of units with owners, moved by what changes hands and what is built, and the opening is where it starts.',
    },
    {
      id: PROPERTY_PARAMS.leaseTerm,
      value: 52,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'Housing A3 (15.3): how long a lease of premises runs for — a year, a convention of the contract stated with it. A lease has a TERM, which is what makes a tenant’s premises a commitment rather than a period’s purchase, and what lets the landlord re-let when it ends.',
    },
    {
      id: PROPERTY_PARAMS.horizon,
      value: 520,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Capital Programme B1, XI-4 (15.3): how far a landlord counts the rent when it prices a building — ten years, at its own cost of capital. A PREFERENCE: what the building is worth to it is those rents discounted, and a landlord that counted further would pay more for the same building.',
    },
    {
      id: PROPERTY_PARAMS.switchingCost,
      value: 250,
      denominated: 'money',
      unit: 'of the money the account is in, per move',
      dimension: 'amount',
      kind: 'preference',
      owner: 'model',
      why: 'Banks Funding A1.b, A1.d, E1 (15.3): what it costs one landlord to move the account its rents land in — a named firm’s cost, because a landlord has as many counterparties to tell (every tenant) and as much to redirect. Weighed against the money it would lose if its bank failed, which nothing insures (E4).',
    },
    {
      id: PROPERTY_EDGES.size[0],
      value: 1_000_000,
      unit: 'pieces of money per member',
      dimension: 'amount',
      denominated: 'money',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the landlord lattice on size. A RESOLUTION: refine every edge by two and the world’s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
  ];
}

export function property(): SystemModule {
  const mine = (v: VenueDecl): boolean => v.clearedBy === 'property';
  return {
    id: 'property',
    spec: 'Housing A3 Capital Programme A2 Capital Programme C1',
    // It needs the premises kind and the buildings they are made from, the firms that rent them and
    // the ground they stand on; its seed runs before the land seed, which gives its cells the ground
    // under their opening premises.
    requires: ['capital-programme', 'goods', 'firms', 'seed.foundation'],
    instrumentKinds: [],
    agreementKinds: [leaseRowKind],
    partyKinds: [landlordKind],
    curveFamilies: [],
    units: [],
    params: paramsOf(),
    phases: [
      {
        /**
         * A3, Clearing C3: THE LETTINGS BOOK of each place clears before the goods markets, so a
         * tenant that took premises this period runs on them from the next (a lease is a
         * commitment, and what it lets the tenant make is read next period's plan).
         */
        name: 'property.let',
        spec: 'Housing A3 Housing B5 Clearing C3 Clearing C4.b',
        anchor: { before: 'markets' },
        reads: [{ kind: 'event', name: 'firms.plan', of: 'thisPeriod' }],
        writes: [{ kind: 'event', name: RENT_PRINT }, { kind: 'event', name: LEASE_SIGNED }, { kind: 'event', name: 'property.unlet' }],
        run: (ctx: MechanismContext): void => {
          for (const v of ctx.venues.filter(mine)) {
            ctx.gather(v.id);
            letIn(ctx, v);
          }
        },
      },
      {
        name: 'property.collect',
        spec: 'Housing A3 Law 5',
        anchor: { after: 'markets' },
        reads: [],
        writes: [{ kind: 'event', name: RENT_PAID }],
        run: (ctx: MechanismContext): void => {
          collect(ctx);
        },
      },
      {
        name: 'property.ask',
        spec: 'Corporate Credit A1 Housing C1',
        anchor: { after: 'corporateActions' },
        reads: [{ kind: 'event', name: RENT_PRINT, of: 'anyPeriod' }],
        writes: [{ kind: 'event', name: 'credit.request' }, { kind: 'event', name: LANDLORD_PLAN }],
        run: (ctx: MechanismContext): void => {
          askForLoans(ctx);
        },
      },
    ],
    // Banks Funding A1.d, E1: where a landlord banks is a decision it revisits when its bank is in trouble.
    bankChoices: [{ partyKind: LANDLORD, chooses: (view: ParticipantView) => banksAwayFromTrouble(view, PROPERTY_PARAMS.switchingCost, 'moneyMarket.window') }],
    participants: [
      {
        // A4, Capital Programme C1: a landlord buys buildings to let, and the ground under them first.
        partyKind: LANDLORD,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => buildOrders(view, m),
      },
    ],
    venueParticipants: [
      { partyKind: LANDLORD, orders: (view, venue) => landlordOrders(view, venue) },
      { partyKind: FIRM, orders: (view, venue) => tenantOrders(view, venue) },
    ],
    families: [],
    seed(ctx: SeedContext): void {
      const kind = capitalKindOf(CAPITAL_KINDS, PREMISES);
      if (kind === undefined) return;
      for (const region of ctx.registry.regions.values()) {
        const good = goodId(kind.madeFrom, region.id);
        if (!ctx.instruments.has(good)) continue;
        ctx.openVenue({
          id: lettingsVenue(region.id),
          name: `Premises to let, ${region.name}`,
          clearedBy: 'property',
          unit: plantUnitId(PREMISES),
          ccy: ctx.registry.currencyOf(region.id),
          key: { region: region.id, capitalKind: PREMISES },
        });
      }
      /**
       * Seed C4, XI-15: THE LANDLORDS OF EACH BANK, as a cell with a count, each member holding its
       * opening premises spread over three vintages of different ages and carried at what is left
       * of what a new building costs — the same sentence the small-firms seed makes of a pool's
       * plant. The ground under them is the land seed's to give (15.1), which runs after this.
       */
      for (const bank of [...ctx.parties.all()].filter((p) => ctx.registry.issuesMoney(p.kind) && p.bank !== p.id)) {
        const region = bank.region;
        const good = goodId(kind.madeFrom, region);
        if (!ctx.instruments.has(good)) continue;
        const newPrice = ctx.prices.latest(good, ctx.period);
        if (!newPrice.some) continue;
        const count = ctx.params.count(PROPERTY_PARAMS.landlordsPerBank);
        if (count <= 0) continue;
        const id = landlordIdFor(bank.id);
        const ccy = ctx.registry.currencyOf(region);
        // 15.4: the foundation makes the cell and its rooms before the shops lease them; what it
        // cannot give there is the cash, which comes off the bank's sheet before it is built. So the
        // cash is given here, once, to a cell that has none — and a world without the foundation
        // gets the whole landlord here, as before.
        const opening = heldAsMoney(ctx.registry.payable(valueAt(newPrice.value.price, asQty(1, 'one building'), 'a building a member')), 'what it opens with');
        if (ctx.parties.has(id)) {
          if (ctx.register.quantity(id, moneyInstrumentId(ctx.parties.get(bank.id).id, ccy)) <= 0) ctx.endowMoney(id, ccy, opening);
          continue;
        }
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
        const life = ctx.params.periods(lifeParam(PREMISES));
        const perMember = asQty(ctx.params.count(PROPERTY_PARAMS.premisesPerLandlord) * ctx.registry.subdivision(plantUnitId(PREMISES)), 'the premises a landlord opens with, in pieces');
        const perVintage = splitOnTick(perMember, SEED_PLANT_AGES.map(() => 1));
        SEED_PLANT_AGES.forEach((age, at) => {
          const units = perVintage[at];
          if (units === undefined || units <= 0) return;
          const serviceDate = addDays(ctx.calendar.epoch, -age * ctx.calendar.periodDays);
          const vintage = seedVintage(ctx, kind, region, serviceDate);
          // Capital Programme A3, A6: carried at the straight line it has been on since it went into service.
          ctx.endowUnits(id, vintage, units, (newPrice.value.price * (life - age)) / life);
        });
        // A landlord opens with a building's worth in hand so a building it wants is not its first
        // question to the bank: one unit's asking a member, the smallest stock that lets it act.
        ctx.endowMoney(id, ccy, opening);
      }
    },
  };
}
