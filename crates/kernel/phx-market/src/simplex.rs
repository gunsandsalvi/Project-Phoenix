use std::collections::BTreeMap;

use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};

/// An arc of a network: its ends, its capacity (absent where nothing bounds it), its cost per unit of flow, and a
/// key naming it across meetings, which a saved basis refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arc {
    pub from: usize,
    pub to: usize,
    pub cap: Missing<i64>,
    pub cost: i128,
    pub key: u128,
}

/// An optimal basis as a later meeting starts from it: the keys of its tree arcs, of the arcs at their capacity, and
/// of the nodes its tree hangs from the root.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Basis {
    pub tree: Vec<u128>,
    pub upper: Vec<u128>,
    pub hung: Vec<u128>,
}

/// A network: its nodes by key, each key naming the node across meetings, and its arcs between them by index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Network {
    pub nodes: Vec<u128>,
    pub arcs: Vec<Arc>,
}

/// A min-cost circulation: each arc's flow, each node's potential, the basis that proves them optimal, and the
/// pivots it took. Every arc with flow strictly within its bounds has zero reduced cost, `cost + p(from) - p(to)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solution {
    pub flow: Vec<i64>,
    pub potential: Vec<i128>,
    pub basis: Basis,
    pub pivots: u64,
    pub warm: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Tree,
    Lower,
    Upper,
}

/// A capacity as the solver reads it: none where nothing bounds the arc.
fn bound(cap: Missing<i64>) -> Option<i64> {
    match cap {
        Missing::Present(c) => Some(c),
        Missing::Absent => None,
    }
}

pub(crate) fn at<T: Copy>(v: &[T], i: usize) -> T {
    let Some(x) = v.get(i) else {
        violation!(clause = "MKT.3", "a network read beyond its nodes or arcs", index = i);
    };
    *x
}

fn slot<T>(v: &mut [T], i: usize) -> &mut T {
    let Some(x) = v.get_mut(i) else {
        violation!(clause = "MKT.3", "a network written beyond its nodes or arcs", index = i);
    };
    x
}

/// The network simplex's working state over the real arcs and one artificial arc from each node to an added root.
struct Net {
    from: Vec<usize>,
    to: Vec<usize>,
    cap: Vec<Option<i64>>,
    cost: Vec<i128>,
    flow: Vec<i64>,
    state: Vec<State>,
    parent: Vec<Option<usize>>,
    pred: Vec<usize>,
    depth: Vec<u32>,
    children: Vec<Vec<usize>>,
    pi: Vec<i128>,
    root: usize,
    next: usize,
    pivots: u64,
}

impl Net {
    fn residual_up(&self, a: usize) -> Option<i64> {
        at(&self.cap, a).map(|c| c - at(&self.flow, a))
    }

    fn reduced(&self, a: usize) -> i128 {
        at(&self.cost, a) + at(&self.pi, at(&self.from, a)) - at(&self.pi, at(&self.to, a))
    }

    /// How much more can pass along a tree arc in the direction from `x` towards `y`, where one is the other's
    /// parent; absent when nothing bounds it.
    fn room(&self, a: usize, x: usize) -> Option<i64> {
        if at(&self.from, a) == x { self.residual_up(a) } else { Some(at(&self.flow, a)) }
    }

    /// The entering arc by block pricing from where the last search stopped: within each block the arc whose reduced
    /// cost is most in its favour, and none when no arc improves the circulation.
    fn entering(&mut self) -> Option<usize> {
        let m = self.from.len();
        let block = m.isqrt() + 1;
        let mut scanned = 0;
        while scanned < m {
            let mut best: Option<(i128, usize)> = None;
            for _ in 0..block {
                if scanned == m {
                    break;
                }
                let a = self.next;
                self.next = (self.next + 1) % m;
                scanned += 1;
                let gain = match at(&self.state, a) {
                    State::Lower if self.residual_up(a) != Some(0) => -self.reduced(a),
                    State::Upper => self.reduced(a),
                    _ => 0,
                };
                if gain > 0 && best.is_none_or(|(g, _)| gain > g) {
                    best = Some((gain, a));
                }
            }
            if let Some((_, a)) = best {
                return Some(a);
            }
        }
        None
    }

    /// The nodes from `x` up to, not including, `apex`.
    fn path_up(&self, mut x: usize, apex: usize) -> Vec<usize> {
        let mut out = Vec::new();
        while x != apex {
            out.push(x);
            let Some(p) = at(&self.parent, x) else {
                violation!(clause = "MKT.3", "a cycle's path passed the root", node = x);
            };
            x = p;
        }
        out
    }

    fn apex(&self, mut a: usize, mut b: usize) -> usize {
        while a != b {
            if at(&self.depth, a) >= at(&self.depth, b) {
                a = at(&self.parent, a).unwrap_or(a);
            } else {
                b = at(&self.parent, b).unwrap_or(b);
            }
        }
        a
    }

    /// The depths and potentials of a subtree recomputed from its root's tree arc.
    fn rehang(&mut self, top: usize) {
        let mut stack = vec![top];
        while let Some(x) = stack.pop() {
            if let Some(p) = at(&self.parent, x) {
                let a = at(&self.pred, x);
                *slot(&mut self.depth, x) = at(&self.depth, p) + 1;
                let c = at(&self.cost, a);
                *slot(&mut self.pi, x) = if at(&self.to, a) == x { at(&self.pi, p) + c } else { at(&self.pi, p) - c };
            }
            stack.extend(self.children.get(x).map_or(&[][..], Vec::as_slice).iter().copied());
        }
    }

    fn detach(&mut self, child: usize, parent: usize) {
        slot(&mut self.children, parent).retain(|c| *c != child);
    }

    /// Room on a tail-side tree arc, where the cycle runs down from the parent to `node`.
    fn room_down(&self, node: usize) -> Option<i64> {
        self.room(at(&self.pred, node), at(&self.parent, node).unwrap_or(node))
    }

    /// Room on a head-side tree arc, where the cycle runs up from `node` to its parent.
    fn room_up(&self, node: usize) -> Option<i64> {
        self.room(at(&self.pred, node), node)
    }

    /// One pivot: the entering arc closes a cycle with the tree; the most flow the cycle carries is pushed round
    /// it, and the last arc to block, taken from the cycle's apex in its direction, leaves, so the tree stays
    /// strongly feasible and no sequence of pivots repeats.
    fn pivot(&mut self, entering: usize) {
        self.pivots += 1;
        let rising = at(&self.state, entering) == State::Lower;
        let (tail, head) = if rising {
            (at(&self.from, entering), at(&self.to, entering))
        } else {
            (at(&self.to, entering), at(&self.from, entering))
        };
        let apex = self.apex(tail, head);
        let (tail_side, head_side) = (self.path_up(tail, apex), self.path_up(head, apex));
        let entering_room = if rising { self.residual_up(entering) } else { Some(at(&self.flow, entering)) };
        let rooms =
            tail_side.iter().map(|node| self.room_down(*node)).chain(head_side.iter().map(|node| self.room_up(*node)));
        let Some(delta) =
            rooms.chain(std::iter::once(entering_room)).flatten().reduce(|best, r| if r < best { r } else { best })
        else {
            violation!(clause = "MKT.3", "a cycle of improving cost and no capacity: the call is unbounded");
        };
        // The last blocking arc in the cycle's direction from the apex: the head side read from the apex down, then
        // the entering arc, then the tail side read up towards the apex.
        let blocked_head = head_side.iter().rev().find(|node| self.room_up(**node) == Some(delta)).copied();
        let leaving = match blocked_head {
            Some(node) => Some(node),
            None if entering_room == Some(delta) => None,
            None => tail_side.iter().find(|node| self.room_down(**node) == Some(delta)).copied(),
        };
        *slot(&mut self.flow, entering) += if rising { delta } else { -delta };
        for node in &tail_side {
            let arc = at(&self.pred, *node);
            *slot(&mut self.flow, arc) += if at(&self.to, arc) == *node { delta } else { -delta };
        }
        for node in &head_side {
            let arc = at(&self.pred, *node);
            *slot(&mut self.flow, arc) += if at(&self.from, arc) == *node { delta } else { -delta };
        }
        let Some(cut) = leaving else {
            *slot(&mut self.state, entering) = if rising { State::Upper } else { State::Lower };
            return;
        };
        let out = at(&self.pred, cut);
        *slot(&mut self.state, out) = if at(&self.flow, out) == 0 { State::Lower } else { State::Upper };
        *slot(&mut self.state, entering) = State::Tree;
        let (hung, holder) = if tail_side.contains(&cut) { (tail, head) } else { (head, tail) };
        self.exchange(entering, cut, hung, holder);
    }

    /// The tree after a pivot: the subtree below the leaving arc's node `cut` hangs instead from `holder` by the
    /// entering arc at `hung`, the path from `hung` up to `cut` reversed.
    fn exchange(&mut self, entering: usize, cut: usize, hung: usize, holder: usize) {
        let mut path = vec![hung];
        let mut node = hung;
        while node != cut {
            let Some(up) = at(&self.parent, node) else {
                violation!(clause = "MKT.3", "a leaving arc outside its subtree", node = node);
            };
            path.push(up);
            node = up;
        }
        if let Some(up) = at(&self.parent, cut) {
            self.detach(cut, up);
        }
        let old_pred: Vec<usize> = path.iter().map(|n| at(&self.pred, *n)).collect();
        for pair in path.windows(2) {
            self.detach(at(pair, 0), at(pair, 1));
        }
        for (i, pair) in path.windows(2).enumerate() {
            let (below, above) = (at(pair, 0), at(pair, 1));
            *slot(&mut self.parent, above) = Some(below);
            *slot(&mut self.pred, above) = at(&old_pred, i);
            slot(&mut self.children, below).push(above);
        }
        *slot(&mut self.parent, hung) = Some(holder);
        *slot(&mut self.pred, hung) = entering;
        slot(&mut self.children, holder).push(hung);
        self.rehang(hung);
    }
}

fn cost_bound(arcs: &[Arc]) -> i128 {
    let sum: i128 = arcs.iter().map(|a| a.cost.abs()).sum();
    sum + 1
}

/// The starting tree: every node hung from the root by its artificial arc, flow nowhere, which is strongly feasible
/// since each node can push flow to the root.
fn cold(nodes: usize, arcs: &[Arc]) -> Net {
    let big = cost_bound(arcs);
    let root = nodes;
    let m = arcs.len();
    let mut net = Net {
        from: arcs.iter().map(|a| a.from).chain(0..nodes).collect(),
        to: arcs.iter().map(|a| a.to).chain(std::iter::repeat_n(root, nodes)).collect(),
        cap: arcs.iter().map(|a| bound(a.cap)).chain(std::iter::repeat_n(None, nodes)).collect(),
        cost: arcs.iter().map(|a| a.cost).chain(std::iter::repeat_n(big, nodes)).collect(),
        flow: vec![0; m + nodes],
        state: std::iter::repeat_n(State::Lower, m).chain(std::iter::repeat_n(State::Tree, nodes)).collect(),
        parent: std::iter::repeat_n(Some(root), nodes).chain(std::iter::once(None)).collect(),
        pred: (m..m + nodes).chain(std::iter::once(0)).collect(),
        depth: std::iter::repeat_n(1, nodes).chain(std::iter::once(0)).collect(),
        children: (0..=nodes).map(|i| if i == root { (0..nodes).collect() } else { Vec::new() }).collect(),
        pi: vec![0; nodes + 1],
        root,
        next: 0,
        pivots: 0,
    };
    net.rehang(root);
    net
}

/// The saved tree arcs that still exist today and close no cycle, and every node the saved tree hung from the root
/// hung there again, then any node left over; each node's tree arcs by its side.
fn saved_tree(network: &Network, basis: &Basis, net: &mut Net) -> Option<Vec<Vec<usize>>> {
    let (nodes, arcs) = (network.nodes.len(), network.arcs.as_slice());
    let mut by_key: BTreeMap<u128, usize> = BTreeMap::new();
    for (i, a) in arcs.iter().enumerate() {
        if by_key.insert(a.key, i).is_some() {
            violation!(clause = "MKT.3", "two arcs of one network share a key", arc = i);
        }
    }
    for k in &basis.upper {
        let &i = by_key.get(k)?;
        let c = at(&net.cap, i)?;
        *slot(&mut net.state, i) = State::Upper;
        *slot(&mut net.flow, i) = c;
    }
    let mut group: Vec<usize> = (0..=nodes).collect();
    let find = |g: &mut Vec<usize>, mut node: usize| {
        while at(g, node) != node {
            let up = at(g, at(g, node));
            *slot(g, node) = up;
            node = up;
        }
        node
    };
    let m = arcs.len();
    let mut adjacent: Vec<Vec<usize>> = vec![Vec::new(); nodes + 1];
    for k in &basis.tree {
        let Some(&i) = by_key.get(k) else { continue };
        if at(&net.state, i) != State::Lower {
            continue;
        }
        let (a, b) = (find(&mut group, at(&net.from, i)), find(&mut group, at(&net.to, i)));
        if a != b {
            *slot(&mut group, a) = b;
            slot(&mut adjacent, at(&net.from, i)).push(i);
            slot(&mut adjacent, at(&net.to, i)).push(i);
        }
    }
    let hung: Vec<usize> =
        network.nodes.iter().enumerate().filter(|(_, k)| basis.hung.contains(k)).map(|(v, _)| v).collect();
    for v in hung.into_iter().chain(0..nodes) {
        let r = find(&mut group, v);
        if r != find(&mut group, net.root) {
            *slot(&mut group, r) = net.root;
            slot(&mut adjacent, v).push(m + v);
            slot(&mut adjacent, net.root).push(m + v);
        }
    }
    Some(adjacent)
}

/// The tree hung from the root over the chosen arcs, its nodes in the order they were reached; none when it does
/// not span.
fn orient(net: &mut Net, adjacent: &[Vec<usize>]) -> Option<Vec<usize>> {
    let nodes = net.root;
    for state in &mut net.state {
        if *state == State::Tree {
            *state = State::Lower;
        }
    }
    for node in 0..=nodes {
        slot(&mut net.children, node).clear();
        *slot(&mut net.parent, node) = None;
    }
    let mut order = Vec::with_capacity(nodes + 1);
    let mut seen = vec![false; nodes + 1];
    *slot(&mut seen, net.root) = true;
    let mut stack = vec![net.root];
    while let Some(node) = stack.pop() {
        order.push(node);
        for &arc in adjacent.get(node).map_or(&[][..], Vec::as_slice) {
            let reached = if at(&net.from, arc) == node { at(&net.to, arc) } else { at(&net.from, arc) };
            if at(&seen, reached) {
                continue;
            }
            *slot(&mut seen, reached) = true;
            *slot(&mut net.parent, reached) = Some(node);
            *slot(&mut net.pred, reached) = arc;
            *slot(&mut net.state, arc) = State::Tree;
            slot(&mut net.children, node).push(reached);
            stack.push(reached);
        }
    }
    (order.len() == nodes + 1).then_some(order)
}

/// The tree's flows solved from conservation, leaves up: each node's excess from the arcs at capacity passes along
/// its tree arc. False when a tree arc would leave its bounds.
fn conserve(net: &mut Net, order: &[usize], real: usize) -> bool {
    let mut excess = vec![0_i128; net.root + 1];
    for arc in (0..real).filter(|a| at(&net.state, *a) == State::Upper) {
        let f = i128::from(at(&net.flow, arc));
        *slot(&mut excess, at(&net.from, arc)) -= f;
        *slot(&mut excess, at(&net.to, arc)) += f;
    }
    for &node in order.iter().rev() {
        let Some(up) = at(&net.parent, node) else { continue };
        let arc = at(&net.pred, node);
        let e = at(&excess, node);
        let signed = if at(&net.from, arc) == node { e } else { -e };
        let Ok(f) = i64::try_from(signed) else { return false };
        if f < 0 || at(&net.cap, arc).is_some_and(|c| f > c) {
            return false;
        }
        *slot(&mut net.flow, arc) = f;
        *slot(&mut excess, up) += e;
    }
    true
}

/// A start from a saved basis, when it gives a feasible, strongly feasible tree over today's arcs: the saved tree
/// arcs that still exist and close no cycle, the rest of the nodes hung from the root, the saved arcs at capacity
/// set there, and the tree's flows solved from conservation.
fn warm(network: &Network, basis: &Basis) -> Option<Net> {
    let (nodes, arcs) = (network.nodes.len(), network.arcs.as_slice());
    let mut net = cold(nodes, arcs);
    let adjacent = saved_tree(network, basis, &mut net)?;
    let order = orient(&mut net, &adjacent)?;
    if !conserve(&mut net, &order, arcs.len()) {
        return None;
    }
    let strong = (0..nodes).all(|node| net.room(at(&net.pred, node), node).is_none_or(|r| r > 0));
    if !strong {
        return None;
    }
    net.rehang(net.root);
    Some(net)
}

/// The min-cost circulation over a network by the primal network simplex on integers, exact: from the saved basis
/// where it still gives a strongly feasible tree, and from the empty flow otherwise. Pricing and the leaving rule
/// depend only on the arcs' order and the start, so one basis and one input give one flow.
#[clause("MKT.3")]
#[must_use]
pub fn solve(network: &Network, start: Missing<&Basis>) -> Solution {
    let (nodes, arcs) = (network.nodes.len(), network.arcs.as_slice());
    for a in arcs {
        if a.from >= nodes || a.to >= nodes || bound(a.cap).is_some_and(|c| c < 0) {
            violation!(clause = "MKT.3", "an arc beyond the network or of negative capacity", from = a.from, to = a.to);
        }
    }
    let (mut net, warm_start) = match start {
        Missing::Present(b) => warm(network, b).map_or_else(|| (cold(nodes, arcs), false), |n| (n, true)),
        Missing::Absent => (cold(nodes, arcs), false),
    };
    while let Some(e) = net.entering() {
        net.pivot(e);
    }
    let m = arcs.len();
    if (m..m + nodes).any(|a| at(&net.flow, a) != 0) {
        violation!(clause = "MKT.3", "a circulation that needs its artificial arcs");
    }
    let key = |i: usize| arcs.get(i).map(|a| a.key);
    let basis = Basis {
        tree: (0..m).filter(|a| at(&net.state, *a) == State::Tree).filter_map(key).collect(),
        upper: (0..m).filter(|a| at(&net.state, *a) == State::Upper).filter_map(key).collect(),
        hung: (0..nodes)
            .filter(|v| at(&net.parent, *v) == Some(net.root))
            .filter_map(|v| network.nodes.get(v).copied())
            .collect(),
    };
    let potential: Vec<i128> = net.pi.iter().take(nodes).copied().collect();
    let Ok(_) = u32::try_from(net.pivots) else {
        capacity_exceeded!("pivots of one call", u32::MAX, net.pivots);
    };
    Solution { flow: net.flow.into_iter().take(m).collect(), potential, basis, pivots: net.pivots, warm: warm_start }
}

/// The total cost of a flow over its arcs.
#[must_use]
pub fn cost_of(arcs: &[Arc], flow: &[i64]) -> i128 {
    arcs.iter().zip(flow).map(|(a, f)| a.cost * i128::from(*f)).sum()
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::{Arc, Network, cost_of, solve};

    fn network(nodes: usize, arcs: &[Arc]) -> Network {
        Network { nodes: (0..nodes).map(|n| 100 + u128::try_from(n).unwrap()).collect(), arcs: arcs.to_vec() }
    }

    fn arc(from: usize, to: usize, cap: i64, cost: i64, key: u128) -> Arc {
        Arc { from, to, cap: Missing::Present(cap), cost: i128::from(cost), key }
    }

    /// The least cost of any integer circulation, by trying every flow within the capacities.
    fn brute(nodes: usize, arcs: &[Arc]) -> i128 {
        let caps: Vec<i64> = arcs.iter().map(|a| super::bound(a.cap).unwrap()).collect();
        let mut flow = vec![0_i64; arcs.len()];
        let mut best = 0_i128;
        loop {
            let mut balance = vec![0_i64; nodes];
            for (a, f) in arcs.iter().zip(&flow) {
                balance[a.from] -= f;
                balance[a.to] += f;
            }
            if balance.iter().all(|b| *b == 0) {
                let c = cost_of(arcs, &flow);
                if c < best {
                    best = c;
                }
            }
            let mut i = 0;
            loop {
                if i == flow.len() {
                    return best;
                }
                flow[i] += 1;
                if flow[i] <= caps[i] {
                    break;
                }
                flow[i] = 0;
                i += 1;
            }
        }
    }

    fn check_optimal(nodes: usize, arcs: &[Arc], s: &super::Solution) {
        let mut balance = vec![0_i64; nodes];
        for (a, f) in arcs.iter().zip(&s.flow) {
            assert!(*f >= 0 && *f <= super::bound(a.cap).unwrap(), "within capacity");
            balance[a.from] -= f;
            balance[a.to] += f;
            let reduced = a.cost + s.potential[a.from] - s.potential[a.to];
            if *f > 0 && *f < super::bound(a.cap).unwrap() {
                assert_eq!(reduced, 0, "an arc strictly within its bounds has zero reduced cost");
            }
            if *f == 0 && *f < super::bound(a.cap).unwrap() {
                assert!(reduced >= 0, "an empty arc cannot improve");
            }
            if *f == super::bound(a.cap).unwrap() && *f > 0 {
                assert!(reduced <= 0, "a full arc cannot improve by carrying less");
            }
        }
        assert!(balance.iter().all(|b| *b == 0), "a circulation");
    }

    #[test]
    fn simplex_matches_brute_force() {
        let networks: Vec<(usize, Vec<Arc>)> = vec![
            (3, vec![arc(0, 1, 3, -2, 1), arc(1, 2, 2, 1, 2), arc(2, 0, 3, -1, 3), arc(1, 0, 1, 1, 4)]),
            (
                4,
                vec![
                    arc(0, 1, 2, -5, 1),
                    arc(1, 2, 3, 2, 2),
                    arc(2, 3, 2, 1, 3),
                    arc(3, 0, 3, 0, 4),
                    arc(1, 3, 1, 4, 5),
                    arc(2, 0, 2, -1, 6),
                ],
            ),
            (
                4,
                vec![
                    arc(0, 1, 3, 1, 1),
                    arc(1, 2, 3, 1, 2),
                    arc(2, 0, 3, 1, 3),
                    arc(2, 3, 2, -4, 4),
                    arc(3, 1, 2, 0, 5),
                ],
            ),
            (3, vec![arc(0, 1, 2, 3, 1), arc(1, 0, 2, -3, 2), arc(1, 2, 3, -1, 3), arc(2, 1, 1, 0, 4)]),
        ];
        for (nodes, arcs) in &networks {
            let s = solve(&network(*nodes, arcs), Missing::Absent);
            check_optimal(*nodes, arcs, &s);
            assert_eq!(cost_of(arcs, &s.flow), brute(*nodes, arcs), "the least cost any flow reaches");
        }
    }

    #[test]
    fn simplex_matches_brute_force_on_random_networks() {
        let mut lot = Draws::new(stream_key(Seed::new(1), "MKT.simplex"), Subject::new(SubjectTag::Market, 0), 1, 0);
        let mut next = |n: u64| below_u64(&mut lot, n);
        let small = |v: u64| i64::try_from(v).unwrap();
        for round in 0..300 {
            let nodes = 3 + usize::try_from(next(2)).unwrap();
            let arcs: Vec<Arc> = (0..3 + next(4))
                .map(|k| {
                    let n = u64::try_from(nodes).unwrap();
                    let from = usize::try_from(next(n)).unwrap();
                    let to = (from + 1 + usize::try_from(next(n - 1)).unwrap()) % nodes;
                    arc(from, to, small(next(4)), small(next(11)) - 5, u128::from(k))
                })
                .collect();
            let cold = solve(&network(nodes, &arcs), Missing::Absent);
            check_optimal(nodes, &arcs, &cold);
            assert_eq!(cost_of(&arcs, &cold.flow), brute(nodes, &arcs), "round {round}");
            let mut moved = arcs.clone();
            for a in &mut moved {
                a.cost += i128::from(next(3)) - 1;
                a.cap = Missing::Present(small(next(4)));
            }
            let warm = solve(&network(nodes, &moved), Missing::Present(&cold.basis));
            check_optimal(nodes, &moved, &warm);
            assert_eq!(cost_of(&moved, &warm.flow), brute(nodes, &moved), "round {round}, warm");
        }
    }

    #[test]
    fn warm_start_same_optimum_and_same_basis_same_flow() {
        let arcs = vec![
            arc(0, 1, 2, -5, 1),
            arc(1, 2, 3, 2, 2),
            arc(2, 3, 2, 1, 3),
            arc(3, 0, 3, 0, 4),
            arc(1, 3, 1, 4, 5),
            arc(2, 0, 2, -1, 6),
        ];
        let first = solve(&network(4, &arcs), Missing::Absent);
        let again = solve(&network(4, &arcs), Missing::Present(&first.basis));
        assert!(again.warm, "the optimal basis still fits");
        assert_eq!(again.pivots, 0, "an optimal start needs no pivot");
        assert_eq!(cost_of(&arcs, &again.flow), cost_of(&arcs, &first.flow));
        // The next meeting's arcs differ: one capacity grows. The warm and cold starts reach the same value.
        let mut next = arcs.clone();
        next[0].cap = Missing::Present(3);
        let (w, c) =
            (solve(&network(4, &next), Missing::Present(&first.basis)), solve(&network(4, &next), Missing::Absent));
        assert_eq!(cost_of(&next, &w.flow), cost_of(&next, &c.flow));
        check_optimal(4, &next, &w);
        assert_eq!(
            w,
            solve(&network(4, &next), Missing::Present(&first.basis)),
            "one basis and one input give one flow"
        );
    }
}
