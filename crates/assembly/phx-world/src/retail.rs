//! Retail in the world: a buyer's want admitted at 5d into its product's retail market; at 6a each market's stalls
//! read from the sellers' declared facts and their free units where they stand, the buyers' tastes drawn and the
//! meeting held; at 6d each sale a purchase, the buyer's money against the seller's units used up at the till, settled
//! in stage 7 with the day's trades.

use std::collections::{BTreeMap, BTreeSet};

use phx_core::{Declarations, FactStore, KindTableRef, Register, SubStep};
use phx_id::{Day, InstrumentId, MarketId, PartyId, ZoneId};
use phx_ledger::goods::GoodKey;
use phx_ledger::instruction::{AccountRef, Denom, Instruction, LegKind, LegRec, Source};
use phx_macros::clause;
use phx_market::intents::ShopIntent;
use phx_market::print::{Buyer, Match};
use phx_market::retail::{InReach, RetailKind, Shopper, Stall, Want, Weights};
use phx_num::{Missing, Qty, capacity_exceeded, violation};
use phx_pop::population::Population;
use phx_rand::{Subject, SubjectTag};
use phx_store::SystemBacking;

use crate::goods::Rows;
use crate::world::World;

/// A retail kind as the world meets it: its place among the market kinds, its declaration, the tables of the kinds
/// that sell in it (each its place among the holders and whether its rows are individuals), how its buyers weigh
/// sellers, and how far in metres they reach.
#[derive(Clone, Debug)]
pub(crate) struct RetailBound {
    pub kind: u16,
    pub decl: RetailKind,
    pub sellers: Vec<(u16, bool)>,
    pub weights: Weights,
    pub reach: u64,
}

/// The kinds of market goods meet buyers and carriers in, with the one set of counterparties a search reaches.
#[derive(Clone, Debug)]
pub(crate) struct TradeKinds {
    pub retail: Vec<RetailBound>,
    pub freight: Vec<crate::freight::FreightBound>,
    pub reach: phx_market::reach::Reach,
}

/// A buyer's want admitted for the day: its market, the product, the buyer, where it stands, its twins and what a twin
/// wants.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shop {
    pub market: MarketId,
    pub product: u16,
    pub party: PartyId,
    pub zone: ZoneId,
    pub twins: i64,
    pub want: Want,
}

/// A stall where it stands: the seller's offer, its zone, and the good it sells there.
pub(crate) type Placed = (Stall, ZoneId, InstrumentId);

/// A sale of the day's meetings awaiting its purchase: the market, the good, the match, and the seller's units it
/// covers until it settles.
#[derive(Debug)]
pub(crate) struct Sale {
    pub market: MarketId,
    pub good: InstrumentId,
    pub matched: Match,
    pub cover: phx_ledger::covered::Covered,
}

/// Every retail kind the systems declare, bound to its market kind, its sellers' tables and its primitives; every
/// refusal at once.
pub(crate) fn bind(
    d: &Declarations,
    kinds: &phx_market::instances::Kinds,
    register: &Register,
) -> Result<Vec<RetailBound>, Vec<String>> {
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (system, k) in &d.markets {
        let Some(decl) = k.downcast_ref::<RetailKind>() else { continue };
        let Missing::Present(kind) = kinds.kind(phx_ledger::instruction::name_code(decl.market.key.kind)) else {
            errors.push(format!("{system}'s retail kind `{}` is no declared market kind", decl.market.key.kind));
            continue;
        };
        let mut sellers = Vec::new();
        for s in decl.sellers {
            if let Some(p) = place_of(d, s) {
                sellers.push(p);
            } else {
                let name = decl.market.name;
                errors.push(format!("`{name}` sells to kind `{s}`, which the world does not keep"));
            }
        }
        let weights = match (register.fixed(decl.price_weight), register.fixed(decl.distance_weight)) {
            (Ok(price), Ok(distance)) => Weights { price, distance },
            (Err(e), _) | (_, Err(e)) => {
                errors.push(format!("{system}'s retail weights: {e}"));
                continue;
            }
        };
        let reach = match register.count(decl.reach) {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("{system}'s retail reach: {e}"));
                continue;
            }
        };
        out.push(RetailBound { kind, decl: *decl, sellers, weights, reach });
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// A kind's table among the holders: its place, and whether its rows are individuals.
pub(crate) fn place_of(d: &Declarations, kind: &str) -> Option<(u16, bool)> {
    let individuals: Vec<&str> =
        d.kinds.iter().filter(|(_, k)| k.table == KindTableRef::Individuals).map(|(_, k)| k.name).collect();
    let agents: Vec<&str> =
        d.kinds.iter().filter(|(_, k)| k.table == KindTableRef::Cells).map(|(_, k)| k.name).collect();
    let place = individuals
        .iter()
        .position(|k| *k == kind)
        .map(|i| (i, true))
        .or_else(|| agents.iter().position(|k| *k == kind).map(|i| (individuals.len() + i, false)))?;
    Some((u16::try_from(place.0).ok()?, place.1))
}

impl World {
    /// A buyer's want of a product admitted into the product's retail market: a want of units in whole lots of the
    /// product's least quantity, or of money, a twin's; one of neither is refused and counted.
    #[clause("SRV.4", "HH.5", "REP.9")]
    pub(crate) fn admit_shop(&mut self, step: SubStep, rows: Rows, s: &ShopIntent) {
        if step.ordinal() > SubStep::S5d.ordinal() {
            violation!(clause = "TIME.6", "a want asked after the day's orders were admitted", step = step.ordinal());
        }
        let Some(row) = self.goods_row(rows, s.row) else { return };
        let Missing::Present(kind) = self.market_kinds.kind(s.kind) else {
            violation!(clause = "MKT.1", "a want in a market kind never declared", party = row.party.get());
        };
        if !self.trade.retail.iter().any(|r| r.kind == kind) {
            violation!(clause = "SRV.4", "a want in a market that is no retail market", party = row.party.get());
        }
        let base = self.goods_frame.base(s.product);
        let whole = match s.want {
            Want::Units(q) => q > 0 && q % base == 0,
            Want::Money(m) => m > 0,
        };
        if !whole {
            self.market_day.tally.refused += 1;
            return;
        }
        let market = self.market_kinds.instance(&mut self.markets.made, kind, u64::from(s.product));
        self.market_day.shops.push(Shop {
            market,
            product: s.product,
            party: row.party,
            zone: row.zone,
            twins: row.twins,
            want: s.want,
        });
        self.market_day.tally.orders += 1;
    }

    /// Each seller's stall of the products the day's buyers want, in one pass over the sellers: its posted price and
    /// its free units of the good where it stands, less what the day's matches between firms already take; a seller
    /// with no price, or no whole lot, has none.
    fn stalls(&mut self, bound: &RetailBound, products: &BTreeSet<u16>) -> BTreeMap<u16, Vec<Placed>> {
        let first = self.books.parties.first_cell_place();
        let mut read: Vec<(Rows, phx_id::Slot, u16, Missing<i64>)> = Vec::new();
        for &(place, individuals) in &bound.sellers {
            let agents = || {
                let Some(k) = place.checked_sub(first) else {
                    violation!(clause = "REP.2", "an agent kind's table before the first agents'", place = place);
                };
                usize::from(k)
            };
            let slots: Vec<phx_id::Slot> = if individuals {
                self.books.parties.table(place).slots().collect()
            } else {
                Population::table::<SystemBacking>(self.books.parties.cells(), agents()).slots().collect()
            };
            let store: &mut dyn FactStore = if individuals {
                let t = self.books.parties.table_mut(place);
                t.trace(None);
                t
            } else {
                let t = Population::table_mut::<SystemBacking>(self.books.parties.cells_mut().0, agents());
                t.trace(None);
                t
            };
            for slot in slots {
                let Missing::Present(sells) = store.read(bound.decl.sells, slot) else { continue };
                let Ok(product) = u16::try_from(sells) else {
                    violation!(clause = "TEC.4", "a seller's product beyond the products' places", value = sells);
                };
                if products.contains(&product) {
                    read.push((Rows { place, individuals }, slot, product, store.read(bound.decl.price, slot)));
                }
            }
        }
        let rows: Vec<(PartyId, u16, Missing<i64>, ZoneId)> = read
            .into_iter()
            .filter_map(|(rows, slot, product, price)| {
                self.goods_row(rows, slot).map(|r| (r.party, product, price, r.zone))
            })
            .collect();
        let mut pending: BTreeMap<(PartyId, InstrumentId), i64> = BTreeMap::new();
        for (_, good, m) in &self.market_day.matches {
            *pending.entry((m.seller, *good)).or_insert(0) += m.qty;
        }
        let mut out: BTreeMap<u16, Vec<Placed>> = BTreeMap::new();
        for (seller, product, price, zone) in rows {
            let Missing::Present(price) = price else { continue };
            let base = self.goods_frame.base(product);
            let Missing::Present(good) = self.books.ledger.goods.of(GoodKey { product, grade: 0, zone }) else {
                continue;
            };
            let (place, slot) = self.books.parties.row(seller);
            let held = match phx_ledger::holding::holding(self.books.parties.holder(place), slot, good) {
                Missing::Present(h) => h.quantity.raw(),
                Missing::Absent => 0,
            };
            let free = held
                - self.books.ledger.liens.pledged(seller, good)
                - self.books.ledger.covers.committed(seller, good)
                - pending.get(&(seller, good)).copied().unwrap_or(0);
            if price > 0 && free >= base {
                let stall = Stall { seller, price: phx_num::PriceRaw::from_raw(price), units: free };
                out.entry(product).or_default().push((stall, zone, good));
            }
        }
        out
    }

    /// 6a: every retail market with buyers today met: the stalls of its product, each buyer's sellers in reach with
    /// its taste for each drawn, the meeting, and each sale's units covered until its purchase settles.
    #[clause("SRV.5", "REP.22", "MKT.6", "HH.15")]
    pub(crate) fn retail_meet(&mut self, day: Day) {
        let mut by_market: BTreeMap<MarketId, Vec<Shop>> = BTreeMap::new();
        for s in std::mem::take(&mut self.market_day.shops) {
            by_market.entry(s.market).or_default().push(s);
        }
        let mut stalls_of: BTreeMap<(u16, u16), Vec<Placed>> = BTreeMap::new();
        for bound in self.trade.retail.clone() {
            let products: BTreeSet<u16> = by_market
                .values()
                .flatten()
                .filter(|s| self.market_kinds.decl(&self.markets.made, s.market).key.kind == bound.decl.market.key.kind)
                .map(|s| s.product)
                .collect();
            if !products.is_empty() {
                for (p, v) in self.stalls(&bound, &products) {
                    stalls_of.insert((bound.kind, p), v);
                }
            }
        }
        for (market, shops) in by_market {
            let decl = self.market_kinds.decl(&self.markets.made, market);
            let Some(bound) = self.trade.retail.iter().find(|r| r.decl.market.key.kind == decl.key.kind).cloned()
            else {
                violation!(
                    clause = "SRV.5",
                    "a retail want in a market no retail kind declares",
                    market = market.get()
                );
            };
            let Some(product) = shops.first().map(|s| s.product) else { continue };
            let stalls = stalls_of.remove(&(bound.kind, product)).unwrap_or_default();
            let geo = self.geo();
            let sites: Vec<phx_market::reach::Site> = stalls
                .iter()
                .map(|(_, z, _)| match geo.zone_country(*z) {
                    Missing::Present(country) => phx_market::reach::Site { zone: *z, country },
                    Missing::Absent => violation!(clause = "GDS.1", "a stall at a zone of no country", zone = z.get()),
                })
                .collect();
            let shoppers = self.shoppers(&bound, &shops, &sites, day);
            let Some(stream) = self.streams.named(decl.stream) else {
                violation!(clause = "CHN.1", "a market drawing from a stream never declared", market = market.get());
            };
            let subject = Subject::new(SubjectTag::Market, u64::from(market.get()));
            let mut lot = self.streams.open(&stream, subject, day, SubStep::S6a.ordinal());
            let only: Vec<Stall> = stalls.iter().map(|(s, _, _)| *s).collect();
            let base = self.goods_frame.base(product);
            let outcome = phx_market::retail::retail(&only, &shoppers, base, bound.weights, &mut lot);
            let t = &mut self.market_day.tally;
            t.shoppers += phx_rand::float::len_u64(shoppers.len());
            t.in_reach += outcome.in_reach;
            t.rounds += outcome.rounds;
            t.unserved += phx_rand::float::len_u64(outcome.unserved.len());
            if matches!(self.goods_frame.at_once.get(usize::from(product)), Some(true)) {
                let offered: i64 = only.iter().map(|s| s.units).sum();
                let sold: i64 = outcome.sales.iter().map(|m| m.qty).sum();
                let Ok(left) = u64::try_from(offered - sold) else {
                    violation!(
                        clause = "SRV.5",
                        "a meeting that sold more than its sellers could",
                        market = market.get()
                    );
                };
                self.market_day.tally.unused += left;
            }
            if let Some((_, _, good)) = stalls.first() {
                let i = self.books.ledger.instruments.get(*good);
                let postings: Vec<phx_market::posted::Posting> = only
                    .iter()
                    .map(|s| phx_market::posted::Posting { seller: s.seller, price: s.price, capacity: s.units })
                    .collect();
                let day_out = phx_market::posted::PostedDay {
                    sales: outcome.sales.clone(),
                    unserved: BTreeMap::new(),
                    rounds: outcome.rounds,
                };
                self.markets.record_posted(market, day, (i.unit, i.ccy), &postings, &day_out);
            }
            self.cover_sales(market, &stalls, outcome.sales);
        }
    }

    /// Each buyer's sellers in reach, among the stalls standing at `sites`, with its taste for each drawn today.
    fn shoppers(
        &self,
        bound: &RetailBound,
        shops: &[Shop],
        sites: &[phx_market::reach::Site],
        day: Day,
    ) -> Vec<Shopper> {
        let geo = self.geo();
        let Some(tastes) = self.streams.named(bound.decl.tastes) else {
            violation!(clause = "CHN.1", "retail tastes drawn from a stream never declared", kind = bound.kind);
        };
        // The stalls in reach of a zone, each with its distance, are found once for all its buyers.
        let mut near: BTreeMap<ZoneId, Vec<(usize, f64)>> = BTreeMap::new();
        let mut out = Vec::with_capacity(shops.len());
        for s in shops {
            let within = near.entry(s.zone).or_insert_with(|| {
                let Missing::Present(country) = geo.zone_country(s.zone) else {
                    violation!(clause = "GEO.2", "a buyer at a zone of no country", party = s.party.get());
                };
                let at = phx_market::reach::Site { zone: s.zone, country };
                self.trade
                    .reach
                    .of(at, bound.reach, sites, &geo.distances)
                    .into_iter()
                    .map(|i| {
                        let Some(metres) = sites.get(i).and_then(|z| geo.distances.between(s.zone, z.zone)) else {
                            violation!(clause = "MKT.1", "a seller in reach with no path to it", party = s.party.get());
                        };
                        (i, phx_rand::float::from_u64(metres) / crate::consts::METRES_PER_KM)
                    })
                    .collect()
            });
            let subject = Subject::new(SubjectTag::Party, s.party.get());
            let mut d = self.streams.open(&tastes, subject, day, SubStep::S6a.ordinal());
            let reach = within
                .iter()
                .map(|&(stall, km)| InReach { stall, km, taste: phx_rand::gumbel(&mut d, 0.0, 1.0) })
                .collect();
            out.push(Shopper { buyer: s.party, twins: s.twins, want: s.want, reach });
        }
        out
    }

    /// Each sale's units covered for its buyer until its purchase settles.
    fn cover_sales(&mut self, market: MarketId, stalls: &[Placed], sales: Vec<Match>) {
        for m in sales {
            let Some((_, _, good)) = stalls.iter().find(|(s, _, _)| s.seller == m.seller) else {
                violation!(clause = "SRV.5", "a sale by no seller at the meeting", party = m.seller.get());
            };
            let good = *good;
            let (place, slot) = self.books.parties.row(m.seller);
            let held = match phx_ledger::holding::holding(self.books.parties.holder(place), slot, good) {
                Missing::Present(h) => h.quantity.raw(),
                Missing::Absent => 0,
            };
            let pledged = self.books.ledger.liens.pledged(m.seller, good);
            let unit = self.books.ledger.instruments.get(good).unit;
            let Ok(cover) = self.books.ledger.covers.cover(m.seller, good, Qty::new(m.qty, unit), held, pledged) else {
                violation!(clause = "REG.10", "a sale of units its seller holds no more", party = m.seller.get());
            };
            self.market_day.sales.push(Sale { market, good, matched: m, cover });
        }
    }

    /// 6d: every sale a purchase due today, under its kind's reason: the buyer's money to the seller, and the seller's
    /// units used up at the till by the purchase that names the buyer, their cost the cost of what it sold.
    #[clause("SRV.6", "SET.1", "SET.4", "GDS.2")]
    pub(crate) fn retail_trade(&mut self, day: Day) {
        for s in std::mem::take(&mut self.market_day.sales) {
            let Buyer::Party(buyer) = s.matched.buyer else {
                violation!(clause = "SRV.5", "a retail sale to no party", market = s.market.get());
            };
            let decl = self.market_kinds.decl(&self.markets.made, s.market);
            let Some(bound) = self.trade.retail.iter().find(|r| r.decl.market.key.kind == decl.key.kind) else {
                continue;
            };
            let code = phx_ledger::instruction::name_code(bound.decl.reason);
            let Missing::Present(reason) = self.books.ledger.reasons.coded(code) else {
                violation!(clause = "SET.1", "a purchase under a reason never declared", market = s.market.get());
            };
            let Missing::Present(key) = self.books.ledger.goods.key(s.good) else {
                violation!(clause = "GDS.1", "a retail market over no good", market = s.market.get());
            };
            let base = self.goods_frame.base(key.product);
            let Some(amount) = (s.matched.qty / base).checked_mul(s.matched.price.raw()) else {
                capacity_exceeded!("a purchase's money", i64::MAX, s.matched.qty);
            };
            let instrument = self.books.ledger.instruments.get(s.good);
            let mut legs = vec![LegRec {
                party: s.matched.seller,
                account: AccountRef::Instrument(s.good),
                qty: -s.matched.qty,
                denom: Denom::Unit(instrument.unit),
                kind: LegKind::Transformation { source: Source::Purchase(buyer.get()), cost: 0 },
            }];
            self.books.pay_into(buyer, s.matched.seller, (amount, instrument.ccy), &mut legs);
            let instruction = Instruction {
                id: self.books.ledger.next_id(day),
                reason,
                trade_day: day,
                settle_day: day,
                legs,
                pays: Missing::Absent,
                covers: vec![s.cover],
            };
            self.market_day.trades.push((instruction, s.matched.seller, s.good, s.matched.qty));
        }
    }
}
