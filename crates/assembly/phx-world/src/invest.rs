//! Investment in the world: an owner's order for plant admitted at 5d; at 6a its builder named, the seller of the
//! kind's product in its reach at the lowest price, and the project begun at that price; at 6d each project's stage,
//! the goods the builder delivers made or taken from its stock and paid for at the agreed price, settled in stage 7
//! into the owner's plant under construction; and on the last stage's settling the project complete, its plant put
//! in service in the chain's newest class at the cost it carries.

use std::collections::BTreeMap;

use phx_core::SubStep;
use phx_id::{Day, InstrumentId, PartyId, ZoneId};
use phx_ledger::apply::ApplyAt;
use phx_ledger::chains::Project;
use phx_ledger::instruction::{AccountRef, Denom, Instruction, InstructionId, LegKind, LegRec, Source};
use phx_macros::clause;
use phx_market::intents::InvestIntent;
use phx_num::{Missing, Qty, capacity_exceeded, violation};

use crate::goods::Rows;
use crate::retail::Placed;
use crate::world::World;

/// The reasons a project's stages and its completion are made under.
const BOUGHT: &str = "CAP bought";
const COMPLETED: &str = "CAP completed";

/// An owner's order for plant admitted for the day: the owner, where it stands, the chain it adds to, the product
/// the kind is bought as, the units over all its twins and the units a stage delivers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Invest {
    pub party: PartyId,
    pub zone: ZoneId,
    pub chain: u32,
    pub product: u16,
    pub units: i64,
    pub stage: i64,
}

/// A whole number of `each` at least `x`.
fn whole_up(x: i64, each: i64) -> Option<i64> {
    let parts = x / each + i64::from(x % each != 0);
    parts.checked_mul(each)
}

impl World {
    /// An owner's order for plant admitted: units of a kind it can hold in its country's chain, in whole lots of the
    /// product for every twin, built in the stages its lead time gives. One of no units or no stages is refused and
    /// counted.
    #[clause("CAP.5", "REP.9")]
    pub(crate) fn admit_invest(&mut self, step: SubStep, rows: Rows, s: &InvestIntent) {
        if step.ordinal() > SubStep::S5d.ordinal() {
            violation!(clause = "TIME.6", "plant ordered after the day's orders were admitted", step = step.ordinal());
        }
        let Some(row) = self.goods_row(rows, s.row) else { return };
        if s.units <= 0 || s.stages == 0 {
            self.market_day.tally.refused += 1;
            return;
        }
        let Missing::Present(country) = self.geo().zone_country(row.zone) else {
            violation!(clause = "GEO.2", "an owner at a zone of no country", party = row.party.get());
        };
        let ccy = phx_ledger::opening::currency(country);
        let ledger = &self.books.ledger;
        let chain = (0_u32..).zip(ledger.chains.iter()).find(|(_, c)| {
            c.tag == u32::from(s.kind) && c.classes.first().is_some_and(|i| ledger.instruments.get(*i).ccy == ccy)
        });
        let Some((chain, _)) = chain else {
            violation!(clause = "CAP.1", "plant ordered of a kind its country keeps no chain of", kind = s.kind);
        };
        let lot = self.goods_frame.base(s.product);
        let Some(each) = lot.checked_mul(row.twins) else {
            capacity_exceeded!("a lot for every twin", i64::MAX, lot);
        };
        let per_stage = s.units / i64::from(s.stages) + i64::from(s.units % i64::from(s.stages) != 0);
        let (Some(units), Some(stage)) = (
            s.units.checked_mul(row.twins).and_then(|u| whole_up(u, each)),
            per_stage.checked_mul(row.twins).and_then(|u| whole_up(u, each)),
        ) else {
            capacity_exceeded!("plant ordered", i64::MAX, s.units);
        };
        self.market_day.investments.push(Invest {
            party: row.party,
            zone: row.zone,
            chain,
            product: s.product,
            units,
            stage,
        });
        self.market_day.tally.orders += 1;
    }

    /// 6a: each order's builder named among the stalls of its product, the seller in its reach and its country at the
    /// lowest price, the lowest party on a tie, and its project begun at that price. An order with no seller in reach
    /// lapses unmet and is counted.
    #[clause("CAP.5", "CAP.2")]
    pub(crate) fn choose_builders(&mut self, day: Day, stalls_of: &BTreeMap<(u16, u16), Vec<Placed>>) {
        let orders = std::mem::take(&mut self.market_day.investments);
        for o in orders {
            let geo = self.geo();
            let Missing::Present(country) = geo.zone_country(o.zone) else {
                violation!(clause = "GEO.2", "an owner at a zone of no country", party = o.party.get());
            };
            let at = phx_market::reach::Site { zone: o.zone, country };
            let mut best: Option<(i64, PartyId, InstrumentId)> = None;
            for bound in &self.trade.retail {
                let Some(stalls) = stalls_of.get(&(bound.kind, o.product)) else { continue };
                let sites: Vec<phx_market::reach::Site> = stalls
                    .iter()
                    .map(|(_, z, _)| match geo.zone_country(*z) {
                        Missing::Present(c) => phx_market::reach::Site { zone: *z, country: c },
                        Missing::Absent => {
                            violation!(clause = "GDS.1", "a stall at a zone of no country", zone = z.get())
                        }
                    })
                    .collect();
                for i in self.trade.reach.of(at, bound.reach, &sites, &geo.distances) {
                    let (Some((stall, _, good)), Some(site)) = (stalls.get(i), sites.get(i)) else { continue };
                    if site.country != country || stall.seller == o.party {
                        continue;
                    }
                    let price = stall.price.raw();
                    if best.is_none_or(|(p, s, _)| (price, stall.seller) < (p, s)) {
                        best = Some((price, stall.seller, *good));
                    }
                }
            }
            let Some((price, builder, good)) = best else {
                self.market_day.tally.unserved += 1;
                continue;
            };
            let way = match self.market_day.makers.get(&builder) {
                Some(w) => Missing::Present(*w),
                None => Missing::Absent,
            };
            let _ = self.books.ledger.chains.begin(Project {
                owner: o.party,
                builder,
                chain: o.chain,
                good,
                way,
                price,
                lot: self.goods_frame.base(o.product),
                stage: o.stage,
                total: o.units,
                left: o.units,
                ordered: day,
            });
            self.market_day.tally.projects += 1;
        }
    }

    /// 6d: each project's stage, due today: the builder's goods, made as they are delivered for a service or taken
    /// from its stock, used up by the purchase that names the owner; the owner's money to the builder at the agreed
    /// price; and the units into the owner's plant under construction at what it paid. A stage its builder cannot
    /// deliver waits for another day.
    #[clause("CAP.5", "CAP.2", "SET.1")]
    pub(crate) fn invest_trade(&mut self, day: Day) {
        let Missing::Present(reason) = self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(BOUGHT))
        else {
            violation!(clause = "SET.1", "a project's stage under a reason never declared");
        };
        let due: Vec<(u64, Project)> =
            self.books.ledger.chains.projects().filter(|(_, p)| p.ordered <= day).map(|(n, p)| (n, *p)).collect();
        for (n, p) in due {
            let qty = if p.left < p.stage { p.left } else { p.stage };
            let Missing::Present(key) = self.books.ledger.goods.key(p.good) else {
                violation!(clause = "GDS.1", "a project delivering no good", project = n);
            };
            let Missing::Present(building) = self.books.ledger.chains.building(p.chain) else {
                violation!(clause = "CAP.2", "a project of a chain with no plant under construction", project = n);
            };
            let Some(amount) = (qty / p.lot).checked_mul(p.price) else {
                capacity_exceeded!("a stage's money", i64::MAX, qty);
            };
            let covers = match p.way {
                Missing::Present(way) => {
                    let made = self.made_by(p.builder, way, (key, p.good), qty);
                    if !self.make_for_sale(day, made) {
                        self.market_day.tally.waiting += 1;
                        continue;
                    }
                    Vec::new()
                }
                Missing::Absent => {
                    let (place, slot) = self.books.parties.row(p.builder);
                    let held = match phx_ledger::holding::holding(self.books.parties.holder(place), slot, p.good) {
                        Missing::Present(h) => h.quantity.raw(),
                        Missing::Absent => 0,
                    };
                    let pledged = self.books.ledger.liens.pledged(p.builder, p.good);
                    let unit = self.books.ledger.instruments.get(p.good).unit;
                    let Ok(c) = self.books.ledger.covers.cover(p.builder, p.good, Qty::new(qty, unit), held, pledged)
                    else {
                        self.market_day.tally.waiting += 1;
                        continue;
                    };
                    vec![c]
                }
            };
            let good = self.books.ledger.instruments.get(p.good);
            let (unit, ccy) = (good.unit, good.ccy);
            let plant_unit = self.books.ledger.instruments.get(building).unit;
            let mut legs = vec![
                LegRec {
                    party: p.builder,
                    account: AccountRef::Instrument(p.good),
                    qty: -qty,
                    denom: Denom::Unit(unit),
                    kind: LegKind::Transformation { source: Source::Purchase(p.owner.get()), cost: 0 },
                },
                LegRec {
                    party: p.owner,
                    account: AccountRef::Instrument(building),
                    qty,
                    denom: Denom::Unit(plant_unit),
                    kind: LegKind::Transformation { source: Source::Built(n), cost: amount },
                },
            ];
            self.books.pay_into(p.owner, p.builder, (amount, ccy), &mut legs);
            let instruction = Instruction {
                id: self.books.ledger.next_id(day),
                reason,
                trade_day: day,
                settle_day: day,
                legs,
                pays: Missing::Absent,
                covers,
            };
            self.market_day.stages.insert(instruction.id, (n, qty));
            self.market_day.trades.push((instruction, p.builder, p.good, qty));
        }
    }

    /// A stage settled: its project's units left less it, and on the last, the project complete: its plant under
    /// construction put in service in the chain's newest class at the cost it carries.
    #[clause("CAP.5", "CAP.8")]
    pub(crate) fn stage_settled(&mut self, day: Day, step: SubStep, id: InstructionId) {
        let Some((n, qty)) = self.market_day.stages.remove(&id) else { return };
        let Missing::Present(done) = self.books.ledger.chains.delivered(n, qty) else { return };
        let chains = &self.books.ledger.chains;
        let (Missing::Present(building), Some(newest)) =
            (chains.building(done.chain), chains.get(done.chain).and_then(|c| c.classes.first().copied()))
        else {
            violation!(clause = "CAP.1", "a project of a chain with no newest class", project = n);
        };
        let (place, slot) = self.books.parties.row(done.owner);
        let arenas = self.books.parties.holder(place);
        let Missing::Present(cost) = phx_ledger::holding::first_in_cost(arenas, slot, building, done.total) else {
            violation!(clause = "CAP.2", "a project complete with less built than it delivered", project = n);
        };
        let Missing::Present(reason) = self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(COMPLETED))
        else {
            violation!(clause = "SET.1", "a project completed under a reason never declared");
        };
        let unit = self.books.ledger.instruments.get(building).unit;
        let leg = |account, qty, cost| LegRec {
            party: done.owner,
            account: AccountRef::Instrument(account),
            qty,
            denom: Denom::Unit(unit),
            kind: LegKind::Transformation { source: Source::Built(n), cost },
        };
        let instruction = Instruction {
            id: self.books.ledger.next_id(day),
            reason,
            trade_day: day,
            settle_day: day,
            legs: vec![leg(building, -done.total, 0), leg(newest, done.total, cost)],
            pays: Missing::Absent,
            covers: Vec::new(),
        };
        match self.books.apply(ApplyAt::Day(step), instruction, self.audit.stream()) {
            Ok(_) => self.market_day.tally.completed += 1,
            Err(_) => violation!(clause = "CAP.2", "a project's plant its owner no longer holds", project = n),
        }
    }
}
