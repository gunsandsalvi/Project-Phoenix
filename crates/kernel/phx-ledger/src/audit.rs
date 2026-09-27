use std::collections::BTreeMap;

use phx_core::{
    AuditFamily, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, InjectTarget, LegDigest, declare_family,
};
use phx_id::{InstrumentId, LineId, PartyId};
use phx_macros::clause;
use phx_num::{Missing, Qty};
use phx_store::Backing;

use crate::algebra::Side;
use crate::books::Books;
use crate::holder::{HolderArenas, HolderKeys, HolderTable};
use crate::holding::holding;
use crate::instruction::{AccountRef, Denom};
use crate::instrument::{Instruments, IssueChange};
use crate::line::Lines;
use crate::rows::rows;

declare_family! { pub OWNERSHIP = "REG.ownership" { mode: Rolling { cycle_days: 30 }, clause: "REG.13" } }
declare_family! { pub CONTRACTS = "REG.contracts" { mode: Rolling { cycle_days: 30 }, clause: "REG.14" } }
declare_family! { pub MONEY = "MON.money" { mode: Rolling { cycle_days: 30 }, clause: "MON.7" } }
declare_family! { pub FLOWS = "SET.flows" { mode: Incremental, clause: "SET.9" } }
declare_family! { pub UNITS = "NUM.units" { mode: Incremental, clause: "NUM.5" } }

/// What a family found wrong with one instrument or line: whose it is, by how much, and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    pub owner: FindingOwner,
    pub size: i128,
    pub detail: String,
}

/// A holder table by its place among the holder tables, which a holder key carries.
fn table<'a>(tables: &[&'a dyn HolderArenas], keys: HolderKeys, key: u32) -> Result<(&'a dyn HolderArenas, u16), u16> {
    let (place, _) = keys.split(key);
    match tables.get(usize::from(place)) {
        Some(t) => Ok((*t, place)),
        None => Err(place),
    }
}

/// Holdings of an instrument, summed over its holder list, against its issued amount; a holder on the list
/// without a holding, or on a table the world does not keep, is a gap of its own.
#[clause("REG.13", "MON.9")]
pub fn ownership<B: Backing>(instruments: &Instruments<B>, tables: &[&dyn HolderArenas], id: InstrumentId) -> Vec<Gap> {
    let owner = FindingOwner::Instrument(id);
    let keys = instruments.keys();
    let mut gaps = Vec::new();
    let mut held = 0_i128;
    for key in instruments.holders(id) {
        let (t, place) = match table(tables, keys, key) {
            Ok(found) => found,
            Err(place) => {
                let detail =
                    format!("instrument {}: a holder on table {place}, which the world does not keep", id.get());
                gaps.push(Gap { owner, size: 1, detail });
                continue;
            }
        };
        let (_, slot) = keys.split(key);
        match holding(t, slot, id) {
            Missing::Present(h) => held += i128::from(h.quantity.raw()),
            Missing::Absent => {
                let detail =
                    format!("instrument {}: holder {} of table {place} listed without a holding", id.get(), slot.get());
                gaps.push(Gap { owner, size: 1, detail });
            }
        }
    }
    let issued = i128::from(instruments.get(id).issued.n());
    if held != issued {
        let detail = format!("instrument {}: holdings of {held} against {issued} issued", id.get());
        gaps.push(Gap { owner, size: held - issued, detail });
    }
    gaps
}

/// A line's two sides against each other, and each side that keeps a holder list against its holders' rows.
#[clause("REG.14", "REP.31")]
pub fn contracts<B: Backing>(lines: &Lines<B>, tables: &[&dyn HolderArenas], line: LineId) -> Vec<Gap> {
    let owner = FindingOwner::Line(line);
    let keys = lines.keys();
    let mut gaps = Vec::new();
    let (asset, liability) = (lines.side_count(line, Side::Asset), lines.side_count(line, Side::Liability));
    if asset != liability {
        let detail =
            format!("line {}: {asset} on its asset side against {liability} on its liability side", line.get());
        gaps.push(Gap { owner, size: i128::from(asset) - i128::from(liability), detail });
    }
    let mut counted = [0_i128; 2];
    for key in lines.holders(line) {
        let Ok((t, _)) = table(tables, keys, key) else {
            let detail = format!("line {}: a holder on a table the world does not keep", line.get());
            gaps.push(Gap { owner, size: 1, detail });
            continue;
        };
        let (_, slot) = keys.split(key);
        for r in crate::rows::iter(t, slot).filter(|r| r.row.line == line) {
            let [a, l] = &mut counted;
            let side = match r.side() {
                Side::Asset => a,
                Side::Liability => l,
            };
            *side += i128::from(r.row.count);
        }
    }
    let [asset_rows, liability_rows] = counted;
    for (side, kept, from_rows) in [(Side::Asset, asset, asset_rows), (Side::Liability, liability, liability_rows)] {
        if lines.listed_side(line, side) && i128::from(kept) != from_rows {
            let detail = format!("line {}: {side:?} side kept at {kept} against {from_rows} in its rows", line.get());
            gaps.push(Gap { owner, size: i128::from(kept) - from_rows, detail });
        }
    }
    gaps
}

/// Money lines' balances: each line's holders' on its asset side and its issuer's on its liability side sum to
/// nothing, so what the issuer records as its money liability is what its holders hold. A listed side is read through
/// the line's holder list; a side of many small holders that keeps none is read in one pass over the tables of the
/// kinds that may hold it, for every such line at once.
#[clause("MON.7", "MON.11")]
pub fn money_lines<B: Backing>(
    lines: &Lines<B>,
    (tables, live): (&[&dyn HolderArenas], &[&dyn HolderTable]),
    ids: &[LineId],
) -> Vec<(LineId, Gap)> {
    let keys = lines.keys();
    let mut sums: BTreeMap<LineId, i128> = ids.iter().map(|l| (*l, 0)).collect();
    let mut gaps = Vec::new();
    let mut unlisted: BTreeMap<LineId, Vec<Side>> = BTreeMap::new();
    for line in ids {
        for side in [Side::Asset, Side::Liability] {
            if !lines.listed_side(*line, side) {
                unlisted.entry(*line).or_default().push(side);
            }
        }
    }
    let mut add = |line: LineId, r: &crate::rows::RowView, gaps: &mut Vec<(LineId, Gap)>| match r.optional.balance {
        Missing::Present(b) => *sums.entry(line).or_insert(0) += i128::from(b),
        Missing::Absent => {
            let detail = format!("line {}: a money row without a balance", line.get());
            gaps.push((line, Gap { owner: FindingOwner::Line(line), size: 1, detail }));
        }
    };
    for line in ids {
        for key in lines.holders(*line) {
            let Ok((t, _)) = table(tables, keys, key) else {
                let detail = format!("line {}: a holder on a table the world does not keep", line.get());
                gaps.push((*line, Gap { owner: FindingOwner::Line(*line), size: 1, detail }));
                continue;
            };
            let (_, slot) = keys.split(key);
            for r in crate::rows::iter(t, slot).filter(|r| r.row.line == *line && lines.listed_side(*line, r.side())) {
                add(*line, &r, &mut gaps);
            }
        }
    }
    if !unlisted.is_empty() {
        let holds = |kind: &str| {
            unlisted.iter().any(|(l, sides)| sides.iter().any(|s| lines.side_decl(*l, *s).holder_kinds.contains(&kind)))
        };
        for t in live.iter().filter(|t| holds(t.kind())) {
            for slot in phx_store::table::live_in(t.live_words()) {
                for r in &rows(*t, slot) {
                    if unlisted.get(&r.row.line).is_some_and(|sides| sides.contains(&r.side())) {
                        add(r.row.line, r, &mut gaps);
                    }
                }
            }
        }
    }
    for (line, sum) in sums {
        if sum != 0 {
            let detail = format!("line {}: its holders' and its issuer's balances differ by {sum}", line.get());
            gaps.push((line, Gap { owner: FindingOwner::Line(line), size: sum, detail }));
        }
    }
    gaps
}

impl<B: Backing> phx_core::BooksAudit for crate::books::Books<B>
where
    crate::books::Books<B>: core::fmt::Debug,
{
    fn instruments(&self) -> usize {
        self.ledger.instruments.len()
    }

    fn lines(&self) -> usize {
        self.ledger.lines.len()
    }

    fn ownership(&self, instrument: usize) -> Vec<phx_core::Gap> {
        let id = InstrumentId::new(narrow(instrument));
        let unit = phx_core::Unit::Qty(self.ledger.instruments.get(id).unit);
        ownership(&self.ledger.instruments, &self.parties.arenas(), id).into_iter().map(|g| g.found(unit)).collect()
    }

    fn contracts(&self, line: usize) -> Vec<phx_core::Gap> {
        let id = LineId::new(narrow(line));
        contracts(&self.ledger.lines, &self.parties.arenas(), id)
            .into_iter()
            .map(|g| g.found(phx_core::Unit::Count))
            .collect()
    }

    fn money(&self, span: core::ops::Range<usize>) -> Vec<phx_core::Gap> {
        let lines = &self.ledger.lines;
        let ids: Vec<LineId> = span.map(|i| LineId::new(narrow(i))).filter(|l| lines.is_money(*l)).collect();
        let tables = self.parties.arenas();
        let live: Vec<&dyn crate::holder::HolderTable> = self.parties.holders().collect();
        money_lines(lines, (&tables, &live), &ids)
            .into_iter()
            .map(|(line, g)| g.found(phx_core::Unit::Money(self.ledger.terms.get(lines.terms(line)).ccy)))
            .collect()
    }

    fn position(&self, party: phx_id::PartyId, account: u64) -> i64 {
        use crate::apply::{Holders, Located};
        match self.parties.locate(party) {
            Located::Live { party: found, table, slot } if found == party => {
                self.ledger.position(self.parties.holder(table), party, slot, account)
            }
            // What an ended party held passed on when it ended, to its successor where it has one, so it holds nothing.
            Located::Live { .. } | Located::Ended => 0,
        }
    }

    fn good(&self, account: u64) -> Missing<phx_core::GoodStock> {
        let AccountRef::Instrument(id) = AccountRef::from_code(account) else { return Missing::Absent };
        match self.ledger.goods.key(id) {
            Missing::Present(k) => Missing::Present(phx_core::GoodStock {
                instrument: id,
                product: k.product,
                grade: k.grade,
                zone: k.zone.get(),
                issued: self.ledger.instruments.get(id).issued.n(),
            }),
            Missing::Absent => Missing::Absent,
        }
    }
}

/// An index the audit counts in as an identity.
fn narrow(index: usize) -> u32 {
    let Ok(n) = u32::try_from(index) else {
        phx_num::capacity_exceeded!("identities of instruments or lines", u32::MAX, index);
    };
    n
}

impl Gap {
    fn found(self, unit: phx_core::Unit) -> phx_core::Gap {
        phx_core::Gap { owner: self.owner, size: self.size, unit, detail: self.detail }
    }
}

fn record(decl: FamilyDecl, ctx: &FamilyCtx<'_>, gaps: Vec<phx_core::Gap>, findings: &mut Findings) {
    for g in gaps {
        findings.record(Finding {
            family: decl.name,
            clause: decl.clause,
            owner: g.owner,
            size: g.size,
            unit: g.unit,
            day: ctx.day(),
            detail: g.detail,
        });
    }
}

/// A rolling family over the books' instruments or lines: today's slice of them, each checked.
fn rolling(
    decl: FamilyDecl,
    ctx: &FamilyCtx<'_>,
    len: usize,
    check: &dyn Fn(usize) -> Vec<phx_core::Gap>,
    findings: &mut Findings,
) -> u64 {
    let span = ctx.rolling(len);
    for i in span.iter() {
        record(decl, ctx, check(i), findings);
    }
    phx_rand::float::len_u64(span.end - span.start)
}

/// The books' instruments, each one's holdings against what it issued.
#[derive(Debug)]
pub struct Ownership;

/// The books' lines, each one's sides against each other and against its holders' rows.
#[derive(Debug)]
pub struct Contracts;

/// Money: each money line's holders against its issuer, and the day's instructions against the money they moved.
#[derive(Debug)]
pub struct Money;

/// The day's instructions: each one's paired legs sum to nothing.
#[derive(Debug)]
pub struct Flows;

/// The day's positions: what each held before plus what its legs moved is what it holds.
#[derive(Debug)]
pub struct Units;

/// The save's books, which an injection breaks.
fn books(target: &mut dyn InjectTarget) -> Result<&mut Books, String> {
    target.books().downcast_mut::<Books>().ok_or_else(|| "the save's books are not the ledger's".to_owned())
}

/// A production leg that moves nothing, on a real money holding, naming `way`, fed to the audit as if it had settled
/// today: no position changes, so only the family that checks production sees it.
///
/// # Errors
/// When the save's books are not the ledger's, or hold no money line with a holder.
pub fn inject_production(target: &mut dyn InjectTarget, way: u32) -> Result<(), String> {
    let (party, account, denom, held) = money_holding(books(target)?)?;
    let digest = LegDigest {
        party,
        account,
        denom,
        qty: 0,
        flow: 0,
        before: held,
        paired: false,
        money: false,
        made: Missing::Present(way),
        worn: Missing::Absent,
        source: Missing::Absent,
        issued: Missing::Absent,
    };
    target.stream().leg(u64::MAX, digest);
    Ok(())
}

/// A real holding to break: the first holder of a money line, the account of its row there, the line's currency and
/// what the account holds.
fn money_holding(books: &Books) -> Result<(PartyId, u64, u32, i64), String> {
    let lines = &books.ledger.lines;
    let tables = books.parties.arenas();
    for i in 0..lines.len() {
        let line = LineId::new(narrow(i));
        if !lines.is_money(line) {
            continue;
        }
        let Some(key) = lines.holders(line).next() else { continue };
        let Ok((t, _)) = table(&tables, lines.keys(), key) else { continue };
        let (_, slot) = lines.keys().split(key);
        let Some(r) = crate::rows::iter(t, slot).find(|r| r.row.line == line) else { continue };
        let party = t.party(slot);
        let account = AccountRef::Line { line, side: r.side() }.code();
        let ccy = Denom::Ccy(books.ledger.terms.get(lines.terms(line)).ccy).code();
        let held = phx_core::BooksAudit::position(books, party, account);
        return Ok((party, account, ccy, held));
    }
    Err("the save's books hold no money line with a holder".to_owned())
}

/// A leg that no instruction settled, on a real holding, fed to the audit as if it had settled today: `before` is
/// what it says the account held, `qty` what it moved.
fn stray_leg(target: &mut dyn InjectTarget, before: i64, paired: bool, money: bool) -> Result<(), String> {
    let (party, account, denom, held) = money_holding(books(target)?)?;
    let digest = LegDigest {
        party,
        account,
        denom,
        qty: 1,
        flow: 1,
        before: held + before,
        paired,
        money,
        made: Missing::Absent,
        worn: Missing::Absent,
        source: Missing::Absent,
        issued: Missing::Absent,
    };
    target.stream().leg(u64::MAX, digest);
    Ok(())
}

/// A wear leg bringing one unit into the second class of `chain` that its first never gave, on a real holding whose
/// before is stated one short so its position still adds up, fed to the audit as if it had settled today: only the
/// family that checks wear sees it.
///
/// # Errors
/// When the save's books are not the ledger's, or hold no money line with a holder.
pub fn inject_wear(target: &mut dyn InjectTarget, chain: u32) -> Result<(), String> {
    let (party, account, denom, held) = money_holding(books(target)?)?;
    let digest = LegDigest {
        party,
        account,
        denom,
        qty: 1,
        flow: 0,
        before: held - 1,
        paired: false,
        money: false,
        made: Missing::Absent,
        worn: Missing::Present((chain, 1)),
        source: Missing::Absent,
        issued: Missing::Absent,
    };
    target.stream().leg(u64::MAX, digest);
    Ok(())
}

/// A good's unit moved by nothing that accounts for it, on a real holder whose before is stated one short so its
/// position still adds up, fed to the audit as if it had settled today: only the family that balances goods sees it.
/// A save that holds no good yet has one issued, alike to its first instrument, with nothing of it in existence.
///
/// # Errors
/// When the save's books are not the ledger's, or hold no instrument or no money line with a holder.
pub fn inject_good(target: &mut dyn InjectTarget) -> Result<(), String> {
    let books = books(target)?;
    let (party, _, _, _) = money_holding(books)?;
    let ledger = &mut books.ledger;
    let first = ledger.goods.iter().next().map(|(_, id)| id);
    let good = if let Some(id) = first {
        id
    } else {
        if ledger.instruments.is_empty() {
            return Err("the save's books hold no instrument to issue a good alike".to_owned());
        }
        let like = ledger.instruments.get(phx_id::InstrumentId::new(0));
        let new = crate::instrument::NewInstrument {
            family: crate::instrument::InstrumentFamily::RealAsset,
            issuer: Missing::Absent,
            unit: like.unit,
            ccy: like.ccy,
            terms: like.terms,
        };
        let key = crate::goods::GoodKey { product: 0, grade: 0, zone: phx_id::ZoneId::new(0) };
        ledger.goods.issue(&mut ledger.instruments, key, new)
    };
    let account = AccountRef::Instrument(good).code();
    let held = phx_core::BooksAudit::position(&*books, party, account);
    let issued = books.ledger.instruments.get(good).issued.n();
    let digest = LegDigest {
        party,
        account,
        denom: Denom::Unit(books.ledger.instruments.get(good).unit).code(),
        qty: 1,
        flow: 0,
        before: held - 1,
        paired: false,
        money: false,
        made: Missing::Absent,
        worn: Missing::Absent,
        source: Missing::Absent,
        issued: Missing::Present(issued),
    };
    target.stream().leg(u64::MAX, digest);
    Ok(())
}

impl AuditFamily for Ownership {
    fn decl(&self) -> FamilyDecl {
        OWNERSHIP
    }
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let books = ctx.books();
        rolling(OWNERSHIP, ctx, books.instruments(), &|i| books.ownership(i), findings)
    }
    /// An instrument's issued amount raised by one with no holder to hold it.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let books = books(target)?;
        if books.ledger.instruments.is_empty() {
            return Err("the save's books hold no instrument".to_owned());
        }
        let id = InstrumentId::new(0);
        let unit = books.ledger.instruments.get(id).unit;
        books.ledger.instruments.change_issued(id, Qty::new(1, unit), IssueChange::Issuance);
        Ok(())
    }
}

impl AuditFamily for Contracts {
    fn decl(&self) -> FamilyDecl {
        CONTRACTS
    }
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let books = ctx.books();
        rolling(CONTRACTS, ctx, books.lines(), &|i| books.contracts(i), findings)
    }
    /// A line's asset side counted one member more than its liability side.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let books = books(target)?;
        if books.ledger.lines.is_empty() {
            return Err("the save's books hold no line".to_owned());
        }
        books.ledger.lines.adjust(LineId::new(0), Side::Asset, 1);
        Ok(())
    }
}

impl AuditFamily for Money {
    fn decl(&self) -> FamilyDecl {
        MONEY
    }
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let books = ctx.books();
        record(MONEY, ctx, ctx.legs().money_gaps(), findings);
        let span = ctx.rolling(books.lines());
        let read = phx_rand::float::len_u64(span.end - span.start);
        record(MONEY, ctx, books.money(span.start..span.end), findings);
        ctx.legs().instructions() + read
    }
    /// A holder's money leg with no issuer's leg to meet it, on a position that holds what it moved.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        stray_leg(target, -1, false, true)
    }
}

impl AuditFamily for Flows {
    fn decl(&self) -> FamilyDecl {
        FLOWS
    }
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        record(FLOWS, ctx, ctx.legs().flow_gaps(), findings);
        ctx.legs().instructions()
    }
    /// One leg of a pair whose other leg never came, on a position that holds what it moved.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        stray_leg(target, -1, true, false)
    }
}

impl AuditFamily for Units {
    fn decl(&self) -> FamilyDecl {
        UNITS
    }
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        record(UNITS, ctx, ctx.legs().unit_gaps(ctx.books()), findings);
        ctx.legs().positions()
    }
    /// A leg the position never received: what it holds is what it held before the leg.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        stray_leg(target, 0, false, false)
    }
}

/// The ledger's families, which the world registers over its books.
#[must_use]
pub fn families() -> Vec<Box<dyn AuditFamily>> {
    vec![Box::new(Ownership), Box::new(Contracts), Box::new(Money), Box::new(Flows), Box::new(Units)]
}
