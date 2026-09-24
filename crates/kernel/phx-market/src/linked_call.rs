use phx_macros::clause;
use phx_num::Missing;
use phx_rand::Draws;

use crate::coupled_call::{Edge, NetworkCall, NetworkCleared, Node, Placed, network_call};
use crate::simplex::Basis;

/// A linked call's state between meetings: the optimal basis of its last meeting. Where optima tie, the start decides
/// which flow the call reaches, so the basis is the world's, saved with the market, and a loaded world continues
/// exactly from it.
#[clause("MKT.3", "SET.15")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedCall {
    pub basis: Missing<Basis>,
}

impl Default for LinkedCall {
    fn default() -> LinkedCall {
        LinkedCall::new()
    }
}

impl LinkedCall {
    /// A call that has not met, with no basis to start from.
    #[must_use]
    pub fn new() -> LinkedCall {
        LinkedCall { basis: Missing::Absent }
    }

    /// One meeting: the call over lenders and borrowers linked by the lenders' budgets (a capacity on a node) and
    /// their reach to each borrower (a cost and a capacity on an edge), started from the last meeting's basis. A
    /// market node with no order today leaves the call with its edges, and each schedule meets as its breakpoints,
    /// the steps at one price being one arc.
    pub fn meet(&mut self, call: &NetworkCall<'_>, lot: &mut Draws) -> NetworkCleared {
        let bids = |v: usize| call.orders.iter().any(|o| o.node == v);
        let kept: Vec<usize> = (0..call.nodes.len())
            .filter(|v| matches!(call.nodes.get(*v).map(|n| n.market), Some(Missing::Absent)) || bids(*v))
            .collect();
        let index = |v: usize| kept.iter().position(|k| *k == v);
        let nodes: Vec<Node> = kept.iter().filter_map(|v| call.nodes.get(*v).copied()).collect();
        let mut edge_of = Vec::new();
        let mut edges: Vec<Edge> = Vec::new();
        for (i, e) in call.edges.iter().enumerate() {
            if let (Some(from), Some(to)) = (index(e.from), index(e.to)) {
                edges.push(Edge { from, to, ..*e });
                edge_of.push(i);
            }
        }
        let orders: Vec<Placed<'_>> =
            call.orders.iter().filter_map(|o| index(o.node).map(|node| Placed { node, order: o.order })).collect();
        let reduced =
            NetworkCall { nodes: &nodes, edges: &edges, orders: &orders, ties: call.ties, ration: call.ration };
        let start = match &self.basis {
            Missing::Present(b) => Missing::Present(b),
            Missing::Absent => Missing::Absent,
        };
        let met = network_call(&reduced, start, lot);
        self.basis = Missing::Present(met.basis.clone());
        let mut prices = vec![Missing::Absent; call.nodes.len()];
        for (k, v) in kept.iter().enumerate() {
            if let (Some(slot), Some(p)) = (prices.get_mut(*v), met.prices.get(k)) {
                *slot = *p;
            }
        }
        let mut flows = vec![0; call.edges.len()];
        for (k, i) in edge_of.iter().enumerate() {
            if let (Some(slot), Some(f)) = (flows.get_mut(*i), met.flows.get(k)) {
                *slot = *f;
            }
        }
        NetworkCleared { prices, flows, ..met }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::MarketId;
    use phx_num::Missing;

    use super::LinkedCall;
    use crate::coupled_call::tests::{TIES, lot, order};
    use crate::coupled_call::{Edge, NetworkCall, NetworkCleared, Node, Placed, network_call};
    use crate::market::Ration;
    use crate::order::{Order, Side};

    fn node(key: u64, cap: Option<i64>, market: bool) -> Node {
        Node {
            key,
            cap: cap.map_or(Missing::Absent, Missing::Present),
            market: if market { Missing::Present(MarketId::new(u16::try_from(key).unwrap())) } else { Missing::Absent },
            last: Missing::Absent,
        }
    }

    fn edge(from: usize, to: usize, cap: i64, cost: i64, key: u64) -> Edge {
        Edge { from, to, cap: Missing::Present(cap), cost, key, owner: Missing::Absent }
    }

    /// Two lenders with budgets, two borrowers, and the lenders' reach to each at a cost.
    fn market() -> (Vec<Node>, Vec<Edge>, Vec<(usize, Order)>) {
        let nodes =
            vec![node(10, Some(3), false), node(11, Some(2), false), node(20, None, true), node(21, None, true)];
        let edges = vec![edge(0, 2, 2, 1, 1), edge(0, 3, 2, 0, 2), edge(1, 2, 1, 2, 3), edge(1, 3, 2, 1, 4)];
        let orders = vec![
            (0, order(1, Side::Sell, 3, 3)),
            (1, order(2, Side::Sell, 2, 2)),
            (2, order(3, Side::Buy, 9, 2)),
            (3, order(4, Side::Buy, 5, 2)),
            (3, order(5, Side::Buy, 4, 1)),
        ];
        (nodes, edges, orders)
    }

    fn surplus(orders: &[(usize, Order)], edges: &[Edge], c: &NetworkCleared) -> i128 {
        let trade: i128 = orders
            .iter()
            .zip(&c.fills)
            .map(|((_, o), f)| {
                let v = i128::from(o.steps[0].limit.raw()) * i128::from(*f);
                if o.side == Side::Buy { v } else { -v }
            })
            .sum();
        let carried: i128 = edges.iter().zip(&c.flows).map(|(e, f)| i128::from(e.cost) * i128::from(*f)).sum();
        trade - carried
    }

    /// The best surplus over every fill and edge flow that conserves each node and keeps each budget.
    fn brute(nodes: &[Node], edges: &[Edge], orders: &[(usize, Order)]) -> i128 {
        let limits: Vec<i64> = orders
            .iter()
            .map(|(_, o)| o.steps[0].qty)
            .chain(edges.iter().map(|e| match e.cap {
                Missing::Present(c) => c,
                Missing::Absent => panic!(),
            }))
            .collect();
        let mut x = vec![0_i64; limits.len()];
        let mut best = 0_i128;
        loop {
            let (fills, flows) = x.split_at(orders.len());
            let mut balance = vec![0_i64; nodes.len()];
            let mut through = vec![0_i64; nodes.len()];
            let mut value = 0_i128;
            for ((v, o), f) in orders.iter().zip(fills) {
                let p = i128::from(o.steps[0].limit.raw());
                match o.side {
                    Side::Sell => {
                        balance[*v] += f;
                        through[*v] += f;
                        value -= p * i128::from(*f);
                    }
                    Side::Buy => {
                        balance[*v] -= f;
                        value += p * i128::from(*f);
                    }
                }
            }
            for (e, f) in edges.iter().zip(flows) {
                balance[e.from] -= f;
                balance[e.to] += f;
                through[e.to] += f;
                value -= i128::from(e.cost) * i128::from(*f);
            }
            let within = nodes.iter().zip(&through).all(|(n, t)| match n.cap {
                Missing::Present(c) => *t <= c,
                Missing::Absent => true,
            });
            if within && balance.iter().all(|b| *b == 0) && value > best {
                best = value;
            }
            let mut i = 0;
            loop {
                if i == x.len() {
                    return best;
                }
                x[i] += 1;
                if x[i] <= limits[i] {
                    break;
                }
                x[i] = 0;
                i += 1;
            }
        }
    }

    fn call<'a>(nodes: &'a [Node], edges: &'a [Edge], placed: &'a [Placed<'a>]) -> NetworkCall<'a> {
        NetworkCall { nodes, edges, orders: placed, ties: TIES, ration: Ration::ProRata }
    }

    #[test]
    fn linked_call_matches_lp_small() {
        let (nodes, edges, orders) = market();
        let placed: Vec<Placed> = orders.iter().map(|(n, o)| Placed { node: *n, order: o }).collect();
        let c = network_call(&call(&nodes, &edges, &placed), Missing::Absent, &mut lot());
        assert_eq!(surplus(&orders, &edges, &c), brute(&nodes, &edges, &orders));
        let lent: i64 = c.fills[..2].iter().sum();
        assert!(c.fills[0] <= 3 && c.fills[1] <= 2 && lent == c.flows.iter().sum::<i64>(), "within the budgets");
    }

    #[test]
    fn linked_call_warm_start_same_optimum() {
        let (nodes, edges, orders) = market();
        let placed: Vec<Placed> = orders.iter().map(|(n, o)| Placed { node: *n, order: o }).collect();
        let mut linked = LinkedCall::new();
        let first = linked.meet(&call(&nodes, &edges, &placed), &mut lot());
        // The next day a borrower bids more and a lender's reach costs more.
        let mut next = orders.clone_orders();
        next[3] = (3, order(4, Side::Buy, 7, 2));
        let mut dearer = edges.clone();
        dearer[1].cost = 2;
        let placed: Vec<Placed> = next.iter().map(|(n, o)| Placed { node: *n, order: o }).collect();
        let warm = linked.meet(&call(&nodes, &dearer, &placed), &mut lot());
        let cold = network_call(&call(&nodes, &dearer, &placed), Missing::Absent, &mut lot());
        assert_eq!(surplus(&next, &dearer, &warm), surplus(&next, &dearer, &cold));
        assert_eq!(surplus(&next, &dearer, &warm), brute(&nodes, &dearer, &next));
        assert!(first.pivots > 0);
    }

    #[test]
    fn linked_call_same_basis_same_flow() {
        let (nodes, edges, orders) = market();
        let placed: Vec<Placed> = orders.iter().map(|(n, o)| Placed { node: *n, order: o }).collect();
        let mut seed = LinkedCall::new();
        let _ = seed.meet(&call(&nodes, &edges, &placed), &mut lot());
        let (mut a, mut b) = (seed.clone(), seed);
        let first = a.meet(&call(&nodes, &edges, &placed), &mut lot());
        let second = b.meet(&call(&nodes, &edges, &placed), &mut lot());
        assert_eq!(first, second, "one basis and one input give one flow");
        assert_eq!(first.pivots, 0, "from its own optimal basis the call needs no pivot");
        // A borrower with no bid leaves the call and has no price.
        let without: Vec<Placed> = placed.iter().filter(|p| p.node != 2).copied().collect();
        let quiet = a.meet(&call(&nodes, &edges, &without), &mut lot());
        assert_eq!(quiet.prices[2], Missing::Absent);
        assert_eq!((quiet.flows[0], quiet.flows[2]), (0, 0), "its edges carry nothing");
    }

    trait CloneOrders {
        fn clone_orders(&self) -> Vec<(usize, Order)>;
    }

    impl CloneOrders for Vec<(usize, Order)> {
        fn clone_orders(&self) -> Vec<(usize, Order)> {
            self.iter()
                .map(|(n, o)| (*n, order(o.party.get(), o.side, o.steps[0].limit.raw(), o.steps[0].qty)))
                .collect()
        }
    }
}
