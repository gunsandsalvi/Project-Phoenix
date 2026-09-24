use std::collections::BTreeMap;

use phx_id::{MarketId, PartyId};
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, capacity_exceeded, violation};
use phx_rand::Draws;

use crate::call::{lesser, pair_net, ration};
use crate::consts::{KEY_SHIFT, NODE_BITS, TAG_BID, TAG_EDGE, TAG_NODE, TAG_OFFER, TAG_RETURN, TAG_SHIFT};
use crate::market::{Ration, TieRule};
use crate::order::{Order, Side};
use crate::print::Match;
use crate::simplex::{Arc, Basis, Network, at, solve};

/// A node of a call over a network: a zone, a lender or a borrower. Its key names it across meetings; a capacity
/// bounds what passes through it; a node that is a market has its price chosen and its last print read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    pub key: u64,
    pub cap: Missing<i64>,
    pub market: Missing<MarketId>,
    pub last: Missing<PriceRaw>,
}

/// An edge between two nodes: a line between zones, which a named owner runs, or a lender's reach to a borrower,
/// with its capacity in each unit of the traded thing and its cost per unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub cap: Missing<i64>,
    pub cost: i64,
    pub key: u64,
    pub owner: Missing<PartyId>,
}

/// An order placed at a node of the network.
#[derive(Clone, Copy, Debug)]
pub struct Placed<'a> {
    pub node: usize,
    pub order: &'a Order,
}

/// A call over a network as it meets: its nodes, edges and orders, with the operator's tie sequence and rationing.
#[derive(Clone, Copy, Debug)]
pub struct NetworkCall<'a> {
    pub nodes: &'a [Node],
    pub edges: &'a [Edge],
    pub orders: &'a [Placed<'a>],
    pub ties: &'a [TieRule],
    pub ration: Ration,
}

/// What a call over a network came to: each market node's price where one is defined, each order's fill, each edge's
/// flow, the optimal basis the next meeting may start from, and the pivots it took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkCleared {
    pub prices: Vec<Missing<PriceRaw>>,
    pub fills: Vec<i64>,
    pub flows: Vec<i64>,
    pub basis: Basis,
    pub pivots: u64,
    pub warm: bool,
}

const SOURCE: usize = 0;
const SINK: usize = 1;

/// An arc's key from its kind, the node or edge it belongs to and the rank of its step, the same in every meeting.
fn arc_key(tag: u128, owner: u64, rank: i64) -> u128 {
    if owner >> NODE_BITS != 0 {
        capacity_exceeded!("a network node's or edge's key", 1_u64 << NODE_BITS, owner);
    }
    (u128::from(owner) << KEY_SHIFT) | (tag << TAG_SHIFT) | u128::from(rank.cast_unsigned())
}

/// The steps of one side at one node and price, gathered into one arc: which orders they came from, what each asked
/// and its priority.
type Gathered = BTreeMap<(usize, Side, i64), Vec<(usize, i128, Missing<u32>)>>;

/// The simplex's nodes for a call node: where offers and incoming edges arrive and where bids and outgoing edges
/// leave, the same node unless a capacity lies between them.
fn ends(nodes: &[Node]) -> Vec<(usize, usize)> {
    let mut next = SINK + 1;
    nodes
        .iter()
        .map(|n| {
            let inn = next;
            next += 1;
            if matches!(n.cap, Missing::Present(_)) {
                next += 1;
                (inn, inn + 1)
            } else {
                (inn, inn)
            }
        })
        .collect()
}

fn end(ends: &[(usize, usize)], v: usize) -> (usize, usize) {
    let Some(e) = ends.get(v) else {
        violation!(clause = "MKT.3", "an order or edge at no node of the call", node = v);
    };
    *e
}

/// Arcs `(from, to, cost)` by the node they leave, and by the node they enter, for searches either way.
struct Adjacency {
    out: Vec<Vec<(usize, i128)>>,
    into: Vec<Vec<(usize, i128)>>,
}

impl Adjacency {
    fn new(count: usize, arcs: &[(usize, usize, i128)]) -> Adjacency {
        let mut adj = Adjacency { out: vec![Vec::new(); count], into: vec![Vec::new(); count] };
        for &(tail, head, cost) in arcs {
            adj.add(tail, head, cost);
        }
        adj
    }

    fn add(&mut self, tail: usize, head: usize, cost: i128) {
        if let (Some(o), Some(i)) = (self.out.get_mut(tail), self.into.get_mut(head)) {
            o.push((head, cost));
            i.push((tail, cost));
        }
    }

    /// Shortest distances from (or, reversed, to) the nodes already reached, lowered from `origin` at `value` by the
    /// queue-based Bellman–Ford: a node's distance changes only where a path through `origin` is shorter, so the
    /// search stops wherever nothing improves. The residual arcs of an optimal flow have no negative cycle; a node
    /// taken more often than there are nodes would prove one.
    fn relax(&self, dist: &mut [Missing<i128>], origin: usize, value: i128, reversed: bool) {
        let count = self.out.len();
        let lists = if reversed { &self.into } else { &self.out };
        let mut queued = vec![false; count];
        let mut taken = vec![0_usize; count];
        let mut queue = std::collections::VecDeque::new();
        if let Some(first) = dist.get_mut(origin)
            && !matches!(*first, Missing::Present(k) if k <= value)
        {
            *first = Missing::Present(value);
            queue.push_back(origin);
        }
        while let Some(near) = queue.pop_front() {
            if let Some(q) = queued.get_mut(near) {
                *q = false;
            }
            let Some(Missing::Present(reached)) = dist.get(near).copied() else { continue };
            if let Some(t) = taken.get_mut(near) {
                *t += 1;
                if *t > count {
                    violation!(clause = "MKT.3", "a residual cycle of negative cost after an optimal call");
                }
            }
            for &(far, cost) in lists.get(near).map_or(&[][..], Vec::as_slice) {
                let Some(known) = dist.get_mut(far) else { continue };
                if matches!(*known, Missing::Present(k) if k <= reached + cost) {
                    continue;
                }
                *known = Missing::Present(reached + cost);
                if let Some(q) = queued.get_mut(far)
                    && !*q
                {
                    *q = true;
                    queue.push_back(far);
                }
            }
        }
    }
}

/// A node's own supply and demand at a price, and what it brings in over its edges net of what it sends.
struct Local {
    offers: Vec<(i64, i128)>,
    bids: Vec<(i64, i128)>,
    imported: i128,
}

impl Local {
    fn supply(&self, p: i64) -> i128 {
        self.offers.iter().filter(|(l, _)| *l <= p).map(|(_, q)| q).sum()
    }

    fn demand(&self, p: i64) -> i128 {
        self.bids.iter().filter(|(l, _)| *l >= p).map(|(_, q)| q).sum()
    }
}

/// A node's price chosen within the range its potentials allow, by the call's tie sequence over the node's own steps:
/// the most volume, the least imbalance with what the network brings, the nearest the last print, the lower price.
fn choose(
    range: (Missing<i128>, Missing<i128>),
    local: &Local,
    last: Missing<PriceRaw>,
    ties: &[TieRule],
) -> Missing<i64> {
    let inside = |p: i128| {
        let above = match range.0 {
            Missing::Present(lo) => p >= lo,
            Missing::Absent => true,
        };
        let below = match range.1 {
            Missing::Present(hi) => p <= hi,
            Missing::Absent => true,
        };
        above && below
    };
    let mut candidates: Vec<i128> = local.offers.iter().chain(&local.bids).map(|(l, _)| i128::from(*l)).collect();
    for bound in [range.0, range.1] {
        if let Missing::Present(b) = bound {
            candidates.push(b);
        }
    }
    if let Missing::Present(l) = last {
        candidates.push(i128::from(l.raw()));
    }
    candidates.retain(|p| inside(*p));
    candidates.sort_unstable();
    candidates.dedup();
    let mut kept: Vec<i64> = candidates.into_iter().filter_map(|p| i64::try_from(p).ok()).collect();
    for rule in ties {
        let score = |p: i64| -> Option<i128> {
            let (s, d) = (local.supply(p) + local.imported, local.demand(p));
            match rule {
                TieRule::MaxVolume => Some(-lesser(s, d)),
                TieRule::MinImbalance => Some((s - d).abs()),
                TieRule::NearestLastPrint => match last {
                    Missing::Present(l) => Some((i128::from(p) - i128::from(l.raw())).abs()),
                    Missing::Absent => None,
                },
                TieRule::LowerPrice => Some(i128::from(p)),
            }
        };
        let Some(best) = kept.iter().filter_map(|p| score(*p)).reduce(|b, s| if s < b { s } else { b }) else {
            continue;
        };
        kept.retain(|p| score(*p) == Some(best));
    }
    match kept.as_slice() {
        [] => Missing::Absent,
        [p] => Missing::Present(*p),
        _ => violation!(clause = "MKT.21", "a call's tie sequence left more than one price", left = kept.len()),
    }
}

/// A call's network as the simplex reads it, with what is needed to read its flow back: the gathered steps in arc
/// order, the call nodes' ends, where the edges' arcs begin, and the scale that puts surplus before volume.
struct Built {
    network: Network,
    gathered: Gathered,
    ends: Vec<(usize, usize)>,
    edges_at: usize,
    scale: i128,
}

/// The call's min-cost circulation: offer steps from the source into their node at their price, bid steps from their
/// node to the sink at minus theirs, edges at their cost within their capacity, a capacity through a node, and the
/// sink back to the source at a cost of one a unit, so among flows of equal surplus the one trading most wins.
fn build(call: &NetworkCall<'_>) -> Built {
    let ends = ends(call.nodes);
    let mut gathered: Gathered = BTreeMap::new();
    for (i, placed) in call.orders.iter().enumerate() {
        for step in &placed.order.steps {
            gathered.entry((placed.node, placed.order.side, step.limit.raw())).or_default().push((
                i,
                i128::from(step.qty),
                placed.order.priority,
            ));
        }
    }
    let offered: i128 =
        gathered.iter().filter(|((_, s, _), _)| *s == Side::Sell).flat_map(|(_, v)| v).map(|m| m.1).sum();
    let scale = offered + 1;
    let mut arcs: Vec<Arc> = Vec::new();
    // A step's arc is keyed by its rank in its node's schedule, not its price, so a schedule that moves keeps its
    // arcs for the next meeting's start.
    let mut rank = 0_i64;
    let mut group: Option<(usize, Side)> = None;
    for ((v, side, price), members) in &gathered {
        if group == Some((*v, *side)) {
            rank += 1;
        } else {
            group = Some((*v, *side));
            rank = 0;
        }
        let (inn, out) = end(&ends, *v);
        let node_key = at(call.nodes, *v).key;
        let total: i128 = members.iter().map(|m| m.1).sum();
        let Ok(cap) = i64::try_from(total) else {
            capacity_exceeded!("a call's steps at one price", i64::MAX, 0);
        };
        if inn != out && call.orders.iter().any(|o| o.node == *v && o.order.side != *side) {
            violation!(clause = "MKT.3", "a node with a capacity posts on both sides", node = *v);
        }
        let cost = i128::from(*price) * scale;
        arcs.push(match side {
            Side::Sell => {
                Arc { from: SOURCE, to: inn, cap: Missing::Present(cap), cost, key: arc_key(TAG_OFFER, node_key, rank) }
            }
            Side::Buy => Arc {
                from: out,
                to: SINK,
                cap: Missing::Present(cap),
                cost: -cost,
                key: arc_key(TAG_BID, node_key, rank),
            },
        });
    }
    let mut keys: Vec<u128> = vec![arc_key(TAG_RETURN, 0, 1), arc_key(TAG_RETURN, 0, 2)];
    for (v, node) in call.nodes.iter().enumerate() {
        let (inn, out) = end(&ends, v);
        keys.push(arc_key(TAG_NODE, node.key, 1));
        if inn != out {
            keys.push(arc_key(TAG_NODE, node.key, 2));
            arcs.push(Arc { from: inn, to: out, cap: node.cap, cost: 0, key: arc_key(TAG_NODE, node.key, 0) });
        }
    }
    let edges_at = arcs.len();
    for edge in call.edges {
        let (_, from) = end(&ends, edge.from);
        let (to, _) = end(&ends, edge.to);
        let cost = i128::from(edge.cost) * scale;
        arcs.push(Arc { from, to, cap: edge.cap, cost, key: arc_key(TAG_EDGE, edge.key, 0) });
    }
    arcs.push(Arc { from: SINK, to: SOURCE, cap: Missing::Absent, cost: -1, key: arc_key(TAG_RETURN, 0, 0) });
    Built { network: Network { nodes: keys, arcs }, gathered, ends, edges_at, scale }
}

/// The residual arcs of a flow at the arcs' own costs, unscaled, which bound the optimal potentials.
fn residual(built: &Built, flow: &[i64]) -> Vec<(usize, usize, i128)> {
    let last = built.network.arcs.len() - 1;
    let mut out = Vec::new();
    for (a, arc) in built.network.arcs.iter().enumerate() {
        let own = if a == last { 0 } else { arc.cost / built.scale };
        let f = at(flow, a);
        if matches!(arc.cap, Missing::Absent) || matches!(arc.cap, Missing::Present(c) if f < c) {
            out.push((arc.from, arc.to, own));
        }
        if f > 0 {
            out.push((arc.to, arc.from, -own));
        }
    }
    out
}

/// Each call node's own steps and what its edges bring it net.
fn locals(call: &NetworkCall<'_>, gathered: &Gathered, flows: &[i64]) -> Vec<Local> {
    let mut out: Vec<Local> =
        call.nodes.iter().map(|_| Local { offers: Vec::new(), bids: Vec::new(), imported: 0 }).collect();
    for ((v, side, price), members) in gathered {
        let q: i128 = members.iter().map(|m| m.1).sum();
        if let Some(here) = out.get_mut(*v) {
            match side {
                Side::Sell => here.offers.push((*price, q)),
                Side::Buy => here.bids.push((*price, q)),
            }
        }
    }
    for (edge, f) in call.edges.iter().zip(flows) {
        if let Some(here) = out.get_mut(edge.to) {
            here.imported += i128::from(*f);
        }
        if let Some(here) = out.get_mut(edge.from) {
            here.imported -= i128::from(*f);
        }
    }
    out
}

/// Each market node's price, in the nodes' order: within the range the optimal potentials allow given the prices
/// already chosen, by the tie sequence. The ranges are the distances from the source and, negated, to it over the
/// residual arcs; a choice inside a node's range binds the nodes after it, and is carried to their ranges by a
/// search from the node that stops wherever no bound tightens.
fn prices(
    call: &NetworkCall<'_>,
    built: &Built,
    residual: &[(usize, usize, i128)],
    flows: &[i64],
) -> Vec<Missing<PriceRaw>> {
    let count = built.network.nodes.len();
    let mut adj = Adjacency::new(count, residual);
    let mut hi = vec![Missing::Absent; count];
    let mut below = vec![Missing::Absent; count];
    adj.relax(&mut hi, SOURCE, 0, false);
    adj.relax(&mut below, SOURCE, 0, true);
    let locals = locals(call, &built.gathered, flows);
    let mut out = vec![Missing::Absent; call.nodes.len()];
    for (v, (node, here)) in call.nodes.iter().zip(&locals).enumerate() {
        if matches!(node.market, Missing::Absent) {
            continue;
        }
        let (inn, out_end) = end(&built.ends, v);
        let end_at = if here.bids.is_empty() { inn } else { out_end };
        let upper = at(&hi, end_at);
        let lower = match at(&below, end_at) {
            Missing::Present(d) => Missing::Present(-d),
            Missing::Absent => Missing::Absent,
        };
        let Missing::Present(p) = choose((lower, upper), here, node.last, call.ties) else { continue };
        if lower != upper {
            let price = i128::from(p);
            adj.add(SOURCE, end_at, price);
            adj.add(end_at, SOURCE, -price);
            adj.relax(&mut hi, end_at, price, false);
            adj.relax(&mut below, end_at, -price, true);
        }
        if let Some(slot) = out.get_mut(v) {
            *slot = Missing::Present(PriceRaw::from_raw(p));
        }
    }
    out
}

/// The call over a network, solved exactly as a min-cost circulation by the network simplex, surplus first and
/// volume second, so steps whose trade adds nothing still trade, as in a call. Equal steps at one node and price are
/// one arc whose flow is rationed among them by the market's rule; each market node's price is chosen within its
/// potentials' range by the tie sequence, in the nodes' order.
#[clause("MKT.3", "MKT.13", "MKT.17", "MKT.21")]
pub fn network_call(call: &NetworkCall<'_>, start: Missing<&Basis>, lot: &mut Draws) -> NetworkCleared {
    let built = build(call);
    let solution = solve(&built.network, start);
    let flow = |a: usize| at(&solution.flow, a);
    let mut fills = vec![0_i128; call.orders.len()];
    for (a, members) in built.gathered.values().enumerate() {
        for (m, got) in members.iter().zip(ration(i128::from(flow(a)), members, call.ration, lot)) {
            if let Some(f) = fills.get_mut(m.0) {
                *f += got;
            }
        }
    }
    let flows: Vec<i64> = (0..call.edges.len()).map(|i| flow(built.edges_at + i)).collect();
    let prices = prices(call, &built, &residual(&built, &solution.flow), &flows);
    let fills = fills
        .into_iter()
        .map(|f| {
            let Ok(f) = i64::try_from(f) else {
                capacity_exceeded!("an order's fill", i64::MAX, 0);
            };
            f
        })
        .collect();
    NetworkCleared { prices, fills, flows, basis: solution.basis, pivots: solution.pivots, warm: solution.warm }
}

/// The matches of a coupled call, zone by zone at the zone's price: each zone's buyers and sellers paired, a line's
/// owner selling in the zone its flow enters and buying in the zone it leaves, so what the line carries between two
/// prices is its owner's congestion rent and every match is between two named parties at one price.
#[clause("MKT.11", "MKT.13")]
#[must_use]
pub fn zone_matches(call: &NetworkCall<'_>, cleared: &NetworkCleared) -> Vec<Vec<Match>> {
    let mut out = Vec::with_capacity(call.nodes.len());
    for (v, price) in cleared.prices.iter().enumerate() {
        let Missing::Present(price) = *price else {
            out.push(Vec::new());
            continue;
        };
        let mut net: BTreeMap<PartyId, i128> = BTreeMap::new();
        for (placed, fill) in call.orders.iter().zip(&cleared.fills) {
            if placed.node == v && *fill > 0 {
                let signed = match placed.order.side {
                    Side::Buy => i128::from(*fill),
                    Side::Sell => -i128::from(*fill),
                };
                *net.entry(placed.order.party).or_insert(0) += signed;
            }
        }
        for (e, f) in call.edges.iter().zip(&cleared.flows) {
            if *f == 0 {
                continue;
            }
            let Missing::Present(owner) = e.owner else {
                violation!(clause = "MKT.11", "a line that carried flow with no owner to trade it", edge = e.key);
            };
            if e.to == v {
                *net.entry(owner).or_insert(0) -= i128::from(*f);
            }
            if e.from == v {
                *net.entry(owner).or_insert(0) += i128::from(*f);
            }
        }
        out.push(pair_net(&net, price));
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use phx_id::{Day, MarketId, PartyId};
    use phx_num::{Missing, PriceRaw};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Edge, NetworkCall, NetworkCleared, Node, Placed, network_call, zone_matches};
    use crate::market::{Ration, TieRule};
    use crate::order::{Asked, Order, Poster, Side, Timing};

    pub(crate) const TIES: &[TieRule] =
        &[TieRule::MaxVolume, TieRule::MinImbalance, TieRule::NearestLastPrint, TieRule::LowerPrice];

    pub(crate) fn lot() -> Draws {
        Draws::new(stream_key(Seed::new(1), "MKT.coupled"), Subject::new(SubjectTag::Market, 0), 1, 0)
    }

    pub(crate) fn order(party: u64, side: Side, limit: i64, qty: i64) -> Order {
        let poster = Poster {
            party: PartyId::new(party),
            market: MarketId::new(0),
            side,
            timing: Timing::AtTheClose,
            day: Day::new(1),
            reason: "trade",
            priority: Missing::Absent,
        };
        Order::new(poster, &[Asked { limit: Missing::Present(PriceRaw::from_raw(limit)), qty }], 1).unwrap()
    }

    pub(crate) fn zone(key: u64) -> Node {
        Node {
            key,
            cap: Missing::Absent,
            market: Missing::Present(MarketId::new(u16::try_from(key).unwrap())),
            last: Missing::Absent,
        }
    }

    fn line(a: usize, b: usize, cap: i64, key: u64) -> Edge {
        Edge { from: a, to: b, cap: Missing::Present(cap), cost: 0, key, owner: Missing::Present(PartyId::new(900)) }
    }

    /// The best surplus any fills and line flows reach on a chain of zones, by trying them all.
    fn brute(zones: usize, orders: &[(usize, &Order)], caps: &[i64]) -> i128 {
        let limits: Vec<i64> = orders.iter().map(|(_, o)| o.steps[0].qty).collect();
        let mut fill = vec![0_i64; orders.len()];
        let mut best = 0_i128;
        loop {
            // Net export of each zone; on a chain the line flows follow from them.
            let mut export = vec![0_i128; zones];
            let mut surplus = 0_i128;
            for ((z, o), f) in orders.iter().zip(&fill) {
                let (p, f) = (i128::from(o.steps[0].limit.raw()), i128::from(*f));
                match o.side {
                    Side::Sell => {
                        export[*z] += f;
                        surplus -= p * f;
                    }
                    Side::Buy => {
                        export[*z] -= f;
                        surplus += p * f;
                    }
                }
            }
            let mut carried = 0_i128;
            let mut ok = export.iter().sum::<i128>() == 0;
            for (z, cap) in caps.iter().enumerate() {
                carried += export[z];
                ok &= carried.abs() <= i128::from(*cap);
            }
            if ok && surplus > best {
                best = surplus;
            }
            let mut i = 0;
            loop {
                if i == fill.len() {
                    return best;
                }
                fill[i] += 1;
                if fill[i] <= limits[i] {
                    break;
                }
                fill[i] = 0;
                i += 1;
            }
        }
    }

    fn surplus(orders: &[(usize, &Order)], c: &NetworkCleared) -> i128 {
        orders
            .iter()
            .zip(&c.fills)
            .map(|((_, o), f)| {
                let v = i128::from(o.steps[0].limit.raw()) * i128::from(*f);
                if o.side == Side::Buy { v } else { -v }
            })
            .sum()
    }

    fn chain(zones: usize, caps: &[i64]) -> (Vec<Node>, Vec<Edge>) {
        let nodes = (0..zones).map(|z| zone(u64::try_from(z).unwrap())).collect();
        let mut edges = Vec::new();
        for (i, cap) in caps.iter().enumerate() {
            let k = 2 * u64::try_from(i).unwrap();
            edges.push(line(i, i + 1, *cap, k));
            edges.push(line(i + 1, i, *cap, k + 1));
        }
        (nodes, edges)
    }

    fn run(nodes: &[Node], edges: &[Edge], orders: &[(usize, &Order)]) -> NetworkCleared {
        let placed: Vec<Placed> = orders.iter().map(|(n, o)| Placed { node: *n, order: o }).collect();
        let call = NetworkCall { nodes, edges, orders: &placed, ties: TIES, ration: Ration::ProRata };
        network_call(&call, Missing::Absent, &mut lot())
    }

    #[test]
    fn coupled_call_matches_lp_small() {
        type Case = (usize, Vec<i64>, Vec<(usize, Order)>);
        let cases: Vec<Case> = vec![
            (
                2,
                vec![1],
                vec![
                    (0, order(1, Side::Sell, 10, 3)),
                    (1, order(2, Side::Sell, 40, 3)),
                    (1, order(3, Side::Buy, 50, 3)),
                    (0, order(4, Side::Buy, 20, 1)),
                ],
            ),
            (
                3,
                vec![2, 1],
                vec![
                    (0, order(1, Side::Sell, -5, 3)),
                    (2, order(2, Side::Buy, 30, 3)),
                    (1, order(3, Side::Sell, 20, 2)),
                    (1, order(4, Side::Buy, 25, 2)),
                    (2, order(5, Side::Sell, 28, 1)),
                ],
            ),
            (
                3,
                vec![3, 3],
                vec![
                    (0, order(1, Side::Sell, 5, 2)),
                    (0, order(2, Side::Sell, 5, 2)),
                    (2, order(3, Side::Buy, 9, 3)),
                    (1, order(4, Side::Buy, 4, 3)),
                ],
            ),
        ];
        for (zones, caps, orders) in &cases {
            let (nodes, edges) = chain(*zones, caps);
            let refs: Vec<(usize, &Order)> = orders.iter().map(|(z, o)| (*z, o)).collect();
            let c = run(&nodes, &edges, &refs);
            assert_eq!(surplus(&refs, &c), brute(*zones, &refs, caps), "the greatest surplus the lines allow");
            for (z, price) in c.prices.iter().enumerate() {
                let Missing::Present(p) = price else { continue };
                for ((oz, o), f) in refs.iter().zip(&c.fills) {
                    if *oz != z {
                        continue;
                    }
                    let (l, q) = (o.steps[0].limit.raw(), o.steps[0].qty);
                    let better = match o.side {
                        Side::Buy => l > p.raw(),
                        Side::Sell => l < p.raw(),
                    };
                    if better {
                        assert_eq!(*f, q, "a step better than its zone's price fills in full");
                    }
                    let worse = match o.side {
                        Side::Buy => l < p.raw(),
                        Side::Sell => l > p.raw(),
                    };
                    if worse {
                        assert_eq!(*f, 0, "a step worse than its zone's price trades nothing");
                    }
                }
            }
            let placed: Vec<Placed> = refs.iter().map(|(n, o)| Placed { node: *n, order: o }).collect();
            let call =
                NetworkCall { nodes: &nodes, edges: &edges, orders: &placed, ties: TIES, ration: Ration::ProRata };
            for m in zone_matches(&call, &c).iter().flatten() {
                assert!(m.qty > 0 && m.buyer != crate::print::Buyer::Party(m.seller));
            }
        }
    }

    #[test]
    fn congested_line_separates_prices() {
        // Cheap power in zone 0 at 10, dear in zone 1 at 40, and 3 wanted in zone 1 at up to 50.
        let orders =
            [(0, order(1, Side::Sell, 10, 3)), (1, order(2, Side::Sell, 40, 3)), (1, order(3, Side::Buy, 50, 3))];
        let refs: Vec<(usize, &Order)> = orders.iter().map(|(z, o)| (*z, o)).collect();
        let (nodes, wide) = chain(2, &[5]);
        let open = run(&nodes, &wide, &refs);
        assert_eq!(open.prices[0], open.prices[1], "zones joined by a line that does not bind share a price");
        let (_, narrow) = chain(2, &[1]);
        let congested = run(&nodes, &narrow, &refs);
        let (Missing::Present(a), Missing::Present(b)) = (congested.prices[0], congested.prices[1]) else { panic!() };
        assert!(a.raw() < b.raw(), "a line at its capacity separates the prices: {a:?} {b:?}");
        assert_eq!(b.raw(), 40, "the dear zone's own offer sets its price");
        let placed: Vec<Placed> = refs.iter().map(|(n, o)| Placed { node: *n, order: o }).collect();
        let call = NetworkCall { nodes: &nodes, edges: &narrow, orders: &placed, ties: TIES, ration: Ration::ProRata };
        let matches = zone_matches(&call, &congested);
        let owner = PartyId::new(900);
        let rent: i128 = matches
            .iter()
            .flatten()
            .map(|m| {
                let v = i128::from(m.price.raw()) * i128::from(m.qty);
                if m.seller == owner {
                    v
                } else if m.buyer == crate::print::Buyer::Party(owner) {
                    -v
                } else {
                    0
                }
            })
            .sum();
        assert_eq!(rent, i128::from(b.raw() - a.raw()), "the line's owner keeps the difference on the unit it carried");
    }
}
