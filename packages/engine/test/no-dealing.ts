/**
 * The banks module with its DEALING LINE taken out, for the worlds that are not about a dealer.
 *
 * A bank is in a kernel-only world for one reason: somebody has to answer Money B3.a when a payment
 * would take an account below zero, and a world where nobody answers cannot be sealed. It is not
 * there to make markets. Left in, its schedule would be in every book those tests post into, and
 * what they measure — the kernel's clearing, its settlement, its fails, a coupon, a door — would be
 * measured through a dealer's quote.
 *
 * @spec Dealer Desks A1 Money B3.a Law 4
 */
import type { SystemModule } from '../src/index.js';

export function notDealing(m: SystemModule): SystemModule {
  return m.id === 'banks' ? { ...m, participants: [] } : m;
}
