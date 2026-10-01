//! The collectors' families: each payer kind owes in its own, found by its kind; a tax arises only where the law
//! withholds one.
#![cfg(test)]

use phx_core::flows::{Denom, Flow};
use phx_id::{CountryId, Date, Day, PartyKey, PartyRef, Slot};
use phx_ledger::levy::{Band, Withholding};
use phx_num::Ccy;
use phx_num::consts::RATE_SCALE;
use phx_store::AddressSpace;

use super::{
    Arising, INCOME, Taxes, collected_family, collector_family, collectors_family, collectors_index, withheld,
};
use crate::consts::AGENT_ROWS_PER_CHUNK;
use crate::consts::reason::{TAXED, WAGE};
use crate::core_day::DatedFamily;

/// The fixtures' kinds, by their places among seven.
const TREASURY: usize = 1;
const FIRM: usize = 3;
const HOUSEHOLD: usize = 5;
const AGENCY: usize = 6;
const KINDS: usize = 7;

fn kind(place: usize) -> u8 {
    u8::try_from(place).unwrap()
}

fn party(place: usize, slot: u32) -> PartyKey {
    PartyKey::new(kind(place), Slot::new(slot))
}

fn family(payer: usize) -> DatedFamily {
    let rows = AGENT_ROWS_PER_CHUNK;
    collectors_family(
        &mut AddressSpace::empty(),
        ("collected", [kind(payer), kind(TREASURY)]),
        ([rows, rows], rows),
        Day::new(0),
    )
}

fn arising(collector: PartyKey, tax: i64) -> Arising {
    let payee = party(HOUSEHOLD, 0);
    Arising { collector, payer: payee, base: INCOME, tax, ccy: 0, on: (collector, payee, 1_000, WAGE, 0) }
}

fn due() -> (phx_core::calendar::period::ScheduleDates, Day) {
    (phx_ledger::opening::monthly(Date::new(2027, 1, 15).unwrap(), CountryId::new(0)), Day::new(30))
}

fn open_edges(f: &DatedFamily) -> Vec<[PartyKey; 2]> {
    f.store.edges.open_slots().filter_map(|e| f.store.edges.row(e)).map(|r| r.ends).collect()
}

/// A wage family per payer kind, then the collectors' families in the same order, among others.
fn families() -> Vec<(u8, u8)> {
    vec![(WAGE, kind(FIRM)), (WAGE, kind(AGENCY)), (TAXED, kind(FIRM)), (TAXED, kind(AGENCY))]
}

#[test]
fn collected_family_for_each_payer_kind() {
    let f = families();
    assert_eq!(collected_family(&f, kind(FIRM)), Some(2));
    assert_eq!(collected_family(&f, kind(AGENCY)), Some(3));
    assert_eq!(collected_family(&f, kind(HOUSEHOLD)), None, "a wage family is no collectors' family");
}

#[test]
fn agency_collector_opens_in_its_family() {
    let mut f = family(AGENCY);
    let (agency, treasury) = (party(AGENCY, 0), party(TREASURY, 0));
    Taxes::default().owe(&mut f, (arising(agency, 70), treasury), due());
    assert_eq!(open_edges(&f), vec![[agency, treasury]]);
    assert_eq!(f.moves.lent, vec![(treasury, 70)]);
}

#[test]
fn collector_kind_without_family_stops() {
    let index = collectors_index(&families(), KINDS);
    assert_eq!(collector_family(&index, party(AGENCY, 3)), 3);
    assert!(std::panic::catch_unwind(|| collector_family(&index, party(HOUSEHOLD, 0))).is_err());
}

#[test]
fn no_withholding_opens_no_debt() {
    let wage = Flow {
        payer: party(AGENCY, 0),
        payee: party(HOUSEHOLD, 0),
        amount: 1_000,
        source: 0,
        denomination: Denom::money(0),
        reason: WAGE,
        order: 0,
    };
    let mut untaxed = wage;
    assert_eq!(withheld(&mut untaxed, None), None);
    assert_eq!(untaxed.amount, 1_000, "nothing is taken where the law withholds nothing");
    let tenth = i64::try_from(RATE_SCALE).unwrap() / 10;
    let law = Withholding {
        kind: 0,
        ccy: Ccy::new(0),
        payee: PartyRef::new(0, 0, Slot::new(1)),
        bands: vec![Band { from: 0, share: tenth }],
        periods: 12,
    };
    let mut taxed = wage;
    let (a, gross) = withheld(&mut taxed, Some(&law)).unwrap();
    assert_eq!((a.collector, a.tax, gross, taxed.amount), (party(AGENCY, 0), 100, 1_000, 900));
    assert_eq!(a.on.2, 900, "the tax arises on the net wage's settlement");
}

#[test]
fn firm_and_agency_debts_in_their_own_families() {
    let index = collectors_index(&families(), KINDS);
    let mut all: Vec<DatedFamily> = vec![family(FIRM), family(AGENCY)];
    let mut taxes = Taxes::default();
    let treasury = party(TREASURY, 0);
    for collector in [party(FIRM, 1), party(AGENCY, 0)] {
        let at = collector_family(&index, collector) - 2;
        taxes.owe(&mut all[at], (arising(collector, 10), treasury), due());
    }
    assert_eq!(open_edges(&all[0]), vec![[party(FIRM, 1), treasury]]);
    assert_eq!(open_edges(&all[1]), vec![[party(AGENCY, 0), treasury]]);
    assert_eq!(taxes.open.len(), 2, "each collector's key is its own across families");
}

#[test]
fn collected_index_rebuilt_equal() {
    let f = families();
    let opened = collectors_index(&f, KINDS);
    let loaded = collectors_index(&f, KINDS);
    assert_eq!(opened, loaded);
    let mut later = f;
    later.push((crate::consts::reason::PENSION, kind(HOUSEHOLD)));
    assert_eq!(collectors_index(&later, KINDS), opened, "families opened later move none");
    assert_eq!(opened.iter().flatten().count(), 2, "one entry for each payer kind");
}

#[test]
fn ended_collector_debts_close_from_its_family() {
    let mut f = family(AGENCY);
    let (agency, treasury) = (party(AGENCY, 0), party(TREASURY, 0));
    let mut taxes = Taxes::default();
    taxes.owe(&mut f, (arising(agency, 40), treasury), due());
    taxes.owe(&mut f, (arising(agency, 2), treasury), due());
    let mine: Vec<Slot> = f.store.of(0, agency.slot()).collect();
    assert_eq!(mine.len(), 1, "one debt a base and a collection day");
    for edge in mine {
        f.close_contract(edge);
    }
    assert!(open_edges(&f).is_empty());
    assert_eq!(f.lost, 42, "what the ended collector owed is lost");
}
