/**
 * A bank fails, and its book goes somewhere. This is where.
 *
 * @spec Banks Capital C1 Banks Capital C1.a Banks Capital C3 Banks Capital C3.a Banks Capital C3.b Banks Capital D1 Banks Capital D1.a Banks Capital D2 Banks Capital D2.a Banks Capital D3 Banks Capital D3.a Banks Capital D3.b Banks Capital D4 Banks Capital D5 Banks Capital D6 Banks Capital E2 Banks Capital E3 Banks Funding A1.a Money A1 Money E4 XI-3 XI-8 Register F2 Law 5
 *
 * C3.b IS THE WHOLE REASON THIS EXISTS AND IS NOT THE ESTATE. A bank that stops being a going
 * concern is not a firm being wound up: its liabilities are the money its depositors pay each other
 * with, and an estate cannot owe them, because an estate does not issue money (Money A1). A bank
 * whose deposits went to an estate would be a world where a third of the money stopped working on
 * the day one bank did — and the audit says exactly that, in the two lines it reports about a dead
 * bank's own money line and the paper still sitting in its account.
 *
 * So the book MOVES TO ANOTHER BANK, and the four things that has to be are the four D says:
 *
 *   - **D1, A VALUATION.** What its assets are actually worth at the marks a market made, against
 *     what it owes. The difference is THE HOLE, and it is a number this reports rather than a number
 *     anything chooses. D1.a's condition is met: a loan in this world can already be worth less than
 *     its face (worklist 5), so there is something to value.
 *   - **D2, THE HIERARCHY.** The hole is borne in order. A bank's own equity is the residual and is
 *     therefore already gone — the hole IS its negative equity, which is A2.a's "absorbs first and
 *     fully" arriving as arithmetic. Then the uninsured — depositors above the limit and the banks
 *     and funds that lent it money — pari passu, by a haircut written into their own balances. Then
 *     the insurer, for what it guaranteed. Then, and only then, the public purse.
 *     A2.b's SUBORDINATED LAYER IS NOT HERE and its absence is the ladder being one short at the
 *     top: nothing this world issues ranks between equity and a depositor, so an uninsured depositor
 *     takes what a subordinated holder should have. That is worklist 11's own raising step and it is
 *     named here rather than papered over.
 *   - **D3, AN ACQUIRER.** Every surviving bank values the book from ITS OWN view and says what it
 *     will pay for it — a number that is usually negative, because a book with a hole in it is
 *     something you have to be PAID to take. It can decline (D3.b), and a bank declines when taking
 *     the book would leave its own equity underwater, which is a real reason and its own.
 *   - **D4, D5, THE INSURER AND THE PURSE.** What the haircut cannot reach, the insurer pays out of
 *     a fund the banks themselves paid premiums into; what the fund cannot cover, the treasury pays,
 *     and that is a fiscal cost with a payer standing where D5 says it stands — last.
 *
 * D6: nothing vanishes. The acquirer is NAMED AS THE SUCCESSOR, so every loan the failed bank made,
 * every row it borrowed on and every lien it gave re-seats on somebody who exists (Register F2) —
 * and the depositors' accounts are re-issued by the acquirer so the money keeps working.
 *
 * E3: and it conserves. What the acquirer was paid, what the insurer paid, what the purse paid and
 * what holders lost sum to the hole, and the audit family says so rather than this function
 * asserting it.
 */
import type { CurrencyCode, InstrumentId, ParamId, PartyId } from '../../core/ids.js';
import { currencyUnit, moneyInstrumentId, paramId } from '../../core/ids.js';
import { add, div, mul, sub, sum } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import type { CellSide } from '../../ledger/instruction.js';
import { cellSide } from '../../ledger/settlement.js';
import { weightOf } from '../../parties/party.js';
import { BANK, TREASURY } from '../../registry/profiles.js';
import type { Family, Violation } from '../../audit/audit.js';
import type { MechanismContext } from '../../world/context.js';
import { failedWhy } from '../../world/failure.js';
import { MM_PARAMS } from './data.js';
import { insuredAt, uninsuredAt } from './deposits.js';
import { INSURER } from './insurer.js';
import type { Instrument } from '../../register/instruments.js';
import { isRow } from './rows.js';

/** D1: what the failed bank has, what it owes, and the difference between them. */
export interface Valuation {
  /** D1: its assets at the marks a market made, not at what its book said. */
  readonly assets: number;
  /** What it owes: the deposits it issued, and every row it borrowed on. */
  readonly deposits: number;
  readonly borrowings: number;
  /** D1.a: the difference. Positive is a hole; a bank that failed on cash alone can have none. */
  readonly hole: number;
  /** A1.a, D4: the part of the deposits this world guarantees, per member (XI-15). */
  readonly insured: number;
}

/** D3.b: what one bank will pay for the book, from its own view, or why it will not take it. */
interface Bid {
  readonly bank: PartyId;
  /** What it will pay. Negative is what it must BE paid, which is the usual answer. */
  readonly pays: number;
  readonly declined: boolean;
  readonly why: string;
}

/**
 * D1: THE VALUATION. Assets at the marks, liabilities at what the holders carry them at, and the
 * hole is the difference. Nothing here is a formula discount off book (XI-8's rule, applied to a
 * bank): what a thing is worth is what its market last said, and a line nobody has priced is worth
 * what its holder carries it at, which is the same answer read from the other side (Register B3).
 */
export function valueBook(ctx: MechanismContext, bank: PartyId, ccy: CurrencyCode): Valuation {
  const own = moneyInstrumentId(bank, ccy);
  const held: number[] = [];
  for (const h of ctx.register.holdingsOf(bank)) {
    if (h.instrument === own) continue;
    held.push(ctx.valuation.valueOfLots(h.instrument, h.lots, ctx.period));
  }
  const limit = ctx.params.amount(MM_PARAMS.insuranceLimit, currencyUnit(ccy));
  const deposits: number[] = [];
  const insured: number[] = [];
  for (const holder of ctx.register.holdersOf(own)) {
    if (holder === bank) continue;
    const p = ctx.parties.get(holder);
    deposits.push(mul(ctx.register.quantity(holder, own), weightOf(p), 'what it owes this holder'));
    insured.push(insuredAt(ctx, bank, holder, ccy, limit));
  }
  const borrowings: number[] = [];
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some || i.issuer.value !== bank || i.id === own) continue;
    // Register B3: a liability is worth what the party on the other side of it carries it at. The
    // holders are named and there are few of them, so this is a read of their books rather than a
    // second valuation of the same claim (Law 4).
    for (const holder of ctx.register.holdersOf(i.id)) {
      const worth = ctx.valuation.worthOf(holder, i.id, ctx.period);
      if (!worth.some) continue;
      borrowings.push(
        mul(worth.value.value, weightOf(ctx.parties.get(holder)), 'what it owes on this row'),
      );
    }
  }
  const assets = sum(held).value;
  const owes = add(sum(deposits).value, sum(borrowings).value, 'what it owes');
  return {
    assets,
    deposits: sum(deposits).value,
    borrowings: sum(borrowings).value,
    hole: sub(owes, assets, 'the hole'),
    insured: sum(insured).value,
  };
}

/**
 * D3, D3.b: what this bank will pay for the book. It is ITS OWN NUMBER: the assets are worth what
 * its own marks say, it will owe what the book owes, and it wants its own required return on the
 * capital taking it would tie up. A bid is usually NEGATIVE — a book with a hole is something you
 * are paid to take — and it DECLINES when taking it at that price would leave its own equity
 * underwater, which is a bank refusing rather than a rule forbidding (D3.b: an assigned acquirer
 * whose bid is the estate's own value by construction is not choosing).
 */
function bidFor(ctx: MechanismContext, bank: PartyId, v: Valuation): Bid {
  const view = ctx.participant(bank);
  const required = ctx.params.get(bankParamOf(bank));
  // What the trouble is worth to it: its own required return on the capital the book consumes for
  // one period. A bank that wants more on its capital bids lower for the same book.
  const wants = mul(v.assets, required, 'what it wants for taking the book on');
  const pays = sub(sub(v.assets, v.deposits, 'assets over deposits'), v.borrowings, 'and its rows');
  const bid = sub(pays, wants, 'what it will pay');
  // C1: it takes the hole onto its own balance sheet before it is paid for it. A bank whose own
  // equity would not stand that is not an acquirer, and saying so is the whole of D3.b.
  const equity = view.equity();
  const declined = equity <= 0 || add(equity, bid, 'what it would be left with') <= 0;
  return {
    bank,
    pays: bid,
    declined,
    why: declined ? 'taking the book would leave its own equity underwater' : 'it will take the book',
  };
}

/** Banks Lending C1.b: each bank's own required return on capital, declared under its own name. */
const bankParamOf = (bank: PartyId): ParamId => paramId(`bank.returnOnCapital.${bank}`);

/**
 * C3, D: the resolution. Returns whether the book was placed — a world with no other bank left has
 * nowhere to put it, and that is a terminal state this says out loud rather than pretending away.
 */
export function resolve(ctx: MechanismContext, bank: PartyId, why: string): boolean {
  const p = ctx.parties.get(bank);
  const ccy = ctx.registry.region(p.region).ccy;
  const v = valueBook(ctx, bank, ccy);
  // C1.a: WHICH TRIGGER FIRED is part of the record. A bank that could not pay and a bank whose
  // liabilities are past its assets are two different failures with two different remedies.
  ctx.record(
    'bank.resolution.valued',
    [bank],
    {
      bank,
      ccy,
      why,
      assets: v.assets,
      deposits: v.deposits,
      borrowings: v.borrowings,
      insured: v.insured,
      hole: v.hole,
    },
    true,
  );
  const bids: Bid[] = [];
  for (const other of ctx.parties.ofKind(BANK)) {
    if (other.id === bank || !other.status.alive) continue;
    bids.push(bidFor(ctx, other.id, v));
  }
  for (const b of bids) {
    ctx.record(
      'bank.resolution.bid',
      [bank, b.bank],
      { failed: bank, bidder: b.bank, pays: b.pays, declined: b.declined, why: b.why },
      true,
    );
  }
  // D3.b: the highest bid wins — the one that will pay the most, or ask to be paid the least. A
  // bank that declined is not a bidder, and a resolution with no bidder at all falls to the purse.
  const taking = bids.filter((b) => !b.declined).sort((a, b) => b.pays - a.pays);
  const winner = taking[0] ?? bids.sort((a, b) => b.pays - a.pays)[0];
  if (winner === undefined) {
    ctx.record('bank.resolution.noBank', [bank], { bank, hole: v.hole }, true);
    return false;
  }
  const acquirer = winner.bank;
  // D2: the hierarchy, and it is written into balances before anything moves. What the acquirer
  // takes on is what is left after the loss has landed on whoever ranks to bear it.
  const borne = allocate(ctx, bank, acquirer, ccy, v);
  moveBook(ctx, bank, acquirer, ccy);
  ctx.cease(bank, acquirer);
  ctx.record(
    'bank.resolution.done',
    [bank, acquirer],
    {
      failed: bank,
      acquirer,
      ccy,
      hole: v.hole,
      // E3: the four numbers that must sum to the hole. Nothing here checks that they do — the
      // audit family does, because a mechanism that checked its own arithmetic would be marking
      // its own homework (Law 11).
      holdersLost: borne.holders,
      insurerPaid: borne.insurer,
      pursePaid: borne.purse,
      equityAbsorbed: borne.equity,
    },
    true,
  );
  return true;
}

interface Borne {
  /** A2.c, D2: what uninsured depositors and lenders lost, written into their own balances. */
  readonly holders: number;
  /** D4: what the insurance fund paid for what it guaranteed. */
  readonly insurer: number;
  /** D5: the last resort, and it has a payer. */
  readonly purse: number;
  /** A2.a: what the bank's own equity absorbed first, which is all of it. */
  readonly equity: number;
}

/**
 * D2, D2.a, D4, D5: WHO BEARS THE HOLE, in order, and every step of it is a real movement of real
 * money to or from a named party.
 *
 * The bank's own equity has already absorbed everything it had — the hole IS its negative equity
 * (A2.a). Below that, the uninsured rank pari passu and take a haircut written into their balances
 * (A2.c); a haircut is not a fee, it is a claim that was worth less than its face, and D2.a is what
 * makes it right: in a liquidation they would have got exactly the same share of the same assets.
 * What the haircut cannot reach — the insured part — the insurer pays for out of the fund the banks
 * paid into (D4), and what the fund cannot cover the treasury pays (D5).
 */
function allocate(
  ctx: MechanismContext,
  bank: PartyId,
  acquirer: PartyId,
  ccy: CurrencyCode,
  v: Valuation,
): Borne {
  const equity = ctx.participant(bank).equity();
  if (v.hole <= 0) return { holders: 0, insurer: 0, purse: 0, equity };
  const limit = ctx.params.amount(MM_PARAMS.insuranceLimit, currencyUnit(ccy));
  const own = moneyInstrumentId(bank, ccy);
  // A2.c, D2: EVERYTHING THAT RANKS WITH AN UNINSURED DEPOSITOR, by name and by size, and they rank
  // together. A depositor above the limit and a bank that lent it a month of money are the same
  // creditor to this book — neither is secured, neither is subordinated, and nothing in this world
  // ranks between them — so they take the same proportion of the same loss. Pari passu is not a
  // rule applied afterwards: it is what "one pool, one share each" means.
  const exposed: { holder: PartyId; owed: number; rank: number; row?: InstrumentId }[] = [];
  const rankOf = (id: InstrumentId): number =>
    ctx.registry.instrumentKind(ctx.instruments.get(id).kind).ranking(ctx.instruments.get(id))
      .seniority;
  for (const holder of ctx.register.holdersOf(own)) {
    if (holder === bank) continue;
    const p = ctx.parties.get(holder);
    const owed = mul(uninsuredAt(ctx, bank, holder, ccy, limit), weightOf(p), 'uninsured');
    if (owed > 0) exposed.push({ holder, owed, rank: rankOf(own) });
  }
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some || i.issuer.value !== bank || i.id === own) continue;
    // A2.c, Appendix B: A SECURED LENDER IS NOT IN THIS POOL FOR WHAT ITS PAPER COVERS. It has the
    // collateral in its hand — the acquirer takes the book WITH the liens on it — so what it stands
    // to lose is the part its own security does not reach, and only that part ranks with an
    // uninsured depositor. Counting the covered part here would take the same collateral twice: once
    // as an asset of the book, which is where the hole is measured, and again as a loss to the
    // lender who is holding it.
    const secured = coveredBy(ctx, i);
    for (const holder of ctx.register.holdersOf(i.id)) {
      if (holder === bank) continue;
      const worth = ctx.valuation.worthOf(holder, i.id, ctx.period);
      if (!worth.some || worth.value.value <= 0) continue;
      const claim = mul(worth.value.value, weightOf(ctx.parties.get(holder)), 'what it is owed');
      const owed = sub(claim, secured, 'what its security does not reach');
      if (owed > 0) exposed.push({ holder, owed, rank: rankOf(i.id), row: i.id });
    }
  }
  // D2, A2, N13.a: IN RANK ORDER, most junior first, and pari passu WITHIN a rank. The order is
  // not a list here: it is the number each claim's own kind states about where it stands (Law 15),
  // so a layer added to this world takes its place in the queue by declaring one. Equity has
  // already absorbed everything it had (A2.a); what follows is the subordinated layer, whose
  // holders were paid to be here (A2.b), and only when that is gone does anything reach the senior
  // creditors and the depositors (A2.c).
  const ranks = [...new Set(exposed.map((e) => e.rank))].sort((a, b) => b - a);
  let holders = 0;
  let left = v.hole;
  for (const rank of ranks) {
    if (left <= 0) break;
    const layer = exposed.filter((e) => e.rank === rank);
    const pool = sum(layer.map((e) => e.owed)).value;
    if (pool <= 0) continue;
    const cut = left > pool ? pool : left;
    for (const e of layer) {
      // Clearing C3: pro rata, in whole pieces, and the odd piece has a named holder.
      const share = ctx.registry.payable(ccy, mul(div(e.owed, pool, 'its share'), cut, 'its loss'));
      if (share <= 0) continue;
      const lost = e.row === undefined
        ? writeDownDeposit(ctx, bank, e.holder, ccy, share)
        : writeDownRow(ctx, bank, e.holder, e.row, share, e.owed);
      holders = add(holders, lost, 'what holders lost');
      // E2, A2.c: a loss that landed on a name, said out loud. Nobody is written down quietly.
      ctx.record(
        'bank.resolution.writtenDown',
        [bank, e.holder],
        { failed: bank, holder: e.holder, claim: e.row ?? own, owed: e.owed, lost, rank },
        true,
      );
    }
    left = sub(v.hole, holders, 'what the hole still is');
  }
  // D4, D5: what is left of the hole is what the guarantee is FOR. The insurer pays what its fund
  // holds and the treasury pays the rest, and both of them pay the acquirer — because the acquirer
  // is the one about to owe the depositors the money nobody took from them.
  const unmet = sub(v.hole, holders, 'what the guarantee must meet');
  if (unmet <= 0) return { holders, insurer: 0, purse: 0, equity };
  const insurer = payFrom(ctx, INSURER, acquirer, ccy, unmet, `${bank} resolution: the guarantee`);
  const still = sub(unmet, insurer, 'what the fund could not meet');
  const purse =
    still <= 0
      ? 0
      : payFrom(
          ctx,
          treasuryOf(ctx, bank),
          acquirer,
          ccy,
          still,
          `${bank} resolution: the public purse, last`,
        );
  return { holders, insurer, purse, equity };
}

/**
 * B3.c, Register D5: what the paper behind a row is worth at the marks in force — the lender's
 * security, valued the way everything else in the resolution is valued (D1). A row with no
 * collateral is covered by nothing, which is what makes it unsecured.
 */
function coveredBy(ctx: MechanismContext, i: Instrument): number {
  if (!isRow(i.terms)) return 0;
  const terms: number[] = [];
  for (const c of i.terms.collateral) {
    const mark = ctx.valuation.markPerUnit(c.instrument, ctx.period);
    terms.push(mul(c.qty, mark, 'what the security is worth'));
  }
  return sum(terms).value;
}

/** A2.c: a depositor's loss, written into the balance it thought it had. */
function writeDownDeposit(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  ccy: CurrencyCode,
  share: number,
): number {
  const p = ctx.parties.get(holder);
  const perMember = ctx.registry.payable(ccy, div(share, weightOf(p), 'per member'));
  if (perMember <= 0) return 0;
  const total = mul(perMember, weightOf(p), 'written off');
  // XI-15: a cell's side is denominated per member, whatever its weight — every member of it is a
  // real holder with a real account, and one of them is not an exception to that.
  const per = cellSide(p, perMember);
  const r = ctx.settle({
    legs: [
      {
        kind: 'money',
        from: { holder, issuer: bank },
        to: { holder: bank, issuer: bank },
        ccy,
        amount: total,
        fromCell: per === undefined ? none<CellSide>() : some(per),
        toCell: none(),
      },
    ],
    // A resolution is a corporate action and this is part of one: a claim that was worth less than
    // its face, written down in the same instruction that says whose it was.
    cause: 'corporateAction',
    reason: `${holder} is written down in ${bank}'s resolution`,
  });
  return r.outcome === 'settled' ? total : 0;
}

/**
 * A2.c: a LENDER's loss, and it is the same loss. Units of the row go back to whoever owes them at
 * nothing (Banks Lending E5), which is what a claim being worth less than its face IS — the holder
 * keeps a smaller claim rather than the same claim marked down by an opinion.
 */
function writeDownRow(
  ctx: MechanismContext,
  bank: PartyId,
  holder: PartyId,
  row: InstrumentId,
  share: number,
  owed: number,
): number {
  const p = ctx.parties.get(holder);
  const units = ctx.register.quantity(holder, row);
  if (units <= 0 || owed <= 0) return 0;
  const total = mul(units, weightOf(p), 'what it holds');
  const wiped = ctx.registry.deliverable(
    ctx.instruments.get(row).unit,
    mul(total, div(share, owed, 'the share of it that is lost'), 'units written off'),
  );
  if (wiped <= 0) return 0;
  const perMember = ctx.registry.deliverable(
    ctx.instruments.get(row).unit,
    div(wiped, weightOf(p), 'per member'),
  );
  if (perMember <= 0) return 0;
  const moved = mul(perMember, weightOf(p), 'units written off');
  const per = cellSide(p, perMember);
  const r = ctx.settle({
    legs: [
      {
        kind: 'asset',
        from: holder,
        to: bank,
        instrument: row,
        qty: moved,
        pricePerUnit: some(0),
        accruedPerUnit: none(),
        fromCell: per === undefined ? none<CellSide>() : some(per),
        toCell: none(),
      },
    ],
    cause: 'corporateAction',
    reason: `${holder} is written down on ${row} in ${bank}'s resolution`,
  });
  return r.outcome === 'settled' ? mul(moved, div(owed, total, 'what a unit was worth'), 'lost') : 0;
}

/** The sovereign of the failed bank's own region: D5's payer has a name. */
function treasuryOf(ctx: MechanismContext, bank: PartyId): PartyId {
  const region = ctx.parties.get(bank).region;
  const t = ctx.parties.ofKind(TREASURY).find((x) => x.region === region && x.status.alive);
  if (t === undefined) return INSURER;
  return t.id;
}

/** As much of `wanted` as this payer actually has: a payer that is empty pays what it holds. */
function payFrom(
  ctx: MechanismContext,
  from: PartyId,
  to: PartyId,
  ccy: CurrencyCode,
  wanted: number,
  reason: string,
): number {
  if (!ctx.parties.has(from)) return 0;
  const payer = ctx.parties.get(from);
  if (!payer.status.alive) return 0;
  const has = ctx.register.quantity(from, moneyInstrumentId(ctx.accountOf(from, ccy).issuer, ccy));
  const amount = ctx.registry.payable(ccy, wanted > has ? has : wanted);
  if (amount <= 0) return 0;
  const r = ctx.settle({
    legs: [
      {
        kind: 'money',
        from: ctx.accountOf(from, ccy),
        to: ctx.accountOf(to, ccy),
        ccy,
        amount,
        fromCell: none(),
        toCell: none(),
      },
    ],
    cause: 'transfer',
    reason,
  });
  return r.outcome === 'settled' ? amount : 0;
}

/**
 * D3.a, D6: the acquirer takes the assets AND the liabilities, and every one of them moves over the
 * wire. Nothing is re-created out of nowhere and nothing is left behind: what the failed bank held
 * lands in the acquirer's register at what it was carried at, what it owed becomes the acquirer's
 * to owe, and every depositor's account is re-issued by the acquirer so that the money keeps
 * working the morning after (C3.b).
 */
function moveBook(ctx: MechanismContext, bank: PartyId, acquirer: PartyId, ccy: CurrencyCode): void {
  const own = moneyInstrumentId(bank, ccy);
  // The rows it borrowed on, and the paper it issued: the obligation moves, the holders do not.
  //
  // Except where the ACQUIRER IS THE HOLDER. A row it lent on becomes, the moment it assumes the
  // borrowing, a claim it has on itself — which is not a claim (Register B3), and which nothing
  // could ever redeem because a payment needs two named sides. So its own claim ends here instead:
  // the units go back to whoever owed them, at nothing, which is what a claim being extinguished IS
  // (Banks Lending E5). Its share of the loss it already took with everybody else's above; what it
  // gives up here is the rest of a claim on a bank it is about to become.
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !i.issuer.some || i.issuer.value !== bank || i.id === own) continue;
    const mine = ctx.register.quantity(acquirer, i.id);
    if (mine > 0) {
      const units = mul(mine, weightOf(ctx.parties.get(acquirer)), 'its own claim');
      const gone = ctx.settle({
        legs: [
          {
            kind: 'asset',
            from: acquirer,
            to: bank,
            instrument: i.id,
            qty: units,
            pricePerUnit: some(0),
            accruedPerUnit: none(),
            fromCell: none(),
            toCell: none(),
          },
        ],
        cause: 'corporateAction',
        reason: `${acquirer} cannot be its own creditor: its claim on ${i.id} ends`,
      });
      ctx.record(
        'bank.resolution.ownClaim',
        [bank, acquirer, i.id],
        { failed: bank, acquirer, row: i.id, units, ended: gone.outcome === 'settled' },
        true,
      );
    }
    if (ctx.instruments.get(i.id).issued <= 0) continue;
    ctx.settle({
      legs: [{ kind: 'assume', from: bank, to: acquirer, instrument: i.id }],
      cause: 'corporateAction',
      reason: `${acquirer} assumes ${i.id} from ${bank}`,
    });
  }
  // What it held. A pledged holding cannot simply move — the units are bound to somebody (Register
  // D5) and settlement checks that BEFORE it applies anything, so the lien has to have ended before
  // the move is even valid. It is therefore three instructions and not one: the lien ends, the
  // paper moves, and the same lien is given again by the acquirer to the same beneficiary for the
  // same reason. Each of them is complete and two-sided on its own terms, and nothing else runs
  // between them — they are consecutive settles inside one phase — so at no point that anything
  // else in this world can observe is the collateral both free and somebody else's.
  for (const h of [...ctx.register.holdingsOf(bank)]) {
    if (h.instrument === own) continue;
    const units = ctx.register.quantity(bank, h.instrument);
    if (units <= 0) continue;
    const i = ctx.instruments.get(h.instrument);
    const money = ctx.registry.instrumentKind(i.kind).pricing === 'money';
    const liens = [...h.liens];
    for (const lien of liens) {
      ctx.settle({
        legs: [
          {
            kind: 'release',
            pledgor: bank,
            // Register F2: whoever this was pledged to may have ceased since — the claim did not
            // die with them, so the release names their successor like every other reference does.
            beneficiary: ctx.parties.resolve(lien.beneficiary).id,
            instrument: h.instrument,
            lien: lien.id,
          },
        ],
        cause: 'corporateAction',
        reason: `${bank}'s lien to ${lien.beneficiary} ends with its book`,
      });
    }
    const moved = ctx.settle({
      legs: [
        money
          ? {
              kind: 'money',
              from: { holder: bank, issuer: i.issuer.some ? i.issuer.value : bank },
              to: { holder: acquirer, issuer: i.issuer.some ? i.issuer.value : bank },
              ccy: i.ccy,
              amount: units,
              fromCell: none(),
              toCell: none(),
            }
          : {
              kind: 'asset',
              from: bank,
              to: acquirer,
              instrument: h.instrument,
              qty: units,
              // Register F2: AT WHAT THE BOOK CARRIED IT AT. Nothing is realised by the move
              // itself — the resolution is not a sale — and the acquirer's new lots are re-marked
              // by the revaluation this phase runs in front of, in the same period, so what it
              // carries and what it is worth are one number by the time anybody reads either.
              // Pricing the move at the mark instead would crystallise a gain the revaluation had
              // already booked, and hand the issuer of the paper a re-mark of the same size.
              pricePerUnit: none(),
              accruedPerUnit: none(),
              fromCell: none(),
              toCell: none(),
            },
      ],
      cause: 'corporateAction',
      reason: `${bank}'s book to ${acquirer}`,
    });
    if (moved.outcome !== 'settled') continue;
    for (const lien of liens) {
      // Register F2, Money E4: the party this paper was pledged TO may itself have ceased — a
      // second bank can fail before this one is resolved — and its claim did not die with it, so
      // the security is given to whoever succeeded it. A leg addressed to the dead party is a
      // defect in this module and settlement refuses it at the site.
      const beneficiary = ctx.parties.resolve(lien.beneficiary).id;
      // Register D5: nobody pledges to itself. A lien the ACQUIRER held over this paper is
      // satisfied by owning the paper — it has the collateral, and it has assumed the row that
      // collateral stood behind, so there is nothing left for a lien to secure and it ends here.
      // Asked after the resolution, because succeeding the beneficiary is one of the ways the
      // acquirer comes to hold it.
      if (beneficiary === acquirer) continue;
      ctx.settle({
        legs: [
          {
            kind: 'pledge',
            pledgor: acquirer,
            beneficiary,
            instrument: h.instrument,
            qty: lien.qty,
            secures: lien.reason,
            pledgorCell: none(),
          },
        ],
        cause: 'corporateAction',
        reason: `${acquirer} gives ${beneficiary} the same security ${bank} had`,
      });
    }
  }
  // Register D5, D6, F2: and the security the failed bank HELD over ANYBODY's paper moves with the
  // rows it stood behind — the beneficiary resolves to the acquirer like every other reference.
  // Except where the pledgor IS the acquirer: a party does not hold security over its own paper, so
  // that lien ends here rather than outliving both ends of the row it secured.
  //
  // OVER ANYBODY'S, and it used to be only over the acquirer's. A lien left naming the dead bank
  // is a claim held by nobody, and it cannot be ended later: when a SECOND bank fails and the
  // chain of successors brings the beneficiary round to the pledgor itself, both ends of the row
  // resolve to one party and there is no release anybody can write — Register D5 refuses a leg
  // whose two sides are the same party, so the paper stays bound for the rest of the run. That is
  // how `bank.b` came to hold 613,744 of a sovereign line bound to a row nothing was owed on, and
  // the fix is that no lien ever names a party that has ceased.
  for (const h of [...ctx.register.allHoldings()]) {
    for (const lien of [...h.liens]) {
      if (lien.beneficiary !== bank) continue;
      const released = ctx.settle({
        legs: [
          {
            kind: 'release',
            pledgor: h.holder,
            beneficiary: bank,
            instrument: h.instrument,
            lien: lien.id,
          },
        ],
        cause: 'corporateAction',
        reason: `${bank} ceases: ${lien.reason} is released to be given again to ${acquirer}`,
      });
      // The acquirer took the row, so it takes the security that stood behind it — unless the
      // pledgor IS the acquirer, and then owning the paper is what the security was for.
      if (released.outcome !== 'settled' || h.holder === acquirer) continue;
      ctx.settle({
        legs: [
          {
            kind: 'pledge',
            pledgor: h.holder,
            beneficiary: acquirer,
            instrument: h.instrument,
            qty: lien.qty,
            secures: lien.reason,
            pledgorCell: none(),
          },
        ],
        cause: 'corporateAction',
        reason: `${acquirer} succeeds ${bank} as the beneficiary of ${lien.reason}`,
      });
    }
  }
  // C3.b: THE DEPOSITS KEEP WORKING. Each holder's balance at the failed bank is extinguished and
  // the same balance is issued to it by the acquirer, in one instruction — so at no instant does
  // anybody hold nothing, and the two legs net to zero in the money family like any other pair.
  for (const holder of [...ctx.register.holdersOf(own)]) {
    if (holder === bank) continue;
    const p = ctx.parties.get(holder);
    const perMember = ctx.register.quantity(holder, own);
    if (perMember <= 0) continue;
    const weight = weightOf(p);
    const held = cellSide(p, perMember);
    const side = held === undefined ? none<CellSide>() : some(held);
    const total = mul(perMember, weight, 'the balance that moves');
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: { holder, issuer: bank },
          to: { holder: bank, issuer: bank },
          ccy,
          amount: total,
          fromCell: side,
          toCell: none(),
        },
        {
          kind: 'money',
          from: { holder: acquirer, issuer: acquirer },
          to: { holder, issuer: acquirer },
          ccy,
          amount: total,
          fromCell: none(),
          toCell: side,
        },
      ],
      cause: 'corporateAction',
      reason: `${acquirer} takes over ${holder}'s account at ${bank}`,
    });
    if (r.outcome === 'settled') ctx.moveBank(holder, acquirer, `${bank} was resolved into ${acquirer}`);
  }
  // Money B1, Money E4: and EVERYBODY WHO BANKED THERE moves, not only the ones with a balance on
  // the day. An account is (holder, issuer, currency), so a party still pointing at a bank that has
  // ceased has no account at all — the next coupon anybody owes it is addressed to a dead issuer,
  // which is a defect in whatever wrote the leg rather than something that party did.
  for (const p of ctx.parties.all()) {
    if (!p.status.alive || p.id === bank || p.bank !== bank) continue;
    ctx.moveBank(p.id, acquirer, `${bank} was resolved into ${acquirer}`);
  }
}

/** XI-3, C3.a: the banks that have failed this period, asked the same way every kind is asked. */
export function failedBanks(ctx: MechanismContext): { bank: PartyId; why: string }[] {
  const out: { bank: PartyId; why: string }[] = [];
  for (const b of ctx.parties.ofKind(BANK)) {
    if (!b.status.alive) continue;
    const why = failedWhy(ctx, ctx.participant(b.id));
    if (why !== undefined) out.push({ bank: b.id, why });
  }
  return out;
}

/**
 * D6, E3, Law 2: A RESOLUTION LEAVES NOTHING BEHIND. A bank that has ceased holds nothing, owes
 * nothing that anybody still holds, and nobody banks there — because every one of those would be a
 * claim on, or a position of, a party that has stopped existing: a residual with no holder, which
 * Law 2 calls a defect whether or not anything trips over it later.
 *
 * It is a family rather than a throw because it is a statement about the WORLD after the fact and
 * not about one instruction (Audit A1), and because what it would report is a size — how much was
 * left behind — which is the shape a finding has.
 */
export function nothingLeftBehind(): Family {
  return {
    name: 'names',
    contributor: 'money-market',
    spec: 'Banks Capital D6 Banks Capital E3 Register F2 Law 2',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const say = (owner: string, size: number, unit: string, message: string): void => {
        out.push({ family: 'names', spec: 'Banks Capital D6', owner, size, unit, period: view.period, message });
      };
      for (const p of view.parties.all()) {
        if (p.status.alive || view.registry.partyKind(p.kind).moneyIssuer === null) continue;
        for (const h of view.register.holdingsOf(p.id)) {
          const qty = view.register.quantity(p.id, h.instrument);
          if (qty !== 0) say(String(p.id), qty, 'units', `${p.id} has ceased and still holds ${qty} of ${h.instrument}`);
        }
        for (const i of view.instruments.all()) {
          if (!i.status.live || !i.issuer.some || i.issuer.value !== p.id) continue;
          const held = view.register.heldTotal(i.id).value;
          if (held !== 0) say(String(i.id), held, 'units', `${i.id} is issued by ${p.id}, which ceased, and ${held} of it is still held`);
        }
        for (const q of view.parties.all()) {
          if (!q.status.alive || q.bank !== p.id) continue;
          say(String(q.id), 1, 'account', `${q.id} still banks at ${p.id}, which ceased`);
        }
      }
      return out;
    },
  };
}

export type { Bid };
