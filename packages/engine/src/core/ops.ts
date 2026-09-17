/**
 * HOW MANY ELEMENTARY READS A PERIOD PERFORMS — the denominator every performance claim needs.
 *
 * @spec Law 18 Part XII
 *
 * `work.ts` counts the QUESTIONS a period asks. This counts what answering one costs in reads of
 * the kernel's own stores, and the two together are the only honest way to say whether a target is
 * reachable: a period that performs 200 million store reads cannot run in three seconds at 20ns a
 * read, and no amount of tuning changes that — the operation COUNT has to fall. A period that
 * performs 20 million can, and then the constants are the whole of the work.
 *
 * It is a plain mutable counter and not a class, because it is incremented on the hottest lines in
 * the engine and anything with a lookup in it would measure itself. It is a DIAGNOSTIC and never a
 * model fact: nothing in the engine reads it, no mechanism branches on it, and the audit does not
 * see it. `docs/IMPLEMENTATION.md` 0g is what it is for.
 */
export const ops = {
  /** `instruments.get` / `has` — an instrument looked up by id. */
  instrument: 0,
  /** `register.quantity` / `free` / `allHoldings` — a holding looked up. */
  holding: 0,
  /** `prices.latest` and the marks derived from it — a print looked up. */
  price: 0,
  /** `parties.get` / `ofKind` — a party looked up. */
  party: 0,
  /** `asCash` / `asQty` / `asRatio` — a measure validated and boxed. */
  measure: 0,
  /** `journal.say` / `record` — an event written. */
  event: 0,
};

/** Every count back to nothing, for the runner between periods. */
export function resetOps(): void {
  ops.instrument = 0;
  ops.holding = 0;
  ops.price = 0;
  ops.party = 0;
  ops.measure = 0;
  ops.event = 0;
}

/** The total, which is the number a target is checked against. */
export function totalOps(): number {
  return ops.instrument + ops.holding + ops.price + ops.party + ops.measure + ops.event;
}
