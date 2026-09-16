/**
 * What every corporate line — fixed or floating — is counted in and promises.
 *
 * @spec Corporate Credit B2 Corporate Credit B2.a Corporate Credit B4 Bond N9 Law 4
 *
 * Both kinds this module declares are par paper with covenants on them, and they must agree about
 * what a unit IS and what a promise is (Law 4). So the two facts they share live here, and neither
 * file imports the other to get them.
 */
import { unitId } from '../../core/ids.js';

/** N9: quoted as a fraction of its own face, like any other bond. */
export const CORPORATE_PAR = unitId('corporate.par');

/**
 * B2, B2.a: WHAT THIS ISSUER PROMISED ITS LENDERS, struck when it borrowed and stated on the paper.
 *
 * It is `registry/credit.ts`'s (17b.8a), because two lenders make one: a holder of paper and a bank
 * that commits a facility, and neither module may import the other. Re-exported here so a corporate
 * line has one spelling for it (Law 4).
 */
export type { Covenants } from '../../registry/credit.js';
