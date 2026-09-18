//! A PARTY MADE TO SELL SOMETHING IT DID NOT WANT TO SELL, at whatever price the market gives it.
//!
//! @spec XI-2 · Clearing C3 · Prime Brokerage B · Fund Shares C · Law 3, Law 6, Appendix B
//!
//! The sale happens **in the same period**, **it moves the price**, and **the price move reaches
//! somebody else**. That chain is the contagion mechanism, and each link is a thing that can be
//! silently deleted.
//!
//! **There are exactly four doors**, and the model needs all of them because they arrive from
//! different directions. Each is a `Door` here, so a world can be asked which one opened rather
//! than being told a position "was reduced".
//!
//! **The requirement must be able to RISE.** A margin expressed as a stated rate cannot, and that
//! deletes precisely the procyclicality that IS the contagion — which is why `Requirement` is a
//! number the broker sets each period from what it sees, and never a declared ratio.
//!
//! **And a forced seller names no price** (Clearing C3, XI-2): the order carries a size and no
//! level, so it is struck at whatever the other side posted. A forced sale with a reservation price
//! is a sale that can decline, which is not forced.

use crate::ids::{InstrumentId, PartyId};

/// XI-2: how a holder came to be selling something it did not want to sell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Door {
    /// A margin call the client cannot meet from cash. The broker's requirement ROSE — because
    /// prices moved, or because it likes what it sees less.
    MarginCall,
    /// A redemption the fund cannot meet from its buffer. **A redemption rationed by the fund's
    /// cash, with the unfilled part dropped, is not a redemption** — so the whole of it comes here.
    Redemption,
    /// A funding line withdrawn: a leveraged holder's lender stopped lending and the position must
    /// go.
    FundingWithdrawn,
    /// A mandate breach — a downgrade past a boundary forces every holder bound by it to sell **at
    /// the same time**. The most mechanical and most synchronised of the four.
    MandateBreach,
}

/// What a broker requires of a position THIS period. It is a number it sets from what it sees, so
/// it can RISE — which is the procyclicality XI-2 is about. A stated rate could not, and a margin
/// that cannot rise deletes the mechanism while looking locally reasonable.
#[derive(Clone, Copy, Debug)]
pub struct Requirement {
    pub of: PartyId,
    pub on: InstrumentId,
    pub period: u32,
    pub wants: f64,
}

/// What a holder must raise, and by which door. `None` where it can meet the call from what it
/// holds — being able to pay is not a forced sale, and calling it one would put sellers into books
/// that nobody made sell.
pub fn must_raise(wants: f64, can_pay_from_cash: f64, door: Door) -> Option<(Door, f64)> {
    let short = wants - can_pay_from_cash;
    if short <= 0.0 {
        return None;
    }
    Some((door, short))
}

/// XI-2, Clearing C3: **the order a forced seller posts.** A size, and NO level — it is struck at
/// whatever the other side posted, which is what makes the sale move the price. A forced sale with
/// a reservation price is a sale that can decline, and then the channel is closed.
///
/// It sells what it holds, and it cannot sell more than that (Law 6: arithmetic impossibility, not
/// a cap). Whether that covers the call is a separate fact the caller reads.
pub fn sells(holding: f64, needs_units: f64) -> f64 {
    if needs_units <= holding {
        needs_units
    } else {
        // It is short of what it needs to raise. That is a REAL state — the position does not
        // cover the call — and the caller handles it. Nothing is invented to close the gap.
        holding
    }
}

/// XI-2: **the price move reaches somebody else.** Every other holder of the line is marked at what
/// the forced sale printed, so a sale by one party is a loss to parties that did nothing — which is
/// the whole of the contagion and is a READ of the print rather than a second channel.
pub fn reaches(printed: f64, was: f64, holders: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let moved = printed - was;
    holders.iter().map(|&(who, units)| (who, units * moved)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_requirement_can_rise_which_is_the_whole_mechanism() {
        // XI-2: a margin expressed as a stated RATE cannot rise, and that deletes precisely the
        // procyclicality that is the contagion. This is a number set each period from what the
        // broker sees, so a worse week makes it bigger.
        let calm = Requirement { of: PartyId::at(1), on: InstrumentId::at(4), period: 3, wants: 100.0 };
        let stressed = Requirement { wants: 260.0, period: 4, ..calm };
        assert!(stressed.wants > calm.wants);
    }

    #[test]
    fn a_holder_that_can_pay_is_not_a_forced_seller() {
        assert!(must_raise(100.0, 250.0, Door::MarginCall).is_none());
        // And one that cannot is forced by a NAMED door, so a world can be asked which opened.
        assert_eq!(must_raise(300.0, 250.0, Door::MarginCall), Some((Door::MarginCall, 50.0)));
    }

    #[test]
    fn all_four_doors_are_here_because_they_arrive_from_different_directions() {
        for door in [Door::MarginCall, Door::Redemption, Door::FundingWithdrawn, Door::MandateBreach] {
            assert_eq!(must_raise(10.0, 0.0, door), Some((door, 10.0)));
        }
    }

    #[test]
    fn a_position_short_of_the_call_sells_what_it_has_and_the_gap_is_real() {
        // Law 6: not a cap. It cannot sell units it does not hold, and the shortfall is a state
        // somebody handles rather than a number quietly closed.
        assert_eq!(sells(40.0, 100.0), 40.0);
        assert_eq!(sells(400.0, 100.0), 100.0);
    }

    #[test]
    fn the_price_move_reaches_holders_who_did_nothing() {
        // XI-2: the sale moves the price and the price move reaches somebody else. A bystander
        // holding the same line is poorer, and it never traded.
        let holders = [(PartyId::at(2), 100.0), (PartyId::at(3), 50.0)];
        let hit = reaches(8.0, 10.0, &holders);
        assert_eq!(hit[0], (PartyId::at(2), -200.0));
        assert_eq!(hit[1], (PartyId::at(3), -100.0));
    }
}
