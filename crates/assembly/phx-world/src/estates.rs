//! Each estate settled from the business day after it opened, once the day's dues are paid: its money pays its debts
//! through the waterfall, what it owes
//! beyond is written off, what it holds beyond goes to the party the law names where no heir is drawn — the treasury
//! of the estate's country — and then its rows leave and it ends. One whose payment fails waits, counted, and is
//! settled again the next day.

use phx_core::{ESTATE_KIND, StreamDef, SubStep};
use phx_id::{Day, PartyId};
use phx_ledger::apply::ApplyAt;
use phx_ledger::transfer::MoveAt;
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, violation};
use phx_pop::prims::HouseholdsStream;
use phx_rand::{Subject, SubjectTag};

use crate::consts::HEIRLESS_DESTINATION;
use crate::world::World;

impl World {
    /// The party the law names in a country to take what an estate leaves with no heir.
    fn destination(&self, estate: PartyId) -> PartyId {
        let geo = crate::world::geo_in(&self.own);
        let Missing::Present(country) = geo.country_of(self.books.parties.site(estate)) else {
            violation!(clause = "PTY.5", "an estate sited in no country", estate = estate.get());
        };
        let found: Vec<PartyId> = self
            .books
            .parties
            .of_kind(HEIRLESS_DESTINATION)
            .filter(|p| geo.country_of(self.books.parties.site(*p)) == Missing::Present(country))
            .collect();
        let [party] = found.as_slice() else {
            violation!(clause = "POP.15", "a country with other than one heirless estates' destination");
        };
        *party
    }

    /// Every estate opened before today settled, or left waiting when a payment fails.
    #[clause("L3", "POP.9", "POP.15", "PTY.9")]
    pub(crate) fn estates_settle(&mut self, day: Day) {
        let open: Vec<PartyId> = self
            .books
            .parties
            .of_kind(ESTATE_KIND.name)
            .filter(|p| {
                let (place, slot) = self.books.parties.row(*p);
                self.books.parties.table(place).created(slot) < day
            })
            .collect();
        let m = MoveAt {
            contracts: phx_ledger::opening::contract_unit(&self.register),
            rounding: Round::HalfEven,
            day,
            at: ApplyAt::Day(SubStep::S7c),
        };
        for estate in open {
            let destination = self.destination(estate);
            let subject = Subject::new(SubjectTag::Party, estate.get());
            let mut draws = self.streams.open(&HouseholdsStream::DECL, subject, day, SubStep::S7c.ordinal());
            match self.books.settle_estate(estate, destination, m, &mut draws, self.audit.stream()) {
                Ok(s) => {
                    self.cell_day.estates_settled += 1;
                    self.cell_day.estates_passed += i128::from(s.passed);
                    self.cell_day.estates_written_off += i128::from(s.written_off);
                }
                Err(_) => self.cell_day.estates_waiting += 1,
            }
        }
    }
}
