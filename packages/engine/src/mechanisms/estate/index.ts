/**
 * The estate: a party dies, its assets are sold into real markets, its claims are ranked, and the
 * proceeds are distributed in rank order. What is left over is loss, and it lands on named holders.
 *
 * @spec XI-2 XI-3 XI-8 Firm Birth D1 Firm Birth D2 Firm Birth D2.a Firm Birth D2.b Firm Birth D3 Firm Birth D4 Firm Birth D4.a Firm Birth D5 Firm Birth D6 Firm Birth D6.a Firm Birth E1 Firm Birth E2 Firm D4 Firm D5 Banks Capital C1 Banks Capital C1.a Banks Lending E4 Banks Lending E5 Register F2 Register E3 Money E4 Law 15
 *
 * XI-3 is the reason this exists: a default state read in many places and written only by the seed
 * makes every institution immortal, and an immortal party is the TERMINATION CONDITION of every loss
 * chain in the model. A cascade that reaches one stops there, and stops without saying so.
 *
 * XI-8 is what makes it real rather than an accounting step, and there are four parts to that.
 * ASSETS ARE SOLD, not valued: the estate offers what it holds into the market that thing always
 * traded in, at a reservation that falls as its programme runs out, and what nobody takes is
 * abandoned — a formula discount off book is a stated price with no buyer. EVERY CLAIM RANKS by the
 * instrument's OWN stated seniority (Bond N13.a), never by what type of thing it is. THE REAL
 * CONSEQUENCES ARE PART OF IT: the people employed by a dead firm lose their jobs through the labour
 * market's own separation path, never by a count being decremented (D4.a). AND IT CONSERVES AND
 * TERMINATES: recoveries plus losses equal the assets at realisation, every currency is paid away,
 * and every reference to the dead party resolves to the estate.
 *
 * WHAT DECIDES A PARTY HAS FAILED is not this module's opinion. Each kind states what it can fail on
 * (XI-3), and this asks the two questions those answers name: did something fall due out of its own
 * balance that it could not pay and still cannot, and are its liabilities past its assets. A kind
 * that names neither cannot die, and the central bank is the one that names neither because it
 * cannot run out of what it alone issues.
 */
import type { Family, Violation } from '../../audit/audit.js';
import { holdsSomething, type AuditView } from '../../audit/view.js';
import type { CurrencyCode, InstrumentId, PartyId } from '../../core/ids.js';
import { currencyUnit, paramId, partyId, partyKindId } from '../../core/ids.js';
import { div, material, mul, sub, sum, withinDust } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import { shareFor } from '../../ledger/settlement.js';
import { weightOf } from '../../parties/party.js';
import { issuerOf } from '../../register/instruments.js';
import type { PartyKindProfile } from '../../registry/kinds.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import { failedWhy } from '../../world/failure.js';
import type { SystemModule } from '../../world/module.js';
import type { Order } from '../../clearing/solver.js';
import type { MarketDecl } from '../../clearing/market.js';

export const ESTATE = partyKindId('estate');

export const ESTATE_PARAMS = {
  programme: paramId('estate.programme.periods'),
} as const;

/** XI-8: what the estate is winding up, and how long it has to do it in. */
interface Winding {
  readonly dead: string;
  readonly opened: number;
  readonly closesAfter: number;
  closed: boolean;
}

interface Book {
  next: number;
  estates: Record<string, Winding>;
}

function book(ctx: MechanismContext): Book {
  return ctx.state<Book>('estates', () => ({ next: 1, estates: {} }));
}

export const estateKind: PartyKindProfile = {
  id: ESTATE,
  representation: 'named',
  moneyIssuer: null,
  // XI-8: it is where the chain of successors ENDS. It may cease with nobody after it, and the
  // names family only lets it once it holds nothing in any currency (Firm Birth D6.a).
  terminal: true,
  // An estate does not fail: it is what failure resolves into.
  fails: [],
  // Banks Lending A1, C3: and nobody lends to it. Its whole business is being wound up — there is
  // nobody left to sign and no future income to repay out of, so a bank asked for an overdraft by
  // one declines, which is the credit decision Money B3.a wants and the refusal B3.c records.
  borrows: false,
  // XI-8: it holds the account of the party it succeeded, and it is realising that party rather
  // than running it. Shopping for a deposit rate is not something a liquidation does.
  // A1.b: and its balance is not funding either — it is proceeds waiting to be paid out. Paying it
  // a deposit rate would give a party being wound up an income, and the treasury a tax claim to
  // rank among the creditors; this world has a mechanism for neither.
  depositClass: null,
};

/**
 * XI-8, Firm Birth D5: the estate opens. Everything the dead party held moves to it by a real
 * instruction at what the book carried it at, and everything the dead party ISSUED is re-seated on
 * it — so a holder of that paper still holds a claim on somebody who exists. Then the party ceases,
 * naming the estate, and every reference to it resolves there.
 *
 * A party with no claims still opens one (D5): it does not keep its cash for ever.
 */
function open(ctx: MechanismContext, dead: PartyId, because: string): void {
  const b = book(ctx);
  const p = ctx.parties.get(dead);
  const id = partyId(`estate.${dead}`);
  ctx.enter({
    id,
    kind: ESTATE,
    region: p.region,
    name: `Estate of ${p.name}`,
    bank: p.bank,
    representation: 'named',
    status: { alive: true },
  });
  const ccy = ctx.registry.region(p.region).ccy;
  // D5: everything the dead party ISSUED is assumed by the estate, so a holder of that paper still
  // holds a claim on somebody who exists. It goes over the wire like any other change of book: the
  // obligation leaves one balance sheet and lands on the other in one numbered instruction (Law 5).
  //
  // BEFORE the assets move, because what a liability is worth is read off the book behind it: a
  // claim ON this party's book (Fund Shares B1) is worth nothing once the book has gone to the
  // estate, and the estate would assume nothing while its holders still carried the old value.
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some || i.issuer.value !== dead) continue;
    ctx.settle({
      legs: [{ kind: 'assume', from: dead, to: id, instrument: i.id }],
      cause: 'corporateAction',
      reason: `${id} assumes ${i.id} from ${dead}`,
    });
  }
  for (const h of ctx.register.holdingsOf(dead)) {
    const i = ctx.instruments.get(h.instrument);
    const units = ctx.register.quantity(dead, h.instrument);
    if (units < 0) {
      // Money B3.c: a raw negative balance is a claim with no instrument behind it, and it should
      // not survive to here — a drawing becomes a loan row before anything can die of it. If one
      // ever does, it is named rather than carried away silently or left on a dead party.
      ctx.record('estate.overdrawn', [id, dead], { estate: id, dead, instrument: h.instrument, short: -units }, true);
      continue;
    }
    if (units === 0) continue;
    const leg: Leg =
      ctx.registry.instrumentKind(i.kind).pricing === 'money'
        ? {
            kind: 'money',
            from: { holder: dead, issuer: i.issuer.some ? i.issuer.value : dead },
            to: { holder: id, issuer: i.issuer.some ? i.issuer.value : dead },
            ccy: i.ccy,
            amount: units,
            fromCell: none(),
            toCell: none(),
          }
        : {
            kind: 'asset',
            from: dead,
            to: id,
            instrument: h.instrument,
            qty: units,
            // Register F2: at what the book carried it at. Nothing is realised by the move itself;
            // what it fetches is what a market gives the estate for it later (XI-8).
            pricePerUnit: none(),
            accruedPerUnit: none(),
            fromCell: none(),
            toCell: none(),
          };
    ctx.settle({ legs: [leg], cause: 'transfer', reason: `${dead} to its estate` });
  }
  ctx.cease(dead, id);
  const closesAfter = ctx.period + ctx.params.get(ESTATE_PARAMS.programme);
  b.estates[id] = { dead, opened: ctx.period, closesAfter, closed: false };
  ctx.record('estate.opened', [id, dead], { estate: id, dead, because, ccy, closesAfter }, true);
}

/**
 * XI-8, D1: assets are SOLD, into the market that thing always traded in, to whoever bids. The
 * reservation falls as the programme runs out — an estate with a year left can wait for a price and
 * one with a week cannot — and by the last period it takes whatever the book gives it. Nothing here
 * is a discount off book: what it fetches is what somebody paid.
 */
function offers(view: ParticipantView, m: MarketDecl, closesAfter: number): readonly Order[] {
  const units = view.quantity(m.instrument);
  if (!material(units, 2, units)) return [];
  const left = sub(closesAfter, view.period, 'periods left in the programme');
  if (left <= 0) return [{ party: view.self.id, side: 'sell', price: 'market', qty: units }];
  const print = view.print(m.instrument);
  if (!print.some) return [{ party: view.self.id, side: 'sell', price: 'market', qty: units }];
  // Its patience is what is left of its programme: it asks the last price while it has time, and
  // less of it as the time goes. There is no discount curve here — the number is how long is left.
  const total = sub(closesAfter, view.period, 'left') + 1;
  return [
    {
      party: view.self.id,
      side: 'sell',
      price: mul(print.value.price, div(left, total, 'how much of its patience is left'), 'reservation'),
      qty: units,
    },
  ];
}

/** A claim on the estate: somebody holding paper the dead party promised (Bond N13, N13.a). */
interface Claim {
  readonly holder: PartyId;
  readonly instrument: InstrumentId;
  readonly units: number;
  readonly seniority: number;
}

/**
 * XI-8: the claims, read from the register rather than kept beside it (Law 19). Every instrument
 * the estate now owes, whoever holds it, ranked by that instrument's OWN stated seniority — never
 * by what type of thing it is, which is what makes subordination decorative.
 */
function claimsOn(ctx: MechanismContext, estate: PartyId): Claim[] {
  const out: Claim[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some || i.issuer.value !== estate) continue;
    const seniority = ctx.registry.instrumentKind(i.kind).ranking(i).seniority;
    for (const holder of ctx.register.holdersOf(i.id)) {
      if (holder === estate) continue;
      const units = ctx.register.totalQuantity(holder, i.id);
      if (units <= 0) continue;
      out.push({ holder, instrument: i.id, units, seniority });
    }
  }
  return out.sort((a, b) => a.seniority - b.seniority);
}

/**
 * XI-8, D2, D2.a: the proceeds are distributed in rank order, pro rata within a rank, and THE
 * RECOVERY IS WHAT THE ASSETS ACTUALLY FETCHED. A junior claim can recover nothing, and that is the
 * point of being junior (Corporate Credit G5.a). Paying a claim redeems that much of it at par; what
 * is not paid stays outstanding until the estate closes and writes it off (Banks Lending E5).
 */
function distribute(ctx: MechanismContext, estate: PartyId, ccy: CurrencyCode): void {
  const claims = claimsOn(ctx, estate);
  if (claims.length === 0) return;
  // Banks Lending B1: a loan creates its deposit at the LENDER, so a party that borrowed from two
  // banks banks in two places and its estate inherits both accounts. Each one is distributed down
  // the same waterfall, which is the same answer as pooling them — a rank is exhausted before the
  // next is reached, from whatever account the money is in — without inventing an interbank payment
  // between them that nobody asked for.
  for (const account of accountsOf(ctx, estate, ccy)) distributeFrom(ctx, estate, claims, account, ccy);
}

/** The estate's own money accounts in one currency: one per bank the dead party banked with. */
function accountsOf(ctx: MechanismContext, estate: PartyId, ccy: CurrencyCode): InstrumentId[] {
  const out: InstrumentId[] = [];
  for (const h of ctx.register.holdingsOf(estate)) {
    const i = ctx.instruments.get(h.instrument);
    if (ctx.registry.instrumentKind(i.kind).pricing === 'money' && i.ccy === ccy) out.push(i.id);
  }
  return out;
}

function distributeFrom(
  ctx: MechanismContext,
  estate: PartyId,
  claims: readonly Claim[],
  account: InstrumentId,
  ccy: CurrencyCode,
): void {
  let cash = ctx.register.quantity(estate, account);
  // D2: the ranks this estate actually owes, in order — read off the claims rather than counted
  // from a number nobody stated. A rank is exhausted before the next one is reached.
  const ranks = [...new Set(claims.map((c) => c.seniority))].sort((a, b) => a - b);
  for (const rank of ranks) {
    if (cash <= 0) break;
    const here = claims.filter((c) => c.seniority === rank);
    const owed = sum(here.map((c) => c.units)).value;
    if (owed <= 0) continue;
    const share = cash < owed ? div(cash, owed, 'what this rank gets on the pound') : 1;
    for (const c of here) {
      const pay = mul(c.units, share, 'what this holder is paid');
      if (!material(pay, 2, c.units)) continue;
      const paid = repay(ctx, estate, c, pay, account, ccy);
      if (!paid.some) continue;
      cash = sub(cash, paid.value, 'cash left to distribute');
    }
  }
}

/**
 * Paying part of a claim: that much of it is redeemed at par, and the rest stays outstanding.
 * Returns what was actually paid — which is what the estate has left to distribute (D2) — or none
 * when nothing could be.
 */
function repay(
  ctx: MechanismContext,
  estate: PartyId,
  claim: Claim,
  amount: number,
  account: InstrumentId,
  ccy: CurrencyCode,
): Option<number> {
  const holder = ctx.parties.get(claim.holder);
  // Law 8, XI-15: a claim is redeemed in whole pieces of the money it is denominated in, and for a
  // cell in whole pieces for each member. What is left below a piece stays outstanding, which is
  // what a partial repayment is: the rest of the claim is still there (D2).
  const share = shareFor(ctx.registry, holder, currencyUnit(ccy), div(amount, weightOf(holder), 'per member'));
  const perMember = share.perMember;
  if (share.total <= 0) return none<number>();
  const legs: Leg[] = [
    {
      kind: 'asset',
      from: claim.holder,
      to: estate,
      instrument: claim.instrument,
      qty: share.total,
      pricePerUnit: some(1),
      accruedPerUnit: none(),
      fromCell: holder.representation === 'cell' ? some({ perMember, weight: holder.weight }) : none(),
      toCell: none(),
    },
    {
      kind: 'money',
      from: { holder: estate, issuer: issuerOf(ctx.instruments.get(account)) },
      to: { holder: claim.holder, issuer: holder.bank },
      ccy,
      amount: share.total,
      fromCell: none(),
      toCell: holder.representation === 'cell' ? some({ perMember, weight: holder.weight }) : none(),
    },
  ];
  const r = ctx.settle({
    legs,
    cause: 'maturity',
    reason: `${estate} pays ${claim.holder} on ${claim.instrument}`,
  });
  if (r.outcome !== 'settled') return none<number>();
  ctx.record(
    'estate.paid',
    [estate, claim.holder, claim.instrument],
    { estate, holder: claim.holder, instrument: claim.instrument, paid: share.total, of: claim.units },
    false,
  );
  return some(share.total);
}

/**
 * XI-8, D6, D6.a: it terminates. What is left of every claim when the programme ends is written off
 * — a real leg back to whoever owes it at what it fetched, which was nothing (Banks Lending E5) —
 * and then the estate must hold nothing in any currency, because a residual on a dead party is a
 * defect that must be found and paid away.
 */
function close(ctx: MechanismContext, estate: PartyId, w: Winding, ccy: CurrencyCode): void {
  // D1: what nobody bought by the last period is ABANDONED. A thing leaves the world by a destroy
  // leg, which is one-sided because nobody is on the other end of a heap of grain nobody wanted
  // (Register A3). A CLAIM cannot be abandoned that way — somebody still owes it, and handing it
  // back to them for nothing would be a windfall to the issuer — so an estate holding paper it
  // could not sell is not finished: it says so and stays open, which is a real state and not a
  // tidy ending invented for it.
  for (const h of ctx.register.holdingsOf(estate)) {
    const i = ctx.instruments.get(h.instrument);
    const kind = ctx.registry.instrumentKind(i.kind);
    if (kind.pricing === 'money') continue;
    const units = ctx.register.quantity(estate, h.instrument);
    if (units <= 0) continue;
    if (kind.physical !== true) {
      ctx.record(
        'estate.residual',
        [estate, h.instrument],
        { estate, left: units, instrument: h.instrument, why: 'nobody bought it and nobody can be given it: the estate stays open' },
        true,
      );
      return;
    }
    ctx.settle({
      legs: [
        {
          kind: 'destroy',
          party: estate,
          instrument: h.instrument,
          qty: units,
          why: 'scrapped',
          fromCell: none(),
        },
      ],
      cause: 'corporateAction',
      reason: `${estate} abandons ${h.instrument}: the programme ended and nobody bought it`,
    });
    ctx.record('estate.abandoned', [estate, h.instrument], { estate, instrument: h.instrument, units }, true);
  }
  for (const c of claimsOn(ctx, estate)) {
    const holder = ctx.parties.get(c.holder);
    const perMember = holder.representation === 'cell' ? div(c.units, holder.weight, 'per member') : c.units;
    const leg: Leg = {
      kind: 'asset',
      from: c.holder,
      to: estate,
      instrument: c.instrument,
      qty: c.units,
      // What it fetched is nothing. The holder's loss is what it was carrying, and it lands there.
      pricePerUnit: some(0),
      accruedPerUnit: none(),
      fromCell: holder.representation === 'cell' ? some({ perMember, weight: holder.weight }) : none(),
      toCell: none(),
    };
    ctx.settle({
      legs: [leg],
      cause: 'default',
      reason: `${estate} writes off what it cannot pay of ${c.instrument}`,
    });
  }
  // D6.a: a residual on a dead party is a defect and must be found and paid away in every account
  // it held. Law 7: what is left is the residue of a balance moved once per leg since the estate
  // opened, so what it may call nothing is that account's own walk — the same number settlement
  // uses when it decides whether the account is short at all (Law 4).
  for (const h of ctx.register.holdingsOf(estate)) {
    if (ctx.registry.instrumentKind(ctx.instruments.get(h.instrument).kind).pricing !== 'money')
      continue;
    const left = ctx.register.quantity(estate, h.instrument);
    if (withinDust(left, 0, ctx.register.moneyWalk(estate, h.instrument).dust)) continue;
    // A residual with a claim to pay it to would have been paid above, so there is none — and the
    // holders it belongs to are its owners, whose claim is a share register (worklist 9). It is
    // named rather than kept quietly or paid to nobody (Law 2: a residual with no holder is a defect).
    ctx.record(
      'estate.residual',
      [estate, h.instrument],
      {
        estate,
        left,
        ccy,
        account: h.instrument,
        why: 'no claim left to pay it to; the owners hold shares, which is worklist 9',
      },
      true,
    );
    return;
  }
  w.closed = true;
  ctx.cease(estate, estate);
  ctx.record('estate.closed', [estate], { estate, dead: w.dead, opened: w.opened }, true);
}

/**
 * XI-8, Firm Birth D6: recoveries plus losses equal the assets at realisation, exactly — money is
 * not destroyed by a default, it is transferred and revalued. What makes that more than an identity
 * is the thing it forbids: cash leaving an estate to anybody who is not one of its claimants. Every
 * claim is either paid or written off, so if nothing leaks then what the assets fetched is what the
 * claimants got, and what they did not get is the loss (D3). A leak would be value going somewhere
 * with no claim behind it, and it would look like a smaller loss.
 */
function nothingLeaksOut(): Family {
  return {
    name: 'flows',
    contributor: 'estate',
    spec: 'XI-8 Firm Birth D6 Firm Birth D3',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const estates = new Set<string>(view.parties.ofKind(ESTATE).map((p) => p.id));
      if (estates.size === 0) return out;
      // Who still holds paper the estate owes, plus anybody it redeemed the last of this period:
      // a claim paid off in full stops being a holding at the instant it is paid.
      const claimants = new Map<string, Set<string>>();
      for (const estate of estates) {
        const here = new Set<string>();
        for (const c of claimsOf(view, estate as PartyId)) here.add(c);
        claimants.set(estate, here);
      }
      for (const e of view.journal.ofKind('estate.paid')) {
        if (e.period !== view.period) continue;
        claimants.get(String(e.data['estate']))?.add(String(e.data['holder']));
      }
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled') continue;
        // D6 is about DISTRIBUTIONS — money an estate paid away out of what it realised. A
        // corporate action is not one: when the bank an estate banks at is resolved, the estate's
        // own balance is extinguished at the failed bank and re-issued by the acquirer in one
        // instruction (Banks Capital C3.b), and the first leg of that reads as a payment to a bank
        // it has no claim on. Nothing left the estate: it holds the same money at a new issuer.
        if (r.instruction.cause === 'corporateAction') continue;
        for (const leg of r.instruction.legs) {
          if (!isMoneyLeg(leg) || !estates.has(leg.from.holder)) continue;
          if (claimants.get(leg.from.holder)?.has(leg.to.holder) === true) continue;
          out.push({
            family: 'flows',
            spec: 'Firm Birth D6',
            owner: leg.from.holder,
            size: leg.amount,
            unit: leg.ccy,
            period: view.period,
            message: `${leg.from.holder} paid ${leg.amount} to ${leg.to.holder}, who has no claim on it`,
          });
        }
      }
      return out;
    },
  };
}

/** Who holds paper this estate owes, read from the register rather than kept beside it (Law 19). */
function claimsOf(view: AuditView, estate: PartyId): string[] {
  const out: string[] = [];
  for (const i of view.instruments.all()) {
    if (!i.status.live || !i.issuer.some || i.issuer.value !== estate) continue;
    for (const holder of view.register.holdersOf(i.id)) out.push(holder);
  }
  return out;
}

/**
 * Firm Birth D6.a, XI-8: no dead party keeps anything. An estate that has ceased holding units or
 * cash is a residual with nobody to claim it, and it is the defect the clause names.
 */
function nothingLeftBehind(): Family {
  return {
    name: 'names',
    contributor: 'estate',
    spec: 'Firm Birth D5 Firm Birth D6.a XI-8 Register F2',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const p of view.parties.all()) {
        if (p.status.alive || p.status.successor !== p.id) continue;
        for (const h of view.register.holdingsOf(p.id)) {
          if (!holdsSomething(view, p.id, h.instrument)) continue;
          const units = view.register.quantity(p.id, h.instrument);
          out.push({
            family: 'names',
            spec: 'Firm Birth D6.a',
            owner: p.id,
            size: units,
            unit: view.instruments.get(h.instrument).unit,
            period: view.period,
            message: `${p.id} ended and still holds ${units} of ${h.instrument}`,
          });
        }
      }
      return out;
    },
  };
}

export const estate: SystemModule = {
  id: 'estate',
  spec: 'XI-8',
  // It reads the register, the ledger and the instrument profiles, all of which are the kernel's.
  requires: [],
  instrumentKinds: [],
  partyKinds: [estateKind],
  curveFamilies: [],
  units: [],
  params: [
    {
      id: ESTATE_PARAMS.programme,
      value: 8,
      unit: 'periods',
      kind: 'policy',
      owner: 'standardSetter',
      why: 'XI-8, Firm Birth D1: how long an estate has to sell what it holds before what nobody has bought is abandoned. An insolvency regime is a rule somebody wrote, and how long it gives is the whole of the difference between a fire sale and an orderly one.',
    },
  ],
  phases: [
    {
      name: 'estates.resolve',
      spec: 'XI-3 XI-8 Firm Birth D1 Firm Birth D5 Banks Capital C1',
      cycle: 'anchor',
      // The resolution slot: after the markets, the period's payments, the drawings that became
      // rows — and after REVALUATION, because Firm D4 asks whether liabilities exceed assets AT
      // MARKS, and the marks are not in anybody's book until revaluation has put them there. Asked
      // before it, a party whose own liabilities are marked (a fund, whose shares ARE its book:
      // Fund Shares A3) reads as insolvent by exactly whatever it paid out this period.
      anchor: { after: 'revaluation' },
      run: (ctx: MechanismContext): void => {
        for (const p of ctx.parties.all()) {
          if (!p.status.alive) continue;
          // An estate is not asked either, and nothing here says so: its kind states that it fails
          // on nothing, and a kind that names nothing cannot die (Law 15, XI-3).
          //
          // XI-3, Banks Capital C3.b: and a kind whose failure another module takes charge of is
          // left to it. A bank is the one: an estate cannot owe a depositor, because an estate does
          // not issue money (Money A1), so a bank goes to a resolution instead — a valuation, a
          // hierarchy, an acquirer and a guarantee. Nothing here knows what a bank is; it asks.
          if (ctx.resolvesItsOwn(p.kind)) continue;
          const why = failedWhy(ctx, ctx.participant(p.id));
          if (why !== undefined) open(ctx, p.id, why);
        }
      },
    },
    {
      name: 'estates.settle',
      spec: 'XI-8 Firm Birth D2 Firm Birth D3 Firm Birth D6 Banks Lending E5',
      cycle: 2,
      /**
       * The period's payments, after the session it sold into (docs/PLAN.md §8 puts it here). It is
       * NOT after revaluation, and the reason is a party whose own liability IS its book: a fund's
       * claim on itself is re-marked when the marks are taken (Fund Shares B1), so an estate that
       * wrote off a line the fund held AFTER that left the fund owing more than it had for a whole
       * period — sixty of equity on a book of sixty, reported as a fund that had mislaid somebody's
       * money (A3) when what had actually happened was a write-off nobody had marked yet.
       *
       * Opening an estate still happens after revaluation, because Firm D4 asks whether liabilities
       * exceed assets AT MARKS. What an estate then DOES is paying and being paid, and that belongs
       * with the period's other payments.
       */
      anchor: { after: 'markets' },
      run: (ctx: MechanismContext): void => {
        const b = book(ctx);
        for (const [id, w] of Object.entries(b.estates)) {
          if (w.closed) continue;
          const estateId = partyId(id);
          if (!ctx.parties.get(estateId).status.alive) continue;
          const ccy = ctx.registry.region(ctx.parties.get(estateId).region).ccy;
          distribute(ctx, estateId, ccy);
          if (ctx.period >= w.closesAfter) close(ctx, estateId, w, ccy);
        }
      },
    },
  ],
  participants: [
    {
      partyKind: ESTATE,
      orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
        const opened = view.lastOwn('estate.opened');
        if (!opened.some) return [];
        const closes = opened.value.data['closesAfter'];
        return offers(view, m, typeof closes === 'number' ? closes : view.period);
      },
    },
  ],
  families: [nothingLeftBehind(), nothingLeaksOut()],
};
