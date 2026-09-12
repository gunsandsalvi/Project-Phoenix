import type { Family } from '../audit.js';
import type { AuditMemory } from '../memory.js';
import { accountsFamily, equityLedgerFamily } from './accounts.js';
import { flowsFamily } from './flows.js';
import { moneyFamily } from './money.js';
import { namesFamily } from './names.js';
import { ownershipFamily } from './ownership.js';
import { pricesFamily } from './prices.js';
import { zeroSumFamily } from './zero-sum.js';
import { unitsFamily } from './units.js';

/**
 * Every family the KERNEL contributes, in the order Part XII lists them.
 *
 * `crossMarket` is not among them and that is the answer rather than an omission: Audit B4 is about
 * two venues for one economic thing, which the kernel has no example of. Its contributions are the
 * modules that do — the spot-fx triangle and an index against its own constituents (item 12) — and
 * the kernel used to register a stub here declaring itself unbuilt beside them. With `built` meaning
 * every contribution (Audit C2), that stub would now make a family that IS checked report as not
 * built, which is the opposite lie to the one it used to tell (item 13b.1).
 */
export function standardFamilies(memory: AuditMemory): Family[] {
  return [
    moneyFamily(memory),
    ownershipFamily(),
    pricesFamily(),
    accountsFamily(),
    equityLedgerFamily(),
    namesFamily(),
    flowsFamily(memory),
    zeroSumFamily(),
    unitsFamily(memory),
  ];
}
