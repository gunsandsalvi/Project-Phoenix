import type { Family } from '../audit.js';
import type { AuditMemory } from '../memory.js';
import { accountsFamily } from './accounts.js';
import { flowsFamily } from './flows.js';
import { moneyFamily } from './money.js';
import { namesFamily } from './names.js';
import { ownershipFamily } from './ownership.js';
import { pricesFamily } from './prices.js';
import { crossMarketFamily, zeroSumFamily } from './unbuilt.js';
import { unitsFamily } from './units.js';

/** Every family, in the order Part XII lists them. */
export function standardFamilies(memory: AuditMemory): Family[] {
  return [
    moneyFamily(memory),
    ownershipFamily(),
    pricesFamily(),
    crossMarketFamily(),
    accountsFamily(),
    namesFamily(),
    flowsFamily(memory),
    zeroSumFamily(),
    unitsFamily(),
  ];
}
