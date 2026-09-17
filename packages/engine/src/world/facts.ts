/**
 * The kernel's own facts, declared (0i).
 *
 * @spec XI-15 Law 4 Law 15 Law 16 Law 19 Appendix A
 *
 * A fact is declared by whoever WRITES it, and the kernel writes these. `registry/facts.ts` is the
 * register; this is the kernel's page in it.
 *
 * These are first because a mismatch in one of them does not stay a mismatch — the audit reads
 * them, so it becomes a false statement about the economy. 0e′.3 made it a build failure for a
 * MECHANISM to read another module's event by name, and that check scans `mechanisms/` only:
 * `world/`, `ledger/` and `audit/` have always read payloads by name with nothing matching them to
 * a writer, and that is where 21.100 and 21.102 both were.
 */
import { fact, type FactDecl } from '../registry/facts.js';
import { PRINT } from '../clearing/facts.js';
import { FAILED, RESERVE_OVERDRAFT, SETTLED } from '../ledger/facts.js';

/**
 * XI-15: ONE OF THE FIVE WAYS A WEIGHT CHANGES, and what it moved.
 *
 * It was written at six sites in five different shapes. One of them said `into` where the others
 * said `to`, and `audit/weights.ts` reads `to` — so every merge in this world was reported by the
 * flows family as a holding that moved with no leg behind it (21.100). Three landlord cells of two
 * hundred became one of six hundred, their eight billion dwellings came with them, and `Money D3`
 * called it unexplained. One fact, one shape, and `into` is gone with the shape that carried it.
 *
 * `from`, `to` and `successor` are `orNone` because an ENTRY has nobody it came from and a DEATH
 * has nobody it became — and the writer says so rather than leaving the key out, which is the
 * difference between a fact that states an absence and a fact that is missing a field (Appendix A).
 * `moved` is empty for an event that moved no holdings; empty is a quantity, absent is not.
 */
export const WEIGHT = fact(
  'weight',
  'one of the five ways a cell’s weight changes, with what moved between the two parties',
  {
    kind: { is: 'text', what: 'which of the five events this is' },
    members: { is: 'count', what: 'how many people the event moved' },
    before: { is: 'count', what: 'the weight before it' },
    after: { is: 'count', what: 'the weight after it' },
    cause: { is: 'text', what: 'why it happened, in the words of whoever caused it' },
    from: { is: 'party', what: 'the cell the members left', orNone: true },
    to: { is: 'party', what: 'the party they became', orNone: true },
    successor: { is: 'party', what: 'who answers for what it held, where it ceased', orNone: true },
    moved: { is: 'byInstrument', what: 'what went with them, per instrument (0f.6)' },
    key: { is: 'latticeKey', what: 'the key the members moved onto, where it is a re-key', orNone: true },
  },
);

/**
 * XI-6, Currency D2, Derivative D8: A MARK MOVED AN ACCOUNT, and which of the four things was
 * marked. A holding at a price per unit, a party's own liabilities, a contract at its value, an
 * agreement row at its schedule — one kind in the journal, so one declaration, and `what` says
 * which, exactly as `weight`'s `kind` does.
 *
 * The bag was hiding a DIMENSION CONFLICT: a holding's `mark` was a price per unit and a contract's
 * `mark` was a money value, under one name (Law 8). They are two fields now. And three of the four
 * sites called the move `deltaPerMember`, which stopped being true at 21a when the equity account
 * became a total — a stale name nothing could catch, because nobody reads it.
 */
export const REVALUATION = fact('revaluation', 'a mark moved a party’s equity account', {
  what: { is: 'text', what: 'which of holding | liabilities | contract | agreement was marked' },
  delta: { is: 'money', what: 'what the account moved by, in the party’s own money' },
  markPerUnit: { is: 'price', what: 'what a unit is now carried at', orNone: true },
  markValue: { is: 'money', what: 'what the contract or row is now carried at', orNone: true },
  markedIn: { is: 'currency', what: 'the money the mark is in', orNone: true },
  subject: { is: 'text', what: 'the contract or agreement it is about', orNone: true },
});

/** Currency D2.a: what a rate move did to a position in a money that is not the holder's. */
export const REVALUATION_FX = fact('revaluation.fx', 'an exchange rate moved a foreign position', {
  delta: { is: 'money', what: 'what the account moved by, in the holder’s own money' },
  ccy: { is: 'currency', what: 'the money the position is in' },
  home: { is: 'currency', what: 'the money the holder books in' },
  was: { is: 'ratio', what: 'the rate in force before this period’s session' },
  now: { is: 'ratio', what: 'the rate this period struck' },
  carried: { is: 'money', what: 'the position, in its own money, at what it is carried at' },
});

/**
 * Register F2: A PARTY CEASED AND EVERY REFERENCE RESOLVES TO A NAMED SUCCESSOR. Two writers, two
 * shapes: the kernel wrote `{ successor }` and a cell's promotion wrote `{ successor, cause }`, so
 * why a party ended was readable for one of them and absent for the other.
 */
export const PARTY_CEASED = fact('party.ceased', 'a party ended and named who answers for it', {
  successor: { is: 'party', what: 'who answers for what it held and owed' },
  cause: { is: 'text', what: 'why it ended' },
});

/** A line that no longer exists, with what ended it. Two writers, two different single fields. */
export const INSTRUMENT_CEASED = fact('instrument.ceased', 'a line ended', {
  reason: { is: 'text', what: 'what ended it' },
});

/**
 * Register E5: A SPLIT RESTATES THE UNIT AND MOVES NO VALUE, and the `flows` family reads the ratio
 * off this event to count a holding in the unit it is now counted in.
 */
export const INSTRUMENT_SPLIT = fact('instrument.split', 'a line’s unit was restated', {
  instrument: { is: 'instrument', what: 'the line restated' },
  ratio: { is: 'ratio', what: 'how many new units one old unit became' },
  issuedBefore: { is: 'count', what: 'what was outstanding before it' },
  issued: { is: 'count', what: 'what is outstanding after it' },
  movedNoMoney: { is: 'text', what: 'why a register moved and no money did' },
});

/** Observer A3: one party showed another one of its own events, and the kernel recorded that it did. */
export const DISCLOSED = fact('disclosed', 'one party showed another one of its own events', {
  from: { is: 'party', what: 'who showed it' },
  to: { is: 'party', what: 'who was shown it' },
  kind: { is: 'text', what: 'what sort of event was shown' },
  event: { is: 'eventRef', what: 'the event itself, which stays where it is' },
});

/** Audit C1: the period's audit ran, and this is how much it found. */
export const AUDITED = fact('audit', 'the audit ran over the period that just closed', {
  total: { is: 'count', what: 'violations found, over every family' },
});

/**
 * Corporate Credit A1: WHAT A BORROWER IS SHORT OF, published once through the kernel's own door.
 *
 * `world.ts requestsIn` rebuilt this out of nine string keys and SKIPPED the request when any one
 * of them was missing or of the wrong type — so a borrower whose ask did not parse was silently not
 * in the lending market, with no throw, no finding and no red test. It is the sharpest example of
 * what 0i is about, and it is why the kernel's facts were declared before anybody else's.
 *
 * A PLEDGE IS INSTRUMENT-TO-QUANTITY, which is a kind the register already had (`byInstrument`).
 * It was a list of (instrument, quantity) pairs on an OPTIONAL field, so a borrower that pledged
 * nothing and a borrower whose pledge was dropped on the way were the same event.
 */
export const CREDIT_REQUEST = fact('credit.request', 'a borrower published what it is short of', {
  borrower: { is: 'party', what: 'who is short' },
  ccy: { is: 'currency', what: 'the money it is short of' },
  short: { is: 'money', what: 'how much' },
  repays: { is: 'text', what: 'atOption or onSchedule, as the borrower asked' },
  wants: { is: 'text', what: 'money, or a lender that has agreed to lend and has not lent' },
  months: { is: 'count', what: 'how long it wants the money for — its own decision, not a convention' },
  statement: { is: 'eventRef', what: 'the latest accounts it prepared, opened with the ask', orNone: true },
  security: { is: 'byInstrument', what: 'what it would put up, per line — empty where it puts up nothing' },
});

/**
 * Appendix C, 0i: EVERY FACT THE KERNEL ITSELF WRITES, in one list, so the register can be built
 * and the count of what is still a bag is a number rather than a search.
 *
 * A module's facts reach the register through its phases' `writes` (`Produces.fact`); these have no
 * phase to hang on, because settlement, the markets, the cell events and the period loop are the
 * kernel and run outside any module's declaration.
 */
export const KERNEL_FACTS: readonly FactDecl[] = [
  WEIGHT,
  REVALUATION,
  REVALUATION_FX,
  PARTY_CEASED,
  INSTRUMENT_CEASED,
  INSTRUMENT_SPLIT,
  DISCLOSED,
  AUDITED,
  CREDIT_REQUEST,
  PRINT,
  SETTLED,
  FAILED,
  RESERVE_OVERDRAFT,
];
