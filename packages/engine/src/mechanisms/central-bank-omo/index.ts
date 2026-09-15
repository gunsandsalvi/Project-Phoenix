/**
 * The central bank in the sovereign market: what it buys, and what it hands back.
 *
 * @spec Central Bank C1 Central Bank C1.a Central Bank C1.b Central Bank C2 Central Bank C2.a Central Bank C3 Central Bank C4 Central Bank E1 Central Bank E3 Central Bank E3.a Central Bank E4 Central Bank E5 Sovereign H1 Sovereign H2 Sovereign H3 Sovereign H3.a Sovereign H4 Sovereign E2.c Treasury D3.a Clearing B4
 *
 * It buys in the SECONDARY market, from a seller, at a price (Treasury D3.a) — never in the primary,
 * where the seller would be the treasury itself and the purchase would be the advance that D3
 * forbids (C1.b). It posts a QUANTITY and no level (C3): a price-taker takes the level the book
 * gives it, and the size is policy's (C1.a), never an auction's weakness.
 *
 * Its purchases pay with money it creates, so there is no debit anywhere (C2, C2.a): settlement's
 * own routing does that when the payer's account is at its own issuer.
 *
 * REINVESTMENT is a separate decision (C4): with it on, the policy is a share of what is
 * outstanding and maturities are replaced; with it off, the book simply runs off and the base
 * shrinks with it — and the difference between the two is quantitative tightening.
 *
 * REMITTANCE (E3) is its net INCOME, not its revaluation (E3.a): the equity its own settled
 * instructions produced since it last remitted. Revaluation never passes through an instruction, so
 * it cannot reach this number. A loss is not remitted (E4): it stands in its equity.
 */
import type { Civil } from '../../calendar/civil.js';
import { compareCivil } from '../../calendar/civil.js';
import { period, type Period } from '../../calendar/calendar.js';
import { paramId, type PartyId } from '../../core/ids.js';
import { add, atMost, material, sum } from '../../core/num.js';
import {
  type PerMember,
  acrossMembers,
  minus,
  negated,
  scale,
} from '../../core/measure.js';
import { months } from '../../core/rate.js';
import type { Order } from '../../clearing/solver.js';
import type { Leg } from '../../ledger/instruction.js';
import { CENTRAL_BANK, TREASURY } from '../../registry/profiles.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { downTick } from '../../core/tick.js';

export const CB_PARAMS = {
  targetShare: paramId('centralBank.omo.targetHoldingShare'),
  reinvest: paramId('centralBank.reinvest'),
  remittanceMonths: paramId('centralBank.remittance.periodicity'),
} as const;

export const centralBankOmo: SystemModule = {
  id: 'central-bank-omo',
  spec: 'Central Bank C, E; Sovereign H',
  requires: ['sovereign-instruments', 'sovereign-curve'],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: CB_PARAMS.targetShare,
      value: 0.25,
      unit: 'ratio of a line outstanding',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank C1, C1.a: it buys sovereign paper in a size IT chooses, set by policy and never by an auction’s weakness. The share of each line it wants to hold is that size, stated.',
    },
    {
      id: CB_PARAMS.reinvest,
      value: 1,
      unit: '1 to reinvest maturities, 0 to let the book run off',
      dimension: 'count',
      kind: 'policy',
      owner: 'centralBank',
      why: 'Central Bank C4: reinvestment of maturities is a decision separate from new purchases, and the difference between the two is quantitative tightening.',
    },
    {
      id: CB_PARAMS.remittanceMonths,
      value: 12,
      unit: 'months',
      dimension: 'months',
      kind: 'policy',
      owner: 'model',
      why: 'Central Bank E3: its net income goes to the treasury because the treasury owns it. How often it settles up is placed on the one calendar by date (Money G3).',
    },
  ],
  phases: [
    {
      name: 'centralBank.remittance',
      spec: 'Central Bank E3 Central Bank E3.a Central Bank E4 Sovereign H3',
      anchor: { before: 'revaluation' },
      // Law 10, Clearing F1.a: this phase has never RUN — no period of either world has reached
      // it — so what it reads is read off its module's source and not off a measurement, and
      // it is the module's whole read set rather than this phase's. It narrows the first time
      // the phase runs and the check can say which of these it actually wanted.
      reads: [
        { kind: 'event', name: 'centralBank.remittance', of: 'anyPeriod' },
      ],
      writes: [],
      run: (ctx: MechanismContext): void => {
        for (const cb of ctx.parties.ofKind(CENTRAL_BANK)) {
          if (cb.status.alive) remit(ctx, cb.id);
        }
      },
    },
  ],
  participants: [
    {
      partyKind: CENTRAL_BANK,
      orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
        // C1.b: never in the primary market. The seller there is the issuer, and buying from it is
        // the advance the treasury is not allowed to have (Treasury D3, D3.a).
        if (view.offer(m.id).some) return [];
        const i = view.instruments.get(m.instrument);
        if (view.registry.instrumentKind(i.kind).pricing !== 'cleared') return [];
        if (i.ccy !== view.registry.currencyOf(view.self.region)) return [];
        // C1: it buys SOVEREIGN paper. Everything else that clears in its money — a tonne of grain,
        // a share, a corporate line — is somebody else's market, and a central bank standing in it
        // with a size set by its own policy is the buyer of last resort this world does not have
        // (Appendix B). What makes paper sovereign is who promised it, which is public.
        if (!i.issuer.some) return [];
        const sovereigns = new Set(view.parties.ofKind(TREASURY).map((t) => t.id));
        if (!sovereigns.has(i.issuer.value)) return [];
        const held = view.quantity(i.id);
        // C4: with reinvestment off there is no target to restore, so the book runs off.
        if (view.params.count(CB_PARAMS.reinvest) === 0) return [];
        const target = scale(i.issued, view.params.ratio(CB_PARAMS.targetShare), 'target holding');
        const gap = minus(target, held, 'omo gap');
        // A gap smaller than the dust of the subtraction that produced it is not a policy decision.
        if (!material(gap, 2, add(Math.abs(target), Math.abs(held), 'gap magnitude'))) return [];
        // Law 8: the target is a share of a line, so the gap it leaves is a fraction of a unit of
        // paper. It buys and sells whole units of it like anybody else — a central bank is not
        // exempt from what a unit IS — and it rounds towards where it already is, because what it
        // is doing is closing a gap and never overshooting one.
        if (gap > 0) {
          const want = downTick(gap);
          return want > 0 ? [{ party: view.self.id, side: 'buy', price: 'market', qty: want }] : [];
        }
        const free = view.free(i.id);
        const size = downTick(
          atMost(negated(gap, 'the other side of the gap'), free, 'it sells what it holds unencumbered and no more'),
        );
        return size > 0 ? [{ party: view.self.id, side: 'sell', price: 'market', qty: size }] : [];
      },
    },
  ],
  families: [],
};

/** E3: everything its own instructions earned it since the last time it settled up. */
function remit(ctx: MechanismContext, cb: PartyId): void {
  if (!dueThisPeriod(ctx)) return;
  const previous = lastRemittance(ctx, cb);
  const terms: PerMember<'money:piece'>[] = [];
  for (let p = previous; p <= ctx.period; p = period(p + 1)) {
    for (const r of ctx.ledger.inPeriod(p)) {
      if (r.outcome !== 'settled') continue;
      for (const e of r.equity) if (e.party === cb) terms.push(e.delta);
    }
  }
  const income = sum(terms).value;
  /**
   * A-61, E3: THE TREASURY THAT OWNS THIS CENTRAL BANK, which is the one in its own region.
   *
   * This was `ofKind(TREASURY).filter(alive)[0]` — whichever the parties store happened to return
   * first, an insertion-order artefact of the seed's draw and not a fact about who owns anything.
   * In a world with four countries (which 13j built and which the rest of the engine is careful
   * about) ALL FOUR central banks remitted to one country's treasury, each in its own money, into
   * accounts that treasury holds at three foreign central banks: three governments never received
   * the seigniorage on their own money and one received all of it, as an unexplained transfer.
   *
   * A central bank whose sovereign has no live treasury remits to nobody — recorded, not defaulted
   * to a stranger (Law 1: a refusal is an answer).
   */
  const region = ctx.parties.get(cb).region;
  const to = ctx.parties.ofKind(TREASURY).find((t) => t.region === region && t.status.alive);
  if (to === undefined) {
    ctx.record('centralBank.unremitted', [cb], { income, since: previous, region }, true);
    return;
  }
  if (income <= 0) {
    // E4: a loss is not remitted. It reduces its equity and stands there until income covers it.
    ctx.record('centralBank.loss', [cb], { income, since: previous }, true);
    return;
  }
  const ccy = ctx.registry.currencyOf(region);
  // Law 8: it remits whole pieces of the money it issues; the piece it cannot divide stays on its
  // own books and is remitted with next period's income (E3).
  // XI-15: a central bank is a NAMED party and not a cell, so what its equity account moved by is
  // what it earned — `acrossMembers` at one is the door that says so in the type.
  const paid = ctx.registry.payable(acrossMembers(income, 1, 'what a central bank of one earned'),
  );
  if (paid <= 0) return;
  const leg: Leg = {
    kind: 'money',
    from: { holder: cb, issuer: cb },
    to: ctx.accountOf(to.id, ccy),
    ccy,
    amount: paid,
  };
  const r = ctx.settle({ legs: [leg], cause: 'transfer', reason: `remittance to ${to.id}` });
  ctx.record(
    'centralBank.remittance',
    [cb, to.id],
    { income, paid, since: previous, settled: r.outcome === 'settled' },
    true,
  );
}

/** Placed on the one calendar by date, like every other periodicity (Money G3.a). */
function dueThisPeriod(ctx: MechanismContext): boolean {
  const every = ctx.params.months(CB_PARAMS.remittanceMonths);
  const epoch = ctx.calendar.epoch;
  const end = ctx.calendar.endOf(ctx.period);
  let date: Civil = epoch;
  for (let k = 0; k < ctx.period + 1; k += 1) {
    date = ctx.calendar.advance(date, months(every));
    if (compareCivil(date, end) > 0) return false;
    if (ctx.calendar.periodOf(date) === ctx.period) return true;
  }
  return false;
}

/**
 * E3, E4: the period it last SETTLED UP in, read from the journal; nothing stores it.
 *
 * A-62: IT USED TO COUNT A LOSS AS SETTLING UP, and that is what forgot the loss. E4 says a loss is
 * not remitted — it stands until income covers it — and the comment at the loss branch says exactly
 * that. But `centralBank.loss` advanced this marker too, so the next window began AFTER the loss and
 * the deficit was never in a window again: the central bank started each period from nothing, and a
 * bank that had lost a fortune remitted its next quarter's income in full.
 *
 * A loss is not an agreement and this is not arrears — the central bank owes it to nobody, and the
 * treasury has no claim it could rank. It is a deficit against the bank's OWN future income, and
 * carrying it is a matter of not moving the line the income is measured from. So the fix is the read
 * that stops: the window runs from the last time it actually paid something over.
 */
function lastRemittance(ctx: MechanismContext, cb: PartyId): Period {
  const events = [...ctx.journal.ofKind('centralBank.remittance')]
    .filter((e) => e.subjects.includes(cb))
    .sort((a, b) => a.period - b.period);
  const last = events[events.length - 1];
  return last === undefined ? period(0) : period(add(last.period, 1, 'after the last remittance'));
}
