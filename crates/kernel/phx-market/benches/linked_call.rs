#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "gungraun's harness prints and exits; a setup that fails is a broken benchmark"
)]

//! The linked call's prototype on a money market of the declared size — a thousand lenders with budgets, five
//! thousand borrowers, twenty thousand reaches — met cold, and met warm the next day from the first day's basis
//! when every bid has moved.

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_id::{Day, MarketId, PartyId};
use phx_market::coupled_call::{Edge, Node};
use phx_market::coupled_call::{NetworkCall, Placed};
use phx_market::linked_call::LinkedCall;
use phx_market::market::{Ration, TieRule};
use phx_market::order::{Asked, Order, Poster, Side, Timing};
use phx_num::{Missing, PriceRaw};
use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

const LENDERS: u64 = 1_000;
const BORROWERS: u64 = 5_000;
const REACH: u64 = 4;

struct MoneyMarket {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    orders: Vec<(usize, Order)>,
}

fn order(party: u64, side: Side, limit: i64, qty: i64) -> Order {
    let poster = Poster {
        party: PartyId::new(party + 1),
        market: MarketId::new(0),
        side,
        timing: Timing::AtTheClose,
        day: Day::new(1),
        reason: "fund",
        priority: Missing::Absent,
    };
    Order::new(poster, &[Asked { limit: Missing::Present(PriceRaw::from_raw(limit)), qty }], 1).unwrap()
}

/// The market on a day: `shift` moves every bid, as the next day's orders differ from the last.
fn money_market(shift: i64) -> MoneyMarket {
    let mut lot = Draws::new(stream_key(Seed::new(1), "MMK.bench"), Subject::new(SubjectTag::Market, 0), 1, 0);
    let mut draw = |n: u64| i64::try_from(below_u64(&mut lot, n)).unwrap();
    let mut nodes = Vec::new();
    let mut orders = Vec::new();
    for l in 0..LENDERS {
        let budget = 50 + draw(200);
        nodes.push(Node { key: l, cap: Missing::Present(budget), market: Missing::Absent, last: Missing::Absent });
        orders.push((nodes.len() - 1, order(l, Side::Sell, 300 + draw(100), budget)));
    }
    let mut edges = Vec::new();
    for b in 0..BORROWERS {
        let key = LENDERS + b;
        nodes.push(Node {
            key,
            cap: Missing::Absent,
            market: Missing::Present(MarketId::new(1)),
            last: Missing::Absent,
        });
        let at = nodes.len() - 1;
        orders.push((at, order(key, Side::Buy, 320 + draw(120) + shift, 10 + draw(60))));
        for k in 0..REACH {
            let lender = usize::try_from(draw(LENDERS)).unwrap();
            edges.push(Edge {
                from: lender,
                to: at,
                cap: Missing::Present(20 + draw(80)),
                cost: draw(20),
                key: b * REACH + k,
                owner: Missing::Absent,
            });
        }
    }
    MoneyMarket { nodes, edges, orders }
}

const TIES: &[TieRule] = &[TieRule::MaxVolume, TieRule::MinImbalance, TieRule::NearestLastPrint, TieRule::LowerPrice];

fn meet(linked: &mut LinkedCall, m: &MoneyMarket, day: u32) -> u64 {
    let placed: Vec<Placed> = m.orders.iter().map(|(n, o)| Placed { node: *n, order: o }).collect();
    let call = NetworkCall { nodes: &m.nodes, edges: &m.edges, orders: &placed, ties: TIES, ration: Ration::ProRata };
    let mut lot = Draws::new(stream_key(Seed::new(1), "MMK.call"), Subject::new(SubjectTag::Market, 0), day, 0);
    linked.meet(&call, &mut lot).pivots
}

fn cold() -> (LinkedCall, MoneyMarket) {
    (LinkedCall::new(), money_market(0))
}

/// The first day met, and the second day's market, every bid three ticks higher.
fn warm() -> (LinkedCall, MoneyMarket) {
    let mut linked = LinkedCall::new();
    let _ = meet(&mut linked, &money_market(0), 1);
    (linked, money_market(3))
}

#[library_benchmark]
#[bench::fresh(setup = cold)]
fn ir_linked_call_cold((mut linked, m): (LinkedCall, MoneyMarket)) -> u64 {
    meet(black_box(&mut linked), &m, 1)
}

#[library_benchmark]
#[bench::next_day(setup = warm)]
fn ir_linked_call_warm((mut linked, m): (LinkedCall, MoneyMarket)) -> u64 {
    meet(black_box(&mut linked), &m, 2)
}

library_benchmark_group!(name = linked_call, benchmarks = [ir_linked_call_cold, ir_linked_call_warm]);

main!(library_benchmark_groups = linked_call);
