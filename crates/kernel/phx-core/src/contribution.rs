use core::any::Any;

use phx_id::{CountryId, PartyId, TileId};
use phx_macros::clause;
use phx_rand::{Draws, Subject};

use crate::consts::{
    OPENING_BALANCES, OPENING_CONTRACTS, OPENING_DECLARATIONS, OPENING_PARTIES, OPENING_PHYSICAL_STOCK,
    OPENING_PRESENT_VALUES,
};
use crate::register::Register;
use crate::streams::{OpeningPhase, Purpose, StreamDecl, Streams};

/// The opening's phases in which systems contribute, in the order they run.
pub const DECLARATIONS: OpeningPhase = OpeningPhase(OPENING_DECLARATIONS);
pub const PARTIES: OpeningPhase = OpeningPhase(OPENING_PARTIES);
pub const PHYSICAL_STOCK: OpeningPhase = OpeningPhase(OPENING_PHYSICAL_STOCK);
pub const CONTRACTS: OpeningPhase = OpeningPhase(OPENING_CONTRACTS);
pub const PRESENT_VALUES: OpeningPhase = OpeningPhase(OPENING_PRESENT_VALUES);
pub const BALANCES: OpeningPhase = OpeningPhase(OPENING_BALANCES);
/// Every contributing phase, in order.
pub const PHASES: [OpeningPhase; 6] = [DECLARATIONS, PARTIES, PHYSICAL_STOCK, CONTRACTS, PRESENT_VALUES, BALANCES];

/// The opening's context: its phase, and draws at the phase's own ordinal.
#[derive(Debug)]
pub struct OpeningCtx<'a> {
    streams: &'a Streams,
    phase: OpeningPhase,
}

impl<'a> OpeningCtx<'a> {
    #[must_use]
    pub fn new(streams: &'a Streams, phase: OpeningPhase) -> OpeningCtx<'a> {
        OpeningCtx { streams, phase }
    }

    #[must_use]
    pub fn phase(&self) -> OpeningPhase {
        self.phase
    }

    /// The draws of an opening stream for a subject, on the opening's day.
    #[must_use]
    pub fn draws(&self, stream: &StreamDecl, subject: Subject) -> Draws {
        if stream.purpose == Purpose::Observer {
            phx_num::violation!(clause = "Law 17", "the opening drawing from the observer's stream");
        }
        self.streams.open(stream, subject, phx_id::Day::new(0), self.phase.ordinal())
    }
}

/// A country as the opening reads it: its identity, its people, its GDP in its currency's smallest units, its derived
/// values on their natural scales, the land tiles its parties may be sited on, and its regions, each with its land.
#[derive(Clone, Debug, PartialEq)]
pub struct OpeningCountry {
    pub id: CountryId,
    pub people: u64,
    pub gdp: f64,
    pub derived: Vec<(String, f64)>,
    pub sites: Vec<TileId>,
    pub regions: Vec<(u32, Vec<TileId>)>,
}

impl OpeningCountry {
    /// A derived value by its register name, if the country's group reports it.
    #[must_use]
    pub fn derived(&self, name: &str) -> Option<f64> {
        self.derived.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }

    /// A site among the country's land tiles, each as likely.
    pub fn site(&self, draws: &mut Draws) -> TileId {
        let (Ok(n), true) = (u64::try_from(self.sites.len()), !self.sites.is_empty()) else {
            phx_num::violation!(clause = "PTY.5", "a party sited in a country with no land", country = self.id.get());
        };
        let Some(t) = usize::try_from(phx_rand::below_u64(draws, n)).ok().and_then(|at| self.sites.get(at)) else {
            phx_num::violation!(clause = "PTY.5", "a party sited in a country with no land", country = self.id.get());
        };
        *t
    }
}

/// The subject of an opening draw: its stratum and its ordinal within it, so a party's draws exist before its
/// identity does.
pub fn opening_subject(stratum: u32, ordinal: u32) -> Subject {
    Subject::new(phx_rand::SubjectTag::Opening, (u64::from(stratum) << u32::BITS) | u64::from(ordinal))
}

/// A write that balanced the books: the party, the amount in its account's smallest units, the opening identity it
/// served, and the counterparty whose entry answers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct WriteRecord {
    pub party: PartyId,
    pub amount: i128,
    pub identity: u64,
    pub counter: PartyId,
}

/// A counterparty's realised side against its drawn size: the stratum apportioned, the party, what its drawn size
/// asked and what the apportionment gave.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Apportioned {
    pub stratum: String,
    pub party: PartyId,
    pub drawn: u64,
    pub realised: u64,
}

/// Where the opening's report lists each balancing write and each apportionment as it is made, so the world keeps
/// only what its checks read of them.
pub trait ReportSink: core::fmt::Debug + Send + Sync {
    fn write(&mut self, w: &WriteRecord);
    fn apportioned(&mut self, a: &Apportioned);
    /// The listing finished.
    ///
    /// # Errors
    /// What could not be put.
    fn finish(self: Box<Self>) -> Result<(), String>;
}

/// What the opening did, for its report: each distribution read with its source, each balancing change, each
/// party's opening equity, the one place equity is computed from assets and liabilities; and of the balancing writes
/// and the apportionments, listed to the report's sink as they are made, how many there were, the parties begun that
/// no write named, and the apportionments that gave a party of no drawn size a share.
#[clause("GEN.4")]
#[derive(Debug, Default, phx_macros::Saved)]
pub struct GenReport {
    pub distributions: Vec<(String, String)>,
    pub writes: u64,
    pub unnamed: Vec<PartyId>,
    pub apportioned: u64,
    pub unfounded: Vec<Apportioned>,
    pub adjustments: Vec<Adjustment>,
    pub equity: Vec<(PartyId, i128)>,
    /// The parties the writes named, a bit to a party, while the opening runs.
    #[saved(skip)]
    named: Vec<u64>,
    #[saved(skip)]
    sink: Option<Box<dyn ReportSink>>,
}

impl GenReport {
    /// A report listing its writes and apportionments to `sink`, or only counting them where there is none.
    #[must_use]
    pub fn new(sink: Option<Box<dyn ReportSink>>) -> GenReport {
        GenReport { sink, ..GenReport::default() }
    }

    /// A balancing write made: listed, counted, and its parties marked named.
    pub fn write(&mut self, w: WriteRecord) {
        if let Some(sink) = &mut self.sink {
            sink.write(&w);
        }
        self.writes += 1;
        for p in [w.party, w.counter] {
            let (word, bit) = bit_of(p);
            if self.named.len() <= word {
                self.named.resize(word + 1, 0);
            }
            if let Some(x) = self.named.get_mut(word) {
                *x |= bit;
            }
        }
    }

    /// An apportionment made: listed and counted, and kept where it gave a party of no drawn size a share.
    pub fn apportion(&mut self, a: Apportioned) {
        if let Some(sink) = &mut self.sink {
            sink.apportioned(&a);
        }
        self.apportioned += 1;
        if a.drawn == 0 && a.realised > 0 {
            self.unfounded.push(a);
        }
    }

    /// Whether a write named the party while the opening ran.
    #[must_use]
    pub fn named(&self, p: PartyId) -> bool {
        let (word, bit) = bit_of(p);
        self.named.get(word).is_some_and(|x| x & bit != 0)
    }

    /// The opening's report closed: the parties begun that no write named kept, the marks let go and the listing
    /// finished.
    ///
    /// # Errors
    /// What the listing could not put.
    pub fn close(&mut self, unnamed: Vec<PartyId>) -> Result<(), String> {
        self.unnamed = unnamed;
        self.named = Vec::new();
        match self.sink.take() {
            Some(sink) => sink.finish(),
            None => Ok(()),
        }
    }
}

/// A party's word and bit in a set of parties kept a bit to a party.
fn bit_of(p: PartyId) -> (usize, u64) {
    let bits = u64::from(u64::BITS);
    let Ok(word) = usize::try_from(p.get() / bits) else {
        phx_num::capacity_exceeded!("parties named", usize::MAX, p.get());
    };
    (word, 1 << (p.get() % bits))
}

/// A drawn amount the balancing changed, only as far as the accounts needed: what it was, as drawn and as set.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Adjustment {
    pub what: String,
    pub drawn: i128,
    pub set: i128,
}

/// What a contribution works with: the phase's draws, the register and the countries it reads, the report it adds
/// to, and the world's books, which the assembly hands as the ledger's type so the kernel below it need not name it.
pub struct Opening<'a> {
    pub ctx: OpeningCtx<'a>,
    pub day: phx_id::Day,
    pub date: phx_id::Date,
    pub calendar: &'a crate::calendar::Calendar,
    pub register: &'a Register,
    pub countries: &'a [OpeningCountry],
    pub report: &'a mut GenReport,
    pub books: &'a mut dyn Any,
    /// The population kinds' keys, landing indexes and levels, beside the books that keep their cells.
    pub population: &'a mut dyn Any,
    /// The systems' draws of the households' lines, in the order of their systems.
    pub attachments: &'a [(&'static str, Box<dyn Any + Send + Sync>)],
}

impl core::fmt::Debug for Opening<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Opening").field("phase", &self.ctx.phase()).field("countries", &self.countries.len()).finish()
    }
}

/// A system's part of the opening: its phase, what it reads and writes, which sides it draws and which it derives.
#[clause("GEN.3")]
pub trait Contribution: Send + Sync {
    fn name(&self) -> &'static str;
    fn phase(&self) -> OpeningPhase;
    fn reads(&self) -> &'static [&'static str];
    fn writes(&self) -> &'static [&'static str];
    fn drawn(&self) -> &'static [&'static str];
    fn derived(&self) -> &'static [&'static str];
    fn contribute(&self, opening: &mut Opening<'_>);
}

/// A total apportioned across counterparties in proportion to their drawn weights by largest remainder, so the parts
/// sum to the total exactly; remainders that tie go by lot, one draw per party from the given draws.
#[clause("GEN.4")]
#[must_use]
pub fn apportion(total: u64, weights: &[u64], lot: &mut Draws) -> Vec<u64> {
    let sum: u128 = weights.iter().map(|w| u128::from(*w)).sum();
    if sum == 0 {
        phx_num::violation!(clause = "GEN.4", "a total apportioned over no weight", total = total);
    }
    let mut parts: Vec<u64> = Vec::with_capacity(weights.len());
    let mut order: Vec<(u128, u64, usize)> = Vec::with_capacity(weights.len());
    for (i, w) in weights.iter().enumerate() {
        let exact = u128::from(total) * u128::from(*w);
        let Ok(whole) = u64::try_from(exact / sum) else {
            phx_num::capacity_exceeded!("an apportioned part", u64::MAX, total);
        };
        parts.push(whole);
        order.push((exact % sum, lot.next_u64(), i));
    }
    let given: u64 = parts.iter().sum();
    let Ok(left) = usize::try_from(total - given) else {
        phx_num::violation!(clause = "GEN.4", "more remainders than parties", left = total - given);
    };
    order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, _, i) in order.into_iter().take(left) {
        let Some(p) = parts.get_mut(i) else {
            phx_num::violation!(clause = "GEN.4", "a remainder for no party", at = i);
        };
        *p += 1;
    }
    parts
}

/// A total apportioned as `apportion` does over parties each of which takes whole multiples of its unit, an agent's
/// its twins, so each twin's share is whole: a part's residue below its unit passes to the parties of unit one by
/// their weights, and the parts still sum to the total.
#[clause("GEN.4", "REP.9")]
#[must_use]
pub fn apportion_in_units(total: u64, weights: &[u64], units: &[u64], lot: &mut Draws) -> Vec<u64> {
    if weights.len() != units.len() {
        phx_num::violation!(clause = "GEN.4", "weights and units of different parties", weights = weights.len());
    }
    let mut parts = apportion(total, weights, lot);
    let mut residue = 0_u64;
    for (p, u) in parts.iter_mut().zip(units) {
        if *u == 0 {
            phx_num::violation!(clause = "REP.17", "a party of no unit");
        }
        residue += *p % u;
        *p -= *p % u;
    }
    if residue == 0 {
        return parts;
    }
    let ones: Vec<usize> = (0..units.len()).filter(|i| units.get(*i) == Some(&1)).collect();
    let one_weights: Vec<u64> = ones.iter().filter_map(|i| weights.get(*i).copied()).collect();
    for (i, extra) in ones.iter().zip(apportion(residue, &one_weights, lot)) {
        if let Some(p) = parts.get_mut(*i) {
            *p += extra;
        }
    }
    parts
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{apportion, apportion_in_units, opening_subject};

    fn lot() -> Draws {
        Draws::new(stream_key(Seed::new(1), "BNK.opening"), Subject::new(SubjectTag::Opening, 0), 0, 0)
    }

    #[test]
    fn largest_remainder_apportions_exactly() {
        assert_eq!(apportion(10, &[1, 1, 1], &mut lot()).iter().sum::<u64>(), 10, "parts sum to the total");
        assert_eq!(apportion(100, &[50, 30, 20], &mut lot()), vec![50, 30, 20], "exact shares need no remainder");
        assert_eq!(
            apportion(7, &[500, 300, 200], &mut lot()),
            vec![4, 2, 1],
            "3.5, 2.1 and 1.4: the largest remainder"
        );
        let tied = apportion(1, &[1, 1], &mut lot());
        assert_eq!(tied.iter().sum::<u64>(), 1, "a tie goes to one party by lot");
        assert_eq!(tied, apportion(1, &[1, 1], &mut lot()), "the same lot breaks the tie the same way");
    }

    #[test]
    fn units_stay_whole_and_the_residue_goes_to_parties_of_one() {
        let parts = apportion_in_units(1_000, &[1, 1, 1], &[1, 170, 170], &mut lot());
        assert_eq!(parts.iter().sum::<u64>(), 1_000, "parts sum to the total");
        assert_eq!(parts, vec![660, 170, 170], "333 and 333 fall to 170 each, 326 passes to the party of one");
        assert_eq!(apportion_in_units(340, &[1, 1], &[170, 170], &mut lot()), vec![170, 170], "whole shares stay");
    }

    #[test]
    fn a_residue_with_no_party_of_one_is_refused() {
        let caught = std::panic::catch_unwind(|| apportion_in_units(100, &[1, 1], &[170, 170], &mut lot()));
        let payload = caught.expect_err("a residue no party of one can take stops the run");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("GEN.4"));
    }

    #[test]
    fn opening_subjects_distinct() {
        let subjects = [opening_subject(0, 1), opening_subject(1, 0), opening_subject(1, 1), opening_subject(0, 0)];
        for (i, a) in subjects.iter().enumerate() {
            assert!(subjects.iter().skip(i + 1).all(|b| a != b), "stratum and ordinal never share a subject");
        }
    }
}
