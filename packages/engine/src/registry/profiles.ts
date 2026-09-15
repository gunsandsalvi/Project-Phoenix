/**
 * The kernel's own kinds: money, and the parties MONEY NEEDS — a central bank, a treasury, a bank.
 * Everything else is registered by the module that owns its behaviour.
 *
 * THE ID AND THE PROFILE ARE DIFFERENT THINGS, and item 11.6 is where that was settled. A kind's
 * PROFILE is behaviour — how it is represented, the ways it can fail, whether it borrows, what
 * class of depositor it is, whether it issues money — and ARCHITECTURE 4.9b says one module owns
 * it. A kind's ID is a NAME, and any module may need to say it: `labour` posts openings for firms,
 * `ratings` charges them, `equity` opens a line on one. So the ids of the kinds more than one
 * module names live here beside the kernel's own, and the profiles of `firm` and `household` live
 * in `firms` and `households` with the behaviour they describe. Naming them from the owning module
 * instead would be a cross-module import, which is the defect this fix exists to avoid.
 *
 * @spec Equity A2 Fund Shares A2 Treasury D3 Sovereign A3.b Central Bank A1 Central Bank A1.a Central Bank E2 Money A1 Money A1.b Money A1.c Money D2 Money B3.a Money B3.b Money B3.c XI-15 Households A2.e Small-Business Pools A1 Small-Business Pools A6 Small-Business Pools A6.b Law 15
 */
import { InvalidRegistry } from '../core/errors.js';
import { currencyUnit, instrumentKindId, partyKindId, unitId, type PartyKindId } from '../core/ids.js';
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
  // Money A1: a deposit is owed at its face, which is what makes 1$ 1$. It never re-marks.
  owes: 'face',
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
/**
 * ARCHITECTURE 4.9b, item 13.3: A NAME A MODULE THAT DOES NOT OWN THE KIND STILL HAS TO SAY.
 *
 * The `funds` module owns what a pool and a manager ARE — their profiles, their objectives, what
 * they fail on — and that stays there. What is here is only the NAME, for the same reason
 * `registry/physical.ts` holds a good's id: a PRIME BROKER lends to a pool, and the module that
 * owns a bank's economics may not import the module that owns a fund (`no-cross-module-import`).
 * An id is a name, and a module that does not own a kind still has to be able to say it.
 */
export const FUND = partyKindId('fund');
export const FUND_MANAGER = partyKindId('fundManager');
/**
 * §42 A1, A6, A6.b (item 11): A SMALL FIRM IS A FIRM WITH A WEIGHT, and the kind is here for the
 * same reason `FUND` is — the modules that have to say the word do not own it.
 *
 * It is a SECOND kind and not a second representation of `FIRM` because a kind states how it is
 * represented and a kind cannot be both (`PartyKindProfile.representation`). What that costs is one
 * name; what it buys is A6.b, which says the boundary between this sector and Corporate Credit's is
 * **not a modelling line but a SIZE** — a weight of one IS a named firm — so the two kinds have to
 * be able to name each other for a cell to be promoted across (A6.c).
 *
 * `PRODUCING_KINDS` below is the list every module that asks "who are the firms in this world"
 * walks, so a mechanism gains the sector by reading a registry row rather than by branching on a
 * kind id (Law 15).
 */
export const SMALL_FIRM = partyKindId('smallFirm');

/**
 * §42 A1, A4, Law 15: THE KINDS THAT ARE FIRMS — they sell, they employ, they buy on trade credit
 * and they can fail, and which of them is a cell is a fact about its representation and not about
 * what it does.
 *
 * Every module that used to walk `ofKind(FIRM)` to find this world's businesses walks this instead.
 * It is registry DATA in the sense Law 15 means — a list a reader can see, extended by adding a row
 * — and it is the alternative to eleven modules each deciding for themselves whether a small firm
 * counts as a firm, which is how a sector comes to exist in the register and in nobody's mechanism.
 */
export const PRODUCING_KINDS: readonly PartyKindId[] = [FIRM, SMALL_FIRM];

/** The party kinds the kernel itself needs: the ones money cannot exist without (Money A1, A1.b, A1.c). */
export const KERNEL_PARTY_KINDS: readonly PartyKindProfile[] = [
  {
    /** item 15: what somebody else set it up to do, and it does not get to change it. */
    objective: 'itsMandate',
    id: CENTRAL_BANK,
    representation: 'named',
    // XI-3's one named exception, and it is a consequence of its balance sheet rather than an
    // oversight: it cannot run out of what it alone issues (§31 A1.a), and a loss reduces its
    // equity without ending it — the deferred asset is a row the treasury may make good (§31 E4).
    fails: [],
    // §31 A1.a: it is the other side of everybody's borrowing, and it does not have a bank.
    borrows: false,
    // Its own money is what everybody else's deposit is made of; nobody's deposit base holds it.
    depositClass: null,
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
    /** item 15: the business of being a bank tomorrow, which is why it will take a loss today. */
    objective: 'itsFranchise',
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
    // Banks Funding A1.c: what a bank holds AT ANOTHER BANK is wholesale money — few, very large,
    // and in the market all day. Its own account is at the central bank because that is what
    // settling in central bank money IS (Money C2.a), so no module gives it a reason to move it.
    depositClass: 'wholesale',
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
  // Treasury D3, Central Bank E2: it banks at the central bank, and that is not a choice it revisits.
  {
    id: TREASURY,
    representation: 'named',
    /** item 15: a DUTY, and duties are not interests — it has no residual and nobody to enrich. */
    objective: 'itsOffice',
    moneyIssuer: null,
    fails: [],
    borrows: true,
    depositClass: null,
    sovereign: true,
  },
];
