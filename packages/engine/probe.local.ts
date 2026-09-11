import { assemble, partyId, type SystemModule, type World, type ParticipantView, type MarketDecl, type Order, FIRM, USD, REGION, goodId, wipId } from './src/index.js';
import { firmIn, rigDraw, rigSpec, mergeModules } from './test/rig.js';
import { perTonne, phx, tonnes } from './test/units.js';

const DREW = rigDraw('firms');
const FIRM_1 = firmIn(DREW, 'grain');
const BUYER = partyId('buyer.1');
const BANK_A = partyId('bank.a');
const GRAIN = goodId('grain', REGION);
const WIP = wipId('grain', REGION);

function buyer(subUnit: string, price: number, qty: number): SystemModule {
  const instrument = goodId(subUnit, REGION);
  return {
    id: 'test.buyer', spec: 'Goods C3', requires: ['goods', 'seed.foundation'],
    instrumentKinds: [], partyKinds: [], curveFamilies: [], units: [], params: [],
    seed(ctx) {
      ctx.parties.add({ id: BUYER, kind: FIRM, region: REGION, name: 'A buyer', bank: BANK_A, representation: 'named', status: { alive: true } });
      ctx.endowMoney(BUYER, USD, phx(100_000_000));
      ctx.endowMoney(BANK_A, USD, phx(100_000_000));
    },
    phases: [],
    participants: [{ partyKind: FIRM, orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => view.self.id === BUYER && m.instrument === instrument ? [{ party: BUYER, side: 'buy', price, qty: qty as never }] : [] }],
    families: [],
  };
}
const spec = rigSpec('firms');
const w: World = assemble({ ...spec, modules: mergeModules(spec.modules, [buyer('grain', perTonne(400), tonnes(90))]) });
console.log('FIRM_1 =', FIRM_1);
for (let i = 0; i < 8; i += 1) {
  w.step();
  const plan = w.journal.ofKind('firms.plan').filter((e) => e.period === w.period && e.subjects.includes(FIRM_1)).pop();
  const started = w.journal.ofKind('firms.started').filter((e) => e.period === w.period && e.subjects.includes(FIRM_1)).pop();
  const idle = w.journal.ofKind('firms.idle').filter((e) => e.period === w.period && e.subjects.includes(FIRM_1)).pop();
  const wages = w.journal.ofKind('labour.wages').filter((e) => e.period === w.period && e.subjects.includes(FIRM_1)).pop();
  console.log(w.period, 'plan=', JSON.stringify(plan?.data), 'started=', JSON.stringify(started?.data), 'idle=', JSON.stringify(idle?.data), 'wages=', JSON.stringify(wages?.data), 'grain=', w.register.quantity(FIRM_1, GRAIN), 'wip=', w.register.quantity(FIRM_1, WIP));
}
