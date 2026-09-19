//! RISK TRANSFER NEEDS A TRANSFEREE: credit risk moves from the bank that originated it to somebody
//! else, NAMED, who then takes the loss.
//!
//! @spec XI-11 · XI-1 · XI-8 · Law 2, Law 3, Law 5, Law 6, Law 19 · Appendix B
//!
//! It needs four things and each is a real object: a vehicle that is a party holding the loans; a
//! tranche instrument with a stated loss attachment and a cleared price; a waterfall that
//! allocates real losses to real tranches; and named holders of each tranche.
//!
//! Why its absence is structural rather than cosmetic. Without it, small-business and mortgage
//! credit risk sits on the originating bank for ever: origination capacity can never be expanded by
//! selling risk, so the lending-capacity channel does not exist. And the event this system exists to
//! produce — correlation worse than the tranching assumed, so the senior tranche takes losses it
//! was not supposed to and every holder is hit at once — has no holders to hit.
//!
//! Its two prerequisites, in order. XI-1 first, because tranching a loss RATE yields senior notes
//! that can never be touched: `allocate` takes realised losses on named loans, and there is no rate
//! anywhere in this module. And loans must be rows in a register before they can be transferred
//! at all — a loan that exists as a field on a lender's balance sheet cannot be sold, pooled, pledged
//! or matured, however true the field is. `Transfer` moves rows, and the vehicle is a party that
//! holds them.
//!
//! The attachment is a stated loss point, not a rating. A tranche's attachment and detachment are
//! terms of the instrument (Law 2: a POLICY of the deal). What it is worth is a cleared price and
//! never a function of them.

use crate::assembly::kinds;
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::afoot;
use crate::ids::{InstrumentId, PartyId};

/// A party that holds the loans. It is a party — it can be owed money, it can fail, it has an estate
///  — and not a bucket on the originator's balance sheet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vehicle {
    pub who: PartyId,
}

/// The rows moving from the originator to the vehicle, against payment. Law 5: two sides, both legs,
/// same pass — the originator delivers the loan rows and receives what the vehicle paid for them.
#[derive(Clone, Debug)]
pub struct Transfer {
    pub from: PartyId,
    pub to: Vehicle,
    /// The loan rows themselves. A loan that is a field cannot be here at all, which is XI-11's
    /// second prerequisite stated as a type.
    pub loans: Vec<InstrumentId>,
    pub paid: f64,
}

/// A tranche instrument: a stated loss attachment, and a price that clears. The attachment is a
/// term of the deal; the price is an outcome and never a function of the attachment.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tranche {
    pub what: InstrumentId,
    pub issued_by: Vehicle,
    /// Losses below this point do not reach it; losses above `detaches` have wiped it out. Both are
    /// in money, on the pool — never percentages of a rate.
    pub attaches: f64,
    pub detaches: f64,
}

impl Tranche {
    pub fn new(what: InstrumentId, issued_by: Vehicle, attaches: f64, detaches: f64) -> Tranche {
        assert!(
            detaches > attaches,
            "XI-11: a tranche that detaches at or below where it attaches absorbs nothing"
        );
        assert!(attaches >= 0.0, "XI-11: a tranche cannot attach below the first loss");
        Tranche { what, issued_by, attaches, detaches }
    }

    pub fn thickness(&self) -> f64 {
        self.detaches - self.attaches
    }
}

/// A REALISED loss on a NAMED loan. Not a rate, and not an expectation — tranching a loss rate
/// yields senior notes that can never be touched, which is the whole reason XI-1 comes first.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Realised {
    pub on: InstrumentId,
    pub lost: f64,
}

/// What a tranche took.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Took {
    pub tranche: InstrumentId,
    pub loss: f64,
}

/// The waterfall: real losses to real tranches, from the bottom up. Law 6: a tranche is not
/// written down "up to" anything — it absorbs what falls in its band and the band runs out, which is
/// arithmetic.
///
/// A loss above the top tranche's detachment point has nowhere to go and is returned as `beyond`: it
/// is the vehicle's own hole and it lands on whoever holds its residual. A residual with no holder
/// is a defect, so it is carried out rather than absorbed silently.
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
        took.push(Took { tranche: t.what, loss });
        if t.detaches > highest {
            highest = t.detaches;
        }
    }
    let beyond = if total > highest { total - highest } else { 0.0 };
    (took, beyond)
}

/// Named holders of each tranche. Without them the event this system exists to produce has
/// nobody to hit.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Holding {
    pub holder: PartyId,
    pub tranche: InstrumentId,
    pub units: f64,
}

/// Who is hit, and by how much, when the waterfall has run. Pro rata within a tranche, because two
/// holdings of the same instrument have no reason to be told apart — and every one of them is a
/// named party (Law 5: the loss has a holder).
pub fn onto_holders(took: &[Took], holdings: &[Holding]) -> Vec<(PartyId, f64)> {
    let mut out: Vec<(PartyId, f64)> = Vec::new();
    for t in took {
        let units: f64 = holdings.iter().filter(|h| h.tranche == t.tranche).map(|h| h.units).sum();
        if units <= 0.0 || t.loss <= 0.0 {
            continue;
        }
        for h in holdings.iter().filter(|h| h.tranche == t.tranche) {
            out.push((h.holder, t.loss * h.units / units));
        }
    }
    out
}

/// What selling the risk does for the originator. Its capacity to originate is expanded by
/// the risk it no longer carries — this is the lending-capacity channel, and without a transferee it
/// does not exist at all. A read of what it transferred, never a bonus anybody grants.
pub fn capacity_freed(transferred: f64, retained: f64) -> f64 {
    transferred - retained
}

// §13 RUNS HERE. `Securitising` was in `running.rs`, apart from `allocate` and the
// tranche arithmetic it is about. One system, one file.

/// A BANK POOLS LOANS AND CUTS NOTES AGAINST THEM.
///
/// The `securitisation` row was a CLOSER for a pool nothing opened, and a pool's whole point is to
/// ISSUE notes — which is what 21.81 said it could not do. `ctx.brings` is the door now and
/// this is what walks through it.
///
/// The loan rows MOVE: the originator delivers them and receives what the vehicle
/// paid. A loan that is a field rather than a row cannot be transferred at all, which is XI-11's
/// second prerequisite — and it is a row here, so it can.
///
/// A tranche has a stated loss attachment and a price that clears: the attachment is
/// a TERM of the deal and the price is an outcome, never a function of the attachment. The losses
/// that reach it are real losses on named borrowers, which is what makes a senior note
/// safe in a way a rate applied smoothly could never express.
///
/// What frees capital is what was SOLD: a bank that retained everything has moved nothing,
/// and the capital relief is a read of what left rather than a number the deal claims.
pub struct Securitising {
    pub kind: u32,
    /// Where the junior tranche detaches — the share of the pool that stands in front of the
    /// senior note. A TERM of the deal, stated by whoever cuts it, never a price.
    pub junior: &'static str,
    /// How much of a book a bank will pool at once. Its own.
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
            if ctx.processes().running(afoot::SECURITISATION).iter().any(|p| ctx.processes().owner(*p) == who) {
                continue;
            }
            // The loan rows it holds. `saleable` names no kind — a house and a shop are
            // both poolable — so this is every claim on its book that somebody else issued, which is
            // what a loan IS from the lender's side.
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
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            cutting.push((who, ctx.instruments().ccy_of(money), size, size * junior));
        }

        for (who, ccy, size, first_loss) in cutting {
            // The SENIOR note, which the junior stands in front of. It is brought like any
            // other obligation and it trades in a book — what it is worth is what that book crosses
            // at, and never a function of where it attaches.
            ctx.brings(crate::module::Brings {
                issuer: who,
                ccy,
                class: crate::instruments::Class::Claim,
                unit: crate::ids::UnitId::at(0),
                // A note's return is what the pool pays through. It carries no coupon of its own,
                // which is the difference between a note and a bond (5 C4.b).
                coupon: None,
                matures: None,
                units: size - first_loss,
                book: Some(crate::protocols::Venue {
                    rule: crate::clearing::PriceRule::BuyersCompete,
                    protocol: crate::protocols::Protocol::Call,
                    seen_by: 1,
                    stands_for: None,
                }),
                owing: Vec::new(),
            });
            ctx.opens(crate::module::Opens {
                kind: afoot::SECURITISATION,
                owner: who,
                closes: Some(ctx.period() + 1),
                size,
            });
            // What it cut, and what stands in front of it. Both are said, because the capital
            // relief is a read of what LEFT and a deal nobody can see the shape of is one nobody
            // can check that against.
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
        // Tranching a loss RATE yields senior notes that can never be touched. These are
        // realised losses on named loans, and the equity tranche is gone before the mezzanine moves.
        let losses = [Realised { on: instrument(10), lost: 40.0 }];
        let (took, beyond) = allocate(&losses, &deal());
        assert_eq!(took[0], Took { tranche: instrument(1), loss: 40.0 });
        assert_eq!(took[1].loss, 0.0);
        assert_eq!(took[2].loss, 0.0);
        assert_eq!(beyond, 0.0);
    }

    #[test]
    fn a_correlation_worse_than_the_tranching_assumed_hits_the_senior_tranche_too() {
        // This is the event the system exists to produce, and without named tranches and
        // named holders there is nobody to hit. 300 of realised loss wipes equity and mezzanine and
        // reaches 150 into the senior notes.
        let losses = [
            Realised { on: instrument(10), lost: 120.0 },
            Realised { on: instrument(11), lost: 180.0 },
        ];
        let (took, beyond) = allocate(&losses, &deal());
        assert_eq!(took[0].loss, 50.0);
        assert_eq!(took[1].loss, 100.0);
        assert_eq!(took[2].loss, 150.0);
        assert_eq!(beyond, 0.0);
    }

    #[test]
    fn a_loss_past_the_top_of_the_deal_is_carried_out_and_not_absorbed_silently() {
        // No residual with no holder. It is the vehicle's own hole, and it lands on
        // whoever holds its residual.
        let losses = [Realised { on: instrument(10), lost: 1_200.0 }];
        let (took, beyond) = allocate(&losses, &deal());
        assert_eq!(took[2].loss, 850.0);
        assert_eq!(beyond, 200.0);
    }

    #[test]
    fn every_holder_is_hit_at_once_and_each_of_them_is_named() {
        // Named holders of each tranche. Pro rata within a tranche, because two holdings of
        // the same instrument have no reason to be told apart.
        let losses = [Realised { on: instrument(10), lost: 300.0 }];
        let (took, _) = allocate(&losses, &deal());
        let holdings = [
            Holding { holder: party(60), tranche: instrument(1), units: 50.0 },
            Holding { holder: party(61), tranche: instrument(2), units: 60.0 },
            Holding { holder: party(62), tranche: instrument(2), units: 40.0 },
            Holding { holder: party(63), tranche: instrument(3), units: 850.0 },
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
        // The loss does not move to another tranche because this one is unheld. It stands where the
        // waterfall put it, and the absence of a holder is visible.
        let losses = [Realised { on: instrument(10), lost: 40.0 }];
        let (took, _) = allocate(&losses, &deal());
        assert_eq!(took[0].loss, 40.0);
        assert!(onto_holders(&took, &[]).is_empty());
    }

    #[test]
    fn selling_the_risk_expands_what_the_originator_can_originate() {
        // Without a transferee the lending-capacity channel does not exist at all. What it
        // retained does not count, which is why retaining the equity piece buys less capacity.
        assert_eq!(capacity_freed(1_000.0, 50.0), 950.0);
        assert_eq!(capacity_freed(1_000.0, 1_000.0), 0.0);
    }

    #[test]
    #[should_panic(expected = "absorbs nothing")]
    fn a_tranche_with_no_thickness_is_not_a_tranche() {
        Tranche::new(instrument(1), vehicle(), 50.0, 50.0);
    }
}
