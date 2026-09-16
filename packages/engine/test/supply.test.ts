/**
 * Supply contracts: a quantity every period at a price, and what walking away from one costs.
 *
 * @spec Goods C3 Clearing A2 Law 1 Law 2 Law 3 Law 5 XI-5
 *
 * A SCALE MODEL, not the world (docs/PLAN.md): what the book does with two crossing schedules is one
 * question and what living with a contract does is another, so this puts a contract into a running
 * world by signing it and then watches the periods go by. No party is named: the seller is whoever
 * the draw left holding a good, and the buyer is whoever else the draw put in the same place.
 */
import { describe, expect, it } from 'vitest';
import {
  assemble,
  asQty,
  instrumentId,
  period,
  isGoodTerms,
  type MechanismContext,
  type PartyId,
  type SystemModule,
} from '../src/index.js';
import { signContract, isSupplyTerms, SUPPLY } from '../src/mechanisms/supply/index.js';
import { asPerPiece } from '../src/core/measure.js';
import { expectedPriceOf } from '../src/registry/expectation.js';
import { mergeModules, rigFor, rigSpec } from './rig.js';

interface Signed {
  readonly buyer: PartyId;
  readonly seller: PartyId;
  readonly instrument: string;
  readonly price: number;
  readonly qty: number;
}

/** Whoever the draw left holding the most of a good, and whoever else can pay for some of it. */
function sign(ctx: MechanismContext, dear: boolean): Signed | undefined {
  let best: { holder: PartyId; instrument: string; qty: number } | undefined;
  for (const h of ctx.register.allHoldings()) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isGoodTerms(i.terms)) continue;
    const free = ctx.register.free(h.holder, i.id);
    if (free <= 0) continue;
    if (best === undefined || free > best.qty) best = { holder: h.holder, instrument: String(i.id), qty: free };
  }
  if (best === undefined) return undefined;
  const seller = best.holder;
  const region = ctx.parties.get(seller).region;
  const ccy = ctx.registry.currencyOf(region);
  const id = instrumentId(best.instrument);
  const asks = expectedPriceOf(ctx.participant(seller), id);
  if (!asks.some) return undefined;
  // A buyer that can pay and that expects this good to cost AT LEAST what the seller expects to get:
  // two parties who disagree the other way round would never have signed one (§46 A3).
  const buyer = ctx.parties
    .all()
    .find((p) => {
      if (!p.status.alive || p.region !== region || String(p.id) === String(seller)) return false;
      const v = ctx.participant(p.id);
      if (v.cash(ccy) <= 1_000_000) return false;
      const pays = expectedPriceOf(v, id);
      return pays.some && pays.value >= asks.value;
    });
  if (buyer === undefined) return undefined;
  // At what the SELLER expects to get, neither of them is losing by staying in, so neither walks.
  // A hundred thousand a piece is another matter, and what the buyer does about it is the point of
  // the second case — no threshold either way: what staying costs against what leaving does.
  const price = dear ? asPerPiece(100_000, 'far above what it is worth') : asks.value;
  // Law 8: enough of it that a period of it comes to more than one piece of the money — a promise
  // the money's own grain cannot carry is a promise that delivers nothing, which is its own case.
  const quarter = Math.floor(best.qty / 4);
  const perPeriod = Math.min(quarter > 0 ? quarter : 1, Math.max(4, Math.ceil(1000 / price)));
  const qty = asQty(perPeriod, 'what it promised a period');
  signContract(ctx, {
    buyer: buyer.id,
    seller,
    region,
    instrument: id,
    qtyPerPeriod: qty,
    pricePerPiece: price,
    until: ctx.calendar.startOf(period(ctx.period + 20)),
    ccy,
  });
  return { buyer: buyer.id, seller, instrument: best.instrument, price, qty };
}

function world(seed: string, dear: boolean) {
  let signed: Signed | undefined;
  const probe: SystemModule = {
    id: 'test.supply',
    spec: 'Goods C3',
    requires: ['supply'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.supply.sign',
        spec: 'Goods C3',
        anchor: { after: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== 2 || signed !== undefined) return;
          signed = sign(ctx, dear);
        },
      },
    ],
    participants: [],
    families: [],
  };
  const { banks, firms } = rigFor(seed, { makes: ['coalRaw'] });
  const spec = rigSpec(seed, banks, firms);
  const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
  for (let i = 0; i < 6; i += 1) w.step();
  return { w, get signed() { return signed; } };
}

describe('a contract delivers every period, both legs (Law 5, XI-5)', () => {
  it('moves the goods one way and the money the other, at the price that was struck', () => {
    const { w, signed } = world('supply-cheap', false);
    expect(signed).toBeDefined();
    if (signed === undefined) return;
    // The world strikes its own contracts in its own books; this is the one the probe signed, found
    // by its two names — a test never names a party, and these two are whoever the draw put here.
    const ours = [...w.agreements.ofKind(SUPPLY)].filter(
      (a) => String(a.creditor) === String(signed.buyer) && String(a.debtor) === String(signed.seller) && isSupplyTerms(a.terms) && a.terms.pricePerPiece === signed.price,
    );
    expect(ours.length).toBe(1);
    const delivered = w.journal
      .ofKind('supply.delivered')
      .filter((e) => e.data['contract'] === ours[0]!.id);
    expect(delivered.length).toBeGreaterThan(0);
    const one = delivered[0]!;
    // Law 19: what it changed hands at is the CONTRACT's price, read off the record and never
    // re-derived — which is the whole point of having signed one.
    expect(one.data['pricePerPiece']).toBe(signed.price);
    expect(Number(one.data['delivered'])).toBeGreaterThan(0);
    expect(Number(one.data['paid'])).toBeGreaterThan(0);
    // A contract a party is happy with is a contract it keeps: nobody paid to leave this one.
    expect(w.journal.ofKind('supply.broke').length).toBe(0);
  });
});

describe('breaking one costs what was agreed (Law 2)', () => {
  it('is paid in one instruction, by whichever side is worse off, and the row ends', () => {
    const { w, signed } = world('supply-dear', true);
    expect(signed).toBeDefined();
    if (signed === undefined) return;
    const broke = w.journal
      .ofKind('supply.broke')
      .filter((e) => e.data['pricePerPiece'] === signed.price);
    expect(broke.length).toBe(1);
    const one = broke[0]!;
    // The buyer is the one overpaying at a hundred thousand a piece, so the buyer is the one that
    // walks — and what it paid is the number struck when the contract was, not one worked out now.
    expect(one.data['broke']).toBe(String(signed.buyer));
    expect(Number(one.data['paid'])).toBeGreaterThan(0);
    expect(Number(one.data['wouldHaveCost'])).toBeGreaterThan(Number(one.data['paid']));
    // XI-8: the row is gone, and the record still says there was one.
    expect(
      [...w.agreements.ofKind(SUPPLY)].filter(
        (a) => a.state === 'performing' && isSupplyTerms(a.terms) && a.terms.pricePerPiece === signed.price,
      ).length,
    ).toBe(0);
  });
});
