/**
 * What this world's money market is made of: the books it has, who counts as which kind of
 * depositor, and what each bank is like as a funder.
 *
 * @spec Money Market B6 Money Market B6.a Money Market C1 Money Market C2 Banks Funding A1 Banks Funding A1.a Banks Funding A1.b Banks Funding A1.c Banks Funding A1.d Banks Funding B1.a Banks Funding C2.a Central Bank B1 Central Bank D1 Law 2 Law 15
 *
 * Data only (Law 15). A1.d is the clause this file exists for: A MODEL WITH ONE DEPOSIT TYPE CANNOT
 * HAVE A RUN, so the classes are stated here — and only what a BANK faces about one, which is
 * whether the state insures it. WHICH class a party is in is a fact about the party's kind and is
 * declared with the kind (`PartyKindProfile.depositClass`); WHAT IT COSTS one of them to move is a
 * fact about the depositor and is declared by the module that owns it. A table here mapping kinds
 * to classes was the kind branch written out as data: a module added a kind and this file kept a
 * list of it.
 */
import { Missing } from '../../core/errors.js';
import { paramId, type ParamId, type PartyKindId } from '../../core/ids.js';
import type { Registry } from '../../registry/registry.js';

/** B6: the books. A tenor and a security, which together say what a session strikes. */
export interface BookDecl {
  readonly id: string;
  readonly tenor: string;
  /** How many periods the row runs for (B6: overnight is this period's; term is longer). */
  readonly periods: number;
  readonly secured: boolean;
  readonly why: string;
}

export const BOOKS: readonly BookDecl[] = [
  {
    id: 'overnight.unsecured',
    tenor: 'overnight',
    periods: 1,
    secured: false,
    why: 'B2: the plainest thing in the market — cash to a name until the next close, priced by what the lender thinks of the name and nothing else.',
  },
  {
    id: 'overnight.secured',
    tenor: 'overnight',
    periods: 1,
    secured: true,
    why: 'B3: the same cash against paper. A name the market doubts can still borrow here while its paper lasts, which is what makes running out of collateral (B3.c) the thing that stops it.',
  },
  {
    id: 'term.unsecured',
    tenor: 'term',
    periods: 4,
    secured: false,
    why: 'B6, A2.a: a month of money, which is what a bank rolls. B6.a reads the gap against overnight as information about expected stress, so both books have to exist for either to mean anything.',
  },
  {
    id: 'term.secured',
    tenor: 'term',
    periods: 4,
    secured: true,
    why: 'B6 and B3 together: term money against paper, which is what a bank funds a securities book with when nobody will lend it a month unsecured.',
  },
];

/**
 * A1: a class of depositor. WHAT IS HERE IS THE REGULATION AND NOTHING ELSE — whether the state
 * insures this class — because that is the only part of a depositor a BANK faces: it is what its
 * board is priced against and what it pays a premium for (Banks Capital D4).
 *
 * What it costs one of them to MOVE is not here, and that is deliberate. It is an amount of that
 * depositor's own money weighed against that depositor's own balance (A1.d, E1), so it belongs with
 * the module that owns the depositor — a household's is `households`', a fund's is `funds`' — and a
 * bank that could read it would be reading a private preference (Observer A4).
 */
export interface DepositClassDecl {
  readonly id: string;
  /** A1.a: insured up to the limit, per member where the depositor is a cell (XI-15). */
  readonly insured: boolean;
  readonly why: string;
}

export const DEPOSIT_CLASSES: readonly DepositClassDecl[] = [
  {
    id: 'retail',
    insured: true,
    why: 'A1.a: many, small, sticky and insured up to a limit — which is what a cell of members IS (XI-15), and what makes E4 break the loop for them and not for anybody else.',
  },
  {
    id: 'corporate',
    insured: false,
    why: 'A1.b: fewer, larger, operational. A firm banks where it transacts, so this money moves because the firm is trading, not because a rate moved. An ESTATE is not here: it is not running a business, it is realising one (XI-8), so its balance is proceeds waiting to be paid out and not funding anybody bids for — and paying it a deposit rate would give a party being wound up an income, and the treasury a tax claim to rank among the creditors, neither of which this world has a mechanism for.',
  },
  {
    id: 'wholesale',
    insured: false,
    why: 'A1.c: few, very large and rate-sensitive. This is the money that leaves first (E4.a), because nothing insures it and its holder is in the market all day anyway.',
  },
];

/**
 * The class a party of this kind is in, or none: a kind whose profile names no class is not
 * anybody's deposit base. The kind says which class (Law 15); this file says what the class is.
 */
export function classOf(registry: Registry, kind: PartyKindId): DepositClassDecl | undefined {
  const named = registry.partyKind(kind).depositClass;
  if (named === null) return undefined;
  const cls = DEPOSIT_CLASSES.find((c) => c.id === named);
  if (cls === undefined) {
    // A kind that says it banks as something this market never declared is MISSING, not unbanked:
    // it would otherwise be silently dropped from every bank's deposit base and from the cover.
    throw new Missing('Banks Funding A1', `party kind ${kind} banks as "${named}", which no deposit class declares`, { kind, named });
  }
  return cls;
}

/** What each bank is like as a funder: its own numbers, stated one bank at a time (Law 2, Seed B4). */
export interface FunderDecl {
  readonly bank: string;
  /**
   * B1.a, B3: what it keeps for itself out of what the money is worth to it — its margin on
   * funding, per annum. Its own: two banks that keep the same margin are one bank, and the one that
   * keeps less wins the deposit and earns less on it, which is what a net interest margin IS.
   */
  readonly depositMargin: number;
  /**
   * C2.a, A2.a: how far back it looks at its own account when it decides what buffer to hold. A
   * bank with a long memory holds against the worst week it has seen; a short one forgets it.
   */
  readonly bufferMemory: number;
  readonly why: string;
}

export const FUNDERS: readonly FunderDecl[] = [
  {
    bank: 'bank.a',
    depositMargin: 0.006,
    bufferMemory: 26,
    why: 'The cautious one: it holds against half a year of its own worst weeks, and it keeps the wider margin on the money it takes in rather than bidding for deposits it does not need. The gap between the two margins is what a depositor decides about, and it is a fifth of a point — which is more than the wholesale money will sit still for and less than the operational money will move for.',
  },
  {
    bank: 'bank.b',
    depositMargin: 0.004,
    bufferMemory: 8,
    why: 'The keen one: a short memory of its own outflows, so a thinner buffer, and a thin margin on funding — it pays up for deposits and lives on the volume, which is the same disposition that makes it the keener lender.',
  },
];

export function funderOf(bank: string): FunderDecl | undefined {
  return FUNDERS.find((f) => f.bank === bank);
}

export const mmParam = (bank: string, what: string): ParamId => paramId(`bank.${what}.${bank}`);

export const MM_PARAMS = {
  floorSpread: paramId('centralBank.corridor.floorSpread'),
  ceilingSpread: paramId('centralBank.corridor.ceilingSpread'),
  /** Central Bank D2: what it takes off the market's price of the paper it lends against. */
  haircut: paramId('centralBank.collateralHaircut'),
  /** Central Bank D3.b: what it charges ABOVE the window for an account that went below zero. */
  overdraftPenalty: paramId('centralBank.overdraftPenalty'),
  policyRate: paramId('centralBank.policyRate'),
  insuranceLimit: paramId('regulation.depositInsurance.limit'),
} as const;
