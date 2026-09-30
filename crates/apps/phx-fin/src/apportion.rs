//! `-F apportion`: every split of a whole among claimants the design point makes at its largest — each of `[store]
//! estates`' ranks paid pro rata over its claims, the residue by the largest remainders in the claims' order, and a
//! dividend over a hundred thousand holders by their shares, ties by keys drawn.

use std::collections::BTreeMap;

use phx_num::{Residue, Ties, apportion};
use phx_rand::uniform::below_u64;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, day_of, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base apportionment is measured under.
pub const BASE: &str = "apportion";

/// An estate's rank holds one to this many claims.
const CLAIMS: u64 = 64;

/// A claim is owed up to this many units, and a holder holds up to this many shares.
const CLAIM_UNITS: u64 = 1 << 40;
const HOLDING: u64 = 1 << 20;

/// The holders a dividend is paid over.
const HOLDERS: usize = 100_000;

/// Each estate's claims end to end with where each ends, the holders' shares and tie keys, the shares written, the
/// streams and the day it is.
#[derive(Debug, Default)]
pub struct Apportion {
    claims: Vec<u64>,
    ends: Vec<usize>,
    holdings: Vec<u64>,
    keys: Vec<u64>,
    out: Vec<i64>,
    streams: Option<Streams>,
    today: u64,
    paid: i64,
}

impl Apportion {
    /// Everything the measured days paid, the same on any machine.
    #[must_use]
    pub fn paid(&self) -> i64 {
        self.paid
    }
}

impl FinBase for Apportion {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] estates` ranks of one to sixty-four claims each, and a hundred thousand holders' shares.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let estates = count(&design.store, "estates", "store")?;
        let mut d = streams.draws(BASE, 0, 0);
        for _ in 0..estates {
            for _ in 0..=below_u64(&mut d, CLAIMS) {
                self.claims.push(1 + below_u64(&mut d, CLAIM_UNITS));
            }
            self.ends.push(self.claims.len());
        }
        self.holdings = (0..HOLDERS).map(|_| 1 + below_u64(&mut d, HOLDING)).collect();
        self.keys = vec![0; HOLDERS];
        self.out = vec![0; HOLDERS];
        self.streams = Some(*streams);
        Ok(Filled { rows: estates + wide(HOLDERS) })
    }

    /// Every estate's rank paid what it holds, a share of what its claims are owed, and the dividend declared.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let Some(streams) = self.streams else {
            return Err(FinError("apportionment measured before its fill".to_owned()));
        };
        self.today += 1;
        let mut d = streams.draws(BASE, self.today, day_of(day)?);
        // What each rank holds to pay, drawn below what it owes, as a failed firm's estate holds.
        let mut held = Vec::with_capacity(self.ends.len());
        let mut start = 0;
        for end in &self.ends {
            let owed: u64 = self.claims.get(start..*end).into_iter().flatten().sum();
            held.push(i64::try_from(below_u64(&mut d, owed)).map_err(|e| FinError(e.to_string()))?);
            start = *end;
        }
        for k in &mut self.keys {
            *k = below_u64(&mut d, u64::MAX);
        }
        let dividend = i64::try_from(below_u64(&mut d, CLAIM_UNITS)).map_err(|e| FinError(e.to_string()))?;
        let (claims, ends, out) = (&self.claims, &self.ends, &mut self.out);
        let shares = wide(claims.len());
        let paid = m.read(BASE, "share", shares, || {
            let (mut start, mut paid) = (0, 0);
            for (end, total) in ends.iter().zip(&held) {
                if let (Some(weights), Some(into)) = (claims.get(start..*end), out.get_mut(..*end - start)) {
                    apportion(*total, weights, Residue::LargestRemainder { ties: Ties::Order }, into);
                    paid += into.iter().sum::<i64>();
                }
                start = *end;
            }
            paid
        });
        let (holdings, keys) = (&self.holdings, &self.keys);
        let dividends = m.read(BASE, "dividend", wide(HOLDERS), || {
            apportion(dividend, holdings, Residue::LargestRemainder { ties: Ties::Keys(keys) }, out);
            out.iter().sum::<i64>()
        });
        if paid != held.iter().sum::<i64>() || dividends != dividend {
            return Err(FinError("an apportionment's shares do not sum to its total".to_owned()));
        }
        self.paid += paid + dividends;
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        let words = self.claims.len() + self.holdings.len() + self.keys.len() + self.out.len();
        Bytes { rows: wide(words * size_of::<u64>() + self.ends.len() * size_of::<usize>()), resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
}
