/**
 * Equity: the residual claim on a firm, priced by opinions that disagree, and what the firm does
 * with it.
 *
 * @spec Equity A1 Equity A2 Equity A3 Equity A4 Equity A5 Equity A6 Equity B1 Equity B2 Equity B3 Equity B4 Equity B4.a Equity B6 Equity C1 Equity C1.a Equity C1.b Equity C2 Equity C2.a Equity C2.e Equity C3 Equity C4 Equity D1 Equity D1.a Equity D1.b Equity D1.c Equity D2 Equity D2.a Equity D2.b Equity D2.c Equity D3 Equity D3.a Equity D3.b Equity D4 Equity E4 Equity F1 Equity F2 Equity F3 Equity F4 Equity G3 Register E1 Register E1.a Register E4 Register E5 Clearing B2 Clearing C4.a Firm E4 Firm E5 XI-13 XI-15 Law 4 Law 9 Law 15
 *
 * WHY IT IS HERE. Nothing in this world had a price that was not a promise: a bond is worth its
 * flows discounted, a bill is worth its face, a good is worth what a buyer paid for a thing it
 * needed. A SHARE PROMISES NOTHING. There is no stream to discount that anybody is entitled to,
 * so the only thing that can price it is what participants think, and the only reason they trade
 * is that they think different things (§46 A3, XI-13). It is the first instrument in this world
 * whose price is nothing but disagreement, and it is what item 10 needs before it can ask what
 * equity costs a firm.
 *
 * WHAT IS NOT HERE. A share is a residual claim on a firm and the firm's own books are not a price
 * of it (B3): what the firm thinks one is worth is its own reservation and it is used as one — to
 * decide whether it would rather buy its own back or pay the money out, and as the level it will
 * not sell below. Every price in this module's markets is what a session cleared at, or it is a
 * stale mark that says so (Clearing E4).
 *
 * WHAT A FIRM'S DEATH DOES TO IT (E4). Nothing here wipes anything. A share ranks LAST (share.ts),
 * so the estate that succeeds a dead firm reaches its shareholders only after every creditor is
 * paid, pays them what is left, and writes off the rest at zero — which is the wipe, arrived at by
 * the waterfall rather than by a special case, and it is also the residual F2 entitles them to when
 * there is one. What this module does is stop pricing the line: a claim on a liquidation is not the
 * claim anybody formed an opinion of, so nobody posts and the print goes visibly stale.
 */
import type { Family, Violation } from '../../audit/audit.js';
import type { AuditView } from '../../audit/view.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { currencyUnit, type InstrumentId, type PartyId } from '../../core/ids.js';
import { combineDust, div, material, mul, sub, sum, withinDust } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import { cellSide, shareFor } from '../../ledger/settlement.js';
import { weightOf } from '../../parties/party.js';
import { issuerOf, type Instrument } from '../../register/instruments.js';
import { MONEY_PIECES } from '../../registry/grid.js';
import { displayName } from '../../registry/naming.js';
import type { ParamDecl } from '../../registry/params.js';
import { FIRM } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { LISTED, OPENING_SHARE, equityParam, listedOf, equityLineOf, equityMarketOf, type ListedDecl } from './data.js';
import { buybackOrder, decideEquity, dividendFor, type EquityPlan } from './decide.js';
import { freeFloat, marketCapitalisation } from './opinion.js';
import { SHARE, shareKind, shareTerms, votesOf, type ShareTerms } from './share.js';

export * from './data.js';
export * from './share.js';
export { decideEquity, buybackOrder, dividendFor } from './decide.js';
export type { EquityPlan } from './decide.js';
export { marketCapitalisation, freeFloat } from './opinion.js';

/** What this module remembers between periods: nothing but what it has already said out loud. */
interface Book {
  /** E4: the lines whose issuer has been succeeded, so the wipe is announced once and not weekly. */
  succeeded: string[];
}

function book(ctx: MechanismContext): Book {
  return ctx.state<Book>('equity', () => ({ succeeded: [] }));
}

/**
 * A6, Register F2: whether the firm that PROMISED this line still owes it. An estate succeeding a
 * dead firm becomes the issuer of record and the terms still name who promised it, so the two
 * disagreeing is exactly the statement "the firm is gone" — read off the instrument rather than
 * inferred from a party's kind or from an event somebody has to remember.
 */
function stillItsIssuer(i: Instrument): boolean {
  return issuerOf(i) === shareTerms(i).issuer;
}

function paramsOf(rows: readonly ListedDecl[]): ParamDecl[] {
  return [
    {
      id: OPENING_SHARE,
      value: MONEY_PIECES,
      unit: 'pieces of money per share at the seed (one PHX)',
      kind: 'resolution',
      owner: 'model',
      why: 'Seed C4, Law 2: a market that has never traded has no price (XI-6) and a world that opens with shares outstanding has to say what one is. It is a RESOLUTION: double it and halve every share count the seed states and no value, flow or decision moves — which is exactly what a split does (D4), so the invariance is a mechanism in this world and a test of it, not a claim about one.',
    },
    ...rows.map((r): ParamDecl => ({
      id: equityParam(r.firm, 'payoutPatience'),
      value: r.payoutPatience,
      unit: 'periods',
      kind: 'preference',
      owner: 'model',
      why: `Equity D2.c, Firm E5: over how many of its own periods ${r.firm}'s management distributes what it has spare. ${r.why} It is the whole of how fast money leaves the firm for its owners, and two managements that are not equally patient distribute differently out of the same cash.`,
    })),
  ];
}

/**
 * D1, D2, D3: the firm's own decision about its own line, taken with the firm's own view, once,
 * published under its own name, and read back by the order it posts (Law 4).
 *
 * It runs after the firm has planned its period, because what it has spare is what its own funding
 * read said (Firm E4) — one number, one writer — and before the session, because a decision that
 * read the session it is about to be in would be reading its own answer (Clearing A4).
 */
function decide(ctx: MechanismContext, row: ListedDecl): void {
  const line = ctx.instruments.get(equityLineOf(row.firm));
  if (!line.status.live) return;
  if (!stillItsIssuer(line)) {
    announceSuccession(ctx, row, line);
    return;
  }
  const firm = row.firm as PartyId;
  if (!ctx.parties.get(firm).status.alive) return;
  const view = ctx.participant(firm);
  const funding = view.lastOwn('firms.funding');
  if (!funding.some || funding.value.period !== ctx.period) return;
  const short = funding.value.data['short'];
  if (typeof short !== 'number') return;
  const decided = decideEquity(
    view,
    line.id,
    equityMarketOf(row.firm),
    line.issued,
    sub(0, short, 'what it has spare'),
    ctx.params.get(equityParam(row.firm, 'payoutPatience')),
  );
  if (!decided.some) return;
  const plan = decided.value;
  // D1, D1.c: new shares are the ISSUER's own supply for this session, cleared by the same solver
  // at one price with everybody else's orders in the book — and withdrawn if nobody will pay the
  // least it will take, which is a failed issue and has consequences (Clearing C4.a).
  if (plan.issue > 0) {
    ctx.offer({
      market: plan.market,
      issuer: firm,
      size: plan.issue,
      reservation: plan.reservation,
      allotment: 'uniformPrice',
    });
  }
  ctx.record(
    'equity.plan',
    [firm, line.id],
    {
      line: line.id,
      bookPerShare: plan.bookPerShare,
      buyback: plan.buyback,
      issue: plan.issue,
      reservation: plan.reservation,
      dividendPerShare: plan.dividendPerShare,
    },
    false,
  );
  if (plan.dividendPerShare > 0) payDividend(ctx, row, line, plan);
}

/**
 * D3, D3.a, Register E1, E1.a: cash per share, out of the firm and into the accounts of whoever
 * the register says holds it AT THE MOMENT IT IS APPLIED. There is no earlier date to remember: a
 * buyer between two dividends paid for what it bought at the price it paid (B2), and what a share
 * has paid so far is public, which is what anybody forming an opinion of it reads (B3).
 *
 * The declaration is PUBLIC and carries the number, because D3.b's cut is only an event others
 * react to if others can see it.
 */
function payDividend(ctx: MechanismContext, row: ListedDecl, line: Instrument, plan: EquityPlan): void {
  const firm = row.firm as PartyId;
  const paid: number[] = [];
  let failed = 0;
  for (const holderId of ctx.register.holdersOf(line.id)) {
    if (holderId === firm) continue;
    const holder = ctx.parties.get(holderId);
    const perMemberUnits = ctx.register.quantity(holderId, line.id);
    if (perMemberUnits <= 0) continue;
    // Law 8: a dividend is paid in whole pieces of the money, and to each member of a cell in
    // whole pieces — every one of them is a shareholder with an account of their own. A holding so
    // small that its share comes to less than one piece is paid nothing, which is what a payout
    // per share that small means.
    const share = shareFor(
      ctx.registry,
      holder,
      currencyUnit(line.ccy),
      dividendFor(plan.dividendPerShare, perMemberUnits),
    );
    const perMemberCash = share.perMember;
    const total = share.total;
    if (!material(total, 2, total) || total <= 0) continue;
    const side = cellSide(holder, perMemberCash);
    const leg: Leg = {
      kind: 'money',
      from: { holder: firm, issuer: ctx.parties.get(firm).bank },
      to: { holder: holderId, issuer: holder.bank },
      ccy: line.ccy,
      amount: total,
      fromCell: none(),
      toCell: side === undefined ? none() : some(side),
    };
    const r = ctx.settle({
      legs: [leg],
      cause: 'corporateAction',
      reason: `payout on ${line.id} to ${holderId}`,
    });
    if (r.outcome === 'settled') paid.push(total);
    else failed += 1;
  }
  ctx.record(
    'payout.declared',
    [firm, line.id],
    {
      line: line.id,
      perShare: plan.dividendPerShare,
      shares: line.issued,
      paid: sum(paid).value,
      failedPayments: failed,
    },
    true,
  );
}

/**
 * E4, Register F2: the firm that promised this line is gone and an estate owes it now. Said once,
 * publicly, because everybody holding it needs to know that what they hold is a claim on a
 * liquidation — and then this module stops pricing the line: nobody posts an opinion of a firm that
 * no longer exists, the session finds no orders, and the print goes visibly stale (Clearing E4)
 * until the estate pays what it can and writes off the rest at zero.
 */
function announceSuccession(ctx: MechanismContext, row: ListedDecl, line: Instrument): void {
  const b = book(ctx);
  if (b.succeeded.includes(row.firm)) return;
  b.succeeded.push(row.firm);
  ctx.record(
    'equity.succeeded',
    [row.firm, line.id, issuerOf(line)],
    {
      line: line.id,
      promisedBy: row.firm,
      owedBy: issuerOf(line),
      shares: line.issued,
      why: 'the firm has ceased: its shares rank last on its estate and are worth what is left, which may be nothing (Equity E4, F2)',
    },
    true,
  );
}

/**
 * D3.a, Fund Shares B3: what was DECLARED is what left the issuer, or a payment failed and is on
 * the record as one.
 *
 * A payout that is declared and does not arrive is an issuer that could not pay it, which is a real
 * state (Money E1) and the beginning of a great deal else. One that is declared and quietly never
 * leaves anybody's account is money credited to holders out of nothing, and it would look exactly
 * like an issuer that paid.
 *
 * It checks every declared payout and not only a firm's. What an issuer said it would pay per unit
 * is one public fact with one shape whoever said it — a firm declaring a dividend (D3) or a fund
 * passing on what it received (B3) — so it is one check, over the events and the ledger, both of
 * which are the kernel's and neither of which belongs to either module.
 */
function declaredIsPaid(): Family {
  return {
    name: 'flows',
    contributor: 'equity',
    spec: 'Equity D3 Equity D3.a Fund Shares B3 Register E5',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const e of view.journal.ofKind('payout.declared')) {
        if (e.period !== view.period) continue;
        const line = e.data['line'];
        const paid = e.data['paid'];
        const firm = e.subjects[0];
        if (typeof line !== 'string' || typeof paid !== 'number' || firm === undefined) continue;
        const moved = dividendLegs(view, line, firm);
        if (withinDust(paid, moved.value, combineDust(sum([paid]), moved))) continue;
        out.push({
          family: 'flows',
          spec: 'Equity D3.a',
          owner: firm,
          size: sub(paid, moved.value, 'recorded paid against what left the firm'),
          unit: view.registry.region(view.parties.get(firm as PartyId).region).ccy,
          period: view.period,
          message: `${line}: ${paid} of payout was recorded paid and ${moved.value} left ${firm}`,
        });
      }
      return out;
    },
  };
}

/** What actually left the issuer this period on a payout instruction (Law 19: read the legs). */
function dividendLegs(view: AuditView, line: string, firm: string): ReturnType<typeof sum> {
  const terms: number[] = [];
  for (const r of view.ledger.inPeriod(view.period)) {
    if (r.outcome !== 'settled') continue;
    if (!r.instruction.reason.startsWith(`payout on ${line} `)) continue;
    for (const leg of r.instruction.legs) {
      if (isMoneyLeg(leg) && leg.from.holder === firm) terms.push(leg.amount);
    }
  }
  return sum(terms);
}

/**
 * C1.b, F3, B4: the reads. Published every period so the observer has them and so the numbers a
 * later item needs — a takeover's majority (A5.a, worklist 13g), an index's float weight (G1,
 * worklist 12) — are one derivation with one writer. G3: every one is computed FROM the cleared
 * price and none of them is ever used to set it.
 */
function publishReads(ctx: MechanismContext, rows: readonly ListedDecl[]): void {
  for (const row of rows) {
    const line = ctx.instruments.get(equityLineOf(row.firm));
    if (!line.status.live) continue;
    const terms = shareTerms(line);
    const voteTerms: number[] = [];
    // C1.b: what is NOT tradeable is what is bound — a block somebody has undertaken not to sell
    // is units the register says are encumbered, and only free units can move (Register D5.a). No
    // holder in this world has bound any yet: the party that does is a founder, and a founder is
    // a party that funded a firm's entry (Firm Birth A, worklist 13g).
    const boundTerms: number[] = [];
    for (const holder of ctx.register.holdersOf(line.id)) {
      const party = ctx.parties.get(holder);
      voteTerms.push(votesOf(party, ctx.register.quantity(holder, line.id), terms));
      boundTerms.push(mul(ctx.register.encumbered(holder, line.id), weightOf(party), 'units bound'));
    }
    const strategic = sum(boundTerms).value;
    const print = ctx.prices.latest(line.id, ctx.period);
    ctx.record(
      'equity.reads',
      [row.firm, line.id],
      {
        line: line.id,
        shares: line.issued,
        // C1.b: what is genuinely tradeable — the count less what is bound and cannot move.
        freeFloat: freeFloat(line.issued, strategic),
        strategic,
        // B4, B4.a: a read. Nothing compares it against its own two inputs and calls that a check.
        marketCapitalisation: print.some ? marketCapitalisation(line.issued, print.value.price) : null,
        // F3, A5.a: how many votes there are, and how many holders there are to cast them. Who
        // holds a majority is the register's to answer when somebody asks it (worklist 13g).
        votes: sum(voteTerms).value,
        holders: ctx.register.holdersOf(line.id).length,
      },
      true,
    );
  }
}

/**
 * The module. `rows` is which firms this world listed (Law 15: the data says). A firm that is not
 * in it has no shares at all, which is a real state and not an omission.
 */
export function equity(rows: readonly ListedDecl[] = LISTED): SystemModule {
  return {
    id: 'equity',
    spec: 'Equity',
    // The firm whose residual claim it is decides about it (Firm E4, E5) and publishes what it has
    // spare; the estate is what a share ranks last on; and the firms it lists are parties somebody
    // else's seed created, so that seed runs first. Its buyers decide in their own modules and need
    // nothing from this one but the line, the market and what the issuer declared.
    requires: ['firms', 'estate', 'seed.foundation'],
    instrumentKinds: [shareKind],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: paramsOf(rows),
    phases: [
      {
        name: 'equity.decide',
        spec: 'Equity D1 Equity D2 Equity D3 Firm E4 Firm E5',
        cycle: 0,
        // After the firm has planned its period, because what it has spare is its own funding read;
        // before the session, because everything it decides it decides on what has already
        // happened (Clearing F1).
        anchor: { after: 'firms.decide' },
        run: (ctx: MechanismContext) => {
          for (const row of rows) decide(ctx, row);
        },
      },
      {
        name: 'equity.reads',
        spec: 'Equity B4 Equity C1.b Equity F3 Equity G3',
        cycle: 'anchor',
        // After the session has printed and the marks are in the books: a read of a price is taken
        // once the price exists, never before (Clearing F1.a).
        anchor: { after: 'revaluation' },
        run: (ctx: MechanismContext) => {
          publishReads(ctx, rows);
        },
      },
    ],
    participants: [
      {
        // D2: the ONE reason a firm is in its own book. Every other reason to be there belongs to
        // the party that has it, in that party's own module (Households D5): a module that wrote
        // other people's schedules would be handing the market its answer (Clearing A3, XI-13).
        partyKind: FIRM,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          const own = view.lastOwn('equity.plan');
          if (!own.some || own.value.period !== view.period) return [];
          if (own.value.data['line'] !== m.instrument) return [];
          const buyback = own.value.data['buyback'];
          const bookPerShare = own.value.data['bookPerShare'];
          if (typeof buyback !== 'number' || typeof bookPerShare !== 'number') return [];
          return buybackOrder(view.self.id, buyback, bookPerShare);
        },
      },
    ],
    families: [declaredIsPaid()],
    seed(ctx: SeedContext): void {
      for (const row of rows) {
        const firm = ctx.parties.get(row.firm as PartyId);
        const ccy = ctx.registry.region(firm.region).ccy;
        // A5: one vote per share. It is a TERM of the instrument and not a parameter: what a share
        // of this line carries is its structure, fixed at issue (Seed C4.b), and a line with two
        // classes of vote is two lines (Register F1.a).
        const terms: ShareTerms = { kind: SHARE, issuer: row.firm as PartyId, votesPerShare: 1 };
        const id = equityLineOf(row.firm);
        const market = equityMarketOf(row.firm);
        ctx.instruments.add({
          id,
          kind: SHARE,
          issuer: some(row.firm as PartyId),
          ccy,
          terms,
          market: some(market),
        });
        ctx.openMarket({
          id: market,
          name: displayName(ctx.instruments.get(id), ctx.parties, ctx.registry),
          instrument: id,
          ccy,
          rationing: 'proRata',
        });
        // Seed C4: the line and the level it opens at. NOTHING is issued here: who opens holding
        // shares is endowment state and belongs to whoever holds them (Seed A3) — the desks that
        // open making a market in each line, exactly as the banks open holding sovereign paper.
        ctx.prices.write({
          instrument: id,
          market,
          period: ctx.period,
          price: ctx.params.get(OPENING_SHARE),
          ccy,
          provenance: { kind: 'opening' },
        });
      }
    },
  };
}

/** The listed row a firm is in, for the observer and the tests (Law 15). */
export const listed = (firm: string): ListedDecl | undefined => listedOf(LISTED, firm);

/** C1.b: how much of a line is bound and therefore not float — a read of the register (D5.a). */
export function strategicOf(ctx: MechanismContext, firm: string): number {
  const line = equityLineOf(firm);
  const terms: number[] = [];
  for (const holder of ctx.register.holdersOf(line)) {
    terms.push(
      mul(ctx.register.encumbered(holder, line), weightOf(ctx.parties.get(holder)), 'units bound'),
    );
  }
  return sum(terms).value;
}

/** B4: shares times price, read once here so nobody derives it a second way (Law 4). */
export function capitalisationOf(ctx: MechanismContext, firm: string): number | null {
  const line = ctx.instruments.get(equityLineOf(firm));
  const print = ctx.prices.latest(line.id, ctx.period);
  return print.some ? marketCapitalisation(line.issued, print.value.price) : null;
}

/** D2.a, D1.a: the count, which is what a dilution raises and a cancellation lowers. */
export function sharesOf(ctx: MechanismContext, firm: string): number {
  return ctx.instruments.get(equityLineOf(firm)).issued;
}

/** Law 7: a per-share number and the count it came from, for readers that need both. */
export function perShare(total: number, shares: number, what: string): number {
  return div(total, shares, what);
}

export type { InstrumentId };
