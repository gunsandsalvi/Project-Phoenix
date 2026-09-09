/**
 * Accounts balance (Audit B5): assets minus liabilities, read from the register at marks, equals
 * the stated equity account, per party, per member.
 *
 * @spec Central Bank A2 Audit B5 Audit B5.a Audit B5.b Banks Funding F3 Central Bank A2.c Firm C3 Fund Shares A3 Households D3 Law 7 Money D2
 *
 * The two sides are independent records: the register and the price store on one side, the equity
 * account moved by named events on the other. Equality is the check.
 *
 * The two sides are not reached by the same arithmetic, and Law 7 says the tolerance is what the
 * arithmetic did: one side is a fresh sum over today's holdings, the other a balance that has been
 * moved once per event since the party was born. So the account brings its own walk (`equityWalk`)
 * and the read brings the sum's, and the dust is the two together — never a band anyone chose.
 *
 * AND THE READ IS NOT ONE ROUNDING OLD EITHER. A money balance is itself a walk: one lot moved once
 * per leg since the account was opened (Money D2). A bank's deposits and a central bank's reserves
 * are read off those balances, so what a party's assets and liabilities in money have accumulated
 * in rounding is every move of every one of those accounts — not the rounding of adding them up
 * today. An issuer of money whose customers are busy is the case that shows it: its liability is a
 * sum over accounts that moved hundreds of times, and a tolerance that saw only the sum reports a
 * violation the moment the world starts paying itself.
 */
import { issuedBy } from '../../register/instruments.js';
import { combineDust, sum, withinDust } from '../../core/num.js';
import { weightOf } from '../../parties/party.js';
import type { Family, Violation } from '../audit.js';
import type { AuditView } from '../view.js';

export function accountsFamily(): Family {
  return {
    name: 'accounts',
    contributor: 'kernel',
    spec: 'Audit B5',
    built: true,
    check(view: AuditView): Violation[] {
      const out: Violation[] = [];
      for (const p of view.parties.alive()) {
        const home = view.registry.region(p.region).ccy;
        const assetTerms: number[] = [];
        // Law 7: the rounding the READ carries, which is the walk behind every money balance it is
        // read off, not the rounding of adding them up today.
        let walked = 0;
        for (const h of view.register.holdingsOf(p.id)) {
          const inst = view.instruments.get(h.instrument);
          if (inst.ccy !== home) continue; // currency layer not built: foreign positions cannot be added (Money A2.b)
          assetTerms.push(view.valuation.valueOfLots(inst.id, h.lots, view.period));
          if (view.registry.instrumentKind(inst.kind).pricing === 'money') {
            walked += view.register.moneyWalk(p.id, inst.id).dust;
          }
        }
        const liabilityTerms: number[] = [];
        for (const inst of view.instruments.all()) {
          if (!issuedBy(inst, p.id) || !view.registry.instrumentKind(inst.kind).liabilityOfIssuer)
            continue;
          if (inst.ccy !== home) continue;
          const isMoney = view.registry.instrumentKind(inst.kind).pricing === 'money';
          for (const holder of view.register.holdersOf(inst.id)) {
            const h = view.register.holding(holder, inst.id);
            if (!h.some) continue;
            const w = weightOf(view.parties.get(holder));
            liabilityTerms.push(view.valuation.valueOfLots(inst.id, h.value.lots, view.period) * w);
            // What this party owes IS those balances, read from the other side (Register B3), so
            // every rounding they have taken since they were opened is a rounding in this number.
            if (isMoney) walked += view.register.moneyWalk(holder, inst.id).dust * w;
          }
        }
        // Per member of the party (XI-15): holdings are per member; liabilities are held by others in
        // total and are divided by the party's own weight.
        const w = weightOf(p);
        const assets = sum(assetTerms);
        const liabilities = sum(liabilityTerms.map((t) => t / w));
        if (!view.register.hasEquityAccount(p.id)) {
          out.push({
            family: 'accounts',
            spec: 'Audit B5',
            owner: p.id,
            size: assets.value - liabilities.value,
            unit: home,
            period: view.period,
            message: `${p.id} has no stated equity account`,
          });
          continue;
        }
        const equity = view.register.equityWalk(p.id);
        const read = sum([assets.value, -liabilities.value]);
        // Per member, like the two sides it belongs to (XI-15).
        const dust = combineDust(assets, liabilities, read) + equity.dust + walked / w;
        if (!withinDust(read.value, equity.value, dust)) {
          out.push({
            family: 'accounts',
            spec: 'Audit B5',
            owner: p.id,
            size: read.value - equity.value,
            unit: home,
            period: view.period,
            message: `${p.id}: assets ${assets.value} - liabilities ${liabilities.value} != equity account ${equity.value} (per member)`,
          });
        }
      }
      return out;
    },
  };
}
