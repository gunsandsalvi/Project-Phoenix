//! A PARTY MADE TO SELL SOMETHING IT DID NOT WANT TO SELL, at whatever price the market gives it.
//!
//! @spec XI-2 · Clearing C3 · Prime Brokerage B · Fund Shares C · Law 3, Law 6, Appendix B

use crate::ids::{InstrumentId, PartyId};

/// How a holder came to be selling something it did not want to sell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Door {
    /// A margin call the client cannot meet from cash.
    MarginCall,
    /// A redemption the fund cannot meet from its buffer.
    Redemption,
    /// A funding line withdrawn: a leveraged holder's lender stopped lending and the position must
    /// go.
    FundingWithdrawn,
    /// A mandate breach — a downgrade past a boundary forces every holder bound by it to sell at the
    /// same time.
    MandateBreach,
}

/// What a broker requires of a position THIS period.
#[derive(Clone, Copy, Debug)]
pub struct Requirement {
    pub of: PartyId,
    pub on: InstrumentId,
    pub period: u32,
    pub wants: f64,
}

/// What a holder must raise, and by which door.
pub fn must_raise(wants: f64, can_pay_from_cash: f64, door: Door) -> Option<(Door, f64)> {
    let short = wants - can_pay_from_cash;
    if short <= 0.0 {
        return None;
    }
    Some((door, short))
}

/// The order a forced seller posts.
pub fn sells(holding: f64, needs_units: f64) -> f64 {
    if needs_units <= holding {
        needs_units
    } else {
        // It is short of what it needs to raise.
        holding
    }
}

/// The price move reaches somebody else.
pub fn reaches(printed: f64, was: f64, holders: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let moved = printed - was;
    holders.iter().map(|&(who, units)| (who, units * moved)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_requirement_can_rise_which_is_the_whole_mechanism() {
        // A margin expressed as a stated RATE cannot rise, and that deletes precisely the
        // procyclicality that is the contagion.
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
        // Not a cap.
        assert_eq!(sells(40.0, 100.0), 40.0);
        assert_eq!(sells(400.0, 100.0), 100.0);
    }

    #[test]
    fn the_price_move_reaches_holders_who_did_nothing() {
        // The sale moves the price and the price move reaches somebody else.
        let holders = [(PartyId::at(2), 100.0), (PartyId::at(3), 50.0)];
        let hit = reaches(8.0, 10.0, &holders);
        assert_eq!(hit[0], (PartyId::at(2), -200.0));
        assert_eq!(hit[1], (PartyId::at(3), -100.0));
    }
}
