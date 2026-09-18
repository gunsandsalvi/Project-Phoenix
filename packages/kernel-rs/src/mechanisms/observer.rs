//! THE OBSERVER SURFACE AND THE NEWS: everything shown is a READ of the state, **news never causes
//! anything**, and **observing must not move the model.**
//!
//! @spec 45 A1 · 45 A1.a · 45 A2 · 45 A3 · 45 A4 · 45 A5 · 45 A5.a · 45 B1 · 45 B2 · 45 B2.a · 45 B3 ·
//! @spec 45 B4 · 45 B5 · 45 C1 · 45 C1.a · 45 C2 · 45 C2.a · 45 C3 · 45 C4 · 45 D1 · 45 D2 · 45 D3 ·
//! @spec 45 E1 · 45 E2 · 45 E3 · 45 F1 · 45 F2 · 45 F3 · Law 3, Law 8, Law 9, Law 19 · Appendix B
//!
//! **No surface that changes the model** (E3). Every function here takes shared references and returns
//! values; there is no `&mut` anywhere in this file, so observing cannot move a balance, a price or a
//! period. That is the prohibition as a type signature rather than as a rule somebody remembers.
//!
//! **News never causes anything** (B2.a). A `Report` is generated FROM a change of state that already
//! happened, carries its subjects, and has no path back into anything — an event that moved a price
//! directly would be an exogenous shock wearing a headline.
//!
//! **No observer sees another party's private state** (A4). `visible_to` answers for the asking party
//! and refuses everybody else's positions, intentions and limits — and C2.a's privileged actor cannot
//! be written, because acting takes the same means anybody needs.
//!
//! **A stale mark must be VISIBLY stale** (A1.a): a screen that shows a price without saying when it
//! traded is misinformation, so `Shown` carries the period the price is from and `is_stale` is a read
//! against now.
//!
//! **A statistic available instantly and exactly is not a statistic; it is the model's internals**
//! (A5.a). `published` answers `None` until the lag has passed.
//!
//! **No display-only number** (E1): if it is worth showing it is worth deriving, and everything here
//! derives from prints, holdings and events.

use crate::ids::{InstrumentId, PartyId};
use crate::prices::{Print, Provenance};

/// A1, A1.a, F2: **a print — a price that cleared, with its instrument, time and unit** — and **a stale
/// mark must be visibly stale.** F2: every priced asset shows its price, and fixed income shows both the
/// price and the spread derived from it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shown {
    pub instrument: InstrumentId,
    pub price: f64,
    /// A1.a: **when it traded.** A screen without this is misinformation.
    pub from_period: u32,
    /// Clearing C4: a book that ran and had nothing cross carries its last level AND SAYS SO.
    pub provenance: Provenance,
}

impl Shown {
    pub fn of(p: &Print) -> Shown {
        Shown { instrument: p.instrument, price: p.price, from_period: p.period, provenance: p.provenance }
    }

    /// A1.a: visibly stale. The reader is told, rather than the number being quietly refreshed.
    pub fn is_stale(&self, now: u32) -> bool {
        self.from_period < now || self.provenance != Provenance::Cleared
    }

    /// F2: fixed income shows the price **and** the spread derived from it — derived, never the other
    /// way round (Law 3, §7 D8).
    pub fn spread_against(&self, risk_free: f64, years: f64) -> Option<f64> {
        if self.price <= 0.0 || risk_free <= 0.0 || years <= 0.0 {
            return None;
        }
        Some((risk_free / self.price).powf(1.0 / years) - 1.0)
    }
}

/// A2, A4: **its own positions and balances, exactly as the register and the accounts hold them** — and
/// **no observer sees another party's private state.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Holding {
    pub holder: PartyId,
    pub what: InstrumentId,
    pub units: f64,
}

/// A4: the refusal, as a read. A party asking about itself is answered; asking about anybody else is
/// not — positions, intentions and limits are private.
pub fn visible_to(asking: PartyId, holdings: &[Holding]) -> Vec<Holding> {
    holdings.iter().filter(|h| h.holder == asking).copied().collect()
}

/// A5, A5.a: **aggregates that are genuinely published — indices, official statistics — WITH THE LAG**
/// they really have. A statistic available instantly and exactly is not a statistic; it is the model's
/// internals.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Statistic {
    pub about_period: u32,
    pub value: f64,
    pub published_in: u32,
    /// A5: and it is revised, like any statistic.
    pub revised_from: Option<f64>,
}

pub fn published(s: &Statistic, now: u32) -> Option<f64> {
    if now < s.published_in {
        return None;
    }
    Some(s.value)
}

/// B1, B3: **a change of state that somebody would notice** — a default, a downgrade, a policy move —
/// **with a time and NAMED SUBJECTS, so it can be checked against the state.**
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub period: u32,
    pub subjects: Vec<PartyId>,
    pub about: Happened,
    /// B4: **it can be wrong or incomplete in the same way real reporting is, but it may never be
    /// about something that did not happen.**
    pub incomplete: bool,
}

/// B2: it describes something that ACTUALLY HAPPENED in the state, and is generated FROM it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Happened {
    Defaulted,
    Downgraded,
    PolicyMoved,
    /// B5: **a story develops** — a workout runs for periods, paying classes and selling assets — so a
    /// report can be one instalment of something still running.
    WorkoutContinues { period_of_it: u32 },
    Failed,
}

/// B2, B2.a, B3: **generated FROM the state.** The function takes what happened and returns the report;
/// **there is no path from a report back into anything** — an event that moved a price directly would be
/// an exogenous shock wearing a headline.
pub fn report(period: u32, subjects: Vec<PartyId>, about: Happened, incomplete: bool) -> Report {
    assert!(!subjects.is_empty(), "45 B3: a report with no named subject cannot be checked against the state");
    Report { period, subjects, about, incomplete }
}

/// B4: **it may never be about something that did not happen.** The check is against the events the
/// state actually recorded, party by party — a report that names nobody who did anything is a fiction.
pub fn is_true_of(r: &Report, what_happened: &[(PartyId, Happened)]) -> bool {
    r.subjects
        .iter()
        .all(|s| what_happened.iter().any(|(who, ev)| who == s && *ev == r.about))
}

/// C1, C1.a, C2, C2.a: **the actions available are the ones ANY participant has** — post a schedule,
/// trade, lend — and **acting means entering a market that must clear: the price is not the actor's to
/// set.** An action **requires the means**, and there is **no privileged actor**.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Acts {
    /// C1.a: it posts into a book. What it gets is what the book gives it.
    Posted { units: f64, at_price: f64 },
    /// C2: it lacks the means — the cash, the holding, the borrow — and so it does not act.
    HasNotTheMeans { short_by: f64 },
}

pub fn act(wants: f64, at_price: f64, has_cash: f64, has_units: f64, selling: bool) -> Acts {
    let needs = if selling { has_units } else { has_cash / at_price };
    if needs < wants {
        return Acts::HasNotTheMeans { short_by: wants - needs };
    }
    // C1.a: posting is all it can do. The clearing decides the rest, exactly as for anybody else.
    Acts::Posted { units: wants, at_price }
}

/// D1: **a history that is a READ of what happened, not a separate log that can drift** (Law 4, Law 19).
pub fn history_of(who: PartyId, events: &[(u32, PartyId, Happened)]) -> Vec<(u32, Happened)> {
    events
        .iter()
        .filter(|(_, p, _)| *p == who)
        .map(|(period, _, what)| (*period, *what))
        .collect()
}

/// D2: **performance is computed from real positions and real prices, SO IT CAN BE BAD.** `None` where
/// something it holds has no price — an unpriced read is missing, not zero (Value XI-6).
pub fn performance(holdings: &[Holding], prices: &[Shown], cost: f64) -> Option<f64> {
    let mut worth = 0.0;
    for h in holdings {
        let p = prices.iter().find(|s| s.instrument == h.what)?;
        worth += h.units * p.price;
    }
    Some(worth - cost)
}

/// D3, E1: **anything shown must be REPRODUCIBLE from the state**; a number on the surface with no
/// derivation is a display-only number, and there are none. This is the read that says where a shown
/// figure came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DerivedFrom {
    APrint,
    AHolding,
    AnEvent,
    /// A5: a published statistic, with its lag.
    AStatistic,
}

/// F1, Law 9: **every instrument is displayed by the name a market would use, from ONE grammar** — and
/// an internal id is never a display name. The grammar takes the facts a market names it by.
pub fn display_name(issuer: &str, coupon: Option<f64>, maturity: Option<u32>) -> String {
    match (coupon, maturity) {
        (Some(c), Some(m)) => format!("{issuer} {c} {m}"),
        (None, Some(m)) => format!("{issuer} {m}"),
        // Law 9: a share is named by its issuer, and nothing else.
        _ => issuer.to_string(),
    }
}

/// F3: **one calendar — and not a second one.** A surface that dates anything by its own clock is
/// showing a different world from the one that ran, so a shown period is the kernel's own.
pub fn dated_by(period: u32) -> u32 {
    period
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CurrencyCode, MarketId};
    use crate::prices::QuotedAs;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn instrument(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    fn print(n: u32, period: u32, price: f64, provenance: Provenance) -> Print {
        Print {
            instrument: instrument(n),
            market: MarketId::at(0),
            period,
            price,
            ccy: CurrencyCode::at(0),
            quoted_as: QuotedAs::Money,
            provenance,
        }
    }

    #[test]
    fn a_stale_mark_is_visibly_stale() {
        // A1.a: a screen that shows a price without saying when it traded is misinformation.
        let fresh = Shown::of(&print(1, 10, 99.5, Provenance::Cleared));
        assert!(!fresh.is_stale(10));
        assert!(fresh.is_stale(11));
        // And a carried price is stale on the period it is shown in, because nothing crossed.
        let carried = Shown::of(&print(1, 10, 99.5, Provenance::Carried));
        assert!(carried.is_stale(10));
    }

    #[test]
    fn fixed_income_shows_the_price_and_the_spread_derived_from_it() {
        // F2, Law 3: derived FROM the price, never into it.
        let s = Shown::of(&print(1, 10, 92.0, Provenance::Cleared));
        assert!(s.spread_against(100.0, 5.0).unwrap() > 0.0);
        let unpriced = Shown { price: 0.0, ..s };
        assert!(unpriced.spread_against(100.0, 5.0).is_none());
    }

    #[test]
    fn no_observer_sees_another_partys_private_state() {
        // A4: positions, intentions and limits are private.
        let holdings = [
            Holding { holder: party(10), what: instrument(1), units: 500.0 },
            Holding { holder: party(11), what: instrument(1), units: 900.0 },
        ];
        let mine = visible_to(party(10), &holdings);
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].holder, party(10));
        assert!(visible_to(party(99), &holdings).is_empty());
    }

    #[test]
    fn a_statistic_available_instantly_is_not_a_statistic() {
        // A5, A5.a: it is the model's internals. The lag is real and the number does not exist before
        // it is published.
        let s = Statistic { about_period: 10, value: 3.2, published_in: 12, revised_from: None };
        assert!(published(&s, 11).is_none());
        assert_eq!(published(&s, 12), Some(3.2));
    }

    #[test]
    fn news_is_generated_from_the_state_and_never_about_something_that_did_not_happen() {
        // B2, B2.a, B4: an event that moved a price directly would be an exogenous shock wearing a
        // headline — and there is no path from a report back into anything.
        let r = report(12, vec![party(9)], Happened::Defaulted, false);
        let what_happened = [(party(9), Happened::Defaulted)];
        assert!(is_true_of(&r, &what_happened));
        let fiction = report(12, vec![party(8)], Happened::Defaulted, false);
        assert!(!is_true_of(&fiction, &what_happened));
    }

    #[test]
    fn a_report_can_be_incomplete_but_must_name_its_subjects() {
        // B3, B4: so it can be checked against the state, and it can be wrong the way real reporting
        // is — without being about nothing.
        let partial = report(12, vec![party(9)], Happened::WorkoutContinues { period_of_it: 3 }, true);
        assert!(partial.incomplete);
        assert_eq!(partial.subjects, vec![party(9)]);
    }

    #[test]
    #[should_panic(expected = "cannot be checked against the state")]
    fn a_report_with_no_named_subject_is_refused() {
        report(12, Vec::new(), Happened::Defaulted, false);
    }

    #[test]
    fn a_story_develops_over_periods() {
        // B5: a workout runs for periods, paying classes and selling assets, and each instalment is a
        // report of what the state did that period.
        let first = report(12, vec![party(9)], Happened::WorkoutContinues { period_of_it: 1 }, true);
        let later = report(15, vec![party(9)], Happened::WorkoutContinues { period_of_it: 4 }, true);
        assert!(later.period > first.period);
    }

    #[test]
    fn there_is_no_privileged_actor_and_acting_needs_the_means() {
        // C1, C1.a, C2, C2.a: nobody transacts without the balance, outside the mechanism, and the
        // price is not the actor's to set — it posts, and the book decides.
        assert_eq!(act(100.0, 10.0, 2_000.0, 0.0, false), Acts::Posted { units: 100.0, at_price: 10.0 });
        assert_eq!(act(100.0, 10.0, 400.0, 0.0, false), Acts::HasNotTheMeans { short_by: 60.0 });
        // Selling needs the holding, exactly as for anybody else.
        assert_eq!(act(100.0, 10.0, 0.0, 400.0, true), Acts::Posted { units: 100.0, at_price: 10.0 });
        assert_eq!(act(100.0, 10.0, 0.0, 40.0, true), Acts::HasNotTheMeans { short_by: 60.0 });
    }

    #[test]
    fn the_history_is_a_read_and_not_a_second_log_that_can_drift() {
        // D1, Law 4, Law 19.
        let events = [
            (10, party(9), Happened::Downgraded),
            (12, party(9), Happened::Defaulted),
            (12, party(8), Happened::Failed),
        ];
        let theirs = history_of(party(9), &events);
        assert_eq!(theirs.len(), 2);
        assert_eq!(theirs[1], (12, Happened::Defaulted));
    }

    #[test]
    fn performance_is_computed_from_real_positions_and_real_prices_so_it_can_be_bad() {
        // D2: and an unpriced holding leaves it MISSING rather than zero.
        let holdings = [Holding { holder: party(10), what: instrument(1), units: 100.0 }];
        let prices = [Shown::of(&print(1, 10, 8.0, Provenance::Cleared))];
        assert_eq!(performance(&holdings, &prices, 1_000.0), Some(-200.0));
        assert!(performance(&holdings, &[], 1_000.0).is_none());
    }

    #[test]
    fn an_instrument_is_displayed_by_the_name_a_market_would_use() {
        // F1, Law 9: one grammar, and an internal id is never a display name.
        assert_eq!(display_name("firm.4", Some(4.5), Some(2031)), "firm.4 4.5 2031");
        assert_eq!(display_name("firm.4", None, Some(2031)), "firm.4 2031");
        assert_eq!(display_name("firm.4", None, None), "firm.4");
    }

    #[test]
    fn the_surface_dates_everything_by_the_one_calendar() {
        // F3: a surface with its own clock is showing a different world from the one that ran.
        assert_eq!(dated_by(12), 12);
    }

    #[test]
    fn observing_changes_nothing() {
        // E3: the prohibition as a type signature — every read here takes a shared reference and
        // returns a value, so there is no path by which looking could move a balance or a price.
        let holdings = [Holding { holder: party(10), what: instrument(1), units: 500.0 }];
        let before = holdings;
        let _ = visible_to(party(10), &holdings);
        let _ = performance(&holdings, &[], 0.0);
        assert_eq!(holdings, before);
    }
}
