//! The estate's shares of what it holds: rank by rank, whole where the money covers a rank, else in proportion.
use phx_id::{PartyKey, Slot};

use super::{CREDITORS, Claim, EMPLOYEES, shares};

fn claim(slot: u32, amount: i64, rank: u8) -> Claim {
    Claim { creditor: PartyKey::new(1, Slot::new(slot)), amount, reason: 0, rank }
}

#[test]
fn every_claim_is_paid_where_the_money_covers_them() {
    let paid = shares(&[claim(0, 30, EMPLOYEES), claim(1, 50, CREDITORS)], 100);
    assert_eq!(paid.iter().map(|p| p.1).collect::<Vec<_>>(), vec![30, 50]);
}

#[test]
fn employees_first_then_creditors_in_proportion() {
    let paid = shares(&[claim(0, 40, EMPLOYEES), claim(1, 60, CREDITORS), claim(2, 20, CREDITORS)], 100);
    assert_eq!(paid.iter().map(|p| p.1).collect::<Vec<_>>(), vec![40, 45, 15]);
}

#[test]
fn a_short_rank_shares_what_is_left_and_later_ranks_get_nothing() {
    let paid = shares(&[claim(0, 30, EMPLOYEES), claim(1, 90, EMPLOYEES), claim(2, 10, CREDITORS)], 60);
    assert_eq!(paid.iter().map(|p| p.1).collect::<Vec<_>>(), vec![15, 45]);
}

#[test]
fn nothing_held_pays_nothing() {
    assert!(shares(&[claim(0, 30, EMPLOYEES)], 0).is_empty());
}
