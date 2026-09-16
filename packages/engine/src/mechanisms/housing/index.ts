/**
 * Housing: who lives where, what they pay for it, and who owns the roof.
 *
 * @spec Housing A1 Housing A1.a Housing A2 Housing A3 Housing A4 Housing A5 Housing B1 Housing B2 Housing B5 Housing D1 Housing E1 Seed D1 Households A2.b Households E1 Households E5 Labour A2 Banks Lending A2 Clearing A2 Clearing B2 Clearing C4.b Expectations A2 Law 2 Law 3 Law 5 Law 8 Law 15 Law 19 XI-8 XI-15
 *
 * A DWELLING IS A GOOD (13d). It is built by named builders out of concrete, timber, steel and
 * glass, it takes half a year, it stands where it was built and it wears out — so there is no
 * dwelling from nowhere, no dwelling that teleports, and the stock is what the register holds. All
 * of that is the goods machinery doing what it already did; what this module adds is the two things
 * a house has that a tonne of grain does not: somebody LIVES in it, and somebody else may own it.
 *
 * A TENANCY IS A VENUE, not a market (A2, A3). What changes hands is not the dwelling — the
 * register would say so — it is the right to be in it for a period, which is exactly what a labour
 * venue sells and exactly why that is the shape used here: the owner keeps the asset, the tenant
 * gets the use, and the rent is struck where the two books cross.
 *
 * WHAT EACH SIDE WILL DO IS A READ, and there is no coefficient in either (Law 3):
 *
 *   THE OWNER will let above what letting COSTS it, which is the wear a period of occupation puts
 *   on the thing — the dwelling's own declared spoilage against the dwelling's own cleared price,
 *   both of them things this world already publishes. Below that it would rather leave it empty.
 *
 *   THE TENANT will pay up to what it has, because the alternative is nowhere to live. That is the
 *   same statement the labour venue makes about a reservation wage (B1.a) read from the other side,
 *   and it is its own outlook of its own income over the dwellings a member needs.
 *
 * So rent clears between the wear at the bottom and what people can pay at the top, and where in
 * that range it lands is how many dwellings there are against how many households want one. A
 * shortage is dear rent, and what answers it is building — slowly, because a dwelling takes half a
 * year (B2). Nothing here bounds the rent and nothing targets it.
 *
 * OWNER-OCCUPATION IS AN OUTCOME AND NOT A TENURE FLAG: a household that owns as many dwellings as
 * its members live in has nothing to rent and does not bid, so it pays nobody. A household that
 * owns none rents all of them. A party that owns more than it lives in is a landlord, and that is
 * the whole of what makes one.
 *
 * A RENTAL STOCK HAS DWELLINGS BEHIND IT (A3, 15.5). The landlord sector (15.3) opens holding
 * dwellings to let beside its premises, and offers in the same book a household does — the same
 * reads, the same reason, and no kind of its own. A tenancy has a TERM: when its day passes the roof
 * is re-let and a tenant still short bids again. And a landlord ENDS a tenancy when what it expects
 * to be paid on it — the rent struck, at its own view of whether this tenant pays — falls below what
 * letting the roof costs it: a comparison of two of its own reads, with no threshold anywhere.
 */
import {
  type Cash,
  type PerPiece,
  amountOf,
  asCash,
  asNamed,
  asPerPiece,
  asRatio,
  heldAsMoney,
  minus,
  over,
  pricedAt,
  scale,
  valueAt,
} from '../../core/measure.js';
import type { Order } from '../../clearing/solver.js';
import { clear, isCleared } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { agreementKindId, type AgreementId, unitId, type PartyId, type RegionId, type UnitId } from '../../core/ids.js';
import type { Agreement, AgreementTerms } from '../../register/agreements.js';
import { atMost, div, sum } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { addQty, asQty, downTick, NO_QTY, type Qty, subQty } from '../../core/tick.js';
import { TONNE_PIECES } from '../../registry/grid.js';
import type { Leg } from '../../ledger/instruction.js';
import { keyOf, weightOf, gridPerMember } from '../../parties/party.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import { goodId, spoilageParam } from '../../registry/physical.js';
import { LOAN, creditorOf, isLoan } from '../../registry/credit.js';
import { LANDLORD } from '../../registry/property.js';
import { addDays, compareCivil, type Civil } from '../../calendar/civil.js';
import { creditDefaults } from '../../registry/banking.js';
import { isTenancy, type TenancyTerms as PublicTenancyTerms } from '../../registry/funding.js';
import type { InstrumentId } from '../../core/ids.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import type { Family, Violation } from '../../audit/audit.js';
import { DWELLING, HOUSING_PARAMS, TENURE, rentVenue, type TenureDecl } from './data.js';
import { about } from '../../world/context.js';

export * from './data.js';

export const RENT_PRINT = 'housing.rent';

/**
 * A2, Law 8: WHAT A TENANCY IS COUNTED IN, and it is not dwellings. A dwelling is indivisible — it
 * is a whole thing and the register counts it in whole ones (Housing A1.a) — but the right to be in
 * one for a period is not: a cell of a million people takes four hundred thousand dwellings' worth
 * of occupancy, and stating that in whole houses per person would make it one each or none. So the
 * venue counts OCCUPANCY, in the same fine grid a tonne is counted in, and the dwelling itself stays
 * a whole thing in the register where it belongs.
 */
export const DWELLING_WEEKS = unitId('dwellingWeeks');

/**
 * A2, A3: ONE TENANCY. It is a row and not a number, for the reason an employment is (XI-10): a
 * rent that moves every period is a spot price for a roof, and what makes housing what it is in a
 * real economy is that the rent was struck once and the tenant is still paying it.
 */
export const TENANCY = agreementKindId('housing.tenancy');

/**
 * A2, A3, Law 15: what a tenancy says. The SHAPE is a public read (`registry/funding.ts`, 0f.7c):
 * a tenant reads what falls due on it off its own commitments, and it may not import this module.
 * What this module adds is the region the letting is in.
 */
/** Law 15: the public shape, and this module's region and day on it. */
const isOwnTenancy = (t: AgreementTerms): t is TenancyTerms => isTenancy(t) && 'region' in t && 'until' in t;

export interface TenancyTerms extends PublicTenancyTerms {
  readonly kind: typeof TENANCY;
  readonly region: RegionId;
  /** A2: struck at the letting and moving only by a new letting. Money per piece of occupancy, a period. */
  readonly rentPerDwelling: PerPiece;
  /** Law 8 (15.5): pieces of OCCUPANCY — the book's own unit (`DWELLING_WEEKS`), never the register's. */
  readonly dwellings: Qty;
  /** A3 (15.5): the day it runs to. When it passes the roof is re-let, and a tenant still short bids again. */
  readonly until: Civil;
}

/** 15.5: a landlord ended a tenancy, and said why — public, because a tenant put out is a fact of the place. */
export const TENANCY_ENDED = 'housing.tenancy.ended';

/**
 * One tenancy as this module reads it. The TENANT owes the rent, so it is the debtor and the
 * landlord the creditor — which is what makes an unpaid rent rank in a tenant's estate.
 */
export interface Lease extends TenancyTerms {
  readonly id: AgreementId;
  readonly tenant: PartyId;
  readonly landlord: PartyId;
}

export function leaseOf(a: Agreement): Lease {
  if (!isOwnTenancy(a.terms)) throw new TypeError(`Housing A2: ${a.id} is not a tenancy`);
  return { ...a.terms, id: a.id, tenant: a.debtor, landlord: a.creditor };
}

/**
 * A2, XI-8: the tenancies standing now, off the kernel's own book. It is a copy of the LIST and not
 * of the tenancies: `collect` ends one while walking it, and the walk still sees the one it was
 * about to end.
 */
export const leasesOf = (ctx: MechanismContext): readonly Lease[] =>
  ctx.agreements
    .ofKind(TENANCY)
    .filter((a) => a.state === 'performing')
    .map(leaseOf);

/**
 * Law 18: the same tenancies, found by the party they belong to. Every question here is about one
 * landlord or one tenant, and each was answered by walking every tenancy in the world — so a world
 * with more housing cost every party all of it. The lists hold the same rows the book holds and are
 * written where a tenancy is signed and where one ends, and nowhere else (Law 4).
 */
/**
 * A2, XI-8: A TENANCY BEGINS, and it is a commitment in the kernel's own book.
 *
 * It owes nothing the instant it is signed: the rent falls due at the end of the period and
 * `collect` moves it then, and a rent that does not arrive is settlement's failure and the tenancy
 * stands (A2). Two named parties, dated terms, a state — which is what it always was, kept where
 * only this module could see it.
 */
function signs(
  ctx: MechanismContext,
  d: { tenant: PartyId; landlord: PartyId; region: RegionId; rentPerDwelling: PerPiece; dwellings: Qty; until: Civil },
): void {
  const terms: TenancyTerms = {
    kind: TENANCY,
    region: d.region,
    rentPerDwelling: d.rentPerDwelling,
    dwellings: d.dwellings,
    until: d.until,
  };
  ctx.owes({
    debtor: d.tenant,
    creditor: d.landlord,
    ccy: ctx.registry.currencyOf(d.region),
    owed: 0,
    terms,
    why: `${d.tenant} rents ${d.dwellings} from ${d.landlord} in ${d.region}`,
  });
}

/** A tenancy ends. XI-8: terminated, so the record still says there was one. */
function ends(ctx: MechanismContext, lease: Lease, why: string): void {
  ctx.endAgreement(lease.id, why);
}

/** Law 8: the unit a dwelling is counted in, which is the instrument's own. */
function goodUnitOf(view: Pick<ParticipantView, 'instruments'>, region: RegionId): UnitId {
  return view.instruments.get(goodId(DWELLING, region)).unit;
}

/**
 * A2, Law 8 (15.5): THE REGISTER COUNTS DWELLINGS AND THE BOOK COUNTS OCCUPANCY, and a dwelling
 * gives one dwelling-week of occupancy a period — so the two grids meet here, once, and every
 * order, every tenancy and every rent in this module is in the book's own unit from here on.
 */
type UnitReads = Pick<ParticipantView, 'registry' | 'instruments'>;

function occupancyOf(view: UnitReads, region: RegionId, held: Qty): Qty {
  const perDwelling = view.registry.subdivision(goodUnitOf(view, region));
  // Law 8: multiply before dividing — both counts are whole and their product is exact, where a
  // quotient of them is not (the publish of a short of 2,112,800,000.0000002 pieces was the tell).
  return asQty(div(held * view.registry.subdivision(DWELLING_WEEKS), perDwelling, 'the occupancy the dwellings it holds give'), 'the occupancy it holds');
}

/** The other way: occupancy back to the register's pieces, for a reader that bids for the THING (`homeBid`). */
function dwellingsOf(view: UnitReads, region: RegionId, occupancy: number): number {
  const perWeek = view.registry.subdivision(DWELLING_WEEKS);
  return div(occupancy * view.registry.subdivision(goodUnitOf(view, region)), perWeek, 'the dwellings that occupancy is');
}

/** A3, D1: the occupancy this party OWNS in a place — the register's own answer, in the book's unit. */
function owned(view: ParticipantView, region: RegionId): Qty {
  const id = goodId(DWELLING, region);
  if (!view.instruments.has(id)) return NO_QTY;
  // XI-15, 0f.1: a cell's holding IS the total the people it stands for own between them.
  return occupancyOf(view, region, view.free(id));
}

/**
 * How many dwellings this party has LET OUT, and how many it has taken (Law 19: off the rows).
 *
 * Observer A4: it is a read of the party's OWN commitments now, through its own view — so a
 * participant answers for itself instead of this module answering for it out of a private book it
 * kept about everybody.
 */
const tenancies = (view: ParticipantView): readonly Lease[] =>
  view
    .commitments()
    .filter((a) => a.state === 'performing' && isOwnTenancy(a.terms))
    .map(leaseOf);

function letOut(view: ParticipantView): Qty {
  const me = view.self.id;
  return sum(tenancies(view).filter((l) => l.landlord === me).map((l) => l.dwellings)).value;
}

function taken(view: ParticipantView): Qty {
  const me = view.self.id;
  return sum(tenancies(view).filter((l) => l.tenant === me).map((l) => l.dwellings)).value;
}

/**
 * E1, XI-15: the occupancy the people in this cell live under, for the whole cell. Law 15: a cell
 * whose lattice has no cohort — a landlord's — houses nobody, which is the shape of its key and not
 * its kind.
 */
function needs(view: ParticipantView, rows: readonly TenureDecl[]): Qty {
  const self = view.self;
  if (self.representation !== 'cell' || !('cohort' in self.key)) return NO_QTY;
  const cohort = keyOf(self, 'cohort');
  const row = rows.find((r) => r.cohort === cohort);
  if (row === undefined) return NO_QTY;
  const per = view.params.ratio(HOUSING_PARAMS.perMember(cohort));
  /**
   * Law 8, 13c.1's lesson twice over: THE DECLARED RATIO IS IN NAMED UNITS AND THE STATE COUNTS IN
   * PIECES, so it is converted where the ratio is READ and nowhere else. A dwelling per member is
   * four tenths of a DWELLING, and what a register, a venue and a price all speak is pieces of one.
   */
  return view.registry.pieces(
    DWELLING_WEEKS,
    asNamed(
      scale(per, asRatio(weightOf(self), 'the people in it'), 'the occupancy the people in it need between them'),
      'the occupancy they need between them',
    ),
  );
}

/**
 * B5, A4: THE LEAST AN OWNER WILL LET FOR — what a period of occupation wears out of the thing,
 * which is its own declared spoilage against its own cleared price. Two reads and a multiplication;
 * nothing here is a required yield and nothing targets one (Law 3).
 */
function wearOf(view: ParticipantView, region: RegionId): PerPiece | undefined {
  const id = goodId(DWELLING, region);
  if (!view.instruments.has(id)) return undefined;
  const print = view.print(id);
  if (!print.some) return undefined;
  // Law 8: the print is money per piece of the THING; the book asks money per piece of occupancy.
  const wearPerDwelling = scale(print.value.price, view.params.ratio(spoilageParam(DWELLING)), 'what a period of it wears') * view.registry.subdivision(goodUnitOf(view, region));
  return asPerPiece(div(wearPerDwelling, view.registry.subdivision(DWELLING_WEEKS), 'per piece of occupancy'), 'what a period of occupancy wears out of the roof');
}

/**
 * B1.a read from the other side: THE MOST A TENANT WILL PAY is what it has, because the alternative
 * is nowhere to live. Its own outlook of its own income, over the dwellings a member of it lives
 * in. A cell that has never observed an income cannot say what it would pay and does not bid.
 */
function reservation(view: ParticipantView, rows: readonly TenureDecl[]): PerPiece | undefined {
  const self = view.self;
  if (self.representation !== 'cell' || !('cohort' in self.key)) return undefined;
  const row = rows.find((r) => r.cohort === keyOf(self, 'cohort'));
  if (row === undefined) return undefined;
  const income = view.outlook(about({ on: 'income' }));
  if (!income.some || income.value.expected <= 0) return undefined;
  const per = view.registry.pieces(
    DWELLING_WEEKS,
    asNamed(
      view.params.ratio(HOUSING_PARAMS.perMember(keyOf(self, 'cohort'))),
      'the occupancy a member lives under',
    ),
  );
  if (per <= 0) return undefined;
  // Law 8: money pieces a member expects, over the PIECES of occupancy a member lives under — so
  // what it bids is money per piece, which is what the venue's book is in.
  return pricedAt(
    asCash(income.value.expected, 'what a member expects to earn'),
    per,
    'what a member would pay for the roof it lives under',
  );
}

/** Clearing B2: what a party has to say in the venue for the place it is in. */
function ordersOf(
  view: ParticipantView,
  venue: VenueDecl,
  rows: readonly TenureDecl[],
): readonly Order[] {
  /**
   * Clearing B2, Law 4: THE VENUES THIS PARTICIPANT SPEAKS FOR, and it is the lettings ones.
   *
   * `venueParticipantDecls` is one global list: every declaration is asked about every venue that is
   * gathered, and the only thing that stops a module answering for somebody else's book is the
   * module's own test. This asked only about the REGION — and a labour venue has a region too, so
   * once `labour` began gathering (item 3) this would have posted dwellings into a book quoted in
   * hours. `banks` guards the same way (`venue.key['market'] !== 'money'`, `occupation !== BANKING`).
   */
  if (venue.clearedBy !== 'housing') return [];
  const region = venue.key['region'] as RegionId | undefined;
  if (region === undefined || view.self.region !== region) return [];
  const mine = view.self.id;
  const out: Order[] = [];
  // The owner offers what it owns and nobody is living in — its own, less what it has already let
  // and less what its own people are under.
  const spare = subQty(
    subQty(owned(view, region), letOut(view), 'less what it has already let'),
    needs(view, rows),
    'less what its own people live in',
  );
  const floor = wearOf(view, region);
  if (spare > 0 && floor !== undefined) {
    out.push({ party: mine, side: 'sell', price: floor, qty: asQty(downTick(spare)) });
  }
  // And a household takes what it is short of, at what it would pay rather than have nowhere.
  const short = subQty(
    needs(view, rows),
    addQty(
      atMost(owned(view, region), needs(view, rows), 'it lives in what it owns, up to what it needs'),
      taken(view),
      'what it already has a roof from',
    ),
    'what it is short of',
  );
  if (short > 0) {
    const bid = reservation(view, rows);
    if (bid !== undefined && bid > 0) {
      out.push({ party: mine, side: 'buy', price: bid, qty: asQty(downTick(short)) });
    }
  }
  return out;
}

/** 15.5, Law 8: the last rent this book struck and when, off its own record — carried onto a print that struck none. */
function lastStruck(ctx: MechanismContext, venue: VenueDecl): { readonly rentPerDwelling?: number; readonly struckIn?: number } {
  const said = ctx.journal.forSubject(RENT_PRINT, String(venue.id));
  for (let n = said.length - 1; n >= 0; n -= 1) {
    const e = said[n];
    const rent = e?.data['rentPerDwelling'];
    const at = e?.data['struckIn'];
    if (typeof rent === 'number' && rent > 0 && typeof at === 'number') return { rentPerDwelling: rent, struckIn: at };
  }
  return {};
}

/** A2, Clearing C4.b: the letting session in one place, and it says what it did either way. */
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
  // Law 19: the rent is the level the solver cleared at, read off its outcome and never re-derived
  // from the fills. The fills say who is under whose roof (below); the price says at what.
  const struck = outcome.price;
  const until = addDays(ctx.calendar.startOf(ctx.period), ctx.params.periods(HOUSING_PARAMS.tenancyTerm) * ctx.calendar.periodDays);
  // A3: who ends up under whose roof. Owners with the least to ask are let first, tenants with the
  // most to pay are housed first, and both queues are walked in the solver's own fills.
  const owners = outcome.fills
    .filter((f) => f.side === 'sell' && f.qty > 0)
    .sort((a, b) => a.at - b.at);
  const tenants = outcome.fills
    .filter((f) => f.side === 'buy' && f.qty > 0)
    .sort((a, b) => b.at - a.at);
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
      const dwellings = asQty(atMost(want, left, 'there is no more of it to let than there is'));
      signs(ctx, {
        tenant: t.party,
        landlord: owner.party,
        region,
        rentPerDwelling: struck,
        dwellings,
        until,
      });
      signed += 1;
      want = subQty(want, dwellings, 'what it is still short of');
      left = subQty(left, dwellings, 'what this owner has left');
    }
  }
  ctx.record(
    RENT_PRINT,
    [venue.id],
    {
      venue: venue.id,
      region,
      outcome: 'cleared',
      rentPerDwelling: struck,
      struckIn: ctx.period,
      dwellings: outcome.volume,
      tenancies: signed,
      until,
      // D1: the levels the owners were prepared to let at, so the print can be checked against them.
      asks: orders.filter((o) => o.side === 'sell').map((o) => o.price),
    },
    true,
  );
}

/**
 * A2, Law 5: THE RENT IS PAID, both legs, every period, by name. A tenant that cannot pay fails to
 * pay — settlement records it — and the tenancy stands, because what ends a tenancy is somebody
 * ending it and not a payment going missing (Housing C4 is the secured lender's path, not this).
 *
 * 15.5, A3, XI-8: WHAT ENDS ONE. Its day passes, and the roof is re-let (a tenant still short bids
 * again in the next session). Or the LANDLORD ends it: it holds its own view of whether this tenant
 * pays — formed from the rents that came and the rents that did not, and from nothing else
 * (Expectations A2, Banks Lending A2) — and when the rent struck, at that view, is worth less to it
 * a period than the wear the tenant puts on the roof, it would rather have the roof back and let it
 * to somebody who pays. Two of its own reads compared; no count of missed rents, no threshold.
 */
function collect(ctx: MechanismContext): void {
  const today = ctx.calendar.startOf(ctx.period);
  for (const lease of leasesOf(ctx)) {
    const tenant = ctx.parties.get(lease.tenant);
    const landlord = ctx.parties.get(lease.landlord);
    if (!tenant.status.alive || !landlord.status.alive) {
      ends(ctx, lease, 'a side of it has ceased');
      continue;
    }
    if (compareCivil(lease.until, today) <= 0) {
      ends(ctx, lease, 'the term ended');
      continue;
    }
    const owner = ctx.participant(lease.landlord);
    const pays = owner.outlook(about({ on: 'credit', party: lease.tenant }));
    const floor = wearOf(owner, lease.region);
    if (pays.some && floor !== undefined) {
      const worth = scale(lease.rentPerDwelling, asRatio(pays.value.expected, 'the share of what falls due it expects to arrive'), 'what a piece of the tenancy is worth to it a period');
      if (worth < floor) {
        ends(ctx, lease, 'the landlord ended it: what it expects to be paid is below what letting costs it');
        ctx.record(
          TENANCY_ENDED,
          [lease.tenant, lease.landlord],
          { tenancy: lease.id, tenant: lease.tenant, landlord: lease.landlord, region: lease.region, rentPerDwelling: lease.rentPerDwelling, expectsPaid: pays.value.expected, worth, wear: floor, by: 'landlord' },
          true,
        );
        continue;
      }
    }
    const ccy = ctx.registry.currencyOf(lease.region);
    const whole = valueAt(lease.rentPerDwelling, lease.dwellings, 'the rent on this tenancy');
    // XI-15: a cell pays per member, because every member of it is paying its own rent.
    const perMember = over(
      whole,
      asRatio(weightOf(tenant), 'the members it has'),
      'per member of the cell that pays it',
    );
    const share = gridPerMember(ctx.registry, tenant, perMember);
    if (share.total <= 0) continue;
    const leg: Leg = {
      kind: 'money',
      from: ctx.accountOf(lease.tenant, ccy),
      to: ctx.accountOf(lease.landlord, ccy),
      // 15.5: a rent is a promise falling due, and whether it comes is what the landlord learns of the tenant.
      receipt: { of: 'rent' },
      ccy,
      amount: share.total,
    };
    ctx.settle({ legs: [leg], cause: 'transfer', reason: `rent on ${lease.id}` });
  }
}

/* ------------------------------------------------------------------------------------------------
 * BUYING ONE, AND BORROWING TO (13d steps 7-10).
 *
 * @spec Housing B1 Housing B1.a Housing B3 Housing C1 Housing C2 Housing C3 Housing C4 Housing C4.a Households E2 Households E3 Banks Lending A4 XI-2 Law 5 Law 19
 *
 * A household that is short of a roof would rather own one than rent it, and what stops it is
 * money. So it asks: it publishes what it is SHORT OF, in money, against the dwelling it would buy
 * — and a bank reads that, prices it against its own book, and writes a loan or does not (C5). The
 * bank never learns what a dwelling is; it learns that the request named an instrument it could
 * take and realise, which is the whole of what security means to a lender (Law 15).
 *
 * WHAT IT ASKS FOR IS TWO READS AND A SUBTRACTION: what it expects a dwelling in this place to cost
 * — its own outlook of a price this world prints — times what it is short of, less the money it
 * already has. There is no loan-to-value here and no affordability rule: what it can borrow is the
 * BANK's decision, taken with the bank's own standard on the bank's own book, and a household that
 * is refused is simply a household that goes on renting (C5.a).
 *
 * THE CHARGE IS REAL (C1, Register D5). Once it owns the dwelling, the units it holds are pledged
 * to the lender in a numbered instruction, so the register itself refuses to let them be sold out
 * from under the loan. Nothing counts that collateral twice, because the lien is the one record of
 * it (Appendix B).
 * ---------------------------------------------------------------------------------------------- */

export const FORECLOSED = 'housing.foreclosed';

/** C1: the loans this party has that are secured on a dwelling — read off the register (Law 19). */
function mortgagesOf(
  ctx: MechanismContext,
  borrower: PartyId,
): readonly { readonly id: InstrumentId; readonly lender: PartyId; readonly outstanding: Cash }[] {
  const out: { id: InstrumentId; lender: PartyId; outstanding: Cash }[] = [];
  // Law 19: A LOAN IS ISSUED BY ITS BORROWER (`banks/index.ts`: `issuer: some(borrower)`), so the
  // register already indexes a party's own rows and this asks it. Walking every instrument in the
  // world to find one household's mortgages is a search that grows with everything the world has
  // written — every invoice, every bond, every share — while the answer is one or two rows.
  // 15.5, Law 15: a mortgage is a loan SECURED ON THE ROOF OF THE BORROWER'S PLACE — the named
  // line, compared by name, never a prefix of a string.
  const roof = goodId(DWELLING, ctx.parties.get(borrower).region);
  for (const i of ctx.instruments.issuedBy(borrower)) {
    if (!i.status.live || !isLoan(i.terms)) continue;
    const t = i.terms;
    if (!t.security.some((sec) => sec.instrument === roof)) continue;
    // C5, XI-11: WHO FORECLOSES is whoever is owed the row today. A mortgage sold into a pool is
    // foreclosed by the pool, which is what makes the transfer a real one (Law 19).
    const owed = creditorOf((held) => ctx.register.holdersOf(held), i);
    // A mortgage nobody is owed is a mortgage that is paid off; there is nothing left to foreclose.
    if (!owed.some) continue;
    out.push({
      id: i.id,
      lender: owed.value,
      outstanding: heldAsMoney(ctx.register.heldTotal(i.id).value, 'what is still owed on it'),
    });
  }
  return out;
}

/**
 * B1, E2: what a household is short of in MONEY to buy the roof it is short of. Its own outlook of
 * what a dwelling costs where it lives, times how many it is short of, less what it holds.
 */
function shortOfMoney(
  ctx: MechanismContext,
  who: PartyId,
  lettings: Qty,
  rows: readonly TenureDecl[],
): Cash | undefined {
  const view = ctx.participant(who);
  const region = view.self.region;
  const id = goodId(DWELLING, region);
  if (!ctx.instruments.has(id)) return undefined;
  const outlook = view.outlook(about({ on: 'price', instrument: id }));
  const print = view.print(id);
  const price = outlook.some
    ? asPerPiece(outlook.value.expected, 'what it expects a roof to cost')
    : print.some
      ? print.value.price
      : undefined;
  if (price === undefined || price <= 0) return undefined;
  const has = owned(view, region);
  /**
   * B1, E2: TWO REASONS TO WANT ONE, and neither of them asks what sort of party is asking.
   *
   * A household is short of the roofs its own people live under and does not own them: that is
   * `needs` against `owned`, both reads, and it is what makes a household a buyer rather than a
   * tenant for ever. A LANDLORD is a party that owns dwellings and has let every one of them — a
   * business with no stock — and what one more would earn it is the rent the venue just printed;
   * one with an empty house wants nothing, because its own vacancy is its answer.
   *
   * Whether it gets it is the BANK's decision either way, taken on the bank's own book (C5).
   */
  let want = subQty(needs(view, rows), has, 'the roofs it is short of owning');
  if (want <= 0) {
    const spare = subQty(has, lettings, 'what it owns and nobody is in');
    if (has <= 0 || spare > 0) return undefined;
    want = asQty(1, 'one more to let');
  }
  /**
   * Law 8, XI-15: `want` is PIECES of the thing and `price` is money a piece, so the product is the
   * money this party needs — and for a cell it is the money ALL its members need between them,
   * because `want` was struck on the whole cell. What it already has is per member, so it is
   * multiplied out to meet it rather than subtracted from a number in a different denomination.
   */
  const cost: Cash = valueAt(price, want, 'what buying them would cost it');
  // 0f.1: the register holds the cell's TOTAL.
  const money = heldAsMoney(view.cash(ctx.registry.currencyOf(region)), 'the money its people have between them');
  const short = minus(cost, money, 'less the money it has');
  if (short <= 0) return undefined;
  return short;
}

/**
 * C1, E2: THE ASKING. It is a public event and nothing else: the household says what it is short of
 * and what it would put up, the banks read it next period and decide, and a household nobody will
 * lend to goes on renting. That is C5.a from the borrower's side — the standard is the lender's.
 */
function askForMortgages(ctx: MechanismContext, rows: readonly TenureDecl[]): void {
  for (const cell of ctx.parties.alive()) {
    /**
     * 13d.1: A HOUSEHOLD CAN HOLD A ROOF AND CAN BORROW FOR ONE — the register's grid says what one
     * member owns, the drawing is struck per member on both legs, the payer's side of a coupon is
     * denominated per member, and a cell issuer's equity moves by its own share of what it issued.
     * All four were built and measured here, and the last two were kernel defects nobody had met
     * because no cell had ever issued anything.
     *
     * WHAT STOPS IT IS ONE MORE THING, and it is not this item's: a borrower that MISSES A PAYMENT
     * goes on accruing on the lender's book at amortised cost while nothing moves on its own, so
     * its balance sheet drifts from the lender's by exactly the interest it did not pay. A
     * household with a mortgage and no money set aside to service it misses immediately, so it
     * shows there first — but it is the same defect for any borrower, and the item that owns it is
     * **13f**, which is corporate credit and rewrites what a bank does with a borrower that has
     * missed. Servicing a mortgage out of a household's own budget is Households E3/E4, and the
     * same item.
     */
    if (cell.representation === 'cell') continue;
    // C3: one mortgage at a time. A household already carrying one is not asking for another
    // until it has paid this one down, which is what a single secured row on one roof means.
    if (mortgagesOf(ctx, cell.id).length > 0) continue;
    const short = shortOfMoney(ctx, cell.id, letOut(ctx.participant(cell.id)), rows);
    if (short === undefined) continue;
    const id = goodId(DWELLING, cell.region);
    const print = ctx.prices.latest(id, ctx.period);
    if (!print.some || print.value.price <= 0) continue;
    /**
     * Corporate Credit A1, A4 (item 0e): one door, one shape. A landlord short of the price of a
     * dwelling asks the same way a firm short of a machine does, so a bank reads one kind.
     *
     * A4: what it would put up is the quantity the money would buy at the price the market last
     * printed, which is the only quantity either side can check.
     */
    ctx.request(cell.id, {
      ccy: ctx.registry.currencyOf(cell.region),
      short,
      security: [
        { instrument: id, qty: amountOf(short, print.value.price, 'what the loan would buy') },
      ],
      // C2: a mortgage is paid down, interest and principal.
      repays: 'onSchedule',
    });
  }
}

/**
 * C1, Register D5: THE CHARGE. Every period, a borrower with a mortgage and free dwellings binds
 * them to its lender up to what it still owes — one lien per loan, placed in a numbered instruction
 * like everything else. After it the register itself refuses to let the roof be sold out from under
 * the loan, so nothing has to remember not to.
 */
function charge(ctx: MechanismContext): void {
  for (const cell of ctx.parties.alive()) {
    const region = cell.region;
    const id = goodId(DWELLING, region);
    if (!ctx.instruments.has(id)) continue;
    const view = ctx.participant(cell.id);
    const free = view.free(id);
    if (free <= 0) continue;
    for (const m of mortgagesOf(ctx, cell.id)) {
      if (m.outstanding <= 0) continue;
      const print = ctx.prices.latest(id, ctx.period);
      if (!print.some || print.value.price <= 0) continue;
      const covers = downTick(
        amountOf(m.outstanding, print.value.price, 'the roofs the debt stands on'),
      );
      /**
       * XI-15, Law 8: what a cell pledges is whole pieces for every member it stands for, and the
       * lien is on the total. `free` is already per member for a cell, so the share is struck the
       * way every other movement on a cell is struck and the two agree by construction.
       */
      // 0f.1: `free` is the cell's TOTAL, so what it pledges is a total; the side is derived.
      const pledged = atMost(covers, free, 'it can pledge no more of it than it holds');
      if (pledged <= 0) continue;
      const share = { total: pledged };
      ctx.settle({
        legs: [
          {
            kind: 'pledge',
            pledgor: cell.id,
            beneficiary: m.lender,
            instrument: id,
            qty: share.total,
            secures: String(m.id),
          },
        ],
        cause: 'transfer',
        reason: `${m.id} is charged on ${share.total} of ${id}`,
      });
    }
  }
}

/**
 * C4, C4.a, XI-2: FORECLOSURE. A borrower that has ceased cannot pay, so the roof goes to the
 * lender: the lien is released and the dwellings move across at what the market last said. The
 * lender then holds a house it did not want, in a place, and sells it into the same session
 * everybody else does — so the recovery is what it FETCHED and never a rate anybody assumed, and
 * the loss lands where the loan is (Appendix B: no fixed recovery).
 */
function foreclose(ctx: MechanismContext): void {
  // C4, XI-1 (12a.4): A LENDER FORECLOSES ON A DEFAULT — a payment on the row that fell due and was
  // not made, which the kernel records against the instrument — and never on the borrower being
  // gone. It read "not alive" as "failed", and a household cell that moved bank is not alive and
  // is succeeded: its dwelling would have been foreclosed on for having moved (Register F2).
  const defaulted = new Set<string>();
  for (const e of creditDefaults(ctx.journal)) {
    const on = e.data['instrument'];
    if (typeof on === 'string') defaulted.add(on);
  }
  // 15.5, Law 18, Law 19: the loans, off the register's own index of the kind — never every line
  // in the world read for the few that are loans.
  for (const i of ctx.instruments.ofKind(LOAN)) {
    if (!i.status.live || !isLoan(i.terms)) continue;
    const t = i.terms;
    if (!defaulted.has(String(i.id))) continue;
    // Register F2: the borrower as it is now — its estate, if it failed; the cell it joined, if it moved.
    const borrower = ctx.parties.resolve(t.borrower);
    const id = goodId(DWELLING, borrower.region);
    if (!ctx.instruments.has(id)) continue;
    // Law 15: secured on the roof of its place, by name.
    if (!t.security.some((sec) => sec.instrument === id)) continue;
    const print = ctx.prices.latest(id, ctx.period);
    if (!print.some) continue;
    // Law 19: the liens are the register's own, read where they live. What the lender may take is
    // exactly what was bound to it and never a quantity worked out from the loan.
    const holding = ctx.register.holding(borrower.id, id);
    if (!holding.some) continue;
    // C5, XI-11: the proceeds go to whoever is owed the row now, not to whoever wrote it.
    const owed = creditorOf((held) => ctx.register.holdersOf(held), i);
    if (!owed.some) continue;
    const lender = owed.value;
    // Register D5.b: a lien says what it secures, and this loan's liens are the ones that say this loan.
    const liens = holding.value.liens.filter((l) => l.reason === String(i.id));
    for (const lien of liens) {
      const settled = ctx.settle({
        legs: [
          { kind: 'release', pledgor: borrower.id, beneficiary: lender, instrument: id, lien: lien.id },
          {
            kind: 'asset',
            from: borrower.id,
            to: lender,
            instrument: id,
            qty: lien.qty,
            pricePerUnit: some(print.value.price),
            accruedPerUnit: none(),
          },
        ],
        cause: 'corporateAction',
        reason: `${i.id} is foreclosed`,
      });
      ctx.record(
        FORECLOSED,
        [String(i.id), String(lender)],
        {
          loan: String(i.id),
          lender: String(lender),
          dwellings: lien.qty,
          at: print.value.price,
          settled: settled.outcome === 'settled',
        },
        true,
      );
    }
  }
}

function params(rows: readonly TenureDecl[]): ParamDecl[] {
  return [
    ...rows.map((r) => ({
      id: HOUSING_PARAMS.perMember(r.cohort),
      value: r.perMember,
      unit: 'dwellings per person',
      dimension: 'ratio' as const,
      kind: 'preference' as const,
      owner: 'model' as const,
      why: `Housing A1, Households E1: ${r.why}`,
    })),
    {
      id: HOUSING_PARAMS.tenancyTerm,
      value: 52,
      unit: 'periods',
      dimension: 'periods',
      kind: 'technology',
      owner: 'standardSetter',
      why: 'Housing A3 (15.5): how long a tenancy runs for — a year, a convention of the contract stated with it. A tenancy has a TERM, which is what makes a rent struck once a rent the tenant is still paying (A2), and what lets the roof be re-let when it ends.',
    },
    {
      id: HOUSING_PARAMS.dwellingsPerLandlord,
      value: 20,
      unit: 'dwellings a landlord',
      dimension: 'count',
      kind: 'placeholder',
      owner: 'model',
      standsInFor: { mechanism: 'Seed C4, Housing A4: the dwellings a landlord opens holding is a stock the opening states', item: '22a' },
      why: 'Housing A3, A4 (15.5): what a landlord opens holding of dwellings, to let. A rental stock must have dwellings behind it, and until this the world had forty thousand households and no dwelling anybody held. A SHAPE with its death at 22a: the stock is a register of units with owners, moved by what changes hands and what is built, and the opening is where it starts.',
    },
  ];
}

/**
 * Housing A1, E1, Seed D1 (15.6): THE ROOFS FAMILY. A place with people in it and no dwelling line
 * is a place nobody can live in — the goods are placed where firms were (`settled(placed)`) and the
 * people where the banks are, and a region with a bank and no firm opened with households and no
 * roof for a year without anybody saying so. The seal runs the audit at period 0, so this is a
 * SEED FINDING, reported there with the people it concerns; it never repairs (Audit A1). And E1's
 * half that can break silently: a dwelling whose holder has ceased is a house without an owner.
 */
export function roofs(): Family {
  return {
    name: 'names',
    contributor: 'housing',
    spec: 'Housing A1 Housing E1 Seed D1',
    built: true,
    check: (view): Violation[] => {
      const out: Violation[] = [];
      const people = new Map<RegionId, number>();
      for (const p of view.parties.ofKind(HOUSEHOLD)) {
        if (!p.status.alive || p.representation !== 'cell') continue;
        const had = people.get(p.region);
        people.set(p.region, had === undefined ? weightOf(p) : had + weightOf(p));
      }
      for (const [region, members] of people) {
        if (view.instruments.has(goodId(DWELLING, region))) continue;
        out.push({
          family: 'names',
          spec: 'Housing A1',
          owner: String(region),
          size: members,
          unit: 'people',
          period: view.period,
          message: `${String(region)}: ${String(members)} people live here and the place has no dwelling line — nobody here can live anywhere (Seed D1)`,
        });
      }
      for (const region of view.registry.regions.values()) {
        const id = goodId(DWELLING, region.id);
        if (!view.instruments.has(id)) continue;
        for (const holder of view.register.holdersOf(id)) {
          if (view.parties.has(holder) && view.parties.get(holder).status.alive) continue;
          out.push({
            family: 'names',
            spec: 'Housing E1',
            owner: String(holder),
            size: view.register.quantity(holder, id),
            unit: String(id),
            period: view.period,
            message: `${String(holder)} holds ${String(id)} and has ceased: a house without an owner`,
          });
        }
      }
      return out;
    },
  };
}

export function housing(rows: readonly TenureDecl[] = TENURE): SystemModule {
  const mine = (v: VenueDecl): boolean => v.clearedBy === 'housing';
  return {
    id: 'housing',
    agreementKinds: [
      {
        id: TENANCY,
        what: 'a named tenant renting dwellings from a named landlord, at a rent',
        // XI-8: a tenancy runs with the dwelling, so a landlord's successor that holds it is the
        // landlord now. An estate sells the dwelling instead, and the tenancy ends at that sale.
        binds: 'aGoingConcern',
      },
    ],
    // XI-8, item 9.1: NO NOUNS. The tenancies were this module's private book and are agreements
    // now; there is no `ctx.state` slot left here at all, which is what a migration looks like when
    // the whole of what a module was keeping turns out to be a thing the kernel should own.
    spec: 'Housing',
    // It needs the dwelling to be a line, the people to be cells, and (15.5) the landlords to exist
    // before its seed gives them the dwellings they let.
    requires: ['goods', 'households', 'property'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [
      {
        id: DWELLING_WEEKS,
        name: 'dwelling weeks',
        // The same fine grid a tonne is counted in: what is traded here is occupancy and not houses.
        perUnit: TONNE_PIECES,
      },
    ],
    params: params(rows),
    phases: [
      {
        name: 'housing.lettings',
        spec: 'Housing A2 Housing A3 Housing B5 Clearing C4.b',
        anchor: { before: 'markets' },
        reads: [],
        writes: [{ kind: 'event', name: 'housing.rent' }],
        run: (ctx: MechanismContext) => {
          /**
           * Clearing B2, B-5: THE LETTINGS VENUE HAD NEVER HAD AN ORDER IN IT. `venueParticipants`
           * declares who bids and offers for a tenancy, and nothing called `gather`, so `letIn` read
           * an empty book every period of every run: no tenancy was ever signed, no rent was ever
           * paid, and `housing.rent` — a whole phase — did nothing for ever.
           *
           * Once per venue per period, before the book is read.
           */
          for (const v of ctx.venues.filter(mine)) {
            ctx.gather(v.id);
            letIn(ctx, v);
          }
        },
      },
      {
        name: 'housing.asking',
        spec: 'Housing B1 Housing C1 Households E2',
        anchor: { after: 'corporateActions' },
        reads: [],
        writes: [
          { kind: 'event', name: 'credit.request' },
          { kind: 'event', name: 'housing.shortfall' },
        ],
        run: (ctx: MechanismContext) => {
          publishShortfall(ctx, rows);
          askForMortgages(ctx, rows);
        },
      },
      {
        name: 'housing.charge',
        spec: 'Housing C1 Register D5',
        anchor: { after: 'markets' },
        reads: [],
        writes: [],
        run: charge,
      },
      {
        name: 'housing.foreclose',
        spec: 'Housing C4 Housing C4.a XI-2',
        anchor: { after: 'revaluation' },
        // XI-1 (12a.4): a default on the row is what a lender forecloses on.
        reads: [{ kind: 'event', name: 'credit.default', of: 'anyPeriod' }],
        writes: [],
        run: foreclose,
      },
      {
        name: 'housing.rent',
        spec: 'Housing A2 Housing A3 Households E5 Law 5 XI-8',
        anchor: { after: 'markets' },
        reads: [],
        writes: [{ kind: 'event', name: TENANCY_ENDED }],
        run: (ctx: MechanismContext) => {
          collect(ctx);
        },
      },
    ],
    participants: [],
    // A3, Clearing B2: whoever owns a roof may let it and whoever needs one may take it, by the same
    // reads. A landlord (15.3) is in the book as an owner with nobody of its own to house.
    venueParticipants: [
      {
        partyKind: HOUSEHOLD,
        orders: (view, venue) => ordersOf(view, venue, rows),
      },
      {
        partyKind: LANDLORD,
        orders: (view, venue) => ordersOf(view, venue, rows),
      },
    ],
    families: [roofs()],
    seed(ctx: SeedContext): void {
      for (const region of ctx.registry.regions.values()) {
        if (!ctx.instruments.has(goodId(DWELLING, region.id))) continue;
        ctx.openVenue({
          id: rentVenue(region.id),
          name: `Lettings, ${region.name}`,
          clearedBy: 'housing',
          unit: DWELLING_WEEKS,
          ccy: ctx.registry.currencyOf(region.id),
          key: { region: region.id },
        });
      }
      /**
       * A3, A4, E1, Seed C4 (15.5): THE RENTAL STOCK, WITH OWNERS. Each landlord member opens holding
       * so many whole dwellings of its place, carried at what a dwelling there was last worth — a
       * stock the opening states and the letting book then acts on. The landlord cells are the
       * property module's (15.3) and the foundation's (15.4); what this module adds is the roofs.
       */
      const per = ctx.params.count(HOUSING_PARAMS.dwellingsPerLandlord);
      if (per <= 0) return;
      for (const cell of ctx.parties.ofKind(LANDLORD)) {
        if (!cell.status.alive) continue;
        const id = goodId(DWELLING, cell.region);
        if (!ctx.instruments.has(id)) continue;
        const print = ctx.prices.latest(id, ctx.period);
        if (!print.some) continue;
        const pieces = ctx.registry.pieces(ctx.instruments.get(id).unit, asNamed(per, 'the dwellings a landlord opens with'));
        ctx.endowUnits(cell.id, id, pieces, print.value.price);
      }
    },
  };
}

/**
 * Housing A1, E1, Observer A3, item 7b: WHAT EACH CELL IS SHORT OF A HOME, published.
 *
 * A household decides what to do with its own money, and one of the things it can do is buy the
 * place its people live in. To decide that it has to know it is short of one — and what a cohort's
 * people need, and what leases it holds, are HOUSING's facts: a module never imports another module
 * (ARCHITECTURE 4.9b), so the route is the one every other cross-module read uses, which is an event
 * the owner of the fact publishes and the reader reads under the party's own name.
 *
 * It is the same read `ordersOf` makes to decide what a cell offers or takes in the lettings venue,
 * taken once and published rather than computed twice (Law 4). Nothing here decides anything: a
 * household that is short of a home may buy one, rent one, or stay where it is.
 */
function publishShortfall(
  ctx: MechanismContext,
  rows: readonly TenureDecl[],
): void {
  for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
    if (p.representation !== 'cell' || !p.status.alive) continue;
    const view = ctx.participant(p.id);
    const need = needs(view, rows);
    if (need <= 0) continue;
    const has = owned(view, p.region);
    const rented = taken(view);
    const short = subQty(subQty(need, has, 'less what it owns'), rented, 'less what it rents');
    ctx.record(
      'housing.shortfall',
      [p.id],
      {
        cell: p.id,
        region: p.region,
        // Law 8 (15.5): the book counts occupancy; a buyer of the THING bids in the register's
        // pieces of it, so what is published is converted once, here, for that reader (`homeBid`).
        needs: dwellingsOf(view, p.region, need),
        owns: dwellingsOf(view, p.region, has),
        rents: dwellingsOf(view, p.region, rented),
        // E1: what its people live in that it neither owns nor has taken a lease on. Negative is a
        // cell with more housing than its people need, which is a real state and is said as one.
        short: dwellingsOf(view, p.region, short),
        dwelling: goodId(DWELLING, p.region),
      },
      true,
    );
  }
}

/** Law 19: what a cohort's people need, for a reader. It is the same read the venue uses. */
export const dwellingsNeeded = (view: ParticipantView, rows: readonly TenureDecl[] = TENURE): number =>
  needs(view, rows);


