//! Settlement's arithmetic over a few firms at two banks: the greatest set, rings, short banks, held and committed
//! flows.
#![cfg(test)]

use phx_id::{PartyKey, Slot};
use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

use super::{Book, Books, Cause, Lines, Outcome, Settle};
use crate::flows::{Denom, Flow, FlowBufs, Grouped, Ranges};

const FIRMS: u8 = 0;
const BANKS: u8 = 1;
const ISSUER: u8 = 2;
const RICH: i64 = 1_000_000;

/// A small graph: each firm's funds, and its flows as (payer, payee, amount).
type Graph<'a> = (&'a [i64], &'a [(usize, usize, i64)]);

/// Firms at two banks, firm i at bank `i % 2`, each bank's reserves at the issuer, and which banks are closed.
struct Fixture {
    bank: Vec<u32>,
    balance: Vec<i64>,
    pending: Vec<i64>,
    held: Vec<i64>,
    facility: Vec<i64>,
    reserves: Vec<i64>,
    bank_pending: Vec<i64>,
    bank_held: Vec<i64>,
    bank_facility: Vec<i64>,
    deposits: Vec<i64>,
    closed: Vec<bool>,
}

impl Fixture {
    fn new(funds: &[i64], reserves: [i64; 2]) -> Fixture {
        let n = funds.len();
        Fixture {
            bank: (0..n).map(|i| u32::try_from(i % 2).unwrap()).collect(),
            balance: funds.to_vec(),
            pending: vec![0; n],
            held: vec![0; n],
            facility: vec![0; n],
            reserves: reserves.to_vec(),
            bank_pending: vec![0; 2],
            bank_held: vec![0; 2],
            bank_facility: vec![0; 2],
            deposits: (0..2).map(|b| funds.iter().skip(b).step_by(2).sum()).collect(),
            closed: vec![false; 2],
        }
    }

    fn books(&mut self) -> Books<'_> {
        let firms = Book {
            bank: &self.bank,
            balance: &mut self.balance,
            pending: &mut self.pending,
            held: &mut self.held,
            facility: &self.facility,
            lines: None,
        };
        let banks = Book {
            bank: &[],
            balance: &mut self.reserves,
            pending: &mut self.bank_pending,
            held: &mut self.bank_held,
            facility: &self.bank_facility,
            lines: None,
        };
        Books {
            kinds: vec![Some(firms), Some(banks), None],
            banks: BANKS,
            deposits: &mut self.deposits,
            closed: &self.closed,
            issuer: PartyKey::new(ISSUER, Slot::new(0)),
        }
    }
}

fn firm(i: usize) -> PartyKey {
    PartyKey::new(FIRMS, Slot::new(u32::try_from(i).unwrap()))
}

/// Each payer's flows in the order given, which is its payment order.
fn flows(edges: &[(usize, usize, i64)]) -> Vec<Flow> {
    edges
        .iter()
        .enumerate()
        .map(|(e, (p, q, a))| {
            let order = edges.get(..e).unwrap().iter().filter(|x| x.0 == *p).count();
            Flow {
                payer: firm(*p),
                payee: firm(*q),
                amount: *a,
                source: u32::try_from(e).unwrap(),
                denomination: Denom::money(0),
                reason: 0,
                order: u8::try_from(order).unwrap(),
            }
        })
        .collect()
}

fn lot(p: PartyKey) -> Draws {
    Draws::new(stream_key(Seed::new(1), "SET.order"), Subject::new(SubjectTag::Party, u64::from(p.word())), 0, 0)
}

fn settle_flows(w: &mut Fixture, made: &[Flow], workers: Option<usize>) -> Outcome {
    let ranges = Ranges::new(2, &[u32::try_from(w.balance.len()).unwrap(), 2, 1]);
    let mut bufs = FlowBufs::default();
    bufs.reset(2);
    let half = made.len() / 2;
    bufs.chunks_mut()[0].extend_from_slice(&made[..half]);
    bufs.chunks_mut()[1].extend_from_slice(&made[half..]);
    let pool = workers.map(|n| phx_exec::Pool::new(&phx_exec::PoolSpec::unpinned(n)).unwrap());
    bufs.group(pool.as_ref(), &ranges, Denom::money(0));
    let g = Grouped::new(&[&bufs], &ranges);
    let mut books = w.books();
    Settle::default().settle(pool.as_ref(), &g, &ranges, &mut books, &lot)
}

fn settle(funds: &[i64], edges: &[(usize, usize, i64)], reserves: [i64; 2]) -> (Outcome, Vec<i64>, Vec<i64>) {
    let mut w = Fixture::new(funds, reserves);
    let out = settle_flows(&mut w, &flows(edges), None);
    (out, w.balance, w.reserves)
}

/// The greatest set by brute force: every choice of how long a prefix of its payments each payer pays, kept where
/// every firm stays at or above nothing; the greatest is the longest prefix each payer has in any kept one.
fn brute_force(funds: &[i64], edges: &[(usize, usize, i64)]) -> Vec<i64> {
    let by_payer: Vec<Vec<usize>> =
        (0..funds.len()).map(|p| (0..edges.len()).filter(|e| edges[*e].0 == p).collect()).collect();
    let mut best = vec![0_usize; funds.len()];
    let mut choice = vec![0_usize; funds.len()];
    let settles =
        |choice: &[usize], e: usize| by_payer[edges[e].0].iter().position(|x| *x == e).unwrap() < choice[edges[e].0];
    loop {
        let after = |choice: &[usize]| -> Vec<i64> {
            let mut d = funds.to_vec();
            for (e, (p, q, a)) in edges.iter().enumerate() {
                if settles(choice, e) {
                    d[*p] -= a;
                    d[*q] += a;
                }
            }
            d
        };
        if after(&choice).iter().all(|x| *x >= 0) {
            for f in 0..funds.len() {
                if choice[f] > best[f] {
                    best[f] = choice[f];
                }
            }
        }
        let mut i = 0;
        loop {
            if i == funds.len() {
                return after(&best);
            }
            choice[i] += 1;
            if choice[i] <= by_payer[i].len() {
                break;
            }
            choice[i] = 0;
            i += 1;
        }
    }
}

#[test]
fn fixed_point_is_greatest_with_prefix_rule() {
    let edges = [(0, 1, 60), (0, 2, 50), (0, 2, 10)];
    let (_, after, _) = settle(&[100, 0, 0], &edges, [RICH, RICH]);
    assert_eq!(after, vec![40, 60, 0], "a prefix: 60 paid, 50 short, and the 10 after it fails with it");
    assert_eq!(settle(&[110, 0, 0], &edges, [RICH, RICH]).1, vec![0, 60, 50], "more funds never settle less");
    let ring = [(0, 1, 100), (1, 2, 100), (2, 0, 100)];
    let (out, after, _) = settle(&[0, 0, 0], &ring, [RICH, RICH]);
    assert_eq!((after, out.settled, out.failed.len()), (vec![0, 0, 0], 3, 0), "a ring with nothing settles whole");
    let chain = [(0, 1, 50), (1, 2, 50), (3, 2, 20)];
    let (out, after, _) = settle(&[10, 0, 0, 20], &chain, [RICH, RICH]);
    assert_eq!((after, out.failed.len()), (vec![10, 0, 20, 0], 2), "a short payer fails exactly its dependants");
    let graphs: [Graph<'_>; 4] = [
        (&[30, 20, 0, 5], &[(0, 1, 25), (1, 2, 40), (2, 3, 10), (3, 0, 15), (0, 2, 10)]),
        (&[0, 50, 0, 0], &[(0, 1, 30), (1, 0, 20), (1, 2, 40), (2, 3, 35), (3, 0, 5)]),
        (&[5, 5, 5, 5], &[(0, 1, 10), (1, 2, 10), (2, 3, 10), (3, 0, 10), (0, 2, 1)]),
        (&[100, 0, 0, 0], &[(0, 1, 70), (0, 3, 40), (1, 2, 70), (2, 3, 30), (3, 1, 60)]),
    ];
    for (funds, edges) in graphs {
        assert_eq!(settle(funds, edges, [RICH, RICH]).1, brute_force(funds, edges), "{edges:?}");
    }
}

#[test]
fn worklist_revisits_payees_and_banks() {
    let chain = [(0, 1, 50), (1, 2, 50), (2, 3, 50)];
    let (out, after, reserves) = settle(&[10, 0, 0, 0], &chain, [RICH, RICH]);
    assert_eq!((after, out.failed.len()), (vec![10, 0, 0, 0], 3), "each failure took its payee again");
    assert_eq!(reserves, vec![RICH, RICH], "nothing crossed between the banks");
    let (out, after, reserves) = settle(&[50, 0, 0, 0], &chain, [RICH, RICH]);
    assert_eq!((after, out.failed.len()), (vec![0, 0, 0, 50], 0));
    assert_eq!(
        reserves,
        vec![RICH - 50, RICH + 50],
        "the first bank's customers pay out 50 twice and are paid it once"
    );
}

#[test]
fn bank_net_removal_resettles() {
    // The first bank holds 30 in reserves and its customers owe 120 across; it cannot cover the net, so every flow
    // through it goes, 2's too, which it could have covered alone. 1 pays 3 within the second bank and settles.
    let (out, after, reserves) = settle(&[100, 10, 30, 0], &[(0, 1, 100), (2, 3, 20), (1, 3, 5)], [30, 1_000]);
    assert_eq!(after, vec![100, 5, 30, 5]);
    assert_eq!(out.settled, 1);
    assert!(out.failed.iter().all(|(_, c)| *c == Cause::Bank) && out.failed.len() == 2, "the bank's to answer for");
    assert_eq!(reserves, vec![30, 1_000], "no reserves moved: the settled flow stayed within the second bank");
}

#[test]
fn a_bank_is_answered_once_its_customers_are_done() {
    // With both flows standing the first bank owes 120 across on 30; but 0 cannot pay its own, and once it fails the
    // bank owes 20, which it covers.
    let (out, after, reserves) = settle(&[10, 0, 30, 0], &[(0, 1, 100), (2, 3, 20)], [30, 1_000]);
    assert_eq!(after, vec![10, 0, 10, 20]);
    assert_eq!((out.settled, out.failed.len()), (1, 1));
    assert_eq!(out.failed.first().map(|(_, c)| *c), Some(Cause::Payer));
    assert_eq!(reserves, vec![10, 1_020], "the settled 20 crossed from the first bank to the second");
}

#[test]
fn a_flow_through_a_closed_bank_is_held() {
    // The second bank is closed: 0's 50 to 1 there is held, neither failing nor funding anyone, and its 60 within the
    // first bank settles; the 50 stays out of 0's funds.
    let mut w = Fixture::new(&[100, 0, 0], [1_000, 1_000]);
    w.closed[1] = true;
    let out = settle_flows(&mut w, &flows(&[(0, 1, 50), (0, 2, 60)]), None);
    assert_eq!((out.settled, out.held.len(), out.failed.len()), (1, 1, 0));
    assert_eq!(w.balance, vec![40, 0, 60], "only the flow within the open bank moved money");
    assert_eq!(w.held[0], 50);
    assert_eq!(w.balance[0] - w.held[0], -10, "the payer's funds exclude what it holds");
}

#[test]
fn a_facility_lets_an_account_fall_below_nothing() {
    let mut w = Fixture::new(&[20, 0], [RICH, RICH]);
    w.facility[0] = 50;
    let out = settle_flows(&mut w, &flows(&[(0, 1, 60), (0, 1, 20)]), None);
    assert_eq!((w.balance.clone(), out.failed.len()), (vec![-40, 60], 1), "60 within 20 and the facility; 20 more not");
}

#[test]
fn the_same_for_any_workers() {
    let mut edges = Vec::new();
    for i in 0..400_usize {
        edges.push((i * 7 % 23, i * 11 % 23, i64::try_from(i % 37 + 1).unwrap()));
    }
    let funds: Vec<i64> = (0..23).map(|i| i64::from(i % 5) * 40).collect();
    let run = |workers| {
        let mut w = Fixture::new(&funds, [300, 200]);
        let out = settle_flows(&mut w, &flows(&edges), workers);
        assert!(w.books().deposit_breaks().is_empty(), "a bank owes what its customers hold");
        w.deposits[1] += 1;
        assert_eq!(w.books().deposit_breaks().len(), 1, "a bank owing other than its customers hold is found");
        w.deposits[1] -= 1;
        (w.balance, w.reserves, out.settled, out.failed.iter().map(|(f, _)| f.source).collect::<Vec<_>>())
    };
    let one = run(None);
    assert_eq!(one, run(Some(1)));
    assert_eq!(one, run(Some(4)));
    assert_eq!(one.0.iter().sum::<i64>() + one.1.iter().sum::<i64>(), funds.iter().sum::<i64>() + 500, "nothing made");
}

#[test]
fn ties_in_payment_order_fall_by_lot() {
    // Two flows of one order, and funds for one: which pays is the lot's, the same whenever drawn.
    let tie = |a: &mut Vec<Flow>| a.iter_mut().for_each(|f| f.order = 0);
    let mut made = flows(&[(0, 1, 30), (0, 2, 30)]);
    tie(&mut made);
    let run = || {
        let mut w = Fixture::new(&[40, 0, 0], [RICH, RICH]);
        let out = settle_flows(&mut w, &made, None);
        (w.balance, out.failed.len())
    };
    let first = run();
    assert_eq!(first.1, 1);
    assert_eq!(first.0.iter().sum::<i64>(), 40);
    assert_eq!(first, run());
}

#[test]
fn a_closed_days_commitments_settle_first() {
    let mut w = Fixture::new(&[10, 0], [RICH, RICH]);
    let ranges = Ranges::new(2, &[2, 2, 1]);
    let mut bufs = FlowBufs::default();
    bufs.reset(1);
    bufs.chunks_mut()[0].extend(flows(&[(1, 0, 30)]));
    bufs.group(None, &ranges, Denom::money(0));
    let mut s = Settle::default();
    s.commit(None, &Grouped::new(&[&bufs], &ranges), &ranges, &mut w.books());
    assert_eq!((w.pending.clone(), w.balance.clone()), (vec![30, -30], vec![10, 0]), "committed, not yet settled");
    assert_eq!((w.bank_pending[0], w.bank_pending[1]), (30, -30), "the reserves' move is committed too");
    // The next business day 0 pays 35 it could pay only with what it was committed.
    bufs.reset(1);
    bufs.chunks_mut()[0].extend(flows(&[(0, 1, 35)]));
    bufs.group(None, &ranges, Denom::money(0));
    let out = s.settle(None, &Grouped::new(&[&bufs], &ranges), &ranges, &mut w.books(), &lot);
    assert_eq!((w.balance.clone(), out.failed.len()), (vec![5, 5], 0));
    assert_eq!(w.reserves, vec![RICH - 5, RICH + 5]);
}

#[test]
fn settled_flows_post_to_their_lines() {
    // 0 pays 1 60 and 2 50 on reason 0 with 100: the 60 settles, the 50 fails and is taken back from both. The
    // firms keep two lines, paid and received; reason 0 feeds both.
    let mut w = Fixture::new(&[100, 0, 0], [RICH, RICH]);
    let mut amounts = vec![0_i64; 6];
    let (paid, received) = ([Some(0)], [Some(1)]);
    let out = {
        let mut books = w.books();
        if let Some(Some(firms)) = books.kinds.get_mut(usize::from(FIRMS)) {
            firms.lines = Some(Lines { amounts: &mut amounts, width: 2, paid: &paid, received: &received });
        }
        let ranges = Ranges::new(2, &[3, 2, 1]);
        let mut bufs = FlowBufs::default();
        bufs.reset(1);
        bufs.chunks_mut()[0].extend(flows(&[(0, 1, 60), (0, 2, 50)]));
        bufs.group(None, &ranges, Denom::money(0));
        Settle::default().settle(None, &Grouped::new(&[&bufs], &ranges), &ranges, &mut books, &lot)
    };
    assert_eq!(out.failed.len(), 1);
    assert_eq!(amounts, vec![60, 0, 0, 60, 0, 0], "0 paid 60 and 1 received it; the failed 50 is on no line");
}
