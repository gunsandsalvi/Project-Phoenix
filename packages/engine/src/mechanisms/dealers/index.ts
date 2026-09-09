/**
 * Dealer desks: named parties inside banks that carry inventory, pay rent on it every period, and
 * quote two prices out of what that costs them.
 *
 * @spec Fund Shares E3 Fund Shares E3.a Dealer Desks A1 Dealer Desks A2 Dealer Desks A3 Dealer Desks A4 Dealer Desks B1 Dealer Desks B2 Dealer Desks B3 Dealer Desks B4 Dealer Desks C1 Dealer Desks C2 Dealer Desks C2.a Dealer Desks C3 Dealer Desks C4 Dealer Desks C5 Dealer Desks C5.a Dealer Desks C5.b Dealer Desks D1 Dealer Desks D2 Dealer Desks D3 Dealer Desks D4 Dealer Desks D4.a Dealer Desks D5 Dealer Desks E3 Dealer Desks E4 Dealer Desks F1 Dealer Desks F3 Clearing B3 Clearing B3.a Clearing B4 Clearing E3 Equity B5 XI-3 XI-4 XI-13 Law 2 Law 15
 *
 * WHY IT IS HERE (XI-4 joint three). Every cleared price in this model is inert until somebody's
 * position costs them something. A desk that carries inventory for free has no reason to shed it,
 * so its quotes never skew, so order flow never moves a price, so the joint that turns a financial
 * price into a real decision is missing. This module is that joint: a position consumes cash and
 * capital, it is charged for both every period it is held, and the charge is a real payment to a
 * real counterparty — the bank the desk lives inside.
 *
 * WHAT A DESK IS (A1-A4). A named party with its own account at its own bank, its own inventory,
 * its own limit and its own view. It makes money on the spread and loses it on the inventory, and
 * the two are the whole business (A4): nothing here computes a P&L, because the kernel's revaluation
 * already moves its equity account with its marks and settlement already moves it with its trades.
 * F3's "a desk that cannot lose money is not taking the other side" is therefore not something this
 * module has to arrange; it is what the wire does to it.
 *
 * WHAT IT IS NOT (B4). It does not quote because the mechanism needs somebody to. Its schedule is a
 * function of its own state and its own history and reads nothing about the book it is posted into,
 * so it posts the same two prices into an empty market as into a busy one — and when its limit
 * binds it posts nothing, which is D4's legitimate state and D4.a's reason a market can fail.
 *
 * WHAT IS NOT HERE YET. Hedging (E1, E2) is a trade with a counterparty and there is nothing yet to
 * trade against a position — an index future is 13b — so a desk carries what it takes on. And a
 * desk's book is not yet consolidated into its bank's capital (F2): a bank has no capital
 * computation until worklist 11, and the half of F2 that exists now is real — the desk pays that
 * bank for the funding and the capital its inventory uses, every period, at what that bank says its
 * own funding costs.
 */
import type { Family, Violation } from '../../audit/audit.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { instrumentId, partyKindId, type InstrumentId, type PartyId } from '../../core/ids.js';
import { div, material, mul, sub, sum } from '../../core/num.js';
import { none } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import type { NamedParty } from '../../parties/party.js';
import { wasTraded } from '../../prices/price-store.js';
import type { PartyKindProfile } from '../../registry/kinds.js';
import type { ParamDecl } from '../../registry/params.js';
import { BANK } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import {
  DESKS,
  TRADING_BOOK_CAPITAL_RATIO,
  TRADING_BOOK_RISK_WEIGHT,
  deskOf,
  deskParam,
  type DeskDecl,
} from './data.js';
import { periodOfYear, quoteFor, rateOf, type DeskQuote, type DeskState } from './quote.js';

export * from './data.js';
export { quoteFor, rateOf, periodOfYear } from './quote.js';
export type { DeskQuote, DeskState } from './quote.js';

export const DESK = partyKindId('desk');

/**
 * A1, F1, XI-3: a desk is a party like any other and its capacity is finite and enumerable — a
 * limit per line and a limit on the whole book, both its own (D1). It borrows from the bank it
 * lives inside, which is what a trading arm funded by its own treasury is, and it can fail on both
 * counts: it can run out of money to pay its rent, and its inventory can be worth less than what it
 * owes. Nothing is immortal, a desk included.
 */
export const deskKind: PartyKindProfile = {
  id: DESK,
  representation: 'named',
  moneyIssuer: null,
  fails: ['cash', 'solvency'],
  borrows: true,
};

function paramsOf(rows: readonly DeskDecl[]): ParamDecl[] {
  return [
    {
      id: TRADING_BOOK_RISK_WEIGHT,
      value: 1,
      unit: 'ratio of the position',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'Dealer Desks D2: how much of a bank capital requirement a unit of a trading position consumes. It is a rule somebody wrote, not a fact about the world, and it is the number that makes carrying inventory cost capital as well as cash. One, because a position taken with a view is the thing the requirement was written about; a weight per kind of position arrives with the derivative layer (worklist 13a).',
    },
    ...rows.flatMap((r): ParamDecl[] => [
      {
        id: deskParam(r.desk, 'limit.perInstrument'),
        value: r.limitPerInstrument,
        unit: 'units of one line',
        kind: 'preference',
        owner: 'model',
        why: `Dealer Desks D1: the most ${r.name} will be long of one line. ${r.why} A dealer without a limit is a synthetic counterparty wearing a dealer's name (Clearing B3.a), and this is the number that makes it one.`,
      },
      {
        id: deskParam(r.desk, 'limit.aggregate'),
        value: r.limitAggregate,
        unit: 'currency of its own book at the last marks',
        kind: 'preference',
        owner: 'model',
        why: `Dealer Desks D1, F1: the most ${r.name}'s whole book may be worth. Every desk's capacity is finite and enumerable, and a desk full of one thing stops bidding for everything — which is how one line's trouble reaches another.`,
      },
      {
        id: deskParam(r.desk, 'requiredReturnOnCapital'),
        value: r.requiredReturnOnCapital,
        unit: 'per annum',
        kind: 'preference',
        owner: 'model',
        why: `Dealer Desks D2: what ${r.name} needs to earn on the capital its inventory consumes. Its own, and it is half of what a unit costs it to carry — the other half is what its bank paid for the money (C1.a).`,
      },
    ]),
  ];
}

/** What this module keeps: nothing. Everything a desk knows is its own view or its own record. */

/**
 * C1.a, D3, XI-4 joint one: what a unit of a desk's book costs it for one period — what its bank
 * paid for the money plus the return it needs on the capital the position consumes.
 *
 * The bank's cost of funds is that bank's own number with one writer, published under its own name
 * (Banks Lending C1.a). A desk that computed its own would be a second answer to one question.
 */
function rateFor(ctx: MechanismContext, d: DeskDecl): number {
  const said = ctx.journal.ofKind('bank.costOfFunds').filter((e) => e.subjects.includes(d.bank));
  const last = said[said.length - 1];
  const perAnnum = last === undefined ? 0 : last.data['perAnnum'];
  return rateOf(
    typeof perAnnum === 'number' ? perAnnum : 0,
    ctx.params.get(TRADING_BOOK_CAPITAL_RATIO),
    ctx.params.get(TRADING_BOOK_RISK_WEIGHT),
    ctx.params.get(deskParam(d.desk, 'requiredReturnOnCapital')),
    periodOfYear(ctx.calendar, ctx.period),
  );
}

/** A3, D5: what the desk's whole book is worth at the last marks — its inventory as a number. */
function bookValue(ctx: MechanismContext, desk: PartyId): number {
  const terms: number[] = [];
  for (const h of ctx.register.holdingsOf(desk)) {
    const i = ctx.instruments.get(h.instrument);
    if (ctx.registry.instrumentKind(i.kind).pricing === 'money') continue;
    const worth = ctx.valuation.worthOf(desk, h.instrument, ctx.period);
    if (worth.some) terms.push(worth.value.value);
  }
  return sum(terms).value;
}

/**
 * D3, D2, XI-4 joint three: the rent. Every period a desk holds inventory it pays the bank it lives
 * inside for the money that inventory ties up and for the capital it consumes. It is an instruction
 * with two named sides, it can fail like any other payment, and it is the whole reason a desk has a
 * reason to shed a position.
 *
 * It runs before the session, so the quote the desk posts is priced at the rate it has just paid —
 * one number, read back rather than computed twice (Law 4).
 */
function payRent(ctx: MechanismContext, d: DeskDecl): void {
  const desk = d.desk as PartyId;
  if (!ctx.parties.get(desk).status.alive) return;
  const rate = rateFor(ctx, d);
  const base = bookValue(ctx, desk);
  const amount = mul(base, rate, 'what its inventory costs it this period');
  let paid = false;
  if (material(amount, 2, base) && amount > 0) {
    const bank = ctx.parties.get(d.bank as PartyId);
    const leg: Leg = {
      kind: 'money',
      from: { holder: desk, issuer: bank.id },
      to: { holder: bank.id, issuer: bank.id },
      ccy: ctx.registry.region(bank.region).ccy,
      amount,
      fromCell: none(),
      toCell: none(),
    };
    const r = ctx.settle({
      legs: [leg],
      cause: 'transfer',
      reason: `${d.name} pays ${d.bank} for what its book costs`,
    });
    paid = r.outcome === 'settled';
  }
  // D5: inventory, what it costs and what it has room for, together and under its own name — which
  // is what makes "spreads widened without inventory moving" a thing anybody can notice.
  ctx.record(
    'dealers.rent',
    [desk, d.bank],
    {
      desk,
      bank: d.bank,
      rate,
      book: base,
      amount,
      paid,
      linesQuoted: linesQuoted(ctx, d),
      limitAggregate: ctx.params.get(deskParam(d.desk, 'limit.aggregate')),
    },
    false,
  );
}

/** The desk's own numbers this period, read back from what it has already done (Law 4). */
function stateOf(view: ParticipantView, d: DeskDecl): DeskState | undefined {
  const own = view.lastOwn('dealers.rent');
  if (!own.some || own.value.period !== view.period) return undefined;
  const rate = own.value.data['rate'];
  const book = own.value.data['book'];
  const lines = own.value.data['linesQuoted'];
  if (typeof rate !== 'number' || typeof book !== 'number' || typeof lines !== 'number') {
    return undefined;
  }
  return {
    limitPerInstrument: view.params.get(deskParam(d.desk, 'limit.perInstrument')),
    limitAggregate: view.params.get(deskParam(d.desk, 'limit.aggregate')),
    ratePerPeriod: rate,
    bookValue: book,
    cash: view.cash(view.registry.region(view.self.region).ccy),
    linesQuoted: lines,
  };
}

/** C5: how many books this desk is making a market in this period — its money is spread over them. */
function linesQuoted(ctx: MechanismContext, d: DeskDecl): number {
  let n = 0;
  for (const m of ctx.markets) {
    const i = ctx.instruments.get(m.instrument);
    if (i.status.live && d.makes.includes(i.kind)) n += 1;
  }
  return n;
}

/**
 * A2, C5, E3: the quote, as two orders in the same book. The same posted quote belongs in every
 * book the desk makes a market in (C5), and every desk that makes a line is in that line's session
 * — so the desks face each other through it, which is the interdealer market (E3) without a second
 * venue for it.
 */
function ordersOf(view: ParticipantView, m: MarketDecl, rows: readonly DeskDecl[]): readonly Order[] {
  const d = deskOf(rows, view.self.id);
  if (d === undefined || !view.self.status.alive) return [];
  const i = view.instruments.get(m.instrument);
  if (!i.status.live || !d.makes.includes(i.kind)) return [];
  const state = stateOf(view, d);
  if (state === undefined) return [];
  const quoted = quoteFor(view, i.id, state);
  if (!quoted.some) return [];
  const q = quoted.value;
  const out: Order[] = [];
  if (q.bidSize > 0 && q.bid > 0) out.push({ party: view.self.id, side: 'buy', price: q.bid, qty: q.bidSize });
  if (q.offerSize > 0 && q.offer > 0) {
    out.push({ party: view.self.id, side: 'sell', price: q.offer, qty: q.offerSize });
  }
  return out;
}

/**
 * Fund Shares E3, E3.a: the desk's reason to close a gap between an exchange-traded fund's two
 * values — and its reason not to.
 *
 * The fund publishes what its book comes to per share; the market prints what somebody paid for
 * one. When the print is above the book by more than what the trade costs the desk, the desk can
 * deliver a basket it already holds and take shares worth more than the basket was; when it is
 * below, it can deliver shares and take a basket worth more than the shares were. Either way it is
 * a TRADE it does because it is worth doing (E1.a's rule for a hedge, applied here), never a rule
 * tying the two numbers together — so a gap nobody will close stays open, which is E3.a exactly,
 * and E4 says a persistently large one is a finding about liquidity.
 *
 * What it costs the desk is one period of carrying the position it is about to take on, at its own
 * rate (D3). What limits it is what it holds and what its own limits leave it room for (D1, F1).
 */
function arbitrage(ctx: MechanismContext, d: DeskDecl): void {
  const desk = d.desk as PartyId;
  if (!ctx.parties.has(desk) || !ctx.parties.get(desk).status.alive) return;
  const view = ctx.participant(desk);
  const state = stateOf(view, d);
  if (state === undefined) return;
  for (const v of ctx.venues) {
    if (v.key['kind'] !== 'etf') continue;
    const fund = v.key['fund'];
    const line = v.key['share'];
    if (fund === undefined || line === undefined) continue;
    const share = instrumentId(line);
    if (!ctx.instruments.has(share) || !ctx.instruments.get(share).status.live) continue;
    if (!d.makes.includes(ctx.instruments.get(share).kind)) continue;
    const struck = view.lastPublicAbout('etf.struck', fund);
    const nav = struck.some ? struck.value.data['perShare'] : undefined;
    const print = view.print(share);
    if (typeof nav !== 'number' || nav <= 0 || !print.some) continue;
    // Appendix B, Clearing E4: the gap is against a price the market MADE. A mark carried forward
    // because nobody traded is not a level the desk could sell into, and a desk that delivered a
    // basket against one would be trading on its own carried number — the arbitrage would be a
    // derivative on an uncleared price, and the "gap" it closed would be an artefact of the carry.
    if (!wasTraded(print.value)) continue;
    const gap = sub(print.value.price, nav, 'what the market pays over the book');
    // What one period of carrying it costs the desk. Below that it is not worth its while, and it
    // does nothing — which is how a gap survives (E3.a).
    const worth = mul(nav, state.ratePerPeriod, 'what a share costs it to carry for a period');
    if (Math.abs(gap) <= worth) continue;
    const held = view.quantity(share);
    // Clearing A3: what it posts is what it can actually do. A creation is a basket it has to
    // DELIVER, so it is worth what the thinnest line of that basket in its own inventory is worth
    // — the fund publishes what a creation unit is made of (E3), and this reads it against what
    // the desk is holding. A redemption is shares it has to deliver, so it is what it holds.
    const shares = gap > 0
      ? deliverable(view, struck.some ? struck.value.data['basket'] : undefined,
          sub(state.limitPerInstrument, held, 'room it has for more of this line'))
      : held;
    if (shares <= 0 || !material(shares, 2, state.limitPerInstrument)) continue;
    // The venue is not a market and it does not clear: everybody who brings a basket gets what
    // that basket is worth (Clearing B2). It names no price because there is no price to name.
    ctx.post(v.id, { party: desk, side: gap > 0 ? 'buy' : 'sell', price: 'market', qty: shares });
    ctx.record(
      'dealers.arbitrage',
      [desk, fund, share],
      { desk, fund, share, nav, price: print.value.price, gap, worth, shares, side: gap > 0 ? 'create' : 'redeem' },
      false,
    );
  }
}

/**
 * E3: how many creation units this desk could actually deliver, out of what it is holding and the
 * room it has left. A basket it cannot make up is a creation it cannot do, and posting one would
 * be posting a schedule it could not honour (Clearing A3).
 */
function deliverable(view: ParticipantView, basket: unknown, room: number): number {
  if (typeof basket !== 'object' || basket === null || room <= 0) return 0;
  let most = room;
  let lines = 0;
  for (const [line, perShare] of Object.entries(basket as Record<string, unknown>)) {
    if (typeof perShare !== 'number' || perShare <= 0) continue;
    lines += 1;
    const canMake = div(view.free(instrumentId(line)), perShare, 'creation units this line backs');
    if (canMake < most) most = canMake;
  }
  return lines > 0 ? most : 0;
}

/**
 * D5, E4: what every desk is carrying, what it is quoting and what room it has left, published
 * together. E4's identity — dealer inventory is the position the rest of the world does not hold —
 * is the ownership family's already (held equals issued); what this adds is that the three numbers
 * D5 says should move together are all readable in one place, so a period in which spreads widened
 * and inventory did not is visible rather than inferred.
 */
function publishBooks(ctx: MechanismContext, rows: readonly DeskDecl[]): void {
  for (const d of rows) {
    const desk = d.desk as PartyId;
    if (!ctx.parties.has(desk) || !ctx.parties.get(desk).status.alive) continue;
    const view = ctx.participant(desk);
    const state = stateOf(view, d);
    if (state === undefined) continue;
    const lines: Record<string, unknown> = {};
    for (const m of ctx.markets) {
      const i = ctx.instruments.get(m.instrument);
      if (!i.status.live || !d.makes.includes(i.kind)) continue;
      const quoted = quoteFor(view, i.id, state);
      if (!quoted.some) continue;
      const q: DeskQuote = quoted.value;
      lines[i.id] = {
        inventory: view.quantity(i.id),
        bid: q.bid,
        offer: q.offer,
        spread: sub(q.offer, q.bid, 'the width it quoted'),
        skew: q.skew,
        bidSize: q.bidSize,
        offerSize: q.offerSize,
        // D4, D5: which limit shrank the bid — the position, the whole book, or the money.
        binds: q.binds,
      };
    }
    ctx.record(
      'dealers.book',
      [desk],
      {
        desk,
        book: state.bookValue,
        // D1, F1: how much of its capacity is used. Finite and enumerable, and here it is enumerated.
        roomLeft: sub(state.limitAggregate, state.bookValue, 'room in the whole book'),
        ratePerPeriod: state.ratePerPeriod,
        lines,
      },
      true,
    );
  }
}

/**
 * D3: a desk that carried inventory and was charged nothing for it is unreachable.
 *
 * The rent is what makes a quote skew, so a period in which a desk held a book and no rent left its
 * account is a period in which the whole mechanism XI-4's third joint describes was absent — and it
 * would be absent silently, because every price would still print. A payment that FAILED is not
 * this: that is a desk that could not pay, which is a real state with real consequences (XI-3).
 */
function inventoryPaysRent(rows: readonly DeskDecl[]): Family {
  return {
    name: 'flows',
    contributor: 'dealers',
    spec: 'Dealer Desks D3 XI-4',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      // Seed A2: at period zero nothing has run and nothing has been held for a period yet.
      if (view.period === 0) return out;
      for (const d of rows) {
        const desk = d.desk as PartyId;
        if (!view.parties.has(desk) || !view.parties.get(desk).status.alive) continue;
        const said = view.journal
          .ofKind('dealers.rent')
          .filter((e) => e.period === view.period && e.subjects.includes(desk));
        const last = said[said.length - 1];
        if (last === undefined) {
          out.push({
            family: 'flows',
            spec: 'Dealer Desks D3',
            owner: desk,
            size: 0,
            unit: view.registry.region(view.parties.get(desk).region).ccy,
            period: view.period,
            message: `${desk} was not charged for its book this period`,
          });
          continue;
        }
        const book = last.data['book'];
        const amount = last.data['amount'];
        const rate = last.data['rate'];
        if (typeof book !== 'number' || typeof amount !== 'number' || typeof rate !== 'number') continue;
        // Law 7: a book worth something at a positive rate owes something, down to the dust of the
        // one multiplication that produced it. A rate of nothing is a bank that pays nothing for
        // its money and a desk that needs nothing on its capital, which is a state and not a defect.
        if (book <= 0 || rate <= 0 || material(amount, 2, book)) continue;
        out.push({
          family: 'flows',
          spec: 'Dealer Desks D3',
          owner: desk,
          size: book,
          unit: view.registry.region(view.parties.get(desk).region).ccy,
          period: view.period,
          message: `${desk} carried a book of ${book} at ${rate} a period and was charged ${amount}`,
        });
      }
      return out;
    },
  };
}

/**
 * The module. `rows` is this world's desks (Law 15: the data says). A world with none has markets
 * that clear only when a natural buyer and a natural seller happen to want the opposite thing at
 * the same moment, which is a real and much thinner world.
 */
export function dealers(rows: readonly DeskDecl[] = DESKS): SystemModule {
  return {
    id: 'dealers',
    spec: 'Dealer Desks, XI-4',
    // A desk lives inside a bank and borrows from it (A1), and it opens holding the lines it makes
    // a market in — so whoever registered those lines, and whoever created the banks, has to have
    // done it first.
    requires: ['bank-lending', 'equity', 'sovereign-instruments', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [deskKind],
    curveFamilies: [],
    units: [],
    params: paramsOf(rows),
    phases: [
      {
        name: 'dealers.rent',
        spec: 'Dealer Desks D2 Dealer Desks D3 XI-4',
        cycle: 0,
        // Before the session, because what a desk quotes is priced at what its book costs it, and
        // the charge is the number the quote reads back (Law 4). After the corporate actions,
        // because a coupon that arrived this morning is money it has to pay with.
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          for (const d of rows) payRent(ctx, d);
        },
      },
      {
        name: 'dealers.arbitrage',
        spec: 'Fund Shares E3 Fund Shares E3.a Dealer Desks D1',
        cycle: 0,
        // With the rent, because what it costs the desk to carry a position is what decides whether
        // closing a gap is worth doing at all, and that is the number the rent phase just struck.
        anchor: { after: 'dealers.rent' },
        run: (ctx: MechanismContext) => {
          for (const d of rows) arbitrage(ctx, d);
        },
      },
      {
        name: 'dealers.book',
        spec: 'Dealer Desks D5 Dealer Desks E4',
        cycle: 'anchor',
        // After the marks are in the books, so what it says the book is worth is what the book is
        // worth (Clearing D4).
        anchor: { after: 'revaluation' },
        run: (ctx: MechanismContext) => {
          publishBooks(ctx, rows);
        },
      },
    ],
    participants: [
      {
        partyKind: DESK,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => ordersOf(view, m, rows),
      },
    ],
    families: [inventoryPaysRent(rows)],
    seed(ctx: SeedContext): void {
      for (const d of rows) {
        const bank = ctx.parties.get(d.bank as PartyId);
        const desk: NamedParty = {
          id: d.desk as PartyId,
          kind: DESK,
          region: bank.region,
          name: d.name,
          // A1, F2: its account is at its own bank, so every payment it makes to that bank moves no
          // reserves and every payment it makes to anybody else moves the bank's.
          bank: bank.id,
          representation: 'named',
          status: { alive: true },
        };
        ctx.parties.add(desk);
        ctx.endowMoney(desk.id, ctx.registry.region(bank.region).ccy, d.cash);
        // A3, Seed A3: the inventory it opens with, of every line of a kind its data says it opens
        // holding. What it opens holding of a line nobody else holds IS that line — the float the
        // rest of the world buys from — and what it opens holding of a line the seed already gave
        // to somebody else is nothing, because a desk endowed with more of the sovereign's paper
        // would be a seed deciding how much the sovereign owes.
        for (const i of ctx.instruments.all()) {
          const units = d.opens[String(i.kind)];
          if (units === undefined || units <= 0 || !i.status.live) continue;
          const opening = ctx.prices.latest(i.id, ctx.period);
          if (!opening.some) continue;
          ctx.endowUnits(desk.id, i.id, units, opening.value.price);
        }
      }
    },
  };
}

/** A3, D5: what a named desk is carrying, for the observer and the tests (one derivation, Law 4). */
export function inventoryOf(ctx: MechanismContext, desk: PartyId, instrument: InstrumentId): number {
  return ctx.register.quantity(desk, instrument);
}

/** The desk row a named party is, for readers that need it (Law 15). */
export const desk = (party: string): DeskDecl | undefined => deskOf(DESKS, party);

/** Which bank a desk lives inside (A1) — used by the observer and by worklist 11's capital. */
export function bankOf(rows: readonly DeskDecl[], party: PartyId): PartyId | undefined {
  const d = deskOf(rows, party);
  return d === undefined ? undefined : (d.bank as PartyId);
}

export { BANK };
