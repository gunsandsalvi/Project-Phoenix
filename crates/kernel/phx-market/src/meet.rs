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
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
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
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Meeting {
    pub unserved: Vec<(PartyKey, Want)>,
    pub rounds: u64,
    /// The sellers' weights the meeting reckoned, a measure of its work beside its sales; never read by the world.
    #[saved(skip, rebuild = Meeting::uncounted)]
    pub weighed: u64,
    #[saved(skip)]
    sales: Vec<Vec<Sale>>,
    #[saved(skip)]
    choosing: Vec<Choosing>,
    #[saved(skip)]
    again: Vec<Vec<Choosing>>,
    #[saved(skip)]
    scratch: Partitioned<Choosing>,
    #[saved(skip)]
    left: Vec<i64>,
    #[saved(skip)]
    log_prices: Vec<Option<f64>>,
}

impl Meeting {
    /// A meeting read back counts nothing yet.
    fn uncounted(&mut self) -> u64 {
        self.weighed = 0;
        0
    }

    /// The sales, chunk by chunk of stalls.
    pub fn sales(&self) -> impl Iterator<Item = &Sale> {
        self.sales.iter().flatten()
    }
}

/// A place's open sellers in a round: in the place's order with their weights, over the best's, and an alias table
/// over them for a buyer of units, made when one stands there; for a buyer with money, the same sellers by price with
/// their weights' running sums, since what it can pay is a prefix of them.
struct Table {
    stalls: Vec<u32>,
    values: Vec<f64>,
    weights: Vec<f64>,
    best: f64,
    alias: Option<AliasTable>,
    by_price: Vec<(i64, u32)>,
    priced: Vec<f64>,
    sums: Vec<f64>,
}

impl Table {
    /// The table after some of its sellers sold out: those left keep their weights while the best of them is the
    /// best it was, and the running sums are added again from the first seller gone, as a table made afresh adds
    /// them; with the best gone, none is left to keep, and the caller makes it afresh.
    fn without_sold_out(mut self, left: &[i64]) -> Option<Table> {
        let open = |s: &u32| left.get(to_usize(*s)).is_some_and(|l| *l > 0);
        let best = self
            .stalls
            .iter()
            .zip(&self.values)
            .filter(|(s, _)| open(s))
            .map(|(_, v)| *v)
            .reduce(|a, b| if b > a { b } else { a })?;
        if best.to_bits() != self.best.to_bits() {
            return None;
        }
        let mut keep = self.stalls.iter().map(open);
        self.values.retain(|_| keep.next().unwrap_or(false));
        let mut keep = self.stalls.iter().map(open);
        self.weights.retain(|_| keep.next().unwrap_or(false));
        self.stalls.retain(open);
        self.alias = None;
        let first = self.by_price.iter().position(|(_, s)| !open(s))?;
        let mut keep = self.by_price.iter().map(|(_, s)| open(s));
        self.priced.retain(|_| keep.next().unwrap_or(false));
        self.by_price.retain(|(_, s)| open(s));
        self.sums.truncate(first);
        let mut sum = first.checked_sub(1).and_then(|k| self.sums.get(k)).copied().unwrap_or(0.0);
        for w in self.priced.get(first..).unwrap_or(&[]) {
            sum += w;
            self.sums.push(sum);
        }
        Some(self)
    }
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
    #[clause("MKT.11")]
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
/// served short chooses again. A buyer with no seller left goes without, recorded as unserved, and nothing is added to
/// make the meeting clear; one with money that has bought keeps its change. Who buys what is the same for any workers
/// and whatever order the buyers are listed in.
#[clause("MKT.6", "MKT.10", "MKT.16", "MKT.17", "SRV.4", "SRV.5", "REP.22")]
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
    out.weighed = 0;
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
    // Each place's sellers by price once for the meeting, and the places each seller stands in: a place's table is
    // kept from round to round and made again only once a seller in it has sold out and a buyer stands there.
    let orders: Vec<Order> = phx_exec::for_chunks(pool, to_u32(places.len()), |p| {
        places.get(to_usize(p)).map_or_else(Vec::new, |place| by_price(place, stalls))
    });
    let standing = Standing::of(places, stalls.len());
    let mut tables: Vec<Option<Table>> = std::iter::repeat_with(|| None).take(places.len()).collect();
    let mut stale = vec![true; places.len()];
    let mut round = 0_u32;
    while !out.choosing.is_empty() {
        round += 1;
        out.rounds += 1;
        if phx_exec::trace::doubling(out.rounds) {
            phx_exec::trace::note(
                "meet.round",
                &[
                    ("round", i64::from(round)),
                    ("choosing", phx_exec::trace::count(out.choosing.len())),
                    ("places", phx_exec::trace::count(places.len())),
                    ("stalls", phx_exec::trace::count(stalls.len())),
                ],
            );
        }
        // A round whose buyers fit one job runs on the calling thread: a dispatch would outweigh it.
        let pool = if out.choosing.len() > CHOICE_CHUNK { pool } else { None };
        let open = (out.left.as_slice(), out.log_prices.as_slice());
        out.weighed += remake(pool, (places, &orders), (open, weights), &out.choosing, (&mut tables, &mut stale));
        let jobs: Vec<&mut [Choosing]> = out.choosing.chunks_mut(CHOICE_CHUNK).collect();
        phx_exec::each_chunk(pool, jobs, |chunk| choose(chunk, &tables, (tastes, lot, round)));
        for c in out.choosing.iter().filter(|c| c.stall.is_none()) {
            // A buyer with money that has bought keeps its change; any other goes without what it still wants.
            if !(c.bought && matches!(c.want, Want::Money(_))) {
                out.unserved.push((c.party, c.want));
            }
        }
        out.choosing.retain(|c| c.stall.is_some());
        by_stall(pool, &mut out.choosing, stalls.len(), &mut out.scratch);
        for s in serve_round(out, pool, stalls, (lot, round), lots) {
            for p in standing.places(s) {
                if let Some(x) = stale.get_mut(to_usize(*p)) {
                    *x = true;
                }
            }
        }
        out.choosing.clear();
        for again in &out.again {
            out.choosing.extend_from_slice(again);
        }
    }
    if phx_exec::trace::on() {
        phx_exec::trace::note(
            "meet",
            &[
                ("buyers", phx_exec::trace::count(buyers.len())),
                ("stalls", phx_exec::trace::count(stalls.len())),
                ("places", phx_exec::trace::count(places.len())),
                ("rounds", i64::from(round)),
                ("sales", phx_exec::trace::count(out.sales.iter().map(Vec::len).sum())),
                ("unserved", phx_exec::trace::count(out.unserved.len())),
            ],
        );
    }
}

/// Whether a buyer wants anything at all.
fn wanted(want: Want) -> bool {
    match want {
        Want::Units(q) | Want::UpTo(q, _) => q > 0,
        Want::Money(m) => m > 0,
    }
}

/// A place's sellers by their price, then who they are, each with its place among the place's sellers.
type Order = Vec<(i64, u32, usize)>;

fn by_price(place: &Place, stalls: &[Stall]) -> Order {
    let price = |s: u32| stalls.get(to_usize(s)).map_or(0, |x| x.price);
    let mut order: Order = place.near.iter().enumerate().map(|(i, (s, _))| (price(*s), *s, i)).collect();
    order.sort_by_key(|(p, s, _)| (*p, *s));
    order
}

/// The places each seller stands in, every seller's run of them in one list.
struct Standing {
    starts: Vec<usize>,
    places: Vec<u32>,
}

impl Standing {
    fn of(places: &[Place], stalls: usize) -> Standing {
        let mut starts = vec![0_usize; stalls + 1];
        for place in places {
            for (s, _) in &place.near {
                if let Some(n) = starts.get_mut(to_usize(*s) + 1) {
                    *n += 1;
                }
            }
        }
        let mut sum = 0;
        for n in &mut starts {
            sum += *n;
            *n = sum;
        }
        let mut next = starts.clone();
        let mut at = vec![0_u32; sum];
        for (p, place) in (0_u32..).zip(places) {
            for (s, _) in &place.near {
                if let Some(k) = next.get_mut(to_usize(*s)) {
                    if let Some(x) = at.get_mut(*k) {
                        *x = p;
                    }
                    *k += 1;
                }
            }
        }
        Standing { starts, places: at }
    }

    fn places(&self, stall: u32) -> &[u32] {
        let s = to_usize(stall);
        match (self.starts.get(s), self.starts.get(s + 1)) {
            (Some(a), Some(b)) => self.places.get(*a..*b).unwrap_or(&[]),
            _ => &[],
        }
    }
}

/// The tables of the places where a buyer stands this round and a seller has sold out since theirs was made, made
/// again; the rest kept, since their sellers are as they were.
fn remake(
    pool: Option<&Pool>,
    (places, orders): (&[Place], &[Order]),
    (open, w): ((&[i64], &[Option<f64>]), Weights),
    choosing: &[Choosing],
    (tables, stale): (&mut [Option<Table>], &mut [bool]),
) -> u64 {
    let mut wanted = vec![false; places.len()];
    for c in choosing {
        if let Some(x) = wanted.get_mut(to_usize(c.place)) {
            *x = true;
        }
    }
    // A place is made again where a buyer stands and a seller in it sold out since.
    let mut todo = vec![false; places.len()];
    for ((t, w), s) in todo.iter_mut().zip(&wanted).zip(stale.iter_mut()) {
        *t = *w && *s;
        *s &= !*w;
    }
    let units: Vec<bool> = {
        let mut u = vec![false; places.len()];
        for c in choosing.iter().filter(|c| matches!(c.want, Want::Units(_))) {
            if let Some(x) = u.get_mut(to_usize(c.place)) {
                *x = true;
            }
        }
        u
    };
    let made: Vec<(usize, &mut Option<Table>)> =
        tables.iter_mut().enumerate().filter(|(p, _)| todo.get(*p).is_some_and(|t| *t)).collect();
    phx_exec::each_chunk(pool, made, |(p, slot)| {
        let kept = slot.take().and_then(|t| t.without_sold_out(open.0));
        *slot = kept.or_else(|| table((places.get(p)?, orders.get(p)?), open, w));
        if units.get(p).is_some_and(|u| *u)
            && let Some(t) = slot.as_mut()
            && t.alias.is_none()
        {
            t.alias = Some(AliasTable::new(&t.weights));
        }
    });
    // The weights reckoned are those of the places made again this round.
    let made = tables.iter().zip(&todo).filter(|(_, t)| **t).filter_map(|(slot, _)| slot.as_ref());
    made.map(|t| len_u64(t.weights.len())).sum()
}

/// A place's table of its open sellers this round, or none if none is open: their weights in the place's order, and
/// by price with the weights' running sums.
fn table(
    (place, order): (&Place, &[(i64, u32, usize)]),
    (left, logs): (&[i64], &[Option<f64>]),
    w: Weights,
) -> Option<Table> {
    let mut at: Vec<Option<usize>> = vec![None; place.near.len()];
    let mut values: Vec<(u32, f64)> = Vec::new();
    for (i, (s, km)) in place.near.iter().enumerate() {
        if left.get(to_usize(*s)).is_none_or(|l| *l <= 0) {
            continue;
        }
        if let Some(lp) = logs.get(to_usize(*s)).copied().flatten() {
            if let Some(a) = at.get_mut(i) {
                *a = Some(values.len());
            }
            values.push((*s, -w.price * lp - w.distance * km));
        }
    }
    let best = values.iter().map(|(_, v)| *v).reduce(|a, b| if b > a { b } else { a })?;
    // Over the best's, so the best weighs one and none overflows.
    let weights: Vec<f64> = values.iter().map(|(_, v)| libm::exp(v - best)).collect();
    let n = values.len();
    let (mut by_price, mut priced, mut sums, mut sum) =
        (Vec::with_capacity(n), Vec::with_capacity(n), Vec::with_capacity(n), 0.0);
    for (p, s, i) in order {
        let Some(weight) = at.get(*i).copied().flatten().and_then(|j| weights.get(j)) else { continue };
        sum += weight;
        sums.push(sum);
        priced.push(*weight);
        by_price.push((*p, *s));
    }
    Some(Table {
        stalls: values.iter().map(|(s, _)| *s).collect(),
        values: values.iter().map(|(_, v)| *v).collect(),
        weights,
        best,
        alias: None,
        by_price,
        priced,
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
        Want::Units(_) => t.stalls.get(t.alias.as_ref()?.draw(d)).copied(),
        Want::Money(_) | Want::UpTo(..) => {
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
/// from the lowest, so each pass writes to few streams.
fn by_stall(pool: Option<&Pool>, items: &mut Vec<Choosing>, stalls: usize, scratch: &mut Partitioned<Choosing>) {
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
}

/// A chunk of stalls, with the buyers that chose them this round, where its sales go and those it serves short.
type ServeJob<'a> = (usize, &'a mut [i64], &'a mut [Choosing], &'a mut Vec<Sale>, &'a mut Vec<Choosing>);

/// The round's service: the buyers, grouped by the stall they chose, served a chunk of stalls a job, each chosen stall
/// its own buyers in place, so a job outweighs its dispatch and no stall no one chose is visited; a job adds to its
/// chunk's sales and lists the buyers it served short. Returns the stalls the round sold out.
fn serve_round(
    out: &mut Meeting,
    pool: Option<&Pool>,
    stalls: &[Stall],
    (lot, round): (i64, u32),
    lots: &(impl Fn(PartyKey, u32) -> Draws + Sync),
) -> Vec<u32> {
    for again in &mut out.again {
        again.clear();
    }
    let mut jobs: Vec<ServeJob<'_>> = Vec::new();
    let mut rest = out.choosing.as_mut_slice();
    for (((c, l), sales), again) in out.left.chunks_mut(STALL_CHUNK).enumerate().zip(&mut out.sales).zip(&mut out.again)
    {
        let (first, end) = (c * STALL_CHUNK, c * STALL_CHUNK + l.len());
        let n = rest.partition_point(|x| x.stall.is_some_and(|s| to_usize(s) < end));
        let (mine, tail) = std::mem::take(&mut rest).split_at_mut(n);
        rest = tail;
        if !mine.is_empty() {
            jobs.push((first, l, mine, sales, again));
        }
    }
    let serve_job = |(first, left, mine, sales, again): ServeJob<'_>| -> Vec<u32> {
        let mut sold_out = Vec::new();
        let mut i = 0;
        while let Some(s) = mine.get(i).and_then(|c| c.stall) {
            let run = mine.get(i..).map_or(0, |m| m.iter().take_while(|c| c.stall == Some(s)).count());
            let (Some(stall), Some(l), Some(list)) =
                (stalls.get(to_usize(s)), left.get_mut(to_usize(s) - first), mine.get_mut(i..i + run))
            else {
                violation!(clause = "MKT.6", "a buyer at a stall beyond the meeting's", stall = s);
            };
            serve(stall, l, list, (lot, round), lots, (sales, again));
            if *l == 0 {
                sold_out.push(s);
            }
            i += run;
        }
        sold_out
    };
    let sold_out: Vec<Vec<u32>> = phx_exec::map_chunks(pool, jobs, serve_job);
    sold_out.into_iter().flatten().collect()
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

fn to_usize(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// A count of places or buyers as a traversal's chunks.
fn to_u32(n: usize) -> u32 {
    match u32::try_from(n) {
        Ok(n) => n,
        Err(_) => phx_num::capacity_exceeded!("a traversal's chunks", u32::MAX, n),
    }
}

#[path = "meet_tests.rs"]
mod tests;
