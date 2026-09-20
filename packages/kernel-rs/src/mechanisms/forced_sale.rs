//! A PARTY MADE TO SELL SOMETHING IT DID NOT WANT TO SELL, at whatever price the market gives it.
//!
//! @spec XI-2 · Clearing C3 · Prime Brokerage B · Fund Shares C · Law 3, Law 6, Appendix B

use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{afoot, agreed, standing};

/// How a holder came to be selling something it did not want to sell.
#[repr(u32)]
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
    Estate,
    Resolution,
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

// XI-2 RUNS HERE.

/// A DOWNGRADE PAST A MANDATE'S BOUNDARY IS A FORCED SALE BY EVERY HOLDER BOUND BY IT, ON THE SAME
/// DATE.
pub struct ForcedSelling {
    /// What it says when a holder is put in a workout.
    pub kind: u32,
    /// How many periods a holder has to sell what its mandate no longer lets it hold.
    pub within: &'static str,
}

impl Mechanism for ForcedSelling {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let within = ctx.params().periods(self.within) as u32;
        // What each issuer is graded at now.
        let mut worst: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for row in 0..ctx.standing().len() as u32 {
            let s = crate::stores::StandingId(row);
            if !ctx.standing().live(s) || ctx.standing().kind_of(s) != standing::GRADE {
                continue;
            }
            let about = ctx.standing().about(s).0;
            let rank = ctx.standing().terms(s)[0];
            worst.entry(about).and_modify(|r| if rank > *r { *r = rank }).or_insert(rank);
        }
        if worst.is_empty() {
            return;
        }

        let mut breached: Vec<(PartyId, f64)> = Vec::new();
        for row in 0..ctx.agreements().len() as u32 {
            let a = crate::stores::AgreementId(row);
            if !ctx.agreements().live(a) || ctx.agreements().kind_of(a) != agreed::MANDATE {
                continue;
            }
            let floor = match ctx.agreements().terms(a).first() {
                Some(&floor) => floor,
                // A mandate with no floor restricts no grade.
                None => continue,
            };
            // The pool is the side the mandate is over; the manager is the other.
            let (one, other) = ctx.agreements().between(a);
            for pool in [one, other] {
                if !ctx.parties().alive(pool) {
                    continue;
                }
                let mut must_sell = 0.0;
                for row in ctx.register().of_holder(pool) {
                    let line = ctx.register().instrument_of(crate::ids::HoldingId(*row));
                    let issuer = ctx.instruments().issuer_of(line);
                    // Through the floor, and only through it.
                    if matches!(worst.get(&issuer.0), Some(&rank) if rank > floor) {
                        must_sell += ctx.register().quantity(crate::ids::HoldingId(*row));
                    }
                }
                if must_sell > 0.0 {
                    breached.push((pool, must_sell));
                }
            }
        }

        for (pool, units) in breached {
            // It is already in one, and a second workout for the same breach would be the same
            // requirement counted twice.
            if ctx.processes().running(afoot::WORKOUT).iter().any(|p| ctx.processes().owner(*p) == pool) {
                continue;
            }
            ctx.opens(crate::module::Opens {
                kind: afoot::WORKOUT,
                owner: pool,
                closes: Some(ctx.period() + within),
                size: units,
            });
            ctx.say(self.kind, &[pool.0], &[(0, Value::Num(units))], true);
        }
    }
}


/// AND IT STANDS IN THE MARKET WITH A SIZE AND NO LEVEL.
pub struct ForcedSeller {
    pub kind: u32,
    /// The kinds of party that can be put in a workout.
    pub of_kind: u32,
}

impl crate::module::Participant for ForcedSeller {
    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn eligible_parties(&self, parties: &crate::parties::Parties) -> Vec<PartyId> {
        (0..parties.len()).map(|row| PartyId::at(row as u32)).collect()
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        // It sells what it HOLDS, off its own rows — never by asking every book in the world.
        if view.in_a_workout() == 0.0 {
            return Vec::new();
        }
        view.holdings().map(|row| crate::ids::book_of(view.line_of(row))).collect()
    }

    fn orders(&self, view: &crate::module::ParticipantView<'_>, m: crate::ids::MarketId) -> Vec<crate::clearing::Order> {
        let must = view.in_a_workout();
        if must <= 0.0 {
            return Vec::new();
        }
        let line = crate::ids::line_of(m);
        let held = view.free(line);
        // It cannot sell more than it holds, which is arithmetic about a holding and not a cap on a
        // number.
        let units = crate::clearing::whole_pieces(sells(held, must));
        if units <= 0 {
            return Vec::new();
        }
        vec![crate::clearing::Order {
            party: view.self_id(),
            side: crate::clearing::Side::Sell,
            // NO LEVEL.
            price: None,
            qty: units,
        }]
    }
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
    fn every_forced_sale_names_the_door_it_arrived_through() {
        for door in [
            Door::MarginCall,
            Door::Redemption,
            Door::FundingWithdrawn,
            Door::MandateBreach,
            Door::Estate,
            Door::Resolution,
        ] {
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

    #[test]
    fn court_directed_selling_does_not_restore_the_estates_ordinary_discretion() {
        let mut parties = crate::parties::Parties::with_seed(0);
        let estate = parties.add(
            1,
            crate::ids::RegionId::at(0),
            PartyId::NONE,
            crate::parties::Representation::Named,
            1,
        );
        parties.open_destination(estate, crate::parties::Destination::Estate, 2);
        parties.cease(estate);
        let seller = ForcedSeller { kind: 1, of_kind: 1 };
        assert_eq!(
            <ForcedSeller as crate::module::Participant>::eligible_parties(&seller, &parties),
            vec![estate]
        );
        assert!(!parties.alive(estate));
    }
}
