/**
 * Securities lending: title passes, the economics do not.
 *
 * @spec Securities Lending A1 Securities Lending A2 Securities Lending A3 Securities Lending A4 Securities Lending A5 Securities Lending A5.a Securities Lending A5.b Securities Lending B1 Securities Lending B2 Securities Lending B2.a Securities Lending B4 Securities Lending C1 Securities Lending C2 Securities Lending C2.a Securities Lending C4 Securities Lending D1 Securities Lending D3 Securities Lending E1 Securities Lending E2 Securities Lending E3 Equity C7 Register D5.a Law 3 Law 4 Law 8 Law 19
 *
 * THE DEFINING PROPERTY IS THAT TWO THINGS COME APART. Legal title moves to the borrower — it can
 * sell what it borrowed, and that is the entire point (A2) — while the economics stay with the
 * lender, which gets a manufactured payment equal to anything the security pays (A3). A world that
 * moved one without the other would have a stock loan in which the lender pays a fee to lose its
 * income, which is the transaction inverted.
 *
 * IT IS BUILT OUT OF THINGS THAT ALREADY EXIST. The security moves in the register like any other
 * units. The collateral is a PLEDGE, so it leaves the poster's free balance and cannot be counted as
 * available by both sides (C4, Register D5.a) — the register already refuses to move encumbered
 * units, so nothing here has to remember not to. The fee is a price and it CLEARS (A5, A5.a): scarce
 * paper is expensive to borrow and abundant paper is cheap, and neither is a number anybody wrote.
 *
 * WHAT IT UNLOCKS IS E1: NO SHORT WITHOUT A BORROW. A negative position nobody lent is an invented
 * security, and until there was a borrow market the only honest answer was to forbid the short. The
 * lendable pool is a read of who actually holds the paper and is willing (B4), and it is what caps
 * how large a short can get — a real constraint, and the reason a squeeze is possible (D2).
 */
import { clear, isCleared } from '../../clearing/solver.js';
import {
  venueId,
  type CurrencyCode,
  type InstrumentId,
  type PartyId,
  type VenueId,
} from '../../core/ids.js';
import { add, atMost, div, mul, sub, sum } from '../../core/num.js';
import { downTick, type Qty } from '../../core/tick.js';
import { none, some, type Option } from '../../core/option.js';
import { isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import type { LienId } from '../../core/ids.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

/** Law 9: one book per line, because what is being priced is the scarcity of THAT paper (A5.a). */
export const borrowVenue = (instrument: InstrumentId): VenueId =>
  venueId(`borrow:${instrument}`);

/** A1: one open loan, and everything about it is a read of the register except the fee it struck. */
export interface StockLoan {
  readonly lender: PartyId;
  readonly borrower: PartyId;
  readonly instrument: InstrumentId;
  readonly units: Qty;
  /** C1: what the borrower put up, and the lien that binds it. */
  readonly collateral: InstrumentId;
  readonly lien: LienId;
  readonly posted: Qty;
  /** A5: per period, as a fraction of what the borrowed paper is worth. It cleared (A5.a). */
  readonly fee: number;
  readonly ccy: CurrencyCode;
  readonly opened: number;
}

interface Book {
  readonly open: StockLoan[];
}

const state = (ctx: MechanismContext): Book => ctx.state<Book>('loans', () => ({ open: [] }));

/** The open loans, for anything that needs to know what is out on loan (Law 19: one writer). */
export const loansOpen = (ctx: MechanismContext): readonly StockLoan[] => state(ctx).open;

/**
 * B4, E1: THE LENDABLE POOL — who actually holds this paper and is willing to part with title for a
 * fee. It is a READ of the register and never a stored number, and it is what caps how large a short
 * can get, because a borrower cannot borrow what nobody has free.
 *
 * Willing is not a flag: a holder is willing with what it holds FREE. Units already pledged or
 * already lent out are not in the pool, which the register answers without being asked twice.
 */
export function lendable(
  ctx: MechanismContext,
  instrument: InstrumentId,
): readonly { readonly lender: PartyId; readonly units: Qty }[] {
  const out: { lender: PartyId; units: Qty }[] = [];
  for (const holder of ctx.register.holdersOf(instrument)) {
    const p = ctx.parties.get(holder);
    // B2: the lender has it sitting there. A CELL does not lend: a million households each lending
    // its own two shares is a million loans, and the per-member grid cannot carry a fraction of one
    // (Law 8, XI-15). Who lends in this world is whoever holds paper as a named party.
    if (!p.status.alive || p.representation === 'cell') continue;
    const free = downTick(ctx.register.free(holder, instrument));
    if (free <= 0) continue;
    out.push({ lender: holder, units: free });
  }
  return out;
}

/**
 * A5, A5.a, Law 3: THE FEE IS A PRICE AND IT CLEARS. Lenders offer what they hold free; borrowers
 * bid what they will pay per period per unit of value. Scarce paper is dear and abundant paper is
 * cheap, and neither is a table.
 *
 * A5.b: where the collateral is cash the same number is quoted as a REBATE on that cash — the
 * lender keeps the difference between what it earns on the cash and what it pays back. It is one
 * number seen from two sides, so there is one number here and the second form is a read of it.
 */
export function runBorrows(ctx: MechanismContext, wanted: readonly Want[]): void {
  for (const w of wanted) {
    const supply = lendable(ctx, w.instrument);
    if (supply.length === 0) {
      // D2, E3: nothing to borrow is a real answer with a consequence — the short cannot be put on.
      ctx.record(
        'borrow.none',
        [w.borrower, w.instrument],
        { borrower: String(w.borrower), instrument: String(w.instrument), wanted: w.units },
        true,
      );
      continue;
    }
    const venue = borrowVenue(w.instrument);
    if (!ctx.venues.some((v) => v.id === venue)) {
      ctx.openVenue({
        id: venue,
        name: `${String(w.instrument)} borrow`,
        clearedBy: 'securities-lending',
        unit: ctx.instruments.get(w.instrument).unit,
        ccy: w.ccy,
        key: { instrument: String(w.instrument) },
      });
    }
    for (const s of supply) {
      ctx.post(venue, { party: s.lender, side: 'sell', price: 'market', qty: s.units });
    }
    ctx.post(venue, { party: w.borrower, side: 'buy', price: w.willPay, qty: w.units });
    const outcome = clear(ctx.posted(venue), 'proRata', 'sellersCompete');
    if (!isCleared(outcome)) continue;
    for (const f of outcome.fills) {
      if (f.side !== 'sell') continue;
      const units = downTick(f.qty);
      if (units <= 0) continue;
      openLoan(ctx, { ...w, lender: f.party, units, fee: outcome.price });
    }
  }
}

/** B1: why a borrower is here — it has to deliver something it has not got, and what it will pay. */
export interface Want {
  readonly borrower: PartyId;
  readonly instrument: InstrumentId;
  readonly units: Qty;
  readonly ccy: CurrencyCode;
  /** A5: the most it will pay per period per unit of value, from its own reason for being short. */
  readonly willPay: number;
  /** C1: what it will put up, which must be worth more than what it takes away. */
  readonly collateral: InstrumentId;
  readonly haircut: number;
}

/** C1: what must be posted for a borrow of this value, at this lender's own haircut. */
const collateralFor = (worth: number, haircut: number): number =>
  mul(worth, add(1, haircut, 'the margin over what it took'), 'what the borrower must put up');

/**
 * A1, A2, C1, C4: THE TRANSACTION, and both legs of it in one numbered instruction. The security
 * goes to the borrower and TITLE GOES WITH IT; the collateral is pledged to the lender and leaves
 * the borrower's free balance, so it cannot be counted as available by both sides.
 *
 * C1: the collateral is worth MORE than what was lent, by the lender's own haircut, because the
 * lender has to be able to sell it and be whole. Collateral exactly equal to the loan, re-marked to
 * the same price, is a gap covered by nothing.
 */
function openLoan(
  ctx: MechanismContext,
  d: Want & { lender: PartyId; units: Qty; fee: number },
): void {
  if (d.lender === d.borrower) return;
  // XI-6: what it is worth is what the market printed for it, at a price a reader can look up.
  const mark = ctx.valuation.markPerUnit(d.instrument, ctx.period);
  if (mark <= 0) return;
  const worth = mul(d.units, mark, 'what the borrowed paper is worth');
  const needed = collateralFor(worth, d.haircut);
  const price = ctx.prices.latest(d.collateral, ctx.period);
  if (!price.some || price.value.price <= 0) return;
  const posted = downTick(div(needed, price.value.price, 'units of collateral'));
  if (posted <= 0 || ctx.register.free(d.borrower, d.collateral) < posted) return;
  const secures = `stockLoan:${String(d.lender)}:${String(d.borrower)}:${String(d.instrument)}`;
  const legs: Leg[] = [
    {
      // A2: legal title passes. The borrower can sell what it borrowed; that is the entire point.
      kind: 'asset',
      from: d.lender,
      to: d.borrower,
      instrument: d.instrument,
      qty: d.units,
      pricePerUnit: some(mark),
      accruedPerUnit: none(),
      fromCell: none(),
      toCell: none(),
    },
    {
      kind: 'pledge',
      pledgor: d.borrower,
      beneficiary: d.lender,
      instrument: d.collateral,
      qty: posted,
      secures,
      pledgorCell: none(),
    },
  ];
  const r = ctx.settle({
    legs,
    cause: 'transfer',
    reason: `${String(d.borrower)} borrows ${d.units} of ${String(d.instrument)} from ${String(d.lender)}`,
  });
  if (r.outcome !== 'settled') return;
  const lien = lienFor(ctx, d.borrower, d.collateral, secures);
  if (!lien.some) return;
  state(ctx).open.push({
    lender: d.lender,
    borrower: d.borrower,
    instrument: d.instrument,
    units: d.units,
    collateral: d.collateral,
    lien: lien.value,
    posted,
    fee: d.fee,
    ccy: d.ccy,
    opened: ctx.period,
  });
  ctx.record(
    'borrow.opened',
    [d.lender, d.borrower, d.instrument],
    {
      lender: String(d.lender),
      borrower: String(d.borrower),
      instrument: String(d.instrument),
      units: d.units,
      collateral: String(d.collateral),
      posted,
      fee: d.fee,
    },
    true,
  );
}

/** Register D5.a: the lien the pledge just wrote, found by what it secures. One name, one lien. */
function lienFor(
  ctx: MechanismContext,
  pledgor: PartyId,
  instrument: InstrumentId,
  secures: string,
): Option<LienId> {
  const holding = ctx.register.holding(pledgor, instrument);
  if (!holding.some) return none<LienId>();
  for (const l of holding.value.liens) if (l.reason.includes(secures)) return some(l.id);
  return none<LienId>();
}

/**
 * A3: THE MANUFACTURED PAYMENT, which is the half that makes it a stock loan. The issuer pays the
 * REGISTERED holder, which is now the borrower, and the borrower passes it on — so the lender's cash
 * flows are unchanged and it has not paid a fee to lose its income.
 *
 * It is read off what actually reached the borrower this period on that line (Law 19), never
 * recomputed from the terms: the payment that was made is the payment that is passed on, and if the
 * issuer paid nothing there is nothing to manufacture.
 */
export function manufacture(ctx: MechanismContext): void {
  for (const loan of state(ctx).open) {
    const paid = receivedOn(ctx, loan);
    if (paid <= 0) continue;
    const r = ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(loan.borrower, loan.ccy),
          to: ctx.accountOf(loan.lender, loan.ccy),
          ccy: loan.ccy,
          amount: downTick(paid),
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'corporateAction',
      reason: `${String(loan.borrower)} passes on ${String(loan.instrument)} to ${String(loan.lender)}`,
    });
    if (r.outcome !== 'settled') continue;
    ctx.record(
      'borrow.manufactured',
      [loan.lender, loan.borrower, loan.instrument],
      {
        lender: String(loan.lender),
        borrower: String(loan.borrower),
        instrument: String(loan.instrument),
        amount: paid,
      },
      true,
    );
  }
}

/** What the issuer actually paid the registered holder this period on this line, off the wire. */
function receivedOn(ctx: MechanismContext, loan: StockLoan): number {
  const amounts: number[] = [];
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled' || r.instruction.cause !== 'coupon') continue;
    if (!r.instruction.reason.includes(String(loan.instrument))) continue;
    for (const leg of r.instruction.legs) {
      // A3: the money that reached the registered holder on that line. `isMoneyLeg` asks what SHAPE
      // a leg is, which is the kernel's own dispatch and not a question about an instrument kind.
      if (!isMoneyLeg(leg) || leg.to.holder !== loan.borrower || leg.ccy !== loan.ccy) continue;
      amounts.push(leg.amount);
    }
  }
  const total = sum(amounts).value;
  // E2: it passes on what the units it BORROWED earned, not what its whole holding earned. A
  // borrower that already owned some of the line keeps its own.
  const held = ctx.register.totalQuantity(loan.borrower, loan.instrument);
  return held <= 0 ? 0 : mul(total, div(loan.units, held, 'the borrowed share of what it holds'), 'passed on');
}

/**
 * A5, C2, C2.a: THE FEE, every period, real money between two named parties — and the re-mark that
 * goes with it, because when the borrowed security rises the borrower owes more collateral.
 */
export function charge(ctx: MechanismContext): void {
  for (const loan of state(ctx).open) {
    const mark = ctx.valuation.markPerUnit(loan.instrument, ctx.period);
    if (mark <= 0) continue;
    const fee = downTick(
      mul(mul(loan.units, mark, 'what is out on loan'), loan.fee, 'what the borrow cost this period'),
    );
    if (fee <= 0) continue;
    ctx.settle({
      legs: [
        {
          kind: 'money',
          from: ctx.accountOf(loan.borrower, loan.ccy),
          to: ctx.accountOf(loan.lender, loan.ccy),
          ccy: loan.ccy,
          amount: fee,
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'transfer',
      reason: `${String(loan.borrower)} pays the borrow fee on ${String(loan.instrument)}`,
    });
  }
}

/**
 * A4, D1, D3: IT TERMINATES. The security comes back and the collateral goes back, in one
 * instruction, so there is no instant where the borrower has both.
 *
 * D1: and a borrower that cannot return it has FAILED — the lender keeps the collateral and is left
 * to buy the paper back in the market at whatever it costs. The failed return terminates, which is
 * the point: the loan does not sit open for ever against a borrower who cannot close it.
 */
export function returnLoans(ctx: MechanismContext, closing: readonly StockLoan[]): void {
  const book = state(ctx);
  for (const loan of closing) {
    const held = downTick(ctx.register.free(loan.borrower, loan.instrument));
    const back = downTick(atMost(held, loan.units, 'it returns what it borrowed'));
    const failed = back < loan.units;
    const legs: Leg[] = [
      {
        kind: 'release',
        pledgor: loan.borrower,
        beneficiary: loan.lender,
        instrument: loan.collateral,
        lien: loan.lien,
      },
    ];
    if (back > 0) {
      const mark = ctx.valuation.markPerUnit(loan.instrument, ctx.period);
      legs.unshift({
        kind: 'asset',
        from: loan.borrower,
        to: loan.lender,
        instrument: loan.instrument,
        qty: back,
        pricePerUnit: mark > 0 ? some(mark) : none(),
        accruedPerUnit: none(),
        fromCell: none(),
        toCell: none(),
      });
    }
    if (failed) {
      // D1: the collateral is the lender's now. It is released to the borrower only when the paper
      // comes back, and it did not — so the lender keeps it and buys the line back itself.
      legs.shift();
      legs.length = 0;
      if (back > 0) {
        legs.push({
          kind: 'asset',
          from: loan.borrower,
          to: loan.lender,
          instrument: loan.instrument,
          qty: back,
          pricePerUnit: none(),
          accruedPerUnit: none(),
          fromCell: none(),
          toCell: none(),
        });
      }
    }
    const r = ctx.settle({
      legs,
      cause: failed ? 'default' : 'transfer',
      reason: `${String(loan.borrower)} returns ${back} of ${String(loan.instrument)}`,
    });
    if (r.outcome !== 'settled') continue;
    const at = book.open.indexOf(loan);
    if (at >= 0) book.open.splice(at, 1);
    ctx.record(
      failed ? 'borrow.failed' : 'borrow.returned',
      [loan.lender, loan.borrower, loan.instrument],
      {
        lender: String(loan.lender),
        borrower: String(loan.borrower),
        instrument: String(loan.instrument),
        returned: back,
        owed: loan.units,
      },
      true,
    );
  }
}

/**
 * E1, E2, B4: the three things that must be true while paper is out on loan, MEASURED and never
 * enforced. Each of them breaks silently, which is why the audit is where they live.
 */
function borrows(): Family {
  return {
    name: 'ownership',
    contributor: 'securities-lending',
    spec: 'Securities Lending B4 Securities Lending E1 Securities Lending E2 Equity C7',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live) continue;
        const holders = view.register.holdersOf(i.id);
        if (holders.length === 0) continue;
        for (const h of holders) {
          const units: number = view.register.totalQuantity(h, i.id);
          if (units >= 0) continue;
          // E1: a negative position nobody lent is an invented security. A short is a BORROW and
          // the borrowed units are in somebody's name; there is no other way to be short here.
          out.push({
            family: 'ownership',
            spec: 'Securities Lending E1',
            owner: i.id,
            size: -units,
            unit: i.unit,
            period: view.period,
            message: `${String(h)} is short ${-units} of ${i.id} and nobody lent it`,
          });
        }
      }
      return out;
    },
  };
}

export function securitiesLending(): SystemModule {
  return {
    id: 'securities-lending',
    spec: 'Securities Lending',
    requires: ['equity'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'borrow.economics',
        spec: 'Securities Lending A3 Securities Lending A5 Securities Lending C2',
        // A3: after the issuer has paid the registered holder, because what is passed on is what
        // arrived. A phase that manufactured a payment before the payment existed would be
        // inventing the lender's income rather than passing it through (Law 19).
        anchor: { after: 'corporateActions' },
        cycle: 'anchor',
        run: (ctx: MechanismContext): void => {
          manufacture(ctx);
          charge(ctx);
        },
      },
    ],
    participants: [],
    families: [borrows()],
  };
}

/** B1, C1: what a party will pay to borrow, from its own reason for needing the paper. */
export function wantsToBorrow(
  view: ParticipantView,
  instrument: InstrumentId,
  units: Qty,
  ccy: CurrencyCode,
  collateral: InstrumentId,
  haircut: number,
): Option<Want> {
  if (units <= 0) return none<Want>();
  const equity = view.equity();
  const owed = view.owedIn(ccy);
  if (equity <= 0) return none<Want>();
  /**
   * A5, Law 3: THE MOST IT WILL PAY, per period per unit of value, and it comes out of its own
   * position rather than a table: what its own money costs it is what a period of anything costs
   * it, and it will not pay more for the use of somebody else's paper than for the use of money.
   */
  return some({
    borrower: view.self.id,
    instrument,
    units,
    ccy,
    willPay: div(owed, add(owed, equity, 'what funds it'), 'what a period of its own money costs'),
    collateral,
    haircut,
  });
}

/** A5.b: the same fee seen from the cash side — what the lender pays back on cash it was given. */
export const rebateOf = (fee: number, earns: number): number =>
  sub(earns, fee, 'what it hands back out of what the cash earned');

/** B4: how large a short in this line could get — a read of the pool, and a real constraint. */
export const poolOf = (supply: readonly { readonly units: Qty }[]): number =>
  sum(supply.map((s) => s.units)).value;
