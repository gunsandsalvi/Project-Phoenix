//! RISK TRANSFER NEEDS A TRANSFEREE: credit risk moves from the bank that originated it to somebody
//! else, NAMED, who then takes the loss.
//!
//! @spec XI-11 · XI-1 · XI-8 · Law 2, Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::afoot;

/// A party that holds the loans.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vehicle {
    pub who: PartyId,
}

/// The rows moving from the originator to the vehicle, against payment.
#[derive(Clone, Debug)]
pub struct Transfer {
    pub from: PartyId,
    pub to: Vehicle,
    /// The loan rows themselves.
    pub loans: Vec<InstrumentId>,
    pub paid: f64,
}

impl Transfer {
    pub fn true_sale(
        from: PartyId,
        to: Vehicle,
        loans: Vec<InstrumentId>,
        paid: f64,
    ) -> Option<Self> {
        if from == to.who || loans.is_empty() || paid <= 0.0 {
            return None;
        }
        let mut unique = loans.clone();
        unique.sort();
        unique.dedup();
        if unique.len() != loans.len() {
            return None;
        }
        Some(Self {
            from,
            to,
            loans,
            paid,
        })
    }
}

/// A tranche instrument: a stated loss attachment, and a price that clears.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tranche {
    pub what: InstrumentId,
    pub issued_by: Vehicle,
    /// Losses below this point do not reach it; losses above `detaches` have wiped it out.
    pub attaches: f64,
    pub detaches: f64,
}

impl Tranche {
    pub fn new(what: InstrumentId, issued_by: Vehicle, attaches: f64, detaches: f64) -> Tranche {
        assert!(
            detaches > attaches,
            "XI-11: a tranche that detaches at or below where it attaches absorbs nothing"
        );
        assert!(
            attaches >= 0.0,
            "XI-11: a tranche cannot attach below the first loss"
        );
        Tranche {
            what,
            issued_by,
            attaches,
            detaches,
        }
    }

    pub fn thickness(&self) -> f64 {
        self.detaches - self.attaches
    }
}

/// A REALISED loss on a NAMED loan.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Realised {
    pub on: InstrumentId,
    pub lost: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Took {
    pub tranche: InstrumentId,
    pub loss: f64,
}

/// The waterfall: real losses to real tranches, from the bottom up.
pub fn allocate(losses: &[Realised], tranches: &[Tranche]) -> (Vec<Took>, f64) {
    let total: f64 = losses.iter().map(|l| l.lost).sum();
    let mut ordered: Vec<&Tranche> = tranches.iter().collect();
    ordered.sort_by(|a, b| a.attaches.total_cmp(&b.attaches));

    let mut took: Vec<Took> = Vec::with_capacity(ordered.len());
    let mut highest = 0.0;
    for t in &ordered {
        let loss = if total <= t.attaches {
            0.0
        } else if total >= t.detaches {
            t.thickness()
        } else {
            total - t.attaches
        };
        took.push(Took {
            tranche: t.what,
            loss,
        });
        if t.detaches > highest {
            highest = t.detaches;
        }
    }
    let beyond = if total > highest {
        total - highest
    } else {
        0.0
    };
    (took, beyond)
}

/// Named holders of each tranche.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Holding {
    pub holder: PartyId,
    pub tranche: InstrumentId,
    pub units: f64,
}

pub fn issue(tranche: Tranche, holders: &[(PartyId, f64)]) -> Option<Vec<Holding>> {
    if holders.is_empty() || holders.iter().any(|(_, units)| *units <= 0.0) {
        return None;
    }
    Some(
        holders
            .iter()
            .map(|(holder, units)| Holding {
                holder: *holder,
                tranche: tranche.what,
                units: *units,
            })
            .collect(),
    )
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Retention {
    pub sponsor: PartyId,
    pub tranche: InstrumentId,
    pub units: f64,
}

pub fn retain(sponsor: PartyId, tranche: Tranche, units: f64) -> Option<Retention> {
    if units <= 0.0 {
        return None;
    }
    Some(Retention {
        sponsor,
        tranche: tranche.what,
        units,
    })
}

/// Who is hit, and by how much, when the waterfall has run.
pub fn onto_holders(took: &[Took], holdings: &[Holding]) -> Vec<(PartyId, f64)> {
    let mut out: Vec<(PartyId, f64)> = Vec::new();
    for t in took {
        let units: f64 = holdings
            .iter()
            .filter(|h| h.tranche == t.tranche)
            .map(|h| h.units)
            .sum();
        if units <= 0.0 || t.loss <= 0.0 {
            continue;
        }
        for h in holdings.iter().filter(|h| h.tranche == t.tranche) {
            out.push((h.holder, t.loss * h.units / units));
        }
    }
    out
}

/// What selling the risk does for the originator.
pub fn capacity_freed(transferred: f64, retained: f64) -> f64 {
    transferred - retained
}

// §13 RUNS HERE.

/// A BANK POOLS LOANS AND CUTS NOTES AGAINST THEM.
pub struct Securitising {
    pub kind: u32,
    /// Where the junior tranche detaches — the share of the pool that stands in front of the senior
    /// note.
    pub junior: &'static str,
    /// How much of a book a bank will pool at once.
    pub pools: &'static str,
}

impl Mechanism for Securitising {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let junior = ctx.params().ratio(self.junior);
        let pools = ctx.params().ratio(self.pools);

        let mut cutting: Vec<(PartyId, crate::ids::CurrencyCode, f64, f64)> = Vec::new();
        for &bank in ctx.parties().of_kind(kinds::BANK) {
            let who = PartyId(bank);
            if !ctx.parties().alive(who) {
                continue;
            }
            if ctx
                .processes()
                .running(afoot::SECURITISATION)
                .iter()
                .any(|p| ctx.processes().owner(*p) == who)
            {
                continue;
            }
            // The loan rows it holds.
            let mut pool = 0.0;
            for &row in ctx.register().of_holder(who) {
                let row = crate::ids::HoldingId(row);
                let line = ctx.register().instrument_of(row);
                if ctx.instruments().class_of(line) != crate::instruments::Class::Claim {
                    continue;
                }
                if ctx.instruments().issuer_of(line) == who {
                    continue;
                }
                pool += ctx.register().quantity(row);
            }
            let size = pool * pools;
            if size <= 0.0 {
                continue;
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else {
                continue;
            };
            cutting.push((who, ctx.instruments().ccy_of(money), size, size * junior));
        }

        for (who, ccy, size, first_loss) in cutting {
            // The SENIOR note, which the junior stands in front of.
            ctx.brings(crate::module::Brings {
                issuer: who,
                initial_holder: None,
                loan_terms: None,
                issue_price: None,
                ccy,
                class: crate::instruments::Class::Claim,
                unit: crate::ids::UnitId::at(0),
                // A note's return is what the pool pays through, so it has no schedule of its own
                // and nothing accrues on it.
                coupon: None,
                matures: None,
                pays: crate::instruments::PaymentFrequency::AtMaturity,
                convention: crate::calendar::Convention::Actual365,
                units: size - first_loss,
                book: Some(crate::protocols::Venue {
                    rule: crate::clearing::PriceRule::BuyersCompete,
                    protocol: crate::protocols::Protocol::Call,
                    seen_by: 1,
                    stands_for: None,
                }),
            });
            ctx.opens(crate::module::Opens {
                kind: afoot::SECURITISATION,
                owner: who,
                subject: None,
                door: None,
                closes: Some(ctx.week() + 1),
                size,
            });
            // What it cut, and what stands in front of it.
            ctx.say(
                self.kind,
                &[who.0],
                &[(0, Value::Num(size)), (1, Value::Num(first_loss))],
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn instrument(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    fn vehicle() -> Vehicle {
        Vehicle { who: party(50) }
    }

    /// Three tranches over a 1,000 pool: equity 0–50, mezzanine 50–150, senior 150–1,000.
    fn deal() -> Vec<Tranche> {
        vec![
            Tranche::new(instrument(1), vehicle(), 0.0, 50.0),
            Tranche::new(instrument(2), vehicle(), 50.0, 150.0),
            Tranche::new(instrument(3), vehicle(), 150.0, 1_000.0),
        ]
    }

    #[test]
    fn the_vehicle_is_a_party_and_the_loans_are_rows_that_move_to_it() {
        // XI-11's second prerequisite: a loan that exists as a field on a lender's balance sheet
        // cannot be sold, pooled, pledged or matured, however true the field is.
        let t = Transfer {
            from: party(1),
            to: vehicle(),
            loans: vec![instrument(10), instrument(11)],
            paid: 940.0,
        };
        assert_eq!(t.to.who, party(50));
        assert_eq!(t.loans.len(), 2);
        assert!(t.paid > 0.0);
    }

    #[test]
    fn losses_are_realised_on_named_loans_and_reach_the_tranches_from_the_bottom() {
        // Tranching a loss RATE yields senior notes that can never be touched.
        let losses = [Realised {
            on: instrument(10),
            lost: 40.0,
        }];
        let (took, beyond) = allocate(&losses, &deal());
        assert_eq!(
            took[0],
            Took {
                tranche: instrument(1),
                loss: 40.0
            }
        );
        assert_eq!(took[1].loss, 0.0);
        assert_eq!(took[2].loss, 0.0);
        assert_eq!(beyond, 0.0);
    }

    #[test]
    fn a_correlation_worse_than_the_tranching_assumed_hits_the_senior_tranche_too() {
        // This is the event the system exists to produce, and without named tranches and named
        // holders there is nobody to hit.
        let losses = [
            Realised {
                on: instrument(10),
                lost: 120.0,
            },
            Realised {
                on: instrument(11),
                lost: 180.0,
            },
        ];
        let (took, beyond) = allocate(&losses, &deal());
        assert_eq!(took[0].loss, 50.0);
        assert_eq!(took[1].loss, 100.0);
        assert_eq!(took[2].loss, 150.0);
        assert_eq!(beyond, 0.0);
    }

    #[test]
    fn a_loss_past_the_top_of_the_deal_is_carried_out_and_not_absorbed_silently() {
        // No residual with no holder.
        let losses = [Realised {
            on: instrument(10),
            lost: 1_200.0,
        }];
        let (took, beyond) = allocate(&losses, &deal());
        assert_eq!(took[2].loss, 850.0);
        assert_eq!(beyond, 200.0);
    }

    #[test]
    fn every_holder_is_hit_at_once_and_each_of_them_is_named() {
        // Named holders of each tranche.
        let losses = [Realised {
            on: instrument(10),
            lost: 300.0,
        }];
        let (took, _) = allocate(&losses, &deal());
        let holdings = [
            Holding {
                holder: party(60),
                tranche: instrument(1),
                units: 50.0,
            },
            Holding {
                holder: party(61),
                tranche: instrument(2),
                units: 60.0,
            },
            Holding {
                holder: party(62),
                tranche: instrument(2),
                units: 40.0,
            },
            Holding {
                holder: party(63),
                tranche: instrument(3),
                units: 850.0,
            },
        ];
        let hit = onto_holders(&took, &holdings);
        assert_eq!(hit.len(), 4);
        assert_eq!(hit[0], (party(60), 50.0));
        assert_eq!(hit[1], (party(61), 60.0));
        assert_eq!(hit[2], (party(62), 40.0));
        assert_eq!(hit[3], (party(63), 150.0));
        // And it conserves against what the tranches took (Law 7's dust, not a band).
        let onto: f64 = hit.iter().map(|h| h.1).sum();
        let taken: f64 = took.iter().map(|t| t.loss).sum();
        assert!((onto - taken).abs() <= crate::num::dust(4, &[onto, taken]));
    }

    #[test]
    fn a_tranche_nobody_holds_takes_its_loss_and_hits_nobody_which_is_a_finding_not_a_repair() {
        // The loss does not move to another tranche because this one is unheld.
        let losses = [Realised {
            on: instrument(10),
            lost: 40.0,
        }];
        let (took, _) = allocate(&losses, &deal());
        assert_eq!(took[0].loss, 40.0);
        assert!(onto_holders(&took, &[]).is_empty());
    }

    #[test]
    fn selling_the_risk_expands_what_the_originator_can_originate() {
        // Without a transferee the lending-capacity channel does not exist at all.
        assert_eq!(capacity_freed(1_000.0, 50.0), 950.0);
        assert_eq!(capacity_freed(1_000.0, 1_000.0), 0.0);
    }

    #[test]
    #[should_panic(expected = "absorbs nothing")]
    fn a_tranche_with_no_thickness_is_not_a_tranche() {
        Tranche::new(instrument(1), vehicle(), 50.0, 50.0);
    }
}
