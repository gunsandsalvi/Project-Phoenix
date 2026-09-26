//! The deposits' rights at the opening: each deposit on a country's land is worked by one of its large firms that make
//! the deposit's product, drawn among them, since a deposit is one thing an agent's twins could not each hold; a
//! deposit no such firm of the country could work is left unworked, its right unissued.

use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::{Contribution, Opening, OpeningCountry, OpeningPhase, PHYSICAL_STOCK, StreamDef, opening_subject};
use phx_geo::GeoState;
use phx_id::PartyId;
use phx_ledger::algebra::Terms;
use phx_ledger::books::Books;
use phx_ledger::instrument::{InstrumentFamily, NewInstrument};
use phx_ledger::opening::{currency, hold, key};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::OpeningStream;

/// Each firm with the product it makes, as the firms' opening draws it.
const PRODUCTS_DRAWN: &str = "FRM.firm_products";
/// The large firms, as the firms' opening draws them.
const LARGE: &str = "FRM.firms";
/// The unit a right is held in: a contract.
const CONTRACT: &str = "contract";

/// The rights to the deposits, issued to the firms that work them.
#[clause("GDS.3", "GDS.4", "GEN.2", "GEN.3")]
#[derive(Debug)]
pub struct Rights;

impl Contribution for Rights {
    fn name(&self) -> &'static str {
        "deposit rights"
    }
    fn phase(&self) -> OpeningPhase {
        PHYSICAL_STOCK
    }
    fn reads(&self) -> &'static [&'static str] {
        &[PRODUCTS_DRAWN, LARGE]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &["GDS.rights"]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (register, countries, date) = (opening.register, opening.countries, opening.date);
        let Ok(products) = register.products("TEC.products") else {
            violation!(clause = "TEC.1", "the products unread")
        };
        let Missing::Present(unit) = register.units().named(CONTRACT) else {
            violation!(clause = "GDS.3", "no contract unit to hold a right in");
        };
        for c in countries {
            let subject = opening_subject(u32::from(c.id.get()), 0);
            let mut lot = opening.ctx.draws(&OpeningStream::DECL, subject);
            let Opening { books, geo, report, .. } = opening;
            let (Some(books), Some(geo)) = (books.downcast_mut::<Books>(), geo.downcast_ref::<GeoState>()) else {
                violation!(clause = "GEN.3", "an opening handed something other than the world's books and map");
            };
            let large: Vec<PartyId> =
                books.drawn.get(&key(LARGE, c.id)).map(|l| l.iter().map(|(p, _)| *p).collect()).unwrap_or_default();
            let firms: Vec<(PartyId, u64)> = books
                .drawn
                .get(&key(PRODUCTS_DRAWN, c.id))
                .map(|l| l.iter().copied().filter(|(p, _)| large.contains(p)).collect())
                .unwrap_or_default();
            let terms = terms(books, c, date);
            let reason = books.ledger.reasons.named(crate::RIGHTS_OPENED.name);
            for (deposit, d) in (0_u32..).zip(&geo.deposits).filter(|(_, d)| c.sites.contains(&d.tile)) {
                let made = products.iter().position(|p| p.extracts == Missing::Present(d.resource));
                let Some(product) = made.and_then(|p| u64::try_from(p).ok()) else { continue };
                let workers: Vec<PartyId> = firms.iter().filter(|(_, p)| *p == product).map(|(f, _)| *f).collect();
                let Ok(n) = u32::try_from(workers.len()) else { continue };
                if n == 0 {
                    continue;
                }
                let Some(holder) = usize::try_from(phx_rand::below_u32(&mut lot, n)).ok().and_then(|i| workers.get(i))
                else {
                    continue;
                };
                let new = NewInstrument {
                    family: InstrumentFamily::RealAsset,
                    issuer: Missing::Absent,
                    unit,
                    ccy: currency(c.id),
                    terms,
                };
                let ledger = &mut books.ledger;
                let right = ledger.goods.issue_right(&mut ledger.instruments, deposit, new);
                books.open(reason, vec![hold(*holder, right, 1, unit, 0, holder.get())], holder.get(), report);
            }
        }
    }
}

/// The terms a country's rights are issued under: an account in its currency, monthly from the opening's day.
fn terms(books: &mut Books, c: &OpeningCountry, date: phx_id::Date) -> phx_ledger::terms::TermsId {
    let Some(months) = Period::months(1) else { violation!(clause = "TIME.4", "a month that is no period") };
    let dates = ScheduleDates {
        anchor: date,
        period: months,
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country: c.id,
    };
    books.ledger.terms.intern(Terms::account(currency(c.id), dates))
}
