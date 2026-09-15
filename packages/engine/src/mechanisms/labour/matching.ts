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
import {
  type Cash,
  type PerPiece,
  asCash,
  asRatio,
  heldAsMoney,
  ratioOf,
  scale,
} from '../../core/measure.js';
import { period as periodOf, type Period } from '../../calendar/calendar.js';
import { clear, isCleared, type Cleared, type Order } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { agreementKindId, cohortId } from '../../core/ids.js';
import type { PartyId, RegionId } from '../../core/ids.js';
import { add, atMost, material, sub } from '../../core/num.js';
import type { AgreementTerms } from '../../register/agreements.js';


/**
 * XI-8, Labour B4, Firm Birth D2.b: WHAT AN EMPLOYER OWES A WORKER IT DID NOT PAY.
 *
 * Two kinds and not one, because they rank differently and a reader has to be able to tell them
 * apart: a wage in arrears is this period's pay that did not arrive; severance is what ending the
 * employment owed, and D2.b ranks it unsecured in the estate. They were one free-text `what` in two
 * spellings, which nothing could dispatch on.
 */
export const SEVERANCE_IN_ARREARS = agreementKindId('labour.severanceInArrears');

export interface SeveranceOwed extends AgreementTerms {
  readonly kind: typeof SEVERANCE_IN_ARREARS;
  readonly cause: string;
}

export const severanceOwed = (cause: string): SeveranceOwed => ({
  kind: SEVERANCE_IN_ARREARS,
  cause,
});
import type { Leg } from '../../ledger/instruction.js';
import { keyOf, weightOf, type Party, gridPerMember } from '../../parties/party.js';
import type { MechanismContext } from '../../world/context.js';
import {
  EMPLOYMENT,
  employmentOf,
  wagePerMember,
  type EmploymentRow,
  type EmploymentTerms,
} from '../../register/employment.js';
import { addQty, asQty, negQty, scaleQty, subQty } from '../../core/tick.js';
import type { Qty } from '../../core/tick.js';

/** The numbers the matching reads, all declared by the module (Law 2). */
/**
 * XI-10 (12b.1): WHAT THIS MODULE KNOWS — the occupation each cell can work in, which is the job it
 * last held. A person with no history can enter any occupation, at the bottom (A3.b); one with a
 * trade looks for that trade. The EMPLOYMENTS are the kernel's (`register/employment.ts`); this is
 * the one thing about a worker the register does not carry, because it is not a fact about a row.
 */
export interface SkillBook {
  skill: Record<string, string>;
}

export const emptySkills = (): SkillBook => ({ skill: {} });

/** A4, XI-8: the terms of a row as the kernel holds them, for a restatement that changes one. */
function termsOf(row: EmploymentRow): EmploymentTerms {
  return {
    kind: EMPLOYMENT,
    occupation: row.occupation,
    region: row.region,
    wagePerHour: row.wagePerHour,
    hoursPerMember: row.hoursPerMember,
    since: row.since,
    productiveFrom: row.productiveFrom,
    notice: row.notice,
    headcount: row.headcount,
  };
}

export interface LabourParams {
  /** Law 8: whole hours, as the venue counts somebody's time. A wage is never struck for part of one. */
  readonly hoursPerMember: Qty;
  readonly retirementAge: number;
  readonly hiringLagPeriods: number;
  readonly severancePeriods: number;
  /**
   * A3.b, XI-10 (13d): periods a person who CHANGES TRADE takes to become productive in the new
   * one, on top of the ordinary hiring lag. It is what moving between occupations costs, and it is
   * TIME rather than money on purpose: a retraining fee would be a flow with no payee (Law 5), and
   * what a new trade actually costs an employer is weeks of wages for work it does not yet get.
   */
  readonly retrainingPeriods: number;
}

/**
 * A3.b (13d): WHICH SEEKERS A ROUND IS FOR. The venues run twice: once for the people who have the
 * trade, and once for the people who do not and are still looking. It is not a kind branch — every
 * seeker goes through the same matching function, the same book and the same print — it is the
 * ORDER a labour market actually fills in: an employer takes somebody who can already do the job
 * before it takes somebody it has to teach, and it does that because teaching costs it weeks.
 */
export type Round = 'trade' | 'anywhere';

/** B3: whether a cohort's members are in the workforce at all, or out of it by age. */
function participates(ctx: MechanismContext, p: Party, retirementAge: number): boolean {
  if (p.representation !== 'cell' || !p.status.alive) return false;
  return ctx.registry.cohort(cohortId(keyOf(p, 'cohort'))).fromAge < retirementAge;
}



/**
 * Labour B3, A3, A3.b, Observer A4 (`A-43`, item 9.6): WHICH OF THE HOURS OFFERED THIS VENUE MAY
 * TAKE — the market's own rules, applied to what it gathered.
 *
 * The offers are the sellers' now: each household cell posted what it will work for, out of its own
 * view, through `gather` (`households/index.ts:willWork`). What the VENUE decides is who is in its
 * book, and that is three facts none of which is the seller's to know:
 *
 *  - B3: a person is in exactly ONE state, so a cell that already holds a job is not also looking.
 *    A hire splits the cell, so a cell is wholly employed or wholly not and the whole offer goes.
 *  - B1, F2: only the WORKFORCE is in the book — a cohort past the retirement age is inactive, and
 *    that is the same read this module's own workforce identity is measured against.
 *  - A3, A3.b: a job in one occupation is not a job in another. In the first round somebody who has
 *    worked looks for the trade they have; in the second it is the other way about, and who moves is
 *    whoever the first round left over.
 *
 * `supply` used to do all of this AND decide what each cell would work for, walking every household
 * in the world out of a `MechanismContext`. What it decided was the seller's; what is left here is
 * the market's.
 */
function eligible(
  ctx: MechanismContext,
  book: SkillBook,
  v: VenueDecl,
  occupation: string,
  region: RegionId,
  p: LabourParams,
  round: Round,
): Order[] {
  const out: Order[] = [];
  for (const offer of ctx.posted(v.id)) {
    if (offer.side !== 'sell' || offer.price === 'market' || offer.qty <= 0) continue;
    const cell = ctx.parties.get(offer.party);
    if (!participates(ctx, cell, p.retirementAge) || cell.region !== region) continue;
    if (ctx.employment.ofWorker(cell.id) !== undefined) continue;
    const skill = book.skill[cell.id];
    const hasTrade = skill === undefined || skill === occupation;
    if (round === 'trade' && !hasTrade) continue;
    if (round === 'anywhere' && hasTrade) continue;
    out.push(offer);
  }
  return out;
}

/**
 * The venue's session: what the employers posted against what they already employ, and the cells
 * that are looking. Returns the wage that cleared, if anything did.
 */
export function runVenue(
  ctx: MechanismContext,
  book: SkillBook,
  v: VenueDecl,
  p: LabourParams,
  round: Round = 'trade',
): void {
  const occupation = v.key['occupation'];
  const region = v.key['region'];
  if (occupation === undefined || region === undefined) return;
  const bids: Order[] = [];
  for (const posting of ctx.posted(v.id)) {
    if (posting.side !== 'buy' || posting.price === 'market') continue;
    const held = ctx.employment.hoursAt(posting.party, occupation, region as RegionId);
    const gap = subQty(posting.qty, held, 'employment gap');
    if (!material(gap, 2, addQty(posting.qty, held, 'employment'))) continue;
    // Both sides of this subtraction are counts of hours — what it posted and what it employs —
    // so the gap is one too, and nothing was rounded to get it.
    if (gap > 0) bids.push({ party: posting.party, side: 'buy', price: posting.price, qty: asQty(gap, 'the hours it is short') });
    // C3, C4: the employer wants fewer hours than it has under contract, so it separates the
    // difference and pays for doing it. It is the employer's decision; this is the mechanism.
    // C3: and only once. An employer posts its desired employment for the period, so the round
    // that follows it is the same posting still being filled — separating twice against one
    // posting would be paying severance for a decision it took once.
    else if (round === 'trade') {
      shed(ctx, book, posting.party, occupation, region as RegionId, negQty(gap, 'the hours it is over'), p);
    }
  }
  const offers = eligible(ctx, book, v, occupation, region as RegionId, p, round);
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
  match(ctx, book, outcome, offers, struck, occupation, region as RegionId, p, round);
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
      // A3.b: which round this print came out of, so a reader can tell a market that filled from
      // its own trade from one that filled by teaching somebody a new one.
      round,
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
function marginalBid(outcome: Cleared): PerPiece | undefined {
  let lowest: PerPiece | undefined;
  for (const f of outcome.fills) {
    if (f.side !== 'buy' || f.qty <= 0) continue;
    if (lowest === undefined || f.at < lowest) lowest = f.at;
  }
  return lowest;
}

function match(
  ctx: MechanismContext,
  book: SkillBook,
  outcome: Cleared,
  offers: readonly Order[],
  /** D1: the lowest allotted bid, which is what every match is struck at. */
  struck: PerPiece,
  occupation: string,
  region: RegionId,
  p: LabourParams,
  round: Round,
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
    let people = Math.floor(ratioOf(f.qty, p.hoursPerMember, 'people hired'));
    while (people > 0 && next < queue.length) {
      const cell = queue[next];
      if (cell === undefined) break;
      const available = weightOf(ctx.parties.get(cell));
      if (available <= 0) {
        next += 1;
        continue;
      }
      const taken = atMost(people, available, 'there are no more people in the cell than there are');
      hire(ctx, book, f.party, cell, taken, struck, occupation, region, p, round);
      people = sub(people, taken, 'people left to hire');
      if (taken === available) next += 1;
    }
  }
}

/** A4.b, A4.c: a hire moves a whole number of people, and part of a cell splits off first. */
function hire(
  ctx: MechanismContext,
  book: SkillBook,
  employer: PartyId,
  worker: PartyId,
  members: number,
  wagePerHour: PerPiece,
  occupation: string,
  region: RegionId,
  p: LabourParams,
  round: Round,
): void {
  if (members <= 0) return;
  // 0f.4: the hired move to the standing cell of the employed key; there is no split.
  const hired = ctx.cells.reKey(worker, members, { employment: 'employed' }, `hired by ${employer}`);
  // XI-8: the row IS the commitment, so the kernel writes it and gives it its identity — there is
  // no `book.next` any more, and no employment id this module invented (item 9.1).
  // An employment owes NOTHING the instant it is struck: the wage falls due at the end of the
  // period and is paid then, and a wage that does not arrive is the employer's arrear (Money E1).
  const terms: EmploymentTerms = {
    kind: EMPLOYMENT,
    occupation,
    region,
    wagePerHour,
    hoursPerMember: p.hoursPerMember,
    since: ctx.period,
    // C2: finding somebody is not having them; the person is productive after the hiring lag —
    // and after the retraining on top of it when they are changing trade (A3.b). That is what
    // mobility costs, it costs the EMPLOYER, and it is weeks of wages for work it does not get.
    productiveFrom: periodOf(
      add(
        ctx.period + p.hiringLagPeriods,
        round === 'anywhere' ? p.retrainingPeriods : 0,
        'and what teaching them takes',
      ),
    ),
    // C3 (12b.1): the notice the job carries — the periods of pay a separation owes (12b.2 runs it).
    notice: p.severancePeriods,
    headcount: weightOf(ctx.parties.get(hired)),
  };
  const row = employmentOf(
    ctx.owes({
      debtor: employer,
      creditor: hired,
      ccy: ctx.registry.currencyOf(region),
      owed: 0,
      terms,
      why: `${hired} works for ${employer} as a ${occupation}`,
    }),
  );
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
      // A3.b: whether this person changed trade to take it, which is what XI-10 is about.
      moved: round === 'anywhere',
    },
    true,
  );
}

/**
 * C3: the employer sheds hours it no longer wants, NEWEST row first, and pays to do it.
 *
 * `b.start - a.start` is descending by start period — last in, first out, which is the ordinary
 * redundancy convention and what this has always done. The comment said "oldest row first" and the
 * code has never done that: one of the two was wrong (Law 16) and it was the comment.
 */
function shed(
  ctx: MechanismContext,
  book: SkillBook,
  employer: PartyId,
  occupation: string,
  region: RegionId,
  hours: Qty,
  p: LabourParams,
): void {
  let left = hours;
  const rows = ctx.employment.at(employer, occupation, region);
  for (const row of rows) {
    if (left <= 0) break;
    const members = Math.floor(ratioOf(left, row.hoursPerMember, 'members to separate'));
    if (members <= 0) break;
    const taken = atMost(members, row.headcount, 'the row employs no more than it employs');
    separate(ctx, book, row, taken, `${employer} cut its hours`, p);
    left = subQty(left, scaleQty(row.hoursPerMember, taken, 'hours shed'), 'hours left to shed');
  }
}

/**
 * C4, Firm Birth D4.a: an employer that has ceased releases its workers AT ONCE, and it does it
 * through the same separation path as any other separation — never by a headcount going down
 * somewhere, which is the shape the clause forbids. What triggers it is the party store itself:
 * the row names an employer that is dead, whatever killed it and whoever is winding it up.
 */
export function release(ctx: MechanismContext, book: SkillBook, p: LabourParams): void {
  for (const row of ctx.employment.all()) {
    if (ctx.parties.get(row.employer).status.alive) continue;
    separate(ctx, book, row, row.headcount, `${row.employer} ceased`, p);
  }
}

/**
 * C3: a separation ends the relationship for those members and costs the employer severance, paid
 * to the people it separates. A4.c: separating part of a row splits its cell, so the members who
 * stay employed and the members who no longer are never share one state.
 */
export function separate(
  ctx: MechanismContext,
  book: SkillBook,
  row: EmploymentRow,
  members: number,
  cause: string,
  p: LabourParams,
): void {
  if (members <= 0) return;
  const whole = members >= row.headcount;
  // 0f.4: the separated move to the standing cell of the unemployed key; there is no split.
  // Register F2, XI-15 (12.4a.2): THE WORKER AS IT IS NOW. The row's terms name the cell that
  // signed; a cell that merged onto the standing cell of its key has ceased and is succeeded, and
  // re-keying the name it had merged it a second time — the world stopped there in period 5 of a
  // scale model. What is separated is the successor's members.
  const worker = ctx.parties.resolve(row.worker).id;
  const gone = ctx.cells.reKey(worker, members, { employment: 'unemployed' }, cause);
  if (whole) {
    // XI-8: TERMINATED and not discharged — the commitment ended by its own terms, and the row
    // stays in the kernel's book saying so. A job that vanished would leave a severance nothing
    // could be a severance FROM.
    ctx.endAgreement(row.id, cause);
  } else {
    // A4.c, Law 15: part of the cell left, so the row's terms changed and the commitment did not.
    // It used to be `row.headcount = ...` on a mutable object in a private book; the kernel's row
    // is frozen, and a change of terms is an event with its own record (item 9.1).
    const fewer: EmploymentTerms = { ...termsOf(row), headcount: sub(row.headcount, members, 'headcount after separation') };
    ctx.restate(row.id, fewer);
  }
  // The trade stays with the person who has it: an unemployed baker looks for baking (A3).
  book.skill[gone] = row.occupation;
  const perMember = scale(
    wagePerMember(row),
    asRatio(p.severancePeriods, 'the periods of it'),
    'severance per member',
  );
  // C3, XI-8: severance is a cost the employer pays — while there is an employer to pay it. One
  // that has ceased owes it to the claimants on its estate, and Firm Birth D2.b says a claim like
  // that RANKS with the other unsecured ones and is paid in the distribution, not in cash at the
  // door: a liquidator does not borrow to settle a claim it is winding up. There is no instrument
  // for it to rank AS until trade payables exist (worklist 13), so it is recorded owed and unpaid.
  // That is a missing mechanism named, not a payment invented (Part II: MISSING is an answer).
  const trading = ctx.parties.get(row.employer).status.alive;
  const paid =
    trading && payFrom(ctx, row.employer, gone, perMember, `severance from ${row.employer}`) > 0;
  /**
   * A-41, XI-8: AND NOW THERE IS SOMEWHERE FOR IT TO RANK. A trading employer that could not pay
   * opened its claim inside `payFrom`; a ceased one never tried, so it opens here. Either way the
   * worker is a named creditor of the employer for what it was owed and did not get, which is the
   * fact `severanceRanking: 0` used to deny for exactly the case that needed it most.
   */
  if (!trading && perMember > 0) {
    const ccy = ctx.registry.currencyOf(ctx.parties.get(row.employer).region);
    const owed = gridPerMember(ctx.registry, ctx.parties.get(gone), perMember).total;
    if (owed > 0) {
      ctx.owes({
        debtor: row.employer,
        creditor: gone,
        ccy,
        owed,
        terms: severanceOwed(cause),
        why: `${row.employer} ceased owing ${cause} severance; Firm Birth D2.b ranks it unsecured`,
      });
    }
  }
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
      // What is owed, per member. It said 0 whenever the employer was still trading — including
      // when its payment had just failed — which recorded "nothing owed" about a worker who worked
      // and was not paid. What is unpaid is owed, and the agreement above is where it ranks.
      severanceRanking: paid ? 0 : perMember,
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
/**
 * E1, F1 (12b.1): WAGES ARE INSTRUCTIONS THAT READ THE REGISTER. One money leg per row, from the
 * employer's account to the worker cell's, for what the row says; what was PAID is the ledger's
 * and what is DUE is the row's, and nothing is tallied beside either. It wrote one `labour.wages`
 * event per employer — hours, due, paid, productive, headcount — and every other employer read its
 * own payroll back off that copy (Law 19, Law 4).
 */
export function payWages(ctx: MechanismContext): void {
  for (const row of ctx.employment.all()) {
    if (!ctx.parties.get(row.employer).status.alive) continue;
    payFrom(ctx, row.employer, ctx.parties.resolve(row.worker).id, wagePerMember(row), `wages from ${row.employer}`);
  }
}

function payFrom(
  ctx: MechanismContext,
  payer: PartyId,
  cell: PartyId,
  perMember: number,
  reason: string,
): Cash {
  const nothing = asCash(0, 'nothing moved');
  if (perMember <= 0) return nothing;
  const from = ctx.parties.get(payer);
  const to = ctx.parties.get(cell);
  const ccy = ctx.registry.currencyOf(from.region);
  // Law 8, E1: a wage is paid in whole pieces of the money, to each worker separately — the cell is
  // a count of people and every one of them is paid the same whole number of pieces. What the
  // fraction below one would have been is not paid, because there is no such coin.
  const share = gridPerMember(ctx.registry, to, perMember);
  // A per-member wage below one piece of the money pays NOTHING — there is no such coin — and this
  // used to answer `true`, which is how a wage with no money leg behind it was booked as paid.
  if (share.total <= 0) return nothing;
  const leg: Leg = {
    kind: 'money',
    from: ctx.accountOf(payer, ccy),
    to: ctx.accountOf(cell, ccy),
    // Treasury C1: THE PAYER SAYS WHAT THIS IS. An employer knows it is paying a wage, and the
    // taxman working it out from the shape of the wire is what taxed a returned principal as income.
    receipt: { of: 'wage' },
    ccy,
    amount: share.total,
  };
  // A failed wage is a real state, recorded by settlement: the employer did not have the money.
  // The row it then owes is the ARREAR settlement writes in the same pass (Money E1, 12a.1); the
  // `labour.wagesInArrears` agreement this wrote beside it was the same debt twice (Law 4).
  const r = ctx.settle({ legs: [leg], cause: 'transfer', reason });
  if (r.outcome === 'settled') return heldAsMoney(share.total, 'what the employer actually paid');
  return nothing;
}

/**
 * D1.c, Observer A5: the going rate is a read over the rows, published about the period that has
 * closed. It causes nothing by itself — a firm forms its own outlook of what it must offer (XI-16)
 * — and it is the only channel from what is paid to what a bid is worth comparing against (D5).
 */
export function publishGoingRate(
  ctx: MechanismContext,
  venues: readonly VenueDecl[],
  now: Period,
): void {
  if (now === periodOf(0)) return;
  const rows: Record<string, number> = {};
  for (const v of venues) {
    const occupation = v.key['occupation'];
    const region = v.key['region'];
    if (occupation === undefined || region === undefined) continue;
    const rate = ctx.employment.goingRate(occupation, region as RegionId);
    if (rate !== undefined) rows[v.id] = rate;
  }
  if (Object.keys(rows).length === 0) return;
  ctx.record(
    'labour.goingRate',
    [],
    { of: now - 1, wagePerHour: rows, employed: ctx.employment.employed() },
    true,
  );
}
