/**
 * The kernel's own kinds: money, and the parties money needs (a central bank, a bank) plus the
 * party kinds every world has. Everything else is registered by a module.
 *
 * @spec Equity A2 Fund Shares A2 Treasury D3 Sovereign A3.b Central Bank A1 Central Bank A1.a Central Bank E2 Money A1 Money A1.b Money A1.c Money D2 Money B3.a Money B3.b Money B3.c XI-15 Households A2.e Small-Business Pools A6
 */
import { InvalidRegistry } from '../core/errors.js';
import { currencyUnit, instrumentKindId, partyKindId, unitId } from '../core/ids.js';
import type { InstrumentKindProfile, PartyKindProfile } from './kinds.js';
import { issuerName } from './naming.js';

export const MONEY_KIND = instrumentKindId('money');

/**
 * Equity A2, Fund Shares A2: a SHARE COUNT. More than one system counts in it — a claim on a book
 * and a claim on a firm are both counted in shares — and no one of them owns it, so it is named
 * here beside the currency unit and declared as registry data by the world that has any of them.
 */
export const SHARES = unitId('shares');

export const moneyKind: InstrumentKindProfile = {
  id: MONEY_KIND,
  pricing: 'money',
  carry: 'mark',
  liabilityOfIssuer: true,
  // Bond N13, N13.a: a deposit is an unsecured claim on the bank that issued it, and there is
  // nothing else of that bank's to rank it against yet. Who pays it when the issuer cannot is the
  // estate (worklist 7); whether anybody insures it is the corridor's neighbour (worklist 11).
  ranking: () => ({
    seniority: 0,
    secured: [],
    claim: 'the balance itself, as an unsecured claim on the issuer that owes it',
  }),
  unit: (ccy) => currencyUnit(ccy),
  validateTerms: (t) => {
    if (t.kind !== MONEY_KIND) throw new InvalidRegistry('Money D2', 'money terms carry no fields');
  },
  displayName: (i, namer) => `${issuerName(namer, i.id)} money ${i.ccy}`,
  due: () => [],
  // Money pays no interest: an account is a holding of it, and a deposit rate is a bank's decision
  // (Banks Funding B1), paid by an instruction, never accrued into the instrument.
  accrued: () => 0,
  // Money promises no dated payment: it is the numéraire, worth one of itself at every date.
  cashFlows: () => [],
};

export const CENTRAL_BANK = partyKindId('centralBank');
export const TREASURY = partyKindId('treasury');
export const BANK = partyKindId('bank');
export const FIRM = partyKindId('firm');
export const HOUSEHOLD = partyKindId('household');
export const SMALL_BUSINESS = partyKindId('smallBusiness');

/** The party kinds the kernel needs or every world has (XI-15: institutions named, populations as cells). */
export const KERNEL_PARTY_KINDS: readonly PartyKindProfile[] = [
  {
    id: CENTRAL_BANK,
    representation: 'named',
    // XI-3's one named exception, and it is a consequence of its balance sheet rather than an
    // oversight: it cannot run out of what it alone issues (§31 A1.a), and a loss reduces its
    // equity without ending it — the deferred asset is a row the treasury may make good (§31 E4).
    fails: [],
    // §31 A1.a: it is the other side of everybody's borrowing, and it does not have a bank.
    borrows: false,
    moneyIssuer: {
      // B3.b, Central Bank D3: a bank overdrawn at the central bank is BORROWING FROM IT, and what
      // the central bank does about that is the lender of last resort's decision — freely, against
      // good collateral, at a penalty, to the solvent (D6). A kind profile could not take it: it
      // weighs the borrower's unencumbered eligible paper against the haircut this central bank
      // declared and its own capital against zero. So it says the answer is a credit decision and
      // the module that owns the corridor registers it (worklist 11), exactly as a bank's own
      // customer overdraft is its bank's decision (B3.a).
      //
      // The treasury is the case that stays refused, and it is refused THERE for the same reason
      // it was refused here: an advance whenever its account is empty converts a fiscal failure
      // into an accounting entry and deletes the reason a funding programme exists at all
      // (Treasury D3, Central Bank E2, Sovereign A3.b).
      overdraft: 'aCreditDecision',
    },
  },
  {
    id: BANK,
    representation: 'named',
    // XI-3, Banks Capital C1, C1.a: it cannot fund itself, or its capital is gone. Both triggers
    // must exist and the resolution must say which fired. What follows a bank's failure — the
    // bail-in hierarchy, an acquirer's bid, deposit insurance (Banks Capital D) — is worklist 11,
    // where its capital becomes raisable and the corridor makes the funding failure reachable.
    // Until then a failed bank resolves through the same estate as anything else.
    fails: ['cash', 'solvency'],
    // Banks Funding: it borrows constantly — deposits, the interbank market, the window (11).
    borrows: true,
    moneyIssuer: {
      // B3.a: a customer overdrawn is BORROWING, and it is a credit decision by its bank — the room
      // its own capital supports, and a refusal past it. A party kind profile cannot take that
      // decision, so it does not pretend to: the lending module registers it (Banks Lending C3).
      overdraft: 'aCreditDecision',
    },
  },
  // Sovereign G1: in its own money the failure mode is inflation, not default. A treasury that
  // cannot pay does not pay, and that is a real recorded state (Treasury D3) — it does not end it.
  // A default in a money it cannot create is real, and that needs the currency layer (worklist 12).
  { id: TREASURY, representation: 'named', moneyIssuer: null, fails: [], borrows: true },
  // XI-3, Firm D4: it can fail two ways and they are different — no cash to pay something due, or
  // liabilities exceeding assets. Both, because a firm can be either without the other.
  { id: FIRM, representation: 'named', moneyIssuer: null, fails: ['cash', 'solvency'], borrows: true },
  // XI-3: a household cell dissolves into a NAMED HEIR CELL rather than into an estate (Households
  // F1, F2), and what happens when its members cannot pay is their lender's enforcement. Both are
  // the household life cycle and consumer credit, which is worklist 13d.
  // Households C1.d: nobody lends to a household in this world; consumer credit is 13d.
  { id: HOUSEHOLD, representation: 'cell', moneyIssuer: null, fails: [], borrows: false },
  { id: SMALL_BUSINESS, representation: 'cell', moneyIssuer: null, fails: [], borrows: false },
];
