use phx_core::{OpeningCountry, Register};
use phx_id::{CountryId, InstrumentId, LineId, PartyId};
use phx_macros::clause;
use phx_num::{Ccy, Missing, UnitId, violation};

use crate::algebra::Side;
use crate::instruction::{AccountRef, Denom, LegKind, LegRec, RowOp};
use crate::line::NewRow;
use crate::rows::{BALANCE, Optional, PENDING};

/// The key a drawn stratum is kept under for the country it belongs to.
#[must_use]
pub fn key(name: &str, country: CountryId) -> String {
    format!("{name}/{}", country.get())
}

/// The currency a country's central bank issues.
pub fn currency(country: CountryId) -> Ccy {
    Ccy::new(country.get())
}

/// The unit rows are counted in: contracts.
pub fn contract_unit(register: &Register) -> UnitId {
    let Missing::Present(u) = register.units().named("contract") else {
        violation!(clause = "NUM.3", "the world declares no contract unit to count rows in");
    };
    u
}

/// A leg opening a party's row on a side of a line, with `count` members and the words its side declares, each at
/// nothing.
#[must_use]
pub fn open_row(register: &Register, party: PartyId, line: LineId, side: Side, count: u32, words: u8) -> LegRec {
    let word = |flag: u8| if words & flag == 0 { Missing::Absent } else { Missing::Present(0) };
    let optional = Optional { balance: word(BALANCE), pending: word(PENDING), amount: Missing::Absent };
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty: i64::from(count),
        denom: Denom::Unit(contract_unit(register)),
        kind: LegKind::Row(RowOp::Open(NewRow { side, within: 0, count, point: 0, optional })),
    }
}

/// A leg writing an opening balance on a party's row, naming the opening identity it served.
#[must_use]
pub fn write(party: PartyId, line: LineId, side: Side, amount: i64, ccy: Ccy, identity: u64) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty: amount,
        denom: Denom::Ccy(ccy),
        kind: LegKind::OpeningWrite { identity, cost: 0 },
    }
}

/// A leg writing an opening holding of an instrument's units, as a lot at its cost.
#[must_use]
pub fn hold(party: PartyId, instrument: InstrumentId, units: i64, unit: UnitId, cost: i64, identity: u64) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Instrument(instrument),
        qty: units,
        denom: Denom::Unit(unit),
        kind: LegKind::OpeningWrite { identity, cost },
    }
}

/// An amount in whole smallest units, rounded to the nearest.
#[must_use]
pub fn whole(x: f64) -> i64 {
    let Some(n) = phx_rand::float::floor_to_i64(x.round()) else {
        violation!(clause = "MON.16", "an opening amount beyond whole smallest units");
    };
    n
}

/// A derived value the opening needs, which the country's group must report.
#[clause("GEN.15")]
#[must_use]
pub fn derived(country: &OpeningCountry, name: &str) -> f64 {
    let Some(v) = country.derived(name) else {
        violation!(
            clause = "GEN.15",
            "an opening reading a derived value its country's group does not report",
            country = country.id.get()
        );
    };
    v
}

/// Monthly dates from an anchor, on the anchor's day of each month, a date on no business day moved to the next.
#[must_use]
pub fn monthly(anchor: phx_id::Date, country: CountryId) -> phx_core::calendar::period::ScheduleDates {
    let Some(months) = phx_core::calendar::period::Period::months(1) else {
        violation!(clause = "TIME.4", "a month that is no period");
    };
    phx_core::calendar::period::ScheduleDates {
        anchor,
        period: months,
        eom: phx_core::calendar::period::EndOfMonth::Plain,
        convention: phx_core::calendar::bizday::BusinessDayConvention::Following,
        country,
    }
}

/// Terms of plain legs on a schedule: senior, unsecured, paid first, never terminated or converted, in default at
/// the first payment missed, with no facility and no stay.
#[must_use]
pub fn plain_terms(
    ccy: Ccy,
    legs: Vec<crate::algebra::Leg>,
    schedule: crate::algebra::Schedule,
) -> crate::algebra::Terms {
    use crate::algebra::{DefaultDefinition, Facility, PaymentOrder, Seniority, Termination, Terms};
    Terms {
        ccy,
        legs,
        schedule,
        seniority: Seniority(0),
        collateral: Missing::Absent,
        payment_order: PaymentOrder(0),
        termination: Termination::None,
        conversion: Missing::Absent,
        default: DefaultDefinition { missed_payments: 1, grace_days: 0 },
        underlying: Missing::Absent,
        facility: Missing::<Facility>::Absent,
        stay: Missing::Absent,
        class: Vec::new(),
    }
}
