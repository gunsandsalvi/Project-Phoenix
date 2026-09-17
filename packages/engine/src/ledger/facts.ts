/**
 * What the wire says (0i): an instruction settled, an instruction failed, an account went short.
 *
 * @spec Money B3.c Money E1 Money G2 XI-5 Law 4 Law 19 Appendix A
 *
 * Settlement is the one writer of a state change, so it is the one writer of these.
 */
import { fact } from '../registry/facts.js';

export const SETTLED = fact('instruction.settled', 'a numbered instruction applied, whole', {
  id: { is: 'count', what: 'the instruction number' },
  cause: { is: 'text', what: 'why it was written, as its drafter said' },
  legs: { is: 'count', what: 'how many legs it applied' },
});

export const FAILED = fact('instruction.failed', 'a numbered instruction that applied none of itself', {
  id: { is: 'count', what: 'the instruction number' },
  cause: { is: 'text', what: 'why it was written, as its drafter said' },
  reason: { is: 'text', what: 'what settlement refused it for' },
});

/**
 * Money B3.c: AN ACCOUNT THAT WENT SHORT AT ITS ISSUER, and by how much. 21a made every holding a
 * total, and this said `shortfallPerMember` — a name that stopped being true at that item and that
 * nothing would have caught, because nobody reads it by name.
 */
export const RESERVE_OVERDRAFT = fact(
  'reserve.overdraft',
  'a holder’s account at its issuer closed a leg short, and the issuer allowed or refused it',
  {
    instruction: { is: 'count', what: 'the instruction that took it short' },
    holder: { is: 'party', what: 'whose account it is' },
    issuer: { is: 'party', what: 'who issues the money it is short of' },
    ccy: { is: 'currency', what: 'the money it is short of' },
    shortfall: { is: 'money', what: 'how far below nothing the account went' },
  },
);
