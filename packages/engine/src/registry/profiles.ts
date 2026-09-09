/**
 * The kernel's own kinds: money, and the parties money needs (a central bank, a bank) plus the
 * party kinds every world has. Everything else is registered by a module.
 *
 * @spec Treasury D3 Sovereign A3.b Central Bank A1 Central Bank A1.a Central Bank E2 Money A1 Money A1.b Money A1.c Money D2 Money B3.a Money B3.b Money B3.c XI-15 Households A2.e Small-Business Pools A6
 */
import { InvalidRegistry } from '../core/errors.js';
import { currencyUnit, instrumentKindId, partyKindId } from '../core/ids.js';
import type { InstrumentKindProfile, OverdraftDecision, PartyKindProfile } from './kinds.js';
import { issuerName } from './naming.js';

export const MONEY_KIND = instrumentKindId('money');

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
    moneyIssuer: {
      // B3.b: a bank overdrawn at the central bank is borrowing from it and the corridor prices it.
      // Until the corridor exists (worklist 11, Central Bank D3.b) the overdraft is allowed and
      // recorded as a reserve overdraft; the Money audit family reports every one as unpriced.
      //
      // Everyone else is REFUSED, and the treasury is the case that matters: an advance whenever
      // its account is empty converts a fiscal failure into an accounting entry and deletes the
      // reason a funding programme exists at all (Treasury D3, Central Bank E2, Sovereign A3.b).
      // A treasury that has not funded itself has failed to fund itself, and the refusal is what
      // makes that a real event with a consequence (Treasury A3.a, D5).
      overdraft: (ctx): OverdraftDecision =>
        ctx.holderIssuesMoney ? { allow: true, recordedAs: 'reserveOverdraft' } : { allow: false },
    },
  },
  {
    id: BANK,
    representation: 'named',
    moneyIssuer: {
      // B3.a: a customer overdrawn is BORROWING, and it is a credit decision by its bank — the room
      // its own capital supports, and a refusal past it. A party kind profile cannot take that
      // decision, so it does not pretend to: the lending module registers it (Banks Lending C3).
      overdraft: 'aCreditDecision',
    },
  },
  { id: TREASURY, representation: 'named', moneyIssuer: null },
  { id: FIRM, representation: 'named', moneyIssuer: null },
  { id: HOUSEHOLD, representation: 'cell', moneyIssuer: null },
  { id: SMALL_BUSINESS, representation: 'cell', moneyIssuer: null },
];
