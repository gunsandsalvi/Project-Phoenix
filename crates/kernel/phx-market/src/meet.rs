//! The posted-price meeting on the core: each buyer goes to a seller in its reach with the logit's chance over their
//! posted prices and distances, drawn once from its own stream; a seller serves those who came in an order drawn by
//! lot until its units run out, and those served short choose again among the sellers with units left, round by round.
//! A place's sellers are weighed once a round for every buyer standing there, so a choice is one draw.

use phx_core::flows::{Denom, Flow};
use phx_core::goods::NATURE;
use phx_exec::Pool;
use phx_exec::partition::Partitioned;
use phx_id::PartyKey;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::consts::LANES;
use phx_rand::float::{from_i64, len_u64};
use phx_rand::uniform::{below_u64, open_unit};
use phx_rand::{AliasTable, Draws, StreamKey, Subject, SubjectTag};

use crate::consts::{CHOICE_CHUNK, ROUND_BLOCKS, STALL_CHUNK, STALL_DIGIT_BITS};
use crate::retail::{Want, Weights, less, paid, units_wanted};

/// A seller at the meeting: who, its posted price for a lot of the product, and the units it can serve today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stall {
    pub seller: PartyKey,
    pub price: i64,
    pub units: i64,
}

/// A buyer: who, the identity its tastes are drawn for, what it wants, and the place it stands at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Buyer {
    pub party: PartyKey,
    pub subject: u64,
    pub want: Want,
    pub place: u32,
}

/// The sellers in reach of a place: each stall's place among the stalls and how far it is, in km.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Place {
    pub near: Vec<(u32, f64)>,
}

/// The stream buyers' tastes are drawn from, and the day and sub-step it is drawn at.
#[derive(Clone, Copy, Debug)]
pub struct Tastes {
    pub key: StreamKey,
    pub day: u32,
    pub substep: u8,
}

/// A buyer buying whole units from a seller, and what it pays for them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sale {
    pub buyer: PartyKey,
    pub seller: PartyKey,
    pub units: i64,
    pub paid: i64,
}

/// A meeting's outcome, kept by its caller from one day to the next so a day's meeting writes into what the heaviest
/// day sized: its sales, a list for each chunk of stalls in the order that chunk made them; what each buyer that found
/// no seller still wanted; and the rounds capacity forced. The rest is the meeting's own working space.
#[derive(Debug, Default)]
pub struct Meeting {
    pub unserved: Vec<(PartyKey, Want)>,
    pub rounds: u64,
    sales: Vec<Vec<Sale>>,
    choosing: Vec<Choosing>,
    again: Vec<Vec<Choosing>>,
    scratch: Partitioned<Choosing>,
    left: Vec<i64>,
    log_prices: Vec<Option<f64>>,
    starts: Vec<usize>,
}

impl Meeting {
    /// The sales, chunk by chunk of stalls.
    pub fn sales(&self) -> impl Iterator<Item = &Sale> {
        self.sales.iter().flatten()
    }
}

/// A place's open sellers in a round: for a buyer of units, an alias table over their weights; for a buyer with
/// money, the same sellers by price with their weights' running sums, since what it can pay is a prefix of them.
struct Table {
    stalls: Vec<u32>,
    alias: AliasTable,
    by_price: Vec<(i64, u32)>,
    sums: Vec<f64>,
}

/// A buyer still choosing, with what its choice and its service read, so neither looks it up: what it still wants,
/// the identity its tastes are drawn for, who it is, where it stands, the seller it chose this round, and whether it
/// has bought.
#[derive(Clone, Copy, Debug)]
struct Choosing {
    want: Want,
    subject: u64,
    party: PartyKey,
    place: u32,
    stall: Option<u32>,
    bought: bool,
}

/// A sale's goods leg: the unit it moves, the reason and payment order its declarations give, and whether the buyer
/// uses the goods up as it buys them, as a household does, or holds them, as a firm does its inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoodsLeg {
    pub unit: Denom,
    pub reason: u8,
    pub order: u8,
    pub used: bool,
}

impl Sale {
    /// The sale's flows: the money from buyer to seller with its reason and payment order, from `source`; and, where
    /// the good is held, its units from the seller, to the buyer who holds them, or to nature as the purchase uses them
    /// up, the buyer then being what accounts for it.
    pub fn flows(&self, money: (Denom, u8, u8), goods: Option<GoodsLeg>, source: u32, out: &mut Vec<Flow>) {
        let (denomination, reason, order) = money;
        out.push(Flow {
            payer: self.buyer,
            payee: self.seller,
            amount: self.paid,
            source,
            denomination,
            reason,
            order,
        });
        if let Some(g) = goods {
            let (payee, source) = if g.used { (NATURE, self.buyer.word()) } else { (self.buyer, source) };
            out.push(Flow {
                payer: self.seller,
                payee,
                amount: self.units,
                source,
                denomination: g.unit,
                reason: g.reason,
                order: g.order,
            });
        }
    }
}

/// The posted-price meeting over one product's stalls, traded in lots of `lot` units. Each round every place weighs
/// its sellers with units left at `exp(−w.price·ln(price) − w.distance·km)`, and each buyer still choosing draws one of
/// its place's with that weight's share, the logit's chance under a standard Gumbel taste, from its own stream's block
/// for the round; a buyer with money draws among those it can buy a unit from. A seller serves its buyers in an order
/// drawn by `lots` for it and the round, each as many whole units as it wants and the seller's units allow; a buyer
/// served short chooses again. A buyer with no seller left goes without; one with money that has bought keeps its
/// change. Who buys what is the same for any workers and whatever order the buyers are listed in.
#[clause("MKT.6", "SRV.4", "SRV.5", "REP.22")]
pub fn meet(
    out: &mut Meeting,
    pool: Option<&Pool>,
    (stalls, places, buyers): (&[Stall], &[Place], &[Buyer]),
    (lot, weights): (i64, Weights),
    tastes: Tastes,
    lots: &(impl Fn(PartyKey, u32) -> Draws + Sync),
) {
    let jobs_n = stalls.len().div_ceil(STALL_CHUNK);
    out.unserved.clear();
    out.rounds = 0;
    out.sales.resize_with(jobs_n, Vec::new);
    out.again.resize_with(jobs_n, Vec::new);
    for v in &mut out.sales {
        v.clear();
    }
    out.left.clear();
    out.left.extend(stalls.iter().map(|s| s.units));
    // A seller with no price weighs nothing: nothing is posted to weigh.
    out.log_prices.clear();
    out.log_prices.extend(stalls.iter().map(|s| (s.price > 0).then(|| libm::log(from_i64(s.price)))));
    out.choosing.clear();
    out.choosing.extend(buyers.iter().filter(|b| wanted(b.want)).map(|b| Choosing {
        want: b.want,
        subject: b.subject,
        party: b.party,
        place: b.place,
        stall: None,
        bought: false,
    }));
    let mut round = 0_u32;
    while !out.choosing.is_empty() {
        round += 1;
        out.rounds += 1;
        let (left, log_prices) = (&out.left, &out.log_prices);
        let tables: Vec<Option<Table>> = phx_exec::pool::map(pool, places.len(), |p| {
            places.get(p).and_then(|place| table(place, stalls, (left, log_prices), weights))
        });
        let jobs: Vec<&mut [Choosing]> = out.choosing.chunks_mut(CHOICE_CHUNK).collect();
        phx_exec::pool::each(pool, jobs, |chunk| choose(chunk, &tables, (tastes, lot, round)));
        for c in out.choosing.iter().filter(|c| c.stall.is_none()) {
            // A buyer with money that has bought keeps its change; any other goes without what it still wants.
            if !(c.bought && matches!(c.want, Want::Money(_))) {
                out.unserved.push((c.party, c.want));
            }
        }
        out.choosing.retain(|c| c.stall.is_some());
        by_stall(pool, &mut out.choosing, stalls.len(), (&mut out.scratch, &mut out.starts));
        // Sellers are served a chunk of stalls a job, each stall its own buyers in place, so a job outweighs its
        // dispatch; a job adds to its chunk's sales and lists the buyers it served short.
        let starts = &out.starts;
        let mut jobs = Vec::with_capacity(jobs_n);
        let mut rest = out.choosing.as_mut_slice();
        for (((c, l), sales), again) in
            out.left.chunks_mut(STALL_CHUNK).enumerate().zip(&mut out.sales).zip(&mut out.again)
        {
            let first = c * STALL_CHUNK;
            let n = at(starts, first + l.len()) - at(starts, first);
            let (mine, tail) = std::mem::take(&mut rest).split_at_mut(n);
            jobs.push((first, l, mine, sales, again));
            rest = tail;
        }
        phx_exec::pool::each(pool, jobs, |(first, left, mine, sales, again)| {
            again.clear();
            let base = at(starts, first);
            for (k, l) in left.iter_mut().enumerate() {
                let s = first + k;
                let (Some(stall), Some(list)) =
                    (stalls.get(s), mine.get_mut(at(starts, s) - base..at(starts, s + 1) - base))
                else {
                    continue;
                };
                serve(stall, l, list, (lot, round), lots, (sales, again));
            }
        });
        out.choosing.clear();
        for again in &out.again {
            out.choosing.extend_from_slice(again);
        }
    }
}

/// Whether a buyer wants anything at all.
fn wanted(want: Want) -> bool {
    match want {
        Want::Units(q) => q > 0,
        Want::Money(m) => m > 0,
    }
}

/// A place's table of its open sellers this round, or none if none is open.
fn table(place: &Place, stalls: &[Stall], (left, logs): (&[i64], &[Option<f64>]), w: Weights) -> Option<Table> {
    let values: Vec<(u32, f64)> = place
        .near
        .iter()
        .filter(|(s, _)| left.get(to_usize(*s)).is_some_and(|l| *l > 0))
        .filter_map(|(s, km)| logs.get(to_usize(*s)).copied().flatten().map(|lp| (*s, -w.price * lp - w.distance * km)))
        .collect();
    let best = values.iter().map(|(_, v)| *v).reduce(|a, b| if b > a { b } else { a })?;
    // Over the best's, so the best weighs one and none overflows.
    let weights: Vec<f64> = values.iter().map(|(_, v)| libm::exp(v - best)).collect();
    let stall_ids: Vec<u32> = values.iter().map(|(s, _)| *s).collect();
    let price = |s: u32| stalls.get(to_usize(s)).map_or(0, |x| x.price);
    let mut order: Vec<(i64, u32, f64)> = stall_ids.iter().zip(&weights).map(|(s, w)| (price(*s), *s, *w)).collect();
    order.sort_by_key(|(p, s, _)| (*p, *s));
    let mut sum = 0.0;
    let sums = order
        .iter()
        .map(|(_, _, w)| {
            sum += w;
            sum
        })
        .collect();
    Some(Table {
        stalls: stall_ids,
        alias: AliasTable::new(&weights),
        by_price: order.iter().map(|(p, s, _)| (*p, *s)).collect(),
        sums,
    })
}

/// A chunk's choices, four buyers' draws made together, each from its own stream's block for the round.
fn choose(chunk: &mut [Choosing], tables: &[Option<Table>], (t, lot, round): (Tastes, i64, u32)) {
    let block = (round - 1) * ROUND_BLOCKS;
    let subject = |c: &Choosing| Subject::new(SubjectTag::Party, c.subject);
    let (fours, rest) = chunk.as_chunks_mut::<LANES>();
    for four in fours {
        let draws = Draws::x4(t.key, four.each_ref().map(subject), t.day, t.substep, block);
        for (c, mut d) in four.iter_mut().zip(draws) {
            c.stall = pick(c, tables, lot, &mut d);
        }
    }
    for c in rest {
        let mut d = Draws::from_block(t.key, subject(c), t.day, t.substep, block);
        c.stall = pick(c, tables, lot, &mut d);
    }
}

/// One buyer's seller this round, drawn from its place's table: by the alias table for units, by the running sums of
/// the sellers it can buy a unit from for money.
fn pick(c: &Choosing, tables: &[Option<Table>], lot: i64, d: &mut Draws) -> Option<u32> {
    let t = tables.get(to_usize(c.place))?.as_ref()?;
    match c.want {
        Want::Units(_) => t.stalls.get(t.alias.draw(d)).copied(),
        Want::Money(_) => {
            let can = t.by_price.partition_point(|(price, _)| units_wanted(c.want, lot, *price) > 0);
            let total = *t.sums.get(can.checked_sub(1)?)?;
            let u = open_unit(d) * total;
            let at = t.sums.partition_point(|s| *s <= u);
            // A draw at the prefix's very top falls to its last seller.
            let at = if at < can { at } else { can - 1 };
            t.by_price.get(at).map(|(_, s)| *s)
        }
    }
}

/// The buyers grouped by the stall they chose, stably, by a pass of `STALL_DIGIT_BITS` bits of the stall at a time
/// from the lowest, so each pass writes to few streams; `starts` is where each stall's buyers start, and one past the
/// last.
fn by_stall(
    pool: Option<&Pool>,
    items: &mut Vec<Choosing>,
    stalls: usize,
    (scratch, starts): (&mut Partitioned<Choosing>, &mut Vec<usize>),
) {
    let buckets = 1_usize << STALL_DIGIT_BITS;
    let mut shift = 0_u32;
    loop {
        let digit = |c: &Choosing| match c.stall {
            Some(s) => to_usize(s >> shift) & (buckets - 1),
            None => violation!(clause = "MKT.6", "a buyer grouped at no seller"),
        };
        phx_exec::partition::partition_into(pool, &[items.as_slice()], buckets, digit, scratch);
        std::mem::swap(items, &mut scratch.items);
        shift += STALL_DIGIT_BITS;
        if stalls.checked_shr(shift).unwrap_or(0) == 0 {
            break;
        }
    }
    starts.clear();
    starts.resize(stalls + 1, 0);
    for c in items.iter() {
        if let Some(n) = c.stall.and_then(|s| starts.get_mut(to_usize(s) + 1)) {
            *n += 1;
        }
    }
    let mut sum = 0;
    for n in starts.iter_mut() {
        sum += *n;
        *n = sum;
    }
}

/// A seller's service of the buyers that chose it this round: all of them if its units reach, else one at a time
/// drawn by lot from those not yet served, from their order by who they are, until its units run out; each takes as
/// much as it wants and the units left allow. A sale is made for each that took any, and each served short goes to
/// choose again.
fn serve(
    stall: &Stall,
    left: &mut i64,
    list: &mut [Choosing],
    (lot, round): (i64, u32),
    lots: &(impl Fn(PartyKey, u32) -> Draws + Sync),
    (sales, again): (&mut Vec<Sale>, &mut Vec<Choosing>),
) {
    let total: i128 = list.iter().map(|c| i128::from(units_wanted(c.want, lot, stall.price))).sum();
    let mut d = None;
    if total > i128::from(*left) {
        // The lot's order starts from who the buyers are, not the order they came in.
        list.sort_unstable_by_key(|c| (c.party.word(), c.subject));
        d = Some(lots(stall.seller, round));
    }
    for k in 0..list.len() {
        if let Some(d) = d.as_mut().filter(|_| *left > 0) {
            let j = k + phx_rand::float::index(below_u64(d, len_u64(list.len() - k)));
            list.swap(k, j);
        }
        let Some(c) = list.get(k).copied() else { break };
        let want = units_wanted(c.want, lot, stall.price);
        let take = if want <= *left { want } else { *left };
        if take < 0 {
            violation!(clause = "MKT.13", "a sale of fewer than no units", units = take);
        }
        *left -= take;
        let mut after = c;
        if take > 0 {
            sales.push(Sale { buyer: c.party, seller: stall.seller, units: take, paid: paid(take, stall.price, lot) });
            after.want = less(c.want, lot, take, stall.price);
            after.bought = true;
        }
        if take < want {
            after.stall = None;
            again.push(after);
        }
    }
}

/// Where a stall's buyers start in the grouped list; every stall, and one past the last, has a start.
fn at(starts: &[usize], s: usize) -> usize {
    let Some(a) = starts.get(s) else {
        violation!(clause = "MKT.6", "a stall beyond the meeting's grouping", stall = s);
    };
    *a
}

fn to_usize(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

#[path = "meet_tests.rs"]
mod tests;
