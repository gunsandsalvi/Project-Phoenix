/**
 * The quarterly statement: every section a reader of one expects, each line a READ.
 *
 * @spec Reporting A1 Reporting A2 Reporting A2.a Reporting A3 Reporting B1 Reporting B4 Reporting G2 Reporting G5 Corporate Credit A3 Corporate Credit A3.a Corporate Credit A3.b Corporate Credit E9 Currency D2 Clearing D4 Observer A3 Law 4 Law 8 Law 9 Law 19
 *
 * A2 IS THE WHOLE OF IT: nothing here composes a number. The income lines are the equity account's
 * own entries over the span, each attributed by what the legs of its instruction WERE — the receipt
 * the wire wrote on the money, a create or a destroy leg, a write-off at nothing, a mark. The cash
 * statement is the wire's own money legs. The balance sheet is the register at the close, at the
 * marks in force, with the fair-value level read off each print's provenance. The debt schedule is
 * the instruments the company issued and their own cash flows. The leases and commitments are the
 * agreements store; the derivatives the contract store; the deals the control events; the employees
 * the employment register; the commentary the party's own outlooks. A line this world has no
 * mechanism for is not here — `docs/IMPLEMENTATION.md` 17.0a says which item builds it.
 *
 * ONE PASS over the span's settled records per company (Law 18): every line that is a sum of legs
 * is collected in the same walk.
 */
import { period as asPeriod, type Period } from '../../calendar/calendar.js';
import { assertNever } from '../../core/assert.js';
import type { Civil } from '../../calendar/civil.js';
import { moneyInstrumentId, type PartyId } from '../../core/ids.js';
import { negQty } from '../../core/tick.js';
import { asCash, type Cash } from '../../core/measure.js';
import {
  isAssetLeg,
  isCreateLeg,
  isDestroyLeg,
  isMoneyLeg,
  type Leg,
  type Receipt,
} from '../../ledger/instruction.js';
import { tradedIn, wasTraded } from '../../prices/price-store.js';
import type { Instrument } from '../../register/instruments.js';
import { balanceSheet } from '../../audit/families/accounts.js';
import { isPlantTerms, isLeaseTerms } from '../../registry/physical.js';
import { isEmployment } from '../../register/employment.js';
import { capitalPublished, liquidHeld, couldLeave, bufferHeld } from '../../registry/banking.js';
import {
  BALANCE_LINES,
  CASH_LINES,
  type CashLine,
  type CashCounterpartyNote,
  type CommitmentNote,
  type CurrencyExposure,
  dealsInvolving,
  dividendsDeclaredIn,
  type DebtNote,
  type DerivativeNote,
  ebitdaOf,
  type EmployeeNote,
  EQUITY_LINES,
  type GeographySegment,
  type HoldingNote,
  INCOME_LINES,
  type IncomeLine,
  type InventoryNote,
  type LeaseNote,
  OCI_LINES,
  FAIR_VALUE,
  type FairValueLevel,
  type PlantNote,
  type ShareNote,
  type StatementRecord,
  SUMMARY_LINES,
} from '../../registry/statements.js';
import type { MechanismContext } from '../../world/context.js';

const zeros = <K extends string>(keys: readonly K[]): Record<K, number> => {
  const out: Partial<Record<K, number>> = {};
  for (const k of keys) out[k] = 0;
  return out as Record<K, number>;
};

const civil = (c: Civil): string => `${String(c.y)}-${String(c.m)}-${String(c.d)}`;

/** The periods a fiscal span covers, inclusive of both ends (A3, Money G3.a). */
function periodsIn(from: Period, to: Period): Period[] {
  const out: Period[] = [];
  for (let p: number = from; p <= to; p += 1) out.push(asPeriod(p));
  return out;
}

/**
 * G2: WHAT ONE MOVEMENT OF THE EQUITY ACCOUNT WAS, read off the legs of the instruction that moved
 * it. A receipt the party was paid, a payment it made, what production and wear did, a claim written
 * off at nothing. The receipt is the classification the wire itself wrote (`Receipt`), so a report
 * cannot call a wage a sale; and an instruction whose legs said none of these is `other`, kept
 * rather than lost.
 */
function lineOf(party: PartyId, legs: readonly Leg[], cause: string): IncomeLine {
  let received: Receipt | undefined;
  let paid: Receipt | undefined;
  let created = false;
  let destroyedPlant = false;
  let destroyedGoods = false;
  for (const leg of legs) {
    if (isMoneyLeg(leg)) {
      if (leg.to.holder === party && received === undefined) received = leg.receipt;
      if (leg.from.holder === party && paid === undefined) paid = leg.receipt;
    } else if (isCreateLeg(leg) && leg.party === party) created = true;
    else if (isDestroyLeg(leg) && leg.party === party) {
      // Capital Programme A4.b: plant leaves by being scrapped; a good perishes or is consumed.
      if (leg.why === 'scrapped') destroyedPlant = true;
      else destroyedGoods = true;
    }
  }
  if (cause === 'default') return 'writeOffs';
  if (received !== undefined) {
    switch (received.of) {
      case 'sale':
        return 'revenue';
      case 'disposal':
        return 'gainsOnDisposal';
      case 'wage':
      case 'pension':
        return 'wagesReceived';
      case 'rent':
        return 'rentReceived';
      case 'interest':
        return 'interestReceived';
      case 'dividend':
        return 'dividendsReceived';
      case 'tax':
        return 'taxReceived';
      case 'claim':
        return 'claimsReceived';
      // 0i.5: A FEE AND A PREMIUM EARNED ARE REVENUE for the service rendered — a manager's
      // mandate (Fund Shares F3), an underwriter's cover (Insurers A4). They were money that
      // reached a party and fell out of this switch into `other`.
      case 'fee':
      case 'premium':
        return 'revenue';
      // 0i.5: margin arriving is a counterparty's money and not a receipt of this party's
      // (Derivative Layer D2.a); a manufactured payment stands in for the coupon or dividend the
      // lender of the stock would have had (Securities Lending A3), and this switch cannot see
      // WHICH — the leg names the line but not its kind — so it is named here rather than guessed.
      case 'returnOfCapital':
      case 'borrowing':
      case 'principal':
      case 'transfer':
      case 'contribution':
      case 'margin':
      case 'manufactured':
        return 'otherReceipts';
      default:
        return assertNever(received, 'Reporting G2');
    }
  }
  if (paid !== undefined) {
    switch (paid.of) {
      case 'wage':
      case 'pension':
        return 'wages';
      case 'rent':
        return 'rent';
      case 'interest':
        return 'interest';
      case 'tax':
        return 'tax';
      case 'claim':
        return 'claimsPaid';
      case 'dividend':
        return 'dividendsPaid';
      case 'sale':
      case 'disposal':
      case 'returnOfCapital':
      case 'borrowing':
      case 'principal':
      case 'transfer':
      case 'contribution':
      case 'fee':
      case 'premium':
      case 'margin':
      case 'manufactured':
        return 'otherPayments';
      default:
        return assertNever(paid, 'Reporting G2');
    }
  }
  if (created) return 'production';
  if (destroyedPlant) return 'wear';
  if (destroyedGoods) return 'spoilage';
  return 'other';
}

/** A money leg's place in the cash statement, from its receipt and what travelled against it. */
function cashLineOf(
  ctx: MechanismContext,
  party: PartyId,
  legs: readonly Leg[],
  cause: string,
  receipt: Receipt | undefined,
  outgoing: boolean,
): CashLine {
  if (cause === 'issuance') return outgoing ? 'securitiesBought' : 'financingIn';
  if (cause === 'coupon') return outgoing ? 'interestPaid' : 'operatingIn';
  if (cause === 'maturity') return outgoing ? 'principalRepaid' : 'securitiesSold';
  if (receipt !== undefined) {
    switch (receipt.of) {
      case 'borrowing':
        return outgoing ? 'securitiesBought' : 'financingIn';
      case 'returnOfCapital':
        return outgoing ? 'financingOut' : 'securitiesSold';
      case 'dividend':
        return outgoing ? 'financingOut' : 'operatingIn';
      case 'contribution':
        return outgoing ? 'operatingOut' : 'financingIn';
      case 'interest':
        return outgoing ? 'interestPaid' : 'operatingIn';
      case 'wage':
      case 'rent':
      case 'tax':
      case 'claim':
      case 'pension':
      case 'transfer':
        return outgoing ? 'operatingOut' : 'operatingIn';
      // 0i.5: a fee, a premium, a manufactured payment and margin are all cash of the operation
      // that produced them; principal repaid has its own line and is the mirror of a drawing.
      case 'fee':
      case 'premium':
      case 'manufactured':
      case 'margin':
        return outgoing ? 'operatingOut' : 'operatingIn';
      case 'principal':
        return outgoing ? 'principalRepaid' : 'securitiesSold';
      case 'sale':
      case 'disposal':
        break;
      default:
        return assertNever(receipt, 'Reporting B4');
    }
  }
  if (outgoing) {
    // What the money bought: plant is capex, a claim is a security, own shares are a buyback, and
    // anything else — inputs, goods, services — is operating.
    for (const leg of legs) {
      if (!isAssetLeg(leg) || leg.to !== party) continue;
      const i = ctx.instruments.get(leg.instrument);
      if (isPlantTerms(i.terms)) return 'capex';
      const profile = ctx.registry.instrumentKind(i.kind);
      if (i.issuer.some && i.issuer.value === party) return 'financingOut';
      if (profile.physical !== true && profile.pricing !== 'money') return 'securitiesBought';
    }
    return 'operatingOut';
  }
  for (const leg of legs) {
    if (!isAssetLeg(leg) || leg.from !== party) continue;
    const i = ctx.instruments.get(leg.instrument);
    const profile = ctx.registry.instrumentKind(i.kind);
    if (isPlantTerms(i.terms)) return 'securitiesSold';
    if (i.issuer.some && i.issuer.value === party) return 'financingIn';
    if (profile.physical !== true && profile.pricing !== 'money') return 'securitiesSold';
  }
  return 'operatingIn';
}


/** Currency C4: a sum of money legs in any currency, stated in the party's own at the rate in force. */
function inOwn(ctx: MechanismContext, party: PartyId, amount: Cash): number {
  return ctx.valuation.inOwnMoney(party, amount, ctx.period).pieces;
}

/**
 * The statement, prepared. Every field of `StatementRecord` is filled here and nowhere else.
 */
export function prepareStatement(
  ctx: MechanismContext,
  company: PartyId,
  quarter: string,
  span: { readonly from: Period; readonly to: Period },
): StatementRecord {
  const p = ctx.parties.get(company);
  const home = ctx.registry.currencyOf(p.region);
  const view = ctx.participant(company);
  const income = zeros(INCOME_LINES);
  const oci = zeros(OCI_LINES);
  const cashFlow = zeros(CASH_LINES);
  const equityChanges = zeros(EQUITY_LINES);
  const byProduct = new Map<string, { revenue: number; unitsSold: number; unitsMade: number; inputsConsumed: number }>();
  const byGeography = new Map<string, number>();
  const byCustomer = new Map<string, number>();
  // A2, Law 9: the direct-method statement, by named counterparty, money and reason.
  const byCounterparty = new Map<string, CashCounterpartyNote>();
  let earned = 0;
  let revaluation = 0;
  let dividendsPaid = 0;
  let boughtBack = 0;
  let boughtBackFor = 0;
  let sharesIssuedFor = 0;

  // ---- one pass over the span's settled records: the income attribution needs the legs, the
  // cash statement is the legs, the segments are the legs paired with what travelled against them.
  const causes = new Map<number, { cause: string; legs: readonly Leg[] }>();
  for (const at of periodsIn(span.from, span.to)) {
    for (const r of ctx.ledger.inPeriod(at)) {
      if (r.outcome !== 'settled') continue;
      const ins = r.instruction;
      causes.set(ins.id, { cause: ins.cause, legs: ins.legs });
      for (const leg of ins.legs) {
        if (!isMoneyLeg(leg)) continue;
        const outgoing = leg.from.holder === company;
        const incoming = leg.to.holder === company;
        if (!outgoing && !incoming) continue;
        const amount = asCash(leg.amount, leg.ccy, 'what moved on this leg');
        const own = inOwn(ctx, company, amount);
        const line = cashLineOf(ctx, company, ins.legs, ins.cause, leg.receipt, outgoing);
        cashFlow[line] += outgoing ? -own : own;
        const side = outgoing ? leg.from : leg.to;
        const counterparty = String(outgoing ? leg.to.holder : leg.from.holder);
        const instrument = String(moneyInstrumentId(side.issuer, leg.ccy));
        const key = `${counterparty}|${instrument}|${ins.cause}`;
        const at = byCounterparty.get(key);
        const moved = outgoing ? negQty(leg.amount, 'what this party paid out') : leg.amount;
        byCounterparty.set(
          key,
          at === undefined
            ? { counterparty, instrument, cause: ins.cause, amount: moved, legs: 1 }
            : { ...at, amount: at.amount + moved, legs: at.legs + 1 },
        );
        if (outgoing && leg.receipt.of === 'dividend') dividendsPaid += own;
        if (incoming && leg.receipt.of === 'sale') {
          const customer = String(leg.from.holder);
          const hadCustomer = byCustomer.get(customer);
          byCustomer.set(customer, hadCustomer === undefined ? own : hadCustomer + own);
          const region = String(ctx.parties.get(leg.from.holder).region);
          const hadRegion = byGeography.get(region);
          byGeography.set(region, hadRegion === undefined ? own : hadRegion + own);
          for (const sold of ins.legs) {
            if (!isAssetLeg(sold) || sold.from !== company) continue;
            const kind = String(ctx.instruments.get(sold.instrument).kind);
            const seg = byProduct.get(kind) ?? { revenue: 0, unitsSold: 0, unitsMade: 0, inputsConsumed: 0 };
            seg.revenue += own;
            seg.unitsSold += sold.qty;
            byProduct.set(kind, seg);
          }
        }
      }
      for (const leg of ins.legs) {
        if (isCreateLeg(leg) && leg.party === company) {
          const kind = String(ctx.instruments.get(leg.instrument).kind);
          const seg = byProduct.get(kind) ?? { revenue: 0, unitsSold: 0, unitsMade: 0, inputsConsumed: 0 };
          seg.unitsMade += leg.qty;
          byProduct.set(kind, seg);
        }
        if (isAssetLeg(leg) && leg.to === company) {
          const i = ctx.instruments.get(leg.instrument);
          if (i.issuer.some && i.issuer.value === company && ins.cause === 'trade') {
            boughtBack += leg.qty;
            for (const m of ins.legs) {
              if (isMoneyLeg(m) && m.from.holder === company)
                boughtBackFor += inOwn(ctx, company, asCash(m.amount, m.ccy, 'what the buyback cost'));
            }
          }
        }
        if (isAssetLeg(leg) && leg.from === company && ins.cause === 'issuance') {
          const i = ctx.instruments.get(leg.instrument);
          if (i.issuer.some && i.issuer.value === company && !ctx.registry.instrumentKind(i.kind).liabilityOfIssuer) {
            for (const m of ins.legs) {
              if (isMoneyLeg(m) && m.to.holder === company)
                sharesIssuedFor += inOwn(ctx, company, asCash(m.amount, m.ccy, 'what the shares raised'));
            }
          }
        }
      }
    }
  }

  // ---- the income statement: the equity account's own entries, attributed.
  for (const e of ctx.register.equityEntries(company, span.from, span.to)) {
    // 21a: an equity entry IS the whole party's money, in its own currency.
    const delta: number = e.delta;
    earned += delta;
    if (e.instruction === undefined) {
      revaluation += delta;
      // Currency D2: a mark on a thing in another money is translation; on a claim it is a gain or
      // loss unrealised; the marks name what they re-marked, so the split is a read of the cause.
      const named = [...ctx.instruments.all()].find((i) => e.cause.includes(String(i.id)));
      if (named !== undefined && named.ccy !== home) oci.fxTranslation += delta;
      else if (named !== undefined && ctx.registry.instrumentKind(named.kind).physical !== true)
        oci.unrealisedOnSecurities += delta;
      else oci.otherMarks += delta;
      if (delta < 0) income.writeDowns += delta;
      else income.writeUps += delta;
      continue;
    }
    const ins = causes.get(e.instruction);
    if (ins === undefined) {
      income.other += delta;
      continue;
    }
    income[lineOf(company, ins.legs, ins.cause)] += delta;
    if (ins.cause === 'production') {
      // What making it consumed, at the basis of what was destroyed: the equity effect of the
      // production, attributed to what it made.
      for (const leg of ins.legs) {
        if (!isCreateLeg(leg) || leg.party !== company) continue;
        const kind = String(ctx.instruments.get(leg.instrument).kind);
        const seg = byProduct.get(kind) ?? { revenue: 0, unitsSold: 0, unitsMade: 0, inputsConsumed: 0 };
        seg.inputsConsumed += -delta;
        byProduct.set(kind, seg);
      }
    }
  }

  // ---- changes in equity: the same account, before and after.
  let opening = 0;
  /**
   * G2, Money G4 (17b′.1): A SPAN THAT STARTS AT THE OPENING HAS NOTHING BEFORE IT, and asking for
   * it is asking for period −1. No quarterly report could reach here — a quarter that began before
   * the epoch is skipped — and the first caller that asked for a span from the world's first period
   * (management accounts prepared for a lender) stopped the run at the tick. The opening balance of
   * a statement about the whole of a company's life is nothing, which is what it was.
   */
  if (span.from > 0) {
    for (const e of ctx.register.equityEntries(company, asPeriod(0), asPeriod(span.from - 1))) {
      opening += e.delta;
    }
  }
  // The identity is opening + earned = closing; the three below are components OF `earned`, named
  // because a reader of a statement of changes in equity wants them by name (G2).
  equityChanges.opening = opening;
  equityChanges.earned = earned;
  equityChanges.dividendsPaid = -dividendsPaid;
  equityChanges.sharesIssued = sharesIssuedFor;
  equityChanges.sharesBoughtBack = -boughtBackFor;
  equityChanges.closing = opening + earned;

  // ---- the balance sheet at the close, and every position behind it.
  const sheet = balanceSheet(ctx, company);
  const balance = zeros(BALANCE_LINES);
  balance.assets = sheet.assets.value.pieces;
  balance.liabilities = sheet.liabilities.value.pieces;
  balance.equity = view.equity().pieces;
  const holdings: HoldingNote[] = [];
  const inventories = new Map<string, { units: number; cost: number }>();
  const plant: PlantNote[] = [];
  const currencies = new Map<string, { held: number; owed: number }>();
  const banks = new Set<string>();
  for (const h of ctx.register.holdingsOf(company)) {
    const i = ctx.instruments.get(h.instrument);
    const profile = ctx.registry.instrumentKind(i.kind);
    const units = ctx.register.quantity(company, h.instrument);
    if (units === 0) continue;
    const worth = ctx.valuation.worthOf(company, h.instrument, ctx.period);
    const carrying = worth.some ? inOwn(ctx, company, worth.value.value) : 0;
    if (profile.pricing === 'money') {
      balance.cash += carrying;
      const c = currencies.get(i.ccy) ?? { held: 0, owed: 0 };
      c.held += carrying;
      currencies.set(i.ccy, c);
      if (i.issuer.some) banks.add(String(i.issuer.value));
      continue;
    }
    if (isPlantTerms(i.terms)) {
      const cost = h.lots.reduce((acc, l) => acc + l.qty * l.basisPerUnit, 0);
      balance.plantAtCost += cost;
      balance.accumulatedWear += cost - carrying;
      plant.push({
        instrument: String(i.id),
        kind: String(i.kind),
        region: String(i.terms.region),
        inService: civil(i.terms.serviceDate),
        retires: civil(i.terms.retires),
        units,
        cost,
        carrying,
      });
      continue;
    }
    if (profile.physical === true) {
      const cost = h.lots.reduce((acc, l) => acc + l.qty * l.basisPerUnit, 0);
      balance.inventories += cost;
      const inv = inventories.get(String(i.kind)) ?? { units: 0, cost: 0 };
      inv.units += units;
      inv.cost += cost;
      inventories.set(String(i.kind), inv);
      continue;
    }
    balance.securities += carrying;
    const print = ctx.prices.latest(h.instrument, ctx.period);
    // Observer A1.a: how the mark was struck, read off the print — this period's own trade, an
    // earlier trade carried forward, or nothing a market said.
    const level: FairValueLevel = !print.some
      ? FAIR_VALUE.atCost
      : tradedIn(print.value, ctx.period)
        ? FAIR_VALUE.printedThisPeriod
        : wasTraded(print.value)
          ? FAIR_VALUE.carriedPrint
          : FAIR_VALUE.atCost;
    holdings.push({
      instrument: String(i.id),
      kind: String(i.kind),
      ccy: i.ccy,
      units,
      carrying,
      level,
      markedIn: worth.some ? worth.value.from : ctx.period,
      encumbered: ctx.register.encumbered(company, h.instrument),
    });
    if (i.ccy !== home) {
      const c = currencies.get(i.ccy) ?? { held: 0, owed: 0 };
      c.held += carrying;
      currencies.set(i.ccy, c);
    }
  }

  // ---- the debt schedule: every claim it issued that is not its money, and its own next payment.
  const debt: DebtNote[] = [];
  const on = ctx.calendar.startOf(ctx.period);
  let shareLine: Instrument | undefined;
  for (const i of ctx.instruments.issuedBy(company)) {
    if (!i.status.live) continue;
    const profile = ctx.registry.instrumentKind(i.kind);
    if (profile.pricing === 'money') continue;
    if (!profile.liabilityOfIssuer) {
      if (i.market.some && shareLine === undefined) shareLine = i;
      continue;
    }
    const face = inOwn(ctx, company, asCash(i.issued, i.ccy, 'what it owes at face'));
    balance.debt += face;
    if (i.ccy !== home) {
      const c = currencies.get(i.ccy) ?? { held: 0, owed: 0 };
      c.owed += face;
      currencies.set(i.ccy, c);
    }
    const flows = profile.cashFlows(i, on, ctx.calendar, ctx.registry);
    const next = flows[0];
    const print = ctx.prices.latest(i.id, ctx.period);
    debt.push({
      instrument: String(i.id),
      kind: String(i.kind),
      ccy: i.ccy,
      face: i.issued,
      nextPaymentOn: next === undefined ? null : civil(next.date),
      nextPayment: next === undefined ? null : next.perUnit * i.issued,
      lastPrint: print.some ? print.value.price : null,
      lastPrintedIn: print.some ? print.value.period : null,
      holders: ctx.register.holdersOf(i.id).filter((h) => h !== company).length,
      traded: i.market.some,
    });
  }

  // ---- leases, commitments as debtor, guarantees as creditor: the agreements store.
  const leases: LeaseNote[] = [];
  const commitments = new Map<string, CommitmentNote>();
  const guarantees = new Map<string, CommitmentNote>();
  const note = (into: Map<string, CommitmentNote>, kind: string, marked: number): void => {
    const had = into.get(kind);
    if (had === undefined) {
      into.set(kind, { kind, what: ctx.agreements.kind(kind as never).what, rows: 1, marked });
    } else into.set(kind, { ...had, rows: had.rows + 1, marked: had.marked + marked });
  };
  for (const a of ctx.agreements.owedBy(company)) {
    if (a.state === 'discharged' || a.state === 'terminated') continue;
    const marked = ctx.agreements.markOf(a.id);
    const markedOwn = marked === undefined ? 0 : inOwn(ctx, company, asCash(marked, a.ccy, 'its mark'));
    balance.commitments += markedOwn;
    note(commitments, String(a.terms.kind), markedOwn);
    if (isLeaseTerms(a.terms)) {
      const rent = inOwn(ctx, company, asCash(a.terms.rentPerUnit * a.terms.units, a.ccy, 'rent per period'));
      balance.leaseRentPerPeriod += rent;
      leases.push({
        agreement: String(a.id),
        kind: String(a.terms.kind),
        units: a.terms.units,
        rentPerPeriod: rent,
        until: civil(a.terms.until),
      });
    }
  }
  for (const a of ctx.agreements.owedTo(company)) {
    if (a.state === 'discharged' || a.state === 'terminated' || isEmployment(a.terms)) continue;
    const marked = ctx.agreements.markOf(a.id);
    note(guarantees, String(a.terms.kind), marked === undefined ? 0 : inOwn(ctx, company, asCash(marked, a.ccy, 'its mark')));
  }

  // ---- derivatives: its own side of every open contract, and what stands behind it.
  const derivatives: DerivativeNote[] = [];
  for (const c of ctx.contracts.openOf(company)) {
    const margin = ctx.contracts.initialMargin(c, ctx.period);
    derivatives.push({
      contract: String(c.id),
      kind: String(c.kind),
      counterparty: String(c.a === company ? c.b : c.a),
      house: c.house === null ? null : String(c.house),
      ccy: c.ccy,
      notional: c.notional,
      valueToUs: inOwn(ctx, company, ctx.contracts.valueTo(c, company, ctx.period)),
      margin: margin.some ? inOwn(ctx, company, margin.value) : null,
    });
  }

  // ---- employees: the employment register, by trade and place.
  const employees = new Map<string, EmployeeNote>();
  for (const row of ctx.employment.by(company)) {
    const key = `${row.occupation}|${String(row.region)}`;
    const had = employees.get(key) ?? {
      occupation: row.occupation,
      region: String(row.region),
      headcount: 0,
      hours: 0,
      wageBill: 0,
      leaving: 0,
    };
    const hours = row.hoursPerMember * row.headcount;
    employees.set(key, {
      ...had,
      headcount: had.headcount + row.headcount,
      hours: had.hours + hours,
      wageBill: had.wageBill + inOwn(ctx, company, asCash(row.wagePerHour * hours, row.ccy, 'its wage bill per period')),
      leaving: had.leaving + row.leaving,
    });
  }

  // ---- share capital: the listed line, its holders, what was declared and paid, and G5's division.
  let shares: ShareNote | null = null;
  if (shareLine !== undefined) {
    const declared = dividendsDeclaredIn(ctx.journal, company, span.from, span.to);
    shares = {
      line: String(shareLine.id),
      issued: shareLine.issued,
      holders: ctx.register.holdersOf(shareLine.id).filter((h) => h !== company).length,
      dividendsDeclared: declared,
      dividendsPaid,
      boughtBack,
    };
  }

  // ---- related parties, subsequent events, what it published about its own regulation.
  const controller = ctx.control.controllerOf(company);
  let largest: [string, number] | undefined;
  let revenueAll = 0;
  for (const [who, amount] of byCustomer) {
    revenueAll += amount;
    if (largest === undefined || amount > largest[1]) largest = [who, amount];
  }
  // Reporting A4: what happened to it, publicly, between the close and this publication.
  const subsequent = periodsIn(asPeriod(span.to + 1), ctx.period).flatMap((at) =>
    ctx.journal
      .inPeriod(at)
      .filter((e) => e.public && e.subjects.includes(String(company)))
      .map((e) => ({ kind: e.kind, period: e.period })),
  );
  const regulatory: Record<string, Record<string, unknown>> = {};
  const capital = capitalPublished(view, String(company));
  if (capital.some) regulatory['capital'] = { ...capital.value };
  const liquid = liquidHeld(view);
  const leave = couldLeave(view);
  const buffer = bufferHeld(view);
  if (liquid.some || leave.some || buffer.some) {
    regulatory['liquidity'] = {
      ...(liquid.some ? { liquid: liquid.value } : {}),
      ...(leave.some ? { couldLeave: leave.value } : {}),
      ...(buffer.some ? { buffer: buffer.value } : {}),
    };
  }

  // ---- management commentary: every outlook the party holds, in the numbers its decisions read.
  const commentary = view.outlookVariables().flatMap((v) => {
    const o = view.outlook(v);
    return o.some
      ? [
          {
            on: String(v),
            expected: o.value.expected,
            unit: o.value.unit,
            ...(o.value.confidence.some ? { confidence: o.value.confidence.value } : {}),
            formed: o.value.formed,
          },
        ]
      : [];
  });

  // ---- the sums a covenant reads, each of named lines.
  const summary = zeros(SUMMARY_LINES);
  summary.ebitda = ebitdaOf(income);
  summary.service = -cashFlow.interestPaid - cashFlow.principalRepaid;
  summary.freeCashFlow = cashFlow.operatingIn + cashFlow.operatingOut + cashFlow.capex;
  summary.netDebt = balance.debt - balance.cash;
  // Cash at the two ends: what it holds now, and what it held before the span's legs moved it.
  cashFlow.closingCash = balance.cash;
  const moved = CASH_LINES.filter((k) => k !== 'openingCash' && k !== 'closingCash').reduce(
    (acc, k) => acc + cashFlow[k],
    0,
  );
  cashFlow.openingCash = balance.cash - moved;

  return {
    company: String(company),
    quarter,
    from: span.from,
    to: span.to,
    ccy: home,
    income,
    earned,
    revaluation,
    oci,
    cashFlow,
    balance,
    equityChanges,
    summary,
    byProduct: [...byProduct].map(([kind, s]) => ({ kind, ...s })).sort((a, b) => (a.kind < b.kind ? -1 : 1)),
    byGeography: [...byGeography]
      .map(([region, revenue]): GeographySegment => ({ region, revenue }))
      .sort((a, b) => (a.region < b.region ? -1 : 1)),
    cash: [...byCounterparty.values()].sort((a, b) =>
      a.counterparty === b.counterparty
        ? a.cause < b.cause
          ? -1
          : 1
        : a.counterparty < b.counterparty
          ? -1
          : 1,
    ),
    holdings,
    inventories: [...inventories].map(([kind, v]): InventoryNote => ({ kind, ...v })),
    plant,
    debt,
    leases,
    commitments: [...commitments.values()],
    guarantees: [...guarantees.values()],
    derivatives,
    deals: dealsInvolving(ctx.journal, company, span.from),
    employees: [...employees.values()],
    shares,
    relatedParties: {
      controller: controller === undefined ? null : String(controller.controller),
      subsidiaries: ctx.control.subsidiariesOf(company).map(String),
      banks: [...banks],
      largestCustomer: largest === undefined ? null : largest[0],
      largestCustomerShare: largest === undefined || revenueAll <= 0 ? null : largest[1] / revenueAll,
    },
    subsequent,
    regulatory,
    commentary,
    currencies: [...currencies].map(([ccy, v]): CurrencyExposure => ({ ccy, ...v })),
  };
}

/** A5: what the span earned, re-read — the one figure a restatement compares (G2). */
export function earnedOver(ctx: MechanismContext, company: PartyId, from: Period, to: Period): number {
  let earned = 0;
  for (const e of ctx.register.equityEntries(company, from, to)) {
    earned += e.delta;
  }
  return earned;
}
