/**
 * HOW THE PHYSICAL STATE CROSSES: as a public event, read by everybody and written by nobody else.
 *
 * @spec Commodities Spot B3 Goods B4 Freight B4 Insurers B4 Law 4 Observer A3
 *
 * Four systems need the same fact and none of them may own it (Law 4). A module never imports
 * another (ARCHITECTURE 4.9b), so what crosses is an EVENT — the same door `bank.capital` and
 * `reporting.report` already cross — and the READ of it is the kernel's (`registry/environment.ts`),
 * because four modules spelling one parser four ways is four ways to misread it. This module owns
 * the mechanism and re-exports the read, so there is one spelling of each.
 */
export {
  ENVIRONMENT_STATE,
  conditionsFor,
  conditionsIn,
  type EnvironmentReads,
} from '../../registry/environment.js';
