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
import { fact } from '../registry/facts.js';

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
