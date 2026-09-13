/**
 * Housing: who lives where, what they pay for it, and who owns the roof.
 *
 * @spec Housing A1 Housing A1.a Housing A2 Housing A3 Housing A4 Housing A5 Housing B1 Housing B2 Housing B5 Housing D1 Households A2.b Households E1 Households E5 Labour A2 Clearing A2 Clearing B2 Clearing C4.b Law 2 Law 3 Law 5 Law 15 Law 19 XI-15
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
 */
import type { Order } from '../../clearing/solver.js';
import { clear, isCleared } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { currencyUnit, unitId, type PartyId, type RegionId } from '../../core/ids.js';
import { add, atMost, div, mul, sub, sum } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { asQty, downTick, type Qty } from '../../core/tick.js';
import { TONNE_PIECES } from '../../registry/grid.js';
import type { Leg } from '../../ledger/instruction.js';
import { cellSide, shareFor } from '../../ledger/settlement.js';
import { keyOf, weightOf } from '../../parties/party.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import { goodId, spoilageParam } from '../../registry/physical.js';
import { isLoan } from '../../registry/credit.js';
import type { InstrumentId } from '../../core/ids.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { DWELLING, HOUSING_PARAMS, TENURE, rentVenue, type TenureDecl } from './data.js';

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
export interface Lease {
  readonly id: string;
  readonly tenant: PartyId;
  readonly landlord: PartyId;
  readonly region: RegionId;
  /** A2: struck at the letting and moving only by a new letting. */
  readonly rentPerDwelling: number;
  dwellings: Qty;
}

export interface LeaseBook {
  rows: Record<string, Lease>;
  next: number;
}

const emptyBook = (): LeaseBook => ({ rows: {}, next: 1 });

const allLeases = (b: LeaseBook): Lease[] => Object.values(b.rows);

/** A3, D1: dwellings this party OWNS in a place — the register's own answer, never a tally. */
function owned(view: ParticipantView, region: RegionId): number {
  const id = goodId(DWELLING, region);
  if (!view.instruments.has(id)) return 0;
  // XI-15: a cell's holdings are per MEMBER and a venue speaks in totals, so the weight goes on
  // here and nowhere else. A named party stands for one of itself, and its weight is one.
  return mul(view.free(id), weightOf(view.self), 'what the people it stands for own between them');
}

/** How many dwellings this party has LET OUT, and how many it has taken (Law 19: off the rows). */
function letOut(b: LeaseBook, who: PartyId): number {
  return sum(allLeases(b).filter((l) => l.landlord === who).map((l) => l.dwellings)).value;
}

function taken(b: LeaseBook, who: PartyId): number {
  return sum(allLeases(b).filter((l) => l.tenant === who).map((l) => l.dwellings)).value;
}

/** E1, XI-15: how many dwellings the people in this cell live in. Whole dwellings, per cell. */
function needs(view: ParticipantView, rows: readonly TenureDecl[]): number {
  const self = view.self;
  if (self.representation !== 'cell') return 0;
  const cohort = keyOf(self, 'cohort');
  const row = rows.find((r) => r.cohort === cohort);
  if (row === undefined) return 0;
  const per = view.params.ratio(HOUSING_PARAMS.perMember(cohort));
  return mul(weightOf(self), per, 'the occupancy the people in it need between them');
}

/**
 * B5, A4: THE LEAST AN OWNER WILL LET FOR — what a period of occupation wears out of the thing,
 * which is its own declared spoilage against its own cleared price. Two reads and a multiplication;
 * nothing here is a required yield and nothing targets one (Law 3).
 */
function wearOf(view: ParticipantView, region: RegionId): number | undefined {
  const id = goodId(DWELLING, region);
  if (!view.instruments.has(id)) return undefined;
  const print = view.print(id);
  if (!print.some) return undefined;
  return mul(view.params.ratio(spoilageParam(DWELLING)), print.value.price, 'what a period of it wears');
}

/**
 * B1.a read from the other side: THE MOST A TENANT WILL PAY is what it has, because the alternative
 * is nowhere to live. Its own outlook of its own income, over the dwellings a member of it lives
 * in. A cell that has never observed an income cannot say what it would pay and does not bid.
 */
function reservation(view: ParticipantView, rows: readonly TenureDecl[]): number | undefined {
  const self = view.self;
  if (self.representation !== 'cell') return undefined;
  const row = rows.find((r) => r.cohort === keyOf(self, 'cohort'));
  if (row === undefined) return undefined;
  const income = view.outlook('income');
  if (!income.some || income.value.expected <= 0) return undefined;
  const per = view.params.ratio(HOUSING_PARAMS.perMember(keyOf(self, 'cohort')));
  if (per <= 0) return undefined;
  return div(income.value.expected, per, 'what a member would pay for the roof it lives under');
}

/** Clearing B2: what a party has to say in the venue for the place it is in. */
function ordersOf(
  view: ParticipantView,
  venue: VenueDecl,
  book: LeaseBook,
  rows: readonly TenureDecl[],
): readonly Order[] {
  const region = venue.key['region'] as RegionId | undefined;
  if (region === undefined || view.self.region !== region) return [];
  const mine = view.self.id;
  const out: Order[] = [];
  // The owner offers what it owns and nobody is living in — its own, less what it has already let
  // and less what its own people are under.
  const spare = sub(
    sub(owned(view, region), letOut(book, mine), 'less what it has already let'),
    needs(view, rows),
    'less what its own people live in',
  );
  const floor = wearOf(view, region);
  if (spare > 0 && floor !== undefined) {
    out.push({ party: mine, side: 'sell', price: floor, qty: asQty(downTick(spare)) });
  }
  // And a household takes what it is short of, at what it would pay rather than have nowhere.
  const short = sub(
    needs(view, rows),
    add(atMost(owned(view, region), needs(view, rows), 'it lives in what it owns, up to what it needs'), taken(book, mine), 'what it already has a roof from'),
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

/** A2, Clearing C4.b: the letting session in one place, and it says what it did either way. */
function letIn(ctx: MechanismContext, book: LeaseBook, venue: VenueDecl): void {
  const region = venue.key['region'] as RegionId | undefined;
  if (region === undefined) return;
  const posted = ctx.posted(venue.id);
  const orders = posted.filter((o) => o.price !== 'market');
  const outcome = clear(orders, 'proRata', 'marginalBid');
  if (!isCleared(outcome)) {
    ctx.record(RENT_PRINT, [venue.id], { venue: venue.id, region, outcome: outcome.kind }, true);
    return;
  }
  let struck: number | undefined;
  for (const f of outcome.fills) {
    if (f.side !== 'buy' || f.qty <= 0) continue;
    if (struck === undefined || f.at < struck) struck = f.at;
  }
  if (struck === undefined) {
    ctx.record(RENT_PRINT, [venue.id], { venue: venue.id, region, outcome: 'noDemand' }, true);
    return;
  }
  // A3: who ends up under whose roof. Owners with the least to ask are let first, tenants with the
  // most to pay are housed first, and both queues are walked a whole dwelling at a time (A1.a).
  const owners = outcome.fills
    .filter((f) => f.side === 'sell' && f.qty > 0)
    .sort((a, b) => a.at - b.at);
  const tenants = outcome.fills
    .filter((f) => f.side === 'buy' && f.qty > 0)
    .sort((a, b) => b.at - a.at);
  let at = 0;
  let left: Qty = owners[0]?.qty ?? asQty(0);
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
      const id = `lease.${book.next}`;
      book.next += 1;
      book.rows[id] = {
        id,
        tenant: t.party,
        landlord: owner.party,
        region,
        rentPerDwelling: struck,
        dwellings,
      };
      want = asQty(sub(want, dwellings, 'what it is still short of'));
      left = asQty(sub(left, dwellings, 'what this owner has left'));
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
      dwellings: outcome.volume,
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
 */
function collect(ctx: MechanismContext, book: LeaseBook): void {
  for (const lease of allLeases(book)) {
    const tenant = ctx.parties.get(lease.tenant);
    const landlord = ctx.parties.get(lease.landlord);
    if (!tenant.status.alive || !landlord.status.alive) {
      book.rows = Object.fromEntries(Object.entries(book.rows).filter(([id]) => id !== lease.id));
      continue;
    }
    const ccy = ctx.registry.currencyOf(lease.region);
    const whole = mul(lease.rentPerDwelling, lease.dwellings, 'the rent on this tenancy');
    // XI-15: a cell pays per member, because every member of it is paying its own rent.
    const perMember = div(whole, weightOf(tenant), 'per member of the cell that pays it');
    const share = shareFor(ctx.registry, tenant, currencyUnit(ccy), perMember);
    if (share.total <= 0) continue;
    const side = cellSide(tenant, share.perMember);
    const leg: Leg = {
      kind: 'money',
      from: ctx.accountOf(lease.tenant, ccy),
      to: ctx.accountOf(lease.landlord, ccy),
      ccy,
      amount: share.total,
      fromCell: side === undefined ? none() : some(side),
      toCell: none(),
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

export const HOUSING_FUNDING = 'housing.funding';
export const FORECLOSED = 'housing.foreclosed';

/** C1: the loans this party has that are secured on a dwelling — read off the register (Law 19). */
function mortgagesOf(
  ctx: MechanismContext,
  borrower: PartyId,
): readonly { readonly id: InstrumentId; readonly lender: PartyId; readonly outstanding: number }[] {
  const out: { id: InstrumentId; lender: PartyId; outstanding: number }[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !isLoan(i.terms)) continue;
    const t = i.terms;
    if (t.borrower !== borrower) continue;
    if (!t.security.some((sec) => String(sec.instrument).startsWith(`good.${DWELLING}.`))) continue;
    out.push({ id: i.id, lender: t.lender, outstanding: ctx.register.heldTotal(i.id).value });
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
  lettings: number,
  rows: readonly TenureDecl[],
): number | undefined {
  const view = ctx.participant(who);
  const region = view.self.region;
  const id = goodId(DWELLING, region);
  if (!ctx.instruments.has(id)) return undefined;
  const outlook = view.outlook(`price.${id}`);
  const print = view.print(id);
  const price = outlook.some ? outlook.value.expected : print.some ? print.value.price : undefined;
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
  let want = sub(needs(view, rows), has, 'the roofs it is short of owning');
  if (want <= 0) {
    const spare = sub(has, lettings, 'what it owns and nobody is in');
    if (has <= 0 || spare > 0) return undefined;
    want = 1;
  }
  const cost = mul(want, price, 'what buying them would cost it');
  const short = sub(cost, view.cash(ctx.registry.currencyOf(region)), 'less the money it has');
  if (short <= 0) return undefined;
  return short;
}

/**
 * C1, E2: THE ASKING. It is a public event and nothing else: the household says what it is short of
 * and what it would put up, the banks read it next period and decide, and a household nobody will
 * lend to goes on renting. That is C5.a from the borrower's side — the standard is the lender's.
 */
function askForMortgages(ctx: MechanismContext, book: LeaseBook, rows: readonly TenureDecl[]): void {
  for (const cell of ctx.parties.alive()) {
    /**
     * 13d.1: A HOUSEHOLD CAN NOW HOLD A ROOF — the register's grid is fine enough to say what one
     * member of a cell owns (`goods/index.ts`) — and it still does not BORROW for one here, for a
     * reason that is measured rather than assumed: a cell's drawing is struck per member, the
     * per-member amount is rounded to whole pieces of money, and what the rounding drops does not
     * reach the cell's equity account, so the balance-sheet family fires on the borrower in the
     * period it borrowed. The plumbing is built and tested to that point (`banks/index.ts` writes a
     * cell's drawing per member, both legs); what is missing is the one place the dust goes, and
     * that is the next step of this item rather than something to leave firing every period.
     */
    if (cell.representation === 'cell') continue;
    // C3: one mortgage at a time. A household already carrying one is not asking for another
    // until it has paid this one down, which is what a single secured row on one roof means.
    if (mortgagesOf(ctx, cell.id).length > 0) continue;
    const short = shortOfMoney(ctx, cell.id, letOut(book, cell.id), rows);
    if (short === undefined) continue;
    const id = goodId(DWELLING, cell.region);
    const print = ctx.prices.latest(id, ctx.period);
    if (!print.some || print.value.price <= 0) continue;
    ctx.record(
      HOUSING_FUNDING,
      [cell.id],
      {
        short,
        // A4: what it would put up. The quantity is what the money would buy at the price the
        // market last printed, which is the only quantity either side can check.
        security: [
          { instrument: String(id), qty: div(short, print.value.price, 'what the loan would buy') },
        ],
      },
      true,
    );
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
      const covers = downTick(div(m.outstanding, print.value.price, 'the roofs the debt stands on'));
      /**
       * XI-15, Law 8: what a cell pledges is whole pieces for every member it stands for, and the
       * lien is on the total. `free` is already per member for a cell, so the share is struck the
       * way every other movement on a cell is struck and the two agree by construction.
       */
      const perMember = atMost(covers, free, 'it can pledge no more of it than it holds');
      const share = shareFor(ctx.registry, cell, ctx.instruments.get(id).unit, perMember);
      if (share.total <= 0) continue;
      const side = cellSide(cell, share.perMember);
      ctx.settle({
        legs: [
          {
            kind: 'pledge',
            pledgor: cell.id,
            beneficiary: m.lender,
            instrument: id,
            qty: share.total,
            secures: String(m.id),
            pledgorCell: side === undefined ? none() : some(side),
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
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !isLoan(i.terms)) continue;
    const t = i.terms;
    if (!t.security.some((sec) => String(sec.instrument).startsWith(`good.${DWELLING}.`))) continue;
    const borrower = ctx.parties.get(t.borrower);
    if (borrower.status.alive) continue;
    const id = goodId(DWELLING, borrower.region);
    if (!ctx.instruments.has(id)) continue;
    const print = ctx.prices.latest(id, ctx.period);
    if (!print.some) continue;
    // Law 19: the liens are the register's own, read where they live. What the lender may take is
    // exactly what was bound to it and never a quantity worked out from the loan.
    const holding = ctx.register.holding(t.borrower, id);
    if (!holding.some) continue;
    const liens = holding.value.liens.filter((l) => l.reason.includes(String(i.id)));
    for (const lien of liens) {
      const settled = ctx.settle({
        legs: [
          { kind: 'release', pledgor: t.borrower, beneficiary: t.lender, instrument: id, lien: lien.id },
          {
            kind: 'asset',
            from: t.borrower,
            to: t.lender,
            instrument: id,
            qty: lien.qty,
            pricePerUnit: some(print.value.price),
            accruedPerUnit: none(),
            fromCell: none(),
            toCell: none(),
          },
        ],
        cause: 'corporateAction',
        reason: `${i.id} is foreclosed`,
      });
      ctx.record(
        FORECLOSED,
        [String(i.id), String(t.lender)],
        {
          loan: String(i.id),
          lender: String(t.lender),
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
  return rows.map((r) => ({
    id: HOUSING_PARAMS.perMember(r.cohort),
    value: r.perMember,
    unit: 'dwellings per person',
    dimension: 'ratio' as const,
    kind: 'preference' as const,
    owner: 'model' as const,
    why: `Housing A1, Households E1: ${r.why}`,
  }));
}

export function housing(rows: readonly TenureDecl[] = TENURE): SystemModule {
  const book = emptyBook();
  const bookOf = (ctx: MechanismContext): LeaseBook => ctx.state<LeaseBook>('leases', () => book);
  const mine = (v: VenueDecl): boolean => v.clearedBy === 'housing';
  return {
    id: 'housing',
    spec: 'Housing',
    // It needs the dwelling to be a line and the people to be cells, and nothing else.
    requires: ['goods', 'households'],
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
        cycle: 1,
        anchor: { before: 'markets' },
        run: (ctx: MechanismContext) => {
          const b = bookOf(ctx);
          for (const v of ctx.venues.filter(mine)) letIn(ctx, b, v);
        },
      },
      {
        name: 'housing.asking',
        spec: 'Housing B1 Housing C1 Households E2',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          askForMortgages(ctx, bookOf(ctx), rows);
        },
      },
      {
        name: 'housing.charge',
        spec: 'Housing C1 Register D5',
        cycle: 2,
        anchor: { after: 'markets' },
        run: charge,
      },
      {
        name: 'housing.foreclose',
        spec: 'Housing C4 Housing C4.a XI-2',
        cycle: 'anchor',
        anchor: { after: 'revaluation' },
        run: foreclose,
      },
      {
        name: 'housing.rent',
        spec: 'Housing A2 Households E5 Law 5',
        cycle: 2,
        anchor: { after: 'markets' },
        run: (ctx: MechanismContext) => {
          collect(ctx, bookOf(ctx));
        },
      },
    ],
    participants: [],
    venueParticipants: [
      {
        partyKind: HOUSEHOLD,
        orders: (view, venue) => ordersOf(view, venue, book, rows),
      },
    ],
    families: [],
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
    },
  };
}

/** Law 19: what a cohort's people need, for a reader. It is the same read the venue uses. */
export const dwellingsNeeded = (view: ParticipantView, rows: readonly TenureDecl[] = TENURE): number =>
  needs(view, rows);

/** The tenancies standing now, for the observer. A read of the module's own rows. */
export const leasesOf = (book: LeaseBook): readonly Lease[] => allLeases(book);

