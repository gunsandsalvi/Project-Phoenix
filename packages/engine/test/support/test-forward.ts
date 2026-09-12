/**
 * A TEST-ONLY CONTRACT KIND, and it says it is one.
 *
 * @spec Derivative D1 Derivative D2 Derivative D3 Derivative D4 Derivative D7 Derivative D7.b Derivative D8 Derivative D11 Derivative D11.a Derivative D12 Derivative Layer D1 Law 11
 *
 * Item 13a builds the LAYER and no class: every class is a module of its own (13b). But a layer
 * with nothing running on it cannot be exercised — a margin call needs a mark, a waterfall needs a
 * loss — so this is the smallest thing that is a derivative by the contract's own definition: a
 * cash-settled forward on a line this world already clears.
 *
 * It lives under `test/` and is deleted when 13b's first class replaces it in the year-long run.
 * Nothing in `src/` may import it, which is the point of where it is.
 */
import {
  none,
  some,
  mul,
  sub,
  unitId,
  type Contract,
  type ContractPayment,
  type ContractReads,
  type ContractTerms,
  type DerivativeKindProfile,
  type Option,
  type InstrumentId,
  type MarketId,
  type Period,
  derivativeKindId,
  partyKindId,
  asQty,
  CENT_TICK,
} from '../../src/index.js';

export const TEST_FORWARD = derivativeKindId('test.forward');
/** D2: the notional is a count of contracts, each for one unit of the underlying. */
export const CONTRACTS = unitId('contracts');

export interface ForwardTerms extends ContractTerms {
  readonly kind: typeof TEST_FORWARD;
  readonly market: MarketId;
  readonly underlying: InstrumentId;
  /** D6: the period it settles in. */
  readonly expiry: Period;
  /** D12: part of the identity. Two forwards at two strikes are two contracts. */
  readonly strike: number;
  /** Which way `a` is: long the underlying, or short it. The book writes `true`; `flip` writes false. */
  readonly long: boolean;
  /** D1 (layer): how many periods of the underlying's own prints the margin is measured over. */
  readonly window: number;
  /** What the fixture wants traded in this book, and by whom. Test-only, like the kind. */
  readonly size: number;
  readonly movers: readonly string[];
}

export const isForward = (t: ContractTerms): t is ForwardTerms =>
  'strike' in t && 'long' in t && 'underlying' in t;

/** D8: what it is worth to `a` — the print against the strike, one way or the other. */
function markOf(c: Contract, at: Period, reads: ContractReads): number {
  if (!isForward(c.terms)) return 0;
  const p = reads.print(c.terms.underlying, at);
  if (!p.some) return 0;
  const move = sub(p.value.price, c.terms.strike, 'the print against the strike');
  return mul(mul(move, c.notional, 'per contract'), c.terms.long ? 1 : -1, 'to this side');
}

export const testForwardKind: DerivativeKindProfile = {
  id: TEST_FORWARD,
  unit: CONTRACTS,
  // Law 8: quoted in cents a contract, like a share (`registry/grid.ts` CENT_TICK). A tick is
  // declared in NAMED money per named unit of the thing, the way `UnitDecl.perUnit` is.
  priceTick: CENT_TICK,
  underlying: (c) =>
    isForward(c.terms)
      ? { kind: 'print', market: c.terms.market, instrument: c.terms.underlying }
      : { kind: 'print', market: '' as MarketId, instrument: '' as InstrumentId },
  validateTerms: (t) => {
    if (!isForward(t)) throw new Error('not forward terms');
  },
  displayName: (c) =>
    isForward(c.terms) ? `forward ${c.terms.underlying} @ ${c.terms.strike}` : String(c.id),
  mark: markOf,
  // A3: the same contract as the other side states it — the direction is what turns over.
  flip: (t) => (isForward(t) ? { ...t, long: !t.long } : t),
  // D4: nothing falls due before expiry; what moves in between is the margin (D9).
  legs: (): readonly ContractPayment[] => [],
  // D7.b: struck at par. The cleared price IS the strike, so nothing changes hands at inception.
  premiumPerUnit: () => 0,
  initialMargin: (c, at, reads): Option<number> => {
    if (!isForward(c.terms)) return none();
    const move = reads.measuredMove(c.terms.underlying, c.terms.window);
    if (!move.some) return none();
    const left = c.terms.expiry > at ? c.terms.expiry - at : 0;
    const horizon = reads.params.periods(
      'clearingHouse.closeOutHorizon' as Parameters<ContractReads['params']['periods']>[0],
    );
    return some(
      mul(
        mul(move.value, c.notional, 'over the notional'),
        Math.sqrt(left > 0 ? left / horizon : 1),
        'over the life it has left',
      ),
    );
  },
  /**
   * Clearing A2, Law 4: THE KIND CARRIES ITS OWN REASONS, because the layer declares the one
   * participant a contract book asks (`DerivativeKindProfile.orders`). The fixture's two banks and
   * its own movers say what they want here rather than in a second participant of their own, which
   * would be two modules speaking for one party in one book.
   */
  orders: (view, m) => {
    const t = m.contract?.terms;
    if (t === undefined || !isForward(t)) return [];
    const me = String(view.self.id);
    if (t.movers.includes(me)) {
      const first = t.movers[0];
      return first === undefined
        ? []
        : [{ party: view.self.id, side: me === first ? 'buy' : 'sell', price: t.strike, qty: asQty(t.size) }];
    }
    const banks = view.parties
      .ofKind(partyKindId('bank'))
      .filter((b) => b.status.alive)
      .map((b) => String(b.id));
    if (banks[0] === me) return [{ party: view.self.id, side: 'buy', price: t.strike, qty: asQty(t.size) }];
    if (banks[1] === me) return [{ party: view.self.id, side: 'sell', price: t.strike, qty: asQty(t.size) }];
    return [];
  },
  // D11.a: the stated close-out value is what it is worth now. A forward's payoff is its mark.
  closeOut: markOf,
  expires: (c, at): boolean => isForward(c.terms) && at >= c.terms.expiry,
};
