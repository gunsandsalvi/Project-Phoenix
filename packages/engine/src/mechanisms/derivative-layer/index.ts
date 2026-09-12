/**
 * The derivative layer: the infrastructure every class of contract runs on.
 *
 * @spec Derivative D1 Derivative D1.a Derivative D1.b Derivative D2 Derivative D2.a Derivative D3 Derivative D3.a Derivative D4 Derivative D5 Derivative D6 Derivative D7 Derivative D7.a Derivative D7.b Derivative D8 Derivative D8.a Derivative D9 Derivative D9.a Derivative D10 Derivative D10.a Derivative D11 Derivative D11.a Derivative D12 Derivative X1 Derivative X2 Derivative X3 Derivative Layer A1 Derivative Layer A2 Derivative Layer A3 Derivative Layer A4 Derivative Layer B1 Derivative Layer B2 Derivative Layer B3 Derivative Layer B3.a Derivative Layer B4 Derivative Layer C1 Derivative Layer C1.a Derivative Layer C2 Derivative Layer C2.a Derivative Layer C3 Derivative Layer C3.a Derivative Layer C3.b Derivative Layer C4 Derivative Layer C4.a Derivative Layer C4.b Derivative Layer C4.c Derivative Layer C4.d Derivative Layer C5 Derivative Layer D1 Derivative Layer D2 Derivative Layer D2.a Derivative Layer D2.b Derivative Layer D2.c Derivative Layer D3 Derivative Layer D4 Derivative Layer D4.a Derivative Layer D5 Derivative Layer E1 Derivative Layer E2 Derivative Layer E3 Derivative Layer E4 Derivative Layer F1 Derivative Layer F2 Derivative Layer F3 Derivative Layer F4 Derivative Layer G1 Derivative Layer G2 Derivative Layer G3 Derivative Layer G4 XI-2 XI-3 XI-8 Firm Birth D2.c Law 2 Law 15
 *
 * WHAT THIS MODULE OWNS is everything a contract needs that is not a payoff: the house, the margin,
 * the fund, the waterfall, the capacity a member has and the calls it gets. WHAT IT DOES NOT OWN is
 * any class: no `cds`, no `irs`, no forward. A class is a derivative kind in its own module (13b),
 * and it plugs in here the way an instrument kind plugs into the kernel — by declaring a profile.
 *
 * THE ONE NUMBER THE WHOLE LAYER TURNS ON is the mark (D8), and it is read from the contract's own
 * kind, once, from public state. Everything here is arithmetic on it: what must be posted (D1, D2),
 * what a call is (D4), what a close-out leaves owing (D11.a, F2), what a waterfall absorbs (C4).
 */
import type { Family, Violation } from '../../audit/audit.js';
import type { Period } from '../../calendar/calendar.js';
import {
  currencyUnit,
  instrumentId,
  partyKindId,
  paramId,
  type ContractId,
  type CurrencyCode,
  type PartyId,
  type PartyKindId,
} from '../../core/ids.js';
import { dustOf, sub, sum, withinDust } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import type { ParamDecl } from '../../registry/params.js';
import type { PartyKindProfile } from '../../registry/kinds.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { BANK, FIRM } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { ClearingCapacity, SystemModule } from '../../world/module.js';
import {
  closeOutKind,
  defaultFundKind,
  isFundTerms,
  isMarginTerms,
  marginKind,
  CLOSE_OUT_CLAIM,
  type CloseOutTerms,
} from './kinds.js';
import {
  callOn,
  marginLineId,
  marginPairsOf,
  moveMargin,
  pledgeInstead,
  posted,
  releasePledges,
  requirement,
} from './margin.js';
import { fundLineId, houseSheet, inFund, membersOf, runWaterfall, trueUpFund } from './house.js';

export * from './kinds.js';
export * from './margin.js';
export * from './house.js';

export const CLEARING_HOUSE = partyKindId('clearingHouse');

/**
 * WHOSE REASONS A CONTRACT BOOK ASKS FOR by default: the kernel's own kinds, which every world has.
 *
 * A world with funds or insurers in it has more, and the world that assembles them says so — a
 * module cannot declare a participant for a kind this world never registered (`addParticipant`
 * refuses it, and rightly: that guard is what catches a mistyped kind).
 */
export const TRADES_CONTRACTS: readonly PartyKindId[] = [BANK, FIRM];

export const LAYER_PARAMS = {
  horizon: paramId('clearingHouse.closeOutHorizon'),
  buffer: paramId('party.marginBuffer'),
} as const;

/**
 * C2, C3, C5, XI-3: a house is a REAL PARTY and it can fail.
 *
 * It fails on both counts and neither is decorative: on cash, because C4.b says the survivors are
 * paid in full and a house that cannot is past the end of its waterfall; on solvency, because C5
 * says running past the end is a real event with real consequences. Nothing about it is exempt from
 * XI-3, which is exactly what C2.a warns is the cost of clearing.
 *
 * It is nobody's deposit base. Its cash is margin it holds FOR its members and must be where they
 * can pay into and be paid from, which is a fact about settlement rather than a rate it shops for.
 */
const houseKind: PartyKindProfile = {
  id: CLEARING_HOUSE,
  representation: 'named',
  moneyIssuer: null,
  fails: ['cash', 'solvency'],
  // C5: nobody lends to it. Its resources are the four lines of the waterfall, they are
  // enumerable, and a lender behind them would be the guarantor of last resort C5 forbids.
  borrows: false,
  depositClass: null,
  terminal: false,
};

function params(): ParamDecl[] {
  return [
    {
      id: LAYER_PARAMS.horizon,
      value: 2,
      unit: 'periods',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Derivative Layer C3.b, D1: how many sessions the house assumes it takes to close a defaulted book. It scales the initial margin and sizes the fund, and it is a policy because somebody decides it, publishes it, and is answerable for it.',
    },
    {
      id: LAYER_PARAMS.buffer,
      value: 0.2,
      unit: 'of its own liquid cash',
      kind: 'preference',
      owner: 'model',
      why: 'Derivative Layer E1: what a member keeps back rather than committing to margin. It is a preference — how much of its own liquidity it is willing to have tied up at a house — and E4 forbids raising it to make a trade fit.',
    },
  ];
}

/**
 * E1, E2, E3: WHAT A MEMBER MAY CARRY, and what it posts against a trade.
 *
 * E1 is read from its own liquid cash, less what it keeps back, through the ONE expression the
 * observer reads too (`capacityOf`) — two arithmetics for one capacity is how a member comes to be
 * shown room the market will not give it (Law 4). E3's "drawn down as it is consumed" is what the
 * cash balance already does: margin settles in the same instruction as the trade (C3.a), so the
 * next trade in a session is admitted against an account that has already paid. And E4 is why
 * nothing here ever raises it: a member cut every period is a member living at its limit, which is
 * a measurement, not a problem with the limit.
 */
function capacity(): ClearingCapacity {
  return {
    admits(ctx, party, wanted, about): number {
      const m = about.market;
      const decl = m.contract;
      if (decl === undefined) return 0;
      // C5, Money E4: A HOUSE THAT HAS CEASED CLEARS NOTHING. Running past the end of the waterfall
      // is a real event (C5) and a house is not exempt from XI-3, so the book it cleared can outlive
      // it — and every fill in it would be a contract naming a party that no longer exists (G1,
      // Register F2). The whole size is refused, by name, and E4's measurement records it.
      const house = decl.house;
      if (house !== null && (!ctx.parties.has(house) || !ctx.parties.get(house).status.alive)) {
        return 0;
      }
      const view = ctx.participant(party);
      const room = capacityOf(view, m.ccy, ctx.params.get(LAYER_PARAMS.buffer));
      if (room <= 0) return 0;
      // What this trade would ask of it, per unit of notional: the kind's own initial margin on one
      // unit at the level that cleared. A trade it cannot margin is a trade it cannot make, and the
      // market cuts it to what it can (E2) rather than anybody raising the room.
      const profile = ctx.registry.derivativeKind(decl.kind);
      const one = ctx.contracts.marginFor(
        {
          kind: decl.kind,
          terms: decl.terms,
          ccy: m.ccy,
          notional: 1,
          struckAt: about.struck,
          house: decl.house,
          a: party,
          b: decl.house ?? party,
        },
        ctx.period,
      );
      // G2: no exposure without margin, or a stated reason there is none — and "the underlying has
      // not moved yet, so nobody can say" is not a reason. The trade is refused (E2) rather than
      // admitted at nothing.
      if (!one.some) return 0;
      if (one.value <= 0) return wanted;
      const affordable = ctx.registry.deliverable(profile.unit, room / one.value);
      return affordable < wanted ? affordable : wanted;
    },
    /**
     * D2, Money Market A2: WHAT THIS MEMBER'S OPEN ROWS WILL ASK IT FOR, per money.
     *
     * Its requirement with each counterparty, against what it has already posted with that one —
     * the same pair-by-pair read the call itself makes (C1.a), so a treasury funding for it is
     * funding for the number it will actually be asked. It never nets across counterparties (G3):
     * what it will get back from one is not cash it can post to another.
     */
    dueNext(ctx, party, ccy): number {
      const terms: number[] = [];
      for (const pair of marginPairsOf(ctx, party)) {
        if (pair.ccy !== ccy) continue;
        const by = sub(requirement(ctx, party, pair.other, ccy), posted(ctx, party, pair.other, ccy), 'the top-up');
        if (by > 0) terms.push(by);
      }
      return sum(terms).value;
    },
    margin(ctx, party, against, size, about): readonly Leg[] {
      const m = about.market;
      const decl = m.contract;
      if (decl === undefined) return [];
      const need = ctx.contracts.marginFor(
        {
          kind: decl.kind,
          terms: decl.terms,
          ccy: m.ccy,
          notional: size,
          struckAt: about.struck,
          house: decl.house,
          a: party,
          b: against,
        },
        ctx.period,
      );
      if (!need.some) return [];
      const amount = ctx.registry.cashFor(m.ccy, need.value);
      if (amount > 0) return moveMargin(ctx, party, against, m.ccy, amount);
      /**
       * G2, Law 8: NO EXPOSURE WITHOUT MARGIN, OR A STATED REASON THERE IS NONE — and here there
       * is one, so it is stated. A requirement of a fraction of a cent is a real requirement that
       * is smaller than the smallest piece of the money it would be posted in: nothing moves,
       * because nothing CAN move, and a trade admitted with nothing behind it is exactly what G2
       * is about. Saying so is the difference between "too small to post" and "nobody measured
       * it" — and a default fund sized from a book of these is a fund of nothing, which somebody
       * should be able to see before a member fails rather than afterwards.
       */
      ctx.record(
        'margin.none',
        [party, against, String(m.id)],
        {
          poster: party,
          holder: against,
          market: m.id,
          ccy: m.ccy,
          required: need.value,
          why: 'the requirement is smaller than one piece of this money',
        },
        true,
      );
      return [];
    },
  };
}

/**
 * D2, D4, D5, XI-2 door one: the period's margin, re-measured against the marks that have just
 * closed, and the call that follows when it is short.
 *
 * A call is not a rate and not a schedule: it is a SHORTFALL — what the requirement now is against
 * what is posted — and it rises when the marks move, which is D5's procyclicality by construction
 * and is a consequence to be measured rather than smoothed. Meeting it may force a sale (D4.a), and
 * that is the party's own module's business: what happens here is the payment, and a party that
 * cannot make it fails the instruction and is in Money E1's state (D2.c).
 */
function marginCalls(ctx: MechanismContext): void {
  const done = new Set<string>();
  for (const c of ctx.contracts.open_()) {
    for (const side of [c.a, c.b] as const) {
      for (const pair of marginPairsOf(ctx, side)) {
        const key = `${side}|${pair.other}|${pair.ccy}`;
        if (done.has(key)) continue;
        done.add(key);
        const need = requirement(ctx, side, pair.other, pair.ccy);
        const have = posted(ctx, side, pair.other, pair.ccy);
        const by = sub(need, have, 'what the margin must move by');
        if (by === 0) continue;
        const legs = moveMargin(ctx, side, pair.other, pair.ccy, by);
        if (legs.length === 0) continue;
        let r = ctx.settle({
          legs,
          cause: 'transfer',
          reason:
            by > 0
              ? `${side} posts ${by} of margin to ${pair.other}`
              : `${pair.other} returns ${-by} of margin to ${side}`,
        });
        /**
         * D4.a, D9, D9.a: A CALL HAS THREE ANSWERS, and cash is only one of them. A party that
         * could not pay may PLEDGE instead — the units stay its own, the coupon and the mark stay
         * its own, and what changes is that they are bound and it cannot move them. The third
         * answer is a forced sale, and that is its own module's business (XI-2 door one).
         *
         * It is tried after the payment failed rather than instead of it, because a party with the
         * cash pays: posting securities when you have money is a choice nobody makes, and the
         * order these are tried in is what makes collateral the answer to a SHORTFALL.
         */
        if (by > 0 && r.outcome !== 'settled') {
          const bound = pledgeInstead(ctx, side, pair.other, pair.ccy, by);
          if (bound.length > 0) {
            r = ctx.settle({
              legs: bound,
              cause: 'transfer',
              reason: `${side} pledges what it holds against ${by} it could not pay ${pair.other}`,
            });
          }
        }
        if (by < 0) {
          // D9.a: and it comes back. What secured a requirement that has gone is free again.
          const free = releasePledges(ctx, side, pair.other, pair.ccy);
          if (free.length > 0) {
            ctx.settle({
              legs: free,
              cause: 'transfer',
              reason: `${pair.other} releases what ${side} pledged`,
            });
          }
        }
        if (by > 0) {
          const call = callOn(ctx, side, pair.other, pair.ccy);
          ctx.record(
            'margin.call',
            [side, pair.other],
            {
              poster: side,
              holder: pair.other,
              ccy: pair.ccy,
              asked: by,
              met: r.outcome === 'settled',
              short: call === undefined ? 0 : call.short,
            },
            true,
          );
        }
      }
    }
  }
}

/**
 * D4, D5, D6, D6.a: WHAT THE TERMS PUT IN THIS PERIOD, PAID.
 *
 * A derivative's periodic payments are not margin and are not the mark: they are what the contract
 * itself says falls due — a protection premium, the net of two swap legs, a notional exchange at
 * maturity — and every one of them is a real payment between two named parties in a stated money
 * (D5: each leg in its own). The kernel already knows what they are, because the kind's profile
 * says so; what was missing until a class had one was anybody settling them.
 *
 * It goes through settlement like any payment, so a party that cannot pay FAILS (Money E1) and is
 * in that state with everything that follows from it. Nothing here nets across contracts: one row,
 * one obligation, one instruction (Law 5, G3).
 */
function payLegs(ctx: MechanismContext): void {
  for (const c of ctx.contracts.open_()) {
    for (const due of ctx.contracts.legsDue(c, ctx.period)) {
      const amount = ctx.registry.cashFor(due.ccy, due.amount);
      if (amount <= 0) continue;
      const r = ctx.settle({
        legs: [
          {
            kind: 'money',
            from: ctx.accountOf(due.from, due.ccy),
            to: ctx.accountOf(due.to, due.ccy),
            ccy: due.ccy,
            amount,
            fromCell: none(),
            toCell: none(),
          },
        ],
        cause: 'coupon',
        reason: `${c.id}: ${due.why}`,
      });
      ctx.record(
        'contract.paid',
        [String(c.id), due.from, due.to],
        {
          contract: String(c.id),
          from: due.from,
          to: due.to,
          ccy: due.ccy,
          amount,
          why: due.why,
          settled: r.outcome === 'settled',
        },
        true,
      );
    }
  }
}

/** C3.b: the fund, trued up every period against what the largest member's book now needs. */
function trueUpFunds(ctx: MechanismContext): void {
  const horizon = ctx.params.get(LAYER_PARAMS.horizon);
  for (const h of ctx.parties.ofKind(CLEARING_HOUSE)) {
    if (!h.status.alive) continue;
    for (const ccy of moniesOf(ctx, h.id)) trueUpFund(ctx, h.id, ccy, horizon);
  }
}

function moniesOf(ctx: MechanismContext, house: PartyId): readonly CurrencyCode[] {
  const out = new Set<CurrencyCode>();
  for (const c of ctx.contracts.openOf(house)) out.add(c.ccy);
  return [...out];
}

/**
 * D4, D11, D11.a, F1-F3, C4: the resolution slot.
 *
 * Two things happen here and they are the same thing seen twice. A contract whose term has run out
 * EXPIRES and is torn up at what it is worth (D11). A contract one of whose sides has ceased is
 * closed out at its stated close-out value, the obligation moves to the successor first because a
 * claim must name somebody who exists (Register F2), and what is left owing becomes an unsecured
 * claim on that estate (C4.c, F2) — never a payment jumped ahead of every ranked claim.
 *
 * It runs after the estate has opened, which is why the successor is there to novate to.
 */
function resolveContracts(ctx: MechanismContext): void {
  // D4: A MARGIN CALL MUST BE MET OR THE POSITION IS CLOSED OUT. Whose calls went unmet is read off
  // what this period's calls said (Law 19) — the call was issued, the payment was attempted, and
  // whether it settled is a fact the event carries rather than one re-derived here.
  const unmet = new Set<string>();
  for (const e of ctx.journal.ofKind('margin.call')) {
    if (e.period !== ctx.period || e.data['met'] === true) continue;
    unmet.add(`${String(e.data['poster'])}|${String(e.data['holder'])}`);
  }
  for (const c of ctx.contracts.open_()) {
    const deadSide = [c.a, c.b].find((p) => !ctx.parties.get(p).status.alive);
    if (deadSide !== undefined) {
      closeOutOnDefault(ctx, c.id, deadSide);
      continue;
    }
    if (unmet.has(`${c.a}|${c.b}`) || unmet.has(`${c.b}|${c.a}`)) {
      // D4, D11.a: closed out at the stated value. The party had the session to find the cash — a
      // sale it posted itself (D4.a, XI-2 door one) — and it did not.
      settleAndTearUp(ctx, c.id, 'a margin call went unmet');
      continue;
    }
    if (!ctx.contracts.expires(c, ctx.period)) continue;
    settleAndTearUp(ctx, c.id, 'the term ran out');
  }
}

/** D11: expiry. What it is worth to `a` moves in cash, and the row leaves both books at once. */
function settleAndTearUp(ctx: MechanismContext, id: ContractId, why: string): void {
  const c = ctx.contracts.get(id);
  const value = ctx.contracts.closeOut(c, ctx.period);
  const legs: Leg[] = [{ kind: 'contract', act: 'close', contract: id, why }];
  const owed = ctx.registry.cashFor(c.ccy, value < 0 ? -value : value);
  // Money E4: THE ROW CLOSES EITHER WAY, AND THE MONEY ONLY MOVES BETWEEN THE LIVING. Both sides
  // of a row can cease in one period — a member and the house it faced — and a termination that
  // insisted on paying would be an instruction addressed to somebody who is not there. What is
  // owed then is a claim on an estate (C4.c), which is where `closeOutOnDefault` puts it.
  const living = ctx.parties.get(c.a).status.alive && ctx.parties.get(c.b).status.alive;
  if (owed > 0 && living) {
    legs.push({
      kind: 'money',
      from: ctx.accountOf(value > 0 ? c.b : c.a, c.ccy),
      to: ctx.accountOf(value > 0 ? c.a : c.b, c.ccy),
      ccy: c.ccy,
      amount: owed,
      fromCell: none(),
      toCell: none(),
    });
  }
  const r = ctx.settle({ legs, cause: 'corporateAction', reason: `${id} terminates: ${why}` });
  ctx.record(
    'contract.closed',
    [String(id), c.a, c.b],
    { contract: String(id), why, value, settled: r.outcome === 'settled' },
    true,
  );
  if (r.outcome === 'settled') returnMargin(ctx, c.a, c.b, c.ccy);
}

/**
 * F2, F3, C4, C4.b, C4.c: a side has ceased with the row still open.
 *
 * The row moves to the successor first (Register F2: every reference resolves to somebody who
 * exists), then it is torn up at its stated close-out value (D11.a). What the survivor is owed is
 * the mark less the collateral it holds (F3) — the collateral is already its own, so what is left
 * is a claim on the estate, ranking with the other unsecured (C4.c). Where the row faced a house,
 * the house pays the survivor in full (C4.b) and the loss goes to its waterfall.
 */
function closeOutOnDefault(ctx: MechanismContext, id: ContractId, dead: PartyId): void {
  const before = ctx.contracts.get(id);
  const estate = ctx.parties.resolve(dead).id;
  if (estate === dead) return;
  const survivor = before.a === dead ? before.b : before.a;
  /**
   * D1.a, Register F2: A CONTRACT HAS TWO SIDES AND THEY ARE DIFFERENT PARTIES. A successor can be
   * the party on the other side of the row — a bank resolved into the one facing it, an estate
   * whose successor is its counterparty — and novating then would make one party both sides of its
   * own obligation, which is not a contract but a statement about itself. The row is torn up
   * instead: what one side owed the other, it now owes nobody.
   */
  if (estate === survivor) {
    settleAndTearUp(ctx, id, `${dead} ceased into ${survivor}, which was the other side`);
    return;
  }
  const value = ctx.contracts.closeOut(before, ctx.period);
  const owedToSurvivor = before.a === survivor ? value : -value;
  const moved = ctx.settle({
    legs: [{ kind: 'contract', act: 'novate', contract: id, from: dead, to: estate }],
    cause: 'default',
    reason: `${dead} ceased; ${id} moves to ${estate}`,
  });
  // Register F2, Money E4: THE ROW MOVES FIRST OR NOTHING ELSE HAPPENS. A close-out paid to a
  // party that has ceased is an instruction addressed to somebody who is not there, and the
  // novation is what puts a living name on the row. If it did not settle, the row stays where it
  // is and is closed out next period — a recorded state, not a payment into the dark.
  if (moved.outcome !== 'settled') return;
  const collateral = posted(ctx, dead, survivor, before.ccy);
  const claim = owedToSurvivor > 0 ? sub(owedToSurvivor, collateral, 'the mark less what it holds') : 0;
  settleAndTearUp(ctx, id, `${dead} ceased`);
  if (claim > 0) issueCloseOutClaim(ctx, estate, survivor, before.ccy, claim);
  const house = before.house;
  // C5, XI-3, Money E4: A HOUSE THAT HAS ITSELF CEASED RUNS NO WATERFALL. Running past the end of
  // one is exactly how a house fails (C5), so a member defaulting into a house that is already
  // gone is two defaults and not one — and the house's own is its estate's business now. Every
  // line of a waterfall is an instruction naming the house, and it is not there to be named.
  if (house !== null && survivor === house && ctx.parties.get(house).status.alive) {
    runWaterfall(ctx, house, dead, before.ccy, claim > 0 ? claim : 0);
  }
}

/** C4.c, F2: what a close-out left owing, as a claim the estate ranks like any other unsecured one. */
function issueCloseOutClaim(
  ctx: MechanismContext,
  owedBy: PartyId,
  owedTo: PartyId,
  ccy: CurrencyCode,
  amount: number,
): void {
  // Law 8: a claim is a COUNT OF PIECES of the money it is owed in, and a residue smaller than the
  // smallest piece is not a claim anybody could be paid — it is the dust of the close-out's own
  // subtraction. Rounded here, before the instrument exists, so that a world where the mark and the
  // collateral agree to within a cent does not open a claim nobody can settle (Register C1).
  const qty = ctx.registry.cashFor(ccy, amount);
  if (qty <= 0) return;
  // Register F2: EVERY REFERENCE RESOLVES TO SOMEBODY WHO EXISTS. Both sides of a row can have
  // ceased by the time it is closed out — a member and the house it faced, in one period — and a
  // claim issued to a party that is gone is a claim nobody could ever be paid on. Each name is
  // resolved to its successor, which is what an estate is for.
  const from = ctx.parties.resolve(owedBy).id;
  const to = ctx.parties.resolve(owedTo).id;
  if (from === to) return;
  const id = instrumentId(`closeOut:${from}:${to}:${ctx.period}`);
  if (!ctx.instruments.has(id)) {
    const terms: CloseOutTerms = {
      kind: CLOSE_OUT_CLAIM,
      owedBy: from,
      owedTo: to,
      struck: ctx.calendar.startOf(ctx.period),
    };
    ctx.issue({ id, kind: CLOSE_OUT_CLAIM, issuer: some(from), ccy, terms, market: none() });
  }
  ctx.settle({
    legs: [
      {
        kind: 'asset',
        from,
        to,
        instrument: id,
        qty,
        pricePerUnit: some(1),
        accruedPerUnit: none(),
        fromCell: none(),
        toCell: none(),
      },
    ],
    cause: 'default',
    reason: `${from} owes ${to} ${amount} on a close-out`,
  });
}

/** D3: margin is returned when there is nothing left to secure — the poster always owned it. */
function returnMargin(ctx: MechanismContext, a: PartyId, b: PartyId, ccy: CurrencyCode): void {
  for (const [poster, holder] of [
    [a, b],
    [b, a],
  ] as const) {
    const need = requirement(ctx, poster, holder, ccy);
    const have = posted(ctx, poster, holder, ccy);
    const by = sub(need, have, 'margin to return');
    if (by >= 0) continue;
    const legs = moveMargin(ctx, poster, holder, ccy, by);
    if (legs.length > 0) {
      ctx.settle({ legs, cause: 'transfer', reason: `${holder} returns margin to ${poster}` });
    }
  }
}

/**
 * D2.b: VARIATION MARGIN PAID EQUALS VARIATION MARGIN RECEIVED, every period, exactly — and the two
 * sides are read independently. What every poster holds of a claim is one read of the register;
 * what every holder has issued of it is the other (Register B2, both directions). They are the same
 * fact and they are counted from opposite ends, which is what makes this a check rather than an
 * assertion about one number.
 */
function marginIsHeld(): Family {
  return {
    name: 'zeroSum',
    contributor: 'derivative-layer',
    spec: 'Derivative Layer D2.b Derivative Layer D3 Derivative D9.a',
    built: true,
    check: (view): Violation[] => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        // Law 15: the shape of the terms, not the id on them — what makes a line one of these is
        // that it names a poster and a holder, or a member and its house.
        if (!isMarginTerms(i.terms) && !isFundTerms(i.terms)) continue;
        const held = view.register.heldTotal(i.id);
        const issued = i.issued;
        const dust = held.dust + dustOf(2, Math.abs(issued) + Math.abs(held.value));
        if (withinDust(held.value, issued, dust)) continue;
        out.push({
          family: 'zeroSum',
          spec: 'Derivative Layer D2.b',
          owner: String(i.id),
          size: sub(held.value, issued, 'margin held against margin owed'),
          unit: currencyUnit(i.ccy),
          period: view.period,
          message: `${i.id}: ${held.value} is posted and ${issued} is owed back`,
        });
      }
      return out;
    },
  };
}

/**
 * G1, F4: every open row names two parties that exist, and a row whose side has CEASED is a claim
 * on somebody the world has forgotten — which is the one thing Register F2 says may never happen.
 */
function contractsNameTheLiving(): Family {
  return {
    name: 'names',
    contributor: 'derivative-layer',
    spec: 'Derivative Layer G1 Derivative Layer F4 Register F2',
    built: true,
    check: (view): Violation[] => {
      const out: Violation[] = [];
      for (const c of view.contracts.open_()) {
        for (const side of [c.a, c.b]) {
          if (view.parties.has(side) && view.parties.get(side).status.alive) continue;
          out.push({
            family: 'names',
            spec: 'Register F2',
            owner: String(c.id),
            size: view.contracts.mark(c, view.period),
            unit: c.ccy,
            period: view.period,
            message: `${c.id} is open against ${side}, which has ceased`,
          });
        }
      }
      return out;
    },
  };
}

/** E4: what the markets struck BEYOND what their members could margin — a standing measurement. */
export function refusedThisPeriod(ctx: MechanismContext): number {
  // `ofKind` is the journal's own index of what happened, which is the read rather than a filter
  // over everything — and an event kind is a name, not a kind id anything dispatches on.
  return sum(
    ctx.journal
      .ofKind('derivatives.refused')
      .filter((e) => e.period === ctx.period)
      .map((e) => (typeof e.data['cut'] === 'number' ? e.data['cut'] : 0)),
  ).value;
}

/**
 * E1, E3: WHAT A MEMBER MAY CARRY — its own liquid cash, less what it keeps back. The observer
 * shows this number and `admits` cuts a trade by it, and there is one of it (Law 4).
 *
 * IT DOES NOT SUBTRACT WHAT IT HAS ALREADY POSTED. Margin is an asset swap settled in the SAME
 * instruction as the trade it covers (C3.a, E2: the cut happens at the strike), so by the time the
 * next trade in a session is admitted the account has already paid — and subtracting the margin
 * claims the member holds took the room down TWICE for every unit posted, which made every book
 * half the size the mechanism says and made E4's refusal measurement wrong in the direction that
 * looks like prudence. What E3 asks for is what the cash balance already is (Law 19: read the
 * source, never a second number derived from it).
 */
export function capacityOf(view: ParticipantView, ccy: CurrencyCode, buffer: number): number {
  const cash = view.cash(ccy);
  return sub(cash, cash * buffer, 'net of what it keeps back');
}

export function derivativeLayer(
  tradesContracts: readonly PartyKindId[] = TRADES_CONTRACTS,
): SystemModule {
  return {
  id: 'derivative-layer',
  spec: 'Derivative Layer, Derivative contract',
  // The estate is what a default resolves into (XI-8) and the money market is where a member finds
  // the cash a call asks for; credit events are what an underlying of that shape reads. And the
  // foundation seed, because a house is a PARTY: it has to be created after the world it clears in
  // has banks for it to hold its money at (Seed A1: opening state runs in assembly order).
  requires: ['credit-events', 'estate', 'money-market', 'seed.foundation'],
  instrumentKinds: [marginKind, defaultFundKind, closeOutKind],
  derivativeKinds: [],
  partyKinds: [houseKind],
  curveFamilies: [],
  units: [],
  params: params(),
  clearingCapacity: capacity(),
  phases: [
    {
      name: 'margin.calls',
      spec: 'Derivative D4 Derivative D5 Derivative D6 Derivative D6.a Derivative Layer D1 Derivative Layer D2 Derivative Layer D4 Derivative Layer D5 XI-2',
      cycle: 0,
      // Requirements are re-measured against LAST CLOSE's marks, at the top of the period, so a
      // call caused by them can be met out of this period's session (docs/PLAN.md §8). A call
      // caused by this period's own marks is next period's, and that lag is Clearing F1 being
      // honest rather than a second session hiding it.
      anchor: { after: 'corporateActions' },
      run: (ctx: MechanismContext): void => {
        // D4 before D2: what the terms put in this period is an obligation the contract created,
        // and the margin is what secures what is LEFT after it. A call measured before the
        // period's own payment moved would be securing an exposure that is about to change.
        payLegs(ctx);
        marginCalls(ctx);
        trueUpFunds(ctx);
      },
    },
    {
      name: 'derivatives.resolve',
      spec: 'Derivative D11 Derivative D11.a Derivative Layer C4 Derivative Layer F1 Derivative Layer F2 XI-3',
      cycle: 'anchor',
      // After the estate has opened for whoever died this period: a claim has to name somebody who
      // exists, and the successor is what the estate is (Register F2). `anchor` takes the estate's
      // own cycle, which is the one after the marks are in (XI-8).
      anchor: { after: 'estates.resolve' },
      run: resolveContracts,
    },
  ],
  /**
   * Clearing A2, Clearing B2, Law 4, Law 15: ONE FACE PER BOOK.
   *
   * A contract book is the layer's, so the layer is what speaks for a party in one — once per
   * party kind — and what it says is whatever the KIND of contract that book carries says (the
   * profile's own `orders`). Six class modules each declaring a participant would be six modules
   * speaking for one bank in one book, which the kernel refuses at the moment they cross and Law 4
   * forbids before that: one fact, one writer.
   */
  participants: tradesContracts.map((partyKind) => ({
    partyKind,
    in: 'contract' as const,
    speculative: true,
    orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
      const decl = m.contract;
      if (decl === undefined) return [];
      const profile = view.registry.derivativeKind(decl.kind);
      return profile.orders?.(view, m) ?? [];
    },
  })),
  families: [marginIsHeld(), contractsNameTheLiving()],
  seed(ctx): void {
    // C2, C3: one house per currency this world has, named and banked like any other party. A house
    // is not created by a trade: it is an institution that exists before anybody clears through it,
    // and which currencies it clears in is which monies its members settle in.
    for (const ccy of ctx.registry.currencies.keys()) {
      const region = [...ctx.registry.regions.values()].find((r) => r.ccy === ccy);
      if (region === undefined) continue;
      const bank = [...ctx.parties.all()].find(
        (p) => p.region === region.id && ctx.registry.issuesMoney(p.kind) && p.bank !== p.id,
      );
      if (bank === undefined) continue;
      ctx.parties.add({
        id: houseIdFor(ccy),
        kind: CLEARING_HOUSE,
        region: region.id,
        name: `${ctx.registry.currency(ccy).name} Clearing House`,
        bank: bank.id,
        representation: 'named',
        status: { alive: true },
      });
    }
  },
  };
}

/** Law 9: an id is an id. The house of a money is named for the money its members settle in. */
export function houseIdFor(ccy: CurrencyCode): PartyId {
  return `clearing.${ccy}` as PartyId;
}

/** A read for a test or the observer: what a house is holding and owes, per money (C3). */
export function sheetOf(ctx: MechanismContext, house: PartyId, ccy: CurrencyCode): ReturnType<typeof houseSheet> {
  return houseSheet(ctx, house, ccy);
}

export { membersOf, inFund, fundLineId, marginLineId, runWaterfall };
export type { Period };
