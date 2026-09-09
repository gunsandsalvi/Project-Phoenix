/**
 * The contexts a module works through. There is no other door into the kernel.
 *
 * @spec Observer A1 Observer A2 Observer A3 Observer A4 Expectations D1 Law 4 Money D4 Clearing A3 Clearing A4 Seed A1 Seed A4
 *
 * - ParticipantView: what one party may see when it decides. Its own positions and accounts, public
 *   prints already produced, public instrument terms, public events. Nothing of any other party.
 * - MechanismContext: what a phase may do. Read the public state and any party's own view, settle
 *   instructions, register instruments and markets, apply cell events, journal. No register writes,
 *   no price writes, no weight writes: those have exactly one writer each.
 * - SeedContext: what a seed module may do at period zero, which is the only time endowments are
 *   written directly (Seed A3: a stock the flows then act on).
 */
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import type { MarketDecl } from '../clearing/market.js';
import type { CurrencyCode, InstrumentId, MarketId, PartyId } from '../core/ids.js';
import type { Option } from '../core/option.js';
import type { Event, EventKind, Journal } from '../journal/journal.js';
import type { InstructionDraft, SettlementRecord } from '../ledger/instruction.js';
import type { Ledger } from '../ledger/ledger.js';
import type { Parties, PartiesReads, Party, WeightEventKind } from '../parties/party.js';
import type { PriceStore, Print } from '../prices/price-store.js';
import type { Valuation } from '../prices/value.js';
import type { Holding, Register, RegisterReads } from '../register/register.js';
import type {
  Instrument,
  InstrumentDecl,
  Instruments,
  InstrumentsReads,
} from '../register/instruments.js';
import type { ParamRegister } from '../registry/params.js';
import type { Registry } from '../registry/registry.js';
import type { Prng } from '../rng/prng.js';

/** Reads every context shares. Every method here is a read; nothing mutates. */
export interface KernelReads {
  readonly period: Period;
  readonly cycle: Cycle;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<ParamRegister, 'get' | 'decl' | 'report' | 'all'>;
  readonly instruments: InstrumentsReads;
  readonly markets: readonly MarketDecl[];
}

/** Observer A1-A4: a party's own state plus the public state, and nothing else. */
export interface ParticipantView extends KernelReads {
  readonly self: Party;
  /** Own holdings, per member (A2). */
  holdings(): readonly Holding[];
  quantity(instrument: InstrumentId): number;
  free(instrument: InstrumentId): number;
  /** Own balance at own bank in a currency, per member. */
  cash(ccy: CurrencyCode): number;
  /** Own equity account, per member. */
  equity(): number;
  /** The latest public print at or before now (A1); its provenance says how stale it is (A1.a). */
  print(instrument: InstrumentId): Option<Print>;
  /** Public events (A3): prints, weight events, cessations, facility draws, the audit's counts. */
  publicEvents(last: number): readonly Event[];
  /** A random stream that is this party's own, deterministic in (seed, party, period). */
  readonly rng: Prng;
}

export interface CellEvents {
  split(cell: PartyId, members: number, cause: string): PartyId;
  merge(into: PartyId, from: PartyId, cause: string): void;
  weight(
    cell: PartyId,
    kind: Exclude<WeightEventKind, 'split' | 'merge'>,
    members: number,
    cause: string,
  ): void;
}

export interface MechanismContext extends KernelReads {
  readonly parties: PartiesReads;
  readonly register: RegisterReads;
  readonly prices: Pick<PriceStore, 'read' | 'latest' | 'history'>;
  readonly valuation: Pick<Valuation, 'markPerUnit' | 'valueOfLots'>;
  readonly journal: Pick<Journal, 'inPeriod' | 'ofKind' | 'tail'>;
  readonly ledger: Pick<Ledger, 'inPeriod' | 'length'>;
  readonly cells: CellEvents;
  /** A random stream that is this module's own, deterministic in (seed, module, period). */
  readonly rng: Prng;
  participant(party: PartyId): ParticipantView;
  /** The only way state moves (Money D4). */
  settle(draft: InstructionDraft): SettlementRecord;
  /** Register a new instrument with nothing issued; issuance is a settlement leg (Register B1). */
  issue(decl: InstrumentDecl): Instrument;
  openMarket(decl: MarketDecl): void;
  /** A party ceases and every reference resolves to a named successor (Register F2). */
  cease(party: PartyId, successor: PartyId): void;
  record(
    kind: EventKind,
    subjects: readonly string[],
    data: Record<string, unknown>,
    isPublic: boolean,
  ): Event;
}

/** Seed A1-A5: the opening world, written directly and once, then audited. */
export interface SeedContext {
  readonly period: Period;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly params: Pick<ParamRegister, 'get' | 'decl'>;
  readonly rng: Prng;
  readonly parties: Parties;
  readonly instruments: Instruments;
  readonly register: Register;
  readonly prices: Pick<PriceStore, 'write' | 'latest'>;
  openMarket(decl: MarketDecl): void;
  /** Endow a party with money at its own bank, per member (Seed A4: every deposit is a liability). */
  endowMoney(party: PartyId, ccy: CurrencyCode, perMember: number): void;
  /** Endow a party with units of an instrument at a basis, per member (Seed C4: an opening condition). */
  endowUnits(
    party: PartyId,
    instrument: InstrumentId,
    perMember: number,
    basisPerUnit: number,
  ): void;
  market(id: MarketId): MarketDecl;
}
