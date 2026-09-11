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
          // Currency C4, C5, D2: A POSITION IN ANOTHER MONEY IS AN ASSET LIKE ANY OTHER, converted
          // at the rate in force — the same rate the same period settled at, so what a balance
          // sheet says and what a payment does cannot disagree. It used to be SKIPPED, with the
          // reason "currency layer not built": the family was reporting a balance sheet with the
          // foreign half missing and calling it balanced, which is a check that passes by not
          // looking (Law 4). The layer is built; the skip is gone.
          assetTerms.push(
            view.valuation.inMoney(
              view.valuation.valueOfLots(inst.id, h.lots, view.period),
              inst.ccy,
              home,
              view.period,
            ),
          );
          if (view.registry.instrumentKind(inst.kind).pricing === 'money') {
            walked += view.register.moneyWalk(p.id, inst.id).dust;
          }
        }
        const liabilityTerms: number[] = [];
        for (const inst of view.instruments.all()) {
          if (!issuedBy(inst, p.id) || !view.registry.instrumentKind(inst.kind).liabilityOfIssuer)
            continue;
          const isMoney = view.registry.instrumentKind(inst.kind).pricing === 'money';
          for (const holder of view.register.holdersOf(inst.id)) {
            const h = view.register.holding(holder, inst.id);
            if (!h.some) continue;
            const w = weightOf(view.parties.get(holder));
            liabilityTerms.push(
              view.valuation.inMoney(
                view.valuation.valueOfLots(inst.id, h.value.lots, view.period),
                inst.ccy,
                home,
                view.period,
              ) * w,
            );
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
        // Central Bank A2.c, Currency D2.a: what stands against the read is the party's equity AND,
        // for the central bank of its own money, its revaluation account — the one place a rate move
        // on foreign reserves goes, because those reserves are the other side of what it printed
        // rather than a position it took. For everybody else the account is zero and this is the
        // equity account, which is the one number they have.
        const equity = view.register.equityWalk(p.id);
        const revaluation = view.register.revaluationWalk(p.id);
        const stands = sum([equity.value, revaluation.value]);
        const read = sum([assets.value, -liabilities.value]);
        // Per member, like the two sides it belongs to (XI-15).
        const dust =
          combineDust(assets, liabilities, read) + equity.dust + revaluation.dust + stands.dust + walked / w;
        if (!withinDust(read.value, stands.value, dust)) {
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
