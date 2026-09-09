/**
 * The matching: who is hired, at what wage, and what the wage bill then does every period.
 *
 * @spec Labour A1 Labour A2 Labour A4 Labour A4.b Labour A4.c Labour B1 Labour B1.a Labour B2 Labour B3 Labour B4 Labour C2 Labour C3 Labour C4 Labour C5 Labour D1 Labour D1.a Labour D1.c Labour D2 Labour D3 Labour D5 Labour E1 Labour F1 Labour F2 XI-10 XI-15 Clearing C3
 *
 * EVERY POSTING IS A BID (D1): the employer's openings at the wage it offers. The period's matches
 * go to the highest bids first and pro rata within a tie — which is what the kernel's own solver
 * does under the marginal-bid rule — and the bid that took the last match is the occupation's print.
 * An offer above the going rate therefore fills and one below it does not (D1.a).
 *
 * A POSTING IS THE EMPLOYER'S DESIRED EMPLOYMENT, not its desired hire: the hours it wants at that
 * wage, against what it already has under contract. The difference upward is a vacancy; downward it
 * is a separation, and a separation costs severance (C3). That asymmetry — a hire is free and a
 * firing is paid for — is where the cycle in employment comes from, and it is a cost, never a pair
 * of speeds.
 *
 * SUPPLY is a decision each cell takes for itself (B1): it will not work below its own outside
 * option, which is what this world already pays it when it does not work (B1.a) — read from its own
 * outlook of its income, never from a stated replacement rate. A cell with a trade looks for that
 * trade; one that has never worked can enter any occupation (A3.b, at the bottom, since it posts at
 * its own reservation and not at the going rate).
 */
import { period as periodOf, type Period } from '../../calendar/calendar.js';
import { clear, isCleared, type Cleared, type Order } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import type { PartyId, RegionId } from '../../core/ids.js';
import { add, div, material, mul, sub } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import { cellSide, totalFor } from '../../ledger/settlement.js';
import { weightOf, type Party } from '../../parties/party.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';
import {
  allRows,
  employed,
  goingRate,
  hoursAt,
  rowOfWorker,
  wagePerMember,
  type EmploymentBook,
  type EmploymentRow,
} from './register.js';

/** The numbers the matching reads, all declared by the module (Law 2). */
export interface LabourParams {
  readonly hoursPerMember: number;
  readonly retirementAge: number;
  readonly hiringLagPeriods: number;
  readonly severancePeriods: number;
}

/** B3: whether a cohort's members are in the workforce at all, or out of it by age. */
function participates(ctx: MechanismContext, p: Party, retirementAge: number): boolean {
  if (p.representation !== 'cell' || !p.status.alive) return false;
  return ctx.registry.cohort(p.key.cohort).fromAge < retirementAge;
}

/**
 * B1.a: the least a cell will work for. Its outside option is what it lives on without the job —
 * read from its own outlook of its own income, which for somebody not working is the benefit this
 * world pays it. A cell that has never observed an income has no outside option to compare against
 * and does not post: it cannot say what it will not work for.
 */
function reservation(ctx: MechanismContext, cell: PartyId, hours: number): number | undefined {
  // `income` is what the expectations module names what a party observes reaching it (A2).
  const outlook = ctx.participant(cell).outlook('income');
  if (!outlook.some) return undefined;
  return div(outlook.value.expected, hours, 'reservation wage');
}

/** B1, B4: every cell that is not working offers its members' hours, at its own reservation. */
function supply(ctx: MechanismContext, book: EmploymentBook, v: VenueDecl, p: LabourParams): Order[] {
  const out: Order[] = [];
  const occupation = v.key['occupation'];
  const region = v.key['region'];
  if (occupation === undefined || region === undefined) return out;
  for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
    if (!participates(ctx, cell, p.retirementAge) || cell.region !== region) continue;
    if (rowOfWorker(book, cell.id) !== undefined) continue;
    const skill = book.skill[cell.id];
    // A3: a job in one occupation is not a job in another. Somebody who has worked looks for the
    // trade they have; somebody who never has can start anywhere.
    if (skill !== undefined && skill !== occupation) continue;
    const hours = mul(weightOf(cell), p.hoursPerMember, 'hours offered');
    const wage = reservation(ctx, cell.id, p.hoursPerMember);
    if (wage === undefined || hours <= 0) continue;
    out.push({ party: cell.id, side: 'sell', price: wage, qty: hours });
  }
  return out;
}

/**
 * The venue's session: what the employers posted against what they already employ, and the cells
 * that are looking. Returns the wage that cleared, if anything did.
 */
export function runVenue(
  ctx: MechanismContext,
  book: EmploymentBook,
  v: VenueDecl,
  p: LabourParams,
): void {
  const occupation = v.key['occupation'];
  const region = v.key['region'];
  if (occupation === undefined || region === undefined) return;
  const bids: Order[] = [];
  for (const posting of ctx.posted(v.id)) {
    if (posting.side !== 'buy' || posting.price === 'market') continue;
    const held = hoursAt(book, posting.party, occupation, region as RegionId);
    const gap = sub(posting.qty, held, 'employment gap');
    if (!material(gap, 2, add(posting.qty, held, 'employment'))) continue;
    if (gap > 0) bids.push({ party: posting.party, side: 'buy', price: posting.price, qty: gap });
    // C3, C4: the employer wants fewer hours than it has under contract, so it separates the
    // difference and pays for doing it. It is the employer's decision; this is the mechanism.
    else shed(ctx, book, posting.party, occupation, region as RegionId, -gap, p);
  }
  const offers = supply(ctx, book, v, p);
  // D1: the highest bids fill first, and THE BID THAT TOOK THE LAST MATCH IS THE PRINT. That is the
  // clause, and it is `marginalBid`: the lowest employer still allotted sets the wage, so one that
  // bid above it fills and keeps the difference (D1.a) and the marginal one earns nothing on the
  // margin, which is what being marginal means.
  //
  // It was `sellersCompete` for one item, on the reasoning that a slack market should fall to what
  // the seekers will work for. That reads well, is not what D1 says, and does not survive contact:
  // a seeker's reservation is what it lives on NOW, so an unemployed cell with a small transfer
  // works for almost nothing, the venue prints there, and the wage never rises towards what the
  // work is worth however hard employers bid. Either rule hands the whole surplus to one side when
  // the book has one bidder on the other; what makes this one a market is that the line has three
  // firms in it and they do not bid the same number (Firm A3, Seed B4).
  const outcome = clear([...bids, ...offers], 'proRata', 'marginalBid');
  if (!isCleared(outcome)) return;
  // D1, and this is the whole of it: THE BID THAT TOOK THE LAST MATCH IS THE PRINT. The solver
  // finds how much trades — the crossing, which is a fact about both sides — but the level it
  // reports is the crossing's, and in a slack market the crossing sits on a SELLER's reservation.
  // A wage struck there is a level no employer offered, which is a level the venue made up. So the
  // wage is read off the book: the lowest bid that was actually allotted. Everyone who bid above it
  // fills and keeps the difference (D1.a); the one that bid it is the marginal employer and keeps
  // nothing. The `prices` family checks the print against the bids for exactly this reason.
  const struck = marginalBid(outcome);
  if (struck === undefined) return;
  match(ctx, book, outcome, offers, struck, occupation, region as RegionId, p);
  // D1: the occupation's print. It is a wage, not an instrument's price, so it is an event and not
  // a mark: nothing is valued at it (Law 8: the unit is money per hour).
  ctx.record(
    'labour.print',
    [v.id],
    {
      venue: v.id,
      occupation,
      region,
      wagePerHour: struck,
      hours: outcome.volume,
      rationed: outcome.rationed,
      // D1: the levels employers actually posted this session, so the print can be checked against
      // them rather than taken on trust (Part XII: no posted benchmark, in the one market whose
      // price is not an instrument's). It is public because every bid in this venue is (C5).
      bids: bids.map((b) => b.price),
    },
    true,
  );
}

/**
 * D1, D3, A4.b: who actually gets the job. The solver said what the wage is and how many hours
 * changed hands; this says which people, and people are whole (A4.b). Each employer's hours become
 * a headcount, largest bid first; the seekers who would work for least are taken first, and among
 * seekers who would work for the same — which is most of them — somebody has to be taken and
 * somebody left, in a stable order. That is what makes matching imperfect (D3) rather than a
 * fraction of every seeker being employed a fraction of the time, which is nobody being employed.
 */
/**
 * D1: the lowest level among the bids that were allotted — the bid that took the last match. None
 * means nothing on the buy side filled, and then there is no wage to print and nobody is hired.
 */
function marginalBid(outcome: Cleared): number | undefined {
  let lowest: number | undefined;
  for (const f of outcome.fills) {
    if (f.side !== 'buy' || f.qty <= 0) continue;
    if (lowest === undefined || f.at < lowest) lowest = f.at;
  }
  return lowest;
}

function match(
  ctx: MechanismContext,
  book: EmploymentBook,
  outcome: Cleared,
  offers: readonly Order[],
  /** D1: the lowest allotted bid, which is what every match is struck at. */
  struck: number,
  occupation: string,
  region: RegionId,
  p: LabourParams,
): void {
  const queue = offers
    .filter((o) => o.price !== 'market' && o.price <= struck)
    .sort((a, b) => (a.price === b.price ? (a.party < b.party ? -1 : 1) : Number(a.price) - Number(b.price)))
    .map((o) => o.party);
  const bidsFilled = outcome.fills
    .filter((f) => f.side === 'buy')
    .sort((a, b) => b.at - a.at);
  let next = 0;
  for (const f of bidsFilled) {
    let people = Math.floor(div(f.qty, p.hoursPerMember, 'people hired'));
    while (people > 0 && next < queue.length) {
      const cell = queue[next];
      if (cell === undefined) break;
      const available = weightOf(ctx.parties.get(cell));
      if (available <= 0) {
        next += 1;
        continue;
      }
      const taken = people < available ? people : available;
      hire(ctx, book, f.party, cell, taken, struck, occupation, region, p);
      people = sub(people, taken, 'people left to hire');
      if (taken === available) next += 1;
    }
  }
}

/** A4.b, A4.c: a hire moves a whole number of people, and part of a cell splits off first. */
function hire(
  ctx: MechanismContext,
  book: EmploymentBook,
  employer: PartyId,
  worker: PartyId,
  members: number,
  wagePerHour: number,
  occupation: string,
  region: RegionId,
  p: LabourParams,
): void {
  const cell = ctx.parties.get(worker);
  if (members <= 0) return;
  const whole = members >= weightOf(cell);
  const hired = whole ? worker : ctx.cells.split(worker, members, `hired by ${employer}`);
  const row: EmploymentRow = {
    id: `row.${book.next}`,
    employer,
    worker: hired,
    occupation,
    region,
    wagePerHour,
    hoursPerMember: p.hoursPerMember,
    start: ctx.period,
    // C2: finding somebody is not having them; the person is productive after the hiring lag.
    productiveFrom: periodOf(ctx.period + p.hiringLagPeriods),
    headcount: weightOf(ctx.parties.get(hired)),
  };
  book.next += 1;
  book.rows[row.id] = row;
  book.skill[hired] = occupation;
  ctx.record(
    'labour.hire',
    [employer, hired],
    {
      row: row.id,
      employer,
      worker: hired,
      occupation,
      headcount: row.headcount,
      wagePerHour,
      productiveFrom: row.productiveFrom,
    },
    true,
  );
}

/** C3: the employer sheds hours it no longer wants, oldest row first, and pays to do it. */
function shed(
  ctx: MechanismContext,
  book: EmploymentBook,
  employer: PartyId,
  occupation: string,
  region: RegionId,
  hours: number,
  p: LabourParams,
): void {
  let left = hours;
  const rows = allRows(book)
    .filter((r) => r.employer === employer && r.occupation === occupation && r.region === region)
    .sort((a, b) => b.start - a.start);
  for (const row of rows) {
    if (left <= 0) break;
    const members = Math.floor(div(left, row.hoursPerMember, 'members to separate'));
    if (members <= 0) break;
    const taken = members >= row.headcount ? row.headcount : members;
    separate(ctx, book, row, taken, `${employer} cut its hours`, p);
    left = sub(left, mul(taken, row.hoursPerMember, 'hours shed'), 'hours left to shed');
  }
}

/**
 * C3: a separation ends the relationship for those members and costs the employer severance, paid
 * to the people it separates. A4.c: separating part of a row splits its cell, so the members who
 * stay employed and the members who no longer are never share one state.
 */
export function separate(
  ctx: MechanismContext,
  book: EmploymentBook,
  row: EmploymentRow,
  members: number,
  cause: string,
  p: LabourParams,
): void {
  if (members <= 0) return;
  const whole = members >= row.headcount;
  const gone = whole ? row.worker : ctx.cells.split(row.worker, members, cause);
  if (whole) {
    book.rows = Object.fromEntries(Object.entries(book.rows).filter(([id]) => id !== row.id));
  } else {
    row.headcount = sub(row.headcount, members, 'headcount after separation');
  }
  // The trade stays with the person who has it: an unemployed baker looks for baking (A3).
  book.skill[gone] = row.occupation;
  const perMember = mul(wagePerMember(row), p.severancePeriods, 'severance per member');
  const paid = payFrom(ctx, row.employer, gone, perMember, `severance from ${row.employer}`);
  ctx.record(
    'labour.separation',
    [row.employer, gone],
    {
      row: row.id,
      employer: row.employer,
      worker: gone,
      occupation: row.occupation,
      members,
      cause,
      severancePerMember: perMember,
      severancePaid: paid,
    },
    true,
  );
}

/**
 * F1, E1, E2: the wage leaves the employer's account and reaches the worker's, every period — and
 * what an employer paid, and for how many hours, is a fact about that employer. It is recorded
 * under its name so the employer can read its own wage bill (Goods B5: it is part of what a unit
 * cost) without anybody keeping a second copy of the register (Law 4). It is private: what one
 * firm pays is between it and the people it employs, and what the market pays is the going rate.
 *
 * A wage that did not settle is a real state (Money E1): the employer had no money, the worker was
 * not paid, and the difference between what was due and what was paid says so.
 */
export function payWages(ctx: MechanismContext, book: EmploymentBook): void {
  const bills = new Map<PartyId, { due: number; paid: number; hours: number; productive: number; headcount: number }>();
  for (const row of allRows(book)) {
    if (!ctx.parties.get(row.employer).status.alive) continue;
    const perMember = wagePerMember(row);
    const settled = payFrom(ctx, row.employer, row.worker, perMember, `wages from ${row.employer}`);
    const bill = bills.get(row.employer) ?? { due: 0, paid: 0, hours: 0, productive: 0, headcount: 0 };
    const total = mul(perMember, row.headcount, 'wage bill');
    const hours = mul(row.hoursPerMember, row.headcount, 'hours under contract');
    bills.set(row.employer, {
      due: add(bill.due, total, 'wages due'),
      paid: add(bill.paid, settled ? total : 0, 'wages paid'),
      hours: add(bill.hours, hours, 'hours'),
      // C2: hours that can make something. Somebody found last period is paid and not yet working.
      productive: add(bill.productive, row.productiveFrom <= ctx.period ? hours : 0, 'productive hours'),
      headcount: add(bill.headcount, row.headcount, 'headcount'),
    });
  }
  for (const [employer, bill] of bills) {
    ctx.record('labour.wages', [employer], { ...bill }, false);
  }
}

/** One payment from a named payer to a named cell, per member (XI-15). Returns whether it settled. */
function payFrom(
  ctx: MechanismContext,
  payer: PartyId,
  cell: PartyId,
  perMember: number,
  reason: string,
): boolean {
  if (perMember <= 0) return true;
  const from = ctx.parties.get(payer);
  const to = ctx.parties.get(cell);
  const side = cellSide(to, perMember);
  const leg: Leg = {
    kind: 'money',
    from: { holder: payer, issuer: from.bank },
    to: { holder: cell, issuer: to.bank },
    ccy: ctx.registry.region(from.region).ccy,
    amount: totalFor(to, perMember),
    fromCell: none(),
    toCell: side === undefined ? none() : some(side),
  };
  // A failed wage is a real state, recorded by settlement: the employer did not have the money.
  return ctx.settle({ legs: [leg], cause: 'transfer', reason }).outcome === 'settled';
}

/**
 * D1.c, Observer A5: the going rate is a read over the rows, published about the period that has
 * closed. It causes nothing by itself — a firm forms its own outlook of what it must offer (XI-16)
 * — and it is the only channel from what is paid to what a bid is worth comparing against (D5).
 */
export function publishGoingRate(
  ctx: MechanismContext,
  book: EmploymentBook,
  venues: readonly VenueDecl[],
  now: Period,
): void {
  if (now === periodOf(0)) return;
  const rows: Record<string, number> = {};
  for (const v of venues) {
    const occupation = v.key['occupation'];
    const region = v.key['region'];
    if (occupation === undefined || region === undefined) continue;
    const rate = goingRate(book, occupation, region as RegionId);
    if (rate !== undefined) rows[v.id] = rate;
  }
  if (Object.keys(rows).length === 0) return;
  ctx.record(
    'labour.goingRate',
    [],
    { of: now - 1, wagePerHour: rows, employed: employed(book) },
    true,
  );
}
