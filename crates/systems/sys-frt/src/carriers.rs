//! The carriers at the opening: every firm that sells carriage is given the mode its vehicles run on, drawn for each
//! by the modes' shares of the people carriage employs, since no source ties a firm's vehicles to one mode.

use phx_core::register::values::Table1;
use phx_core::{Contribution, FactDef, Opening, OpeningPhase, PRESENT_VALUES, Prim, StreamDef, opening_subject};
use phx_id::PartyId;
use phx_ledger::books::Books;
use phx_ledger::opening::key;
use phx_macros::clause;
use phx_num::violation;
use phx_pop::population::Population;
use phx_store::SystemBacking;

/// Each firm with the product it makes, as the firms' opening draws it.
const PRODUCTS_DRAWN: &str = "FRM.firm_products";

/// The carriers' modes.
#[clause("FRT.1", "FRT.4", "GEN.3")]
#[derive(Debug)]
pub struct Carriers {
    pub share: Prim<Table1>,
    pub product: Prim<phx_num::Count>,
}

impl Contribution for Carriers {
    fn name(&self) -> &'static str {
        "carriers"
    }
    fn phase(&self) -> OpeningPhase {
        PRESENT_VALUES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[PRODUCTS_DRAWN]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &["FRT.modes"]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (register, countries) = (opening.register, opening.countries);
        let shares = self.share.shared(register);
        // Each mode's share as the running total of the shares up to it, so a draw below the total names one mode.
        let mut upto: Vec<u64> = Vec::with_capacity(shares.values().len());
        let mut total = 0_u64;
        for v in shares.values() {
            let Some(next) = u64::try_from(*v).ok().and_then(|w| total.checked_add(w)) else {
                violation!(clause = "FRT.12", "a mode's share that is no weight");
            };
            total = next;
            upto.push(total);
        }
        if total == 0 {
            violation!(clause = "FRT.12", "modes' shares that sum to nothing");
        }
        let carriage = self.product.shared(register).get();
        let mode = <if_firm::freight::Mode as FactDef>::ITEM.name;
        for c in countries {
            let mut lot = opening.ctx.draws(&crate::OpeningStream::DECL, opening_subject(u32::from(c.id.get()), 0));
            let Some(books) = opening.books.downcast_mut::<Books>() else {
                violation!(clause = "GEN.3", "an opening handed something other than the world's books");
            };
            let carriers: Vec<PartyId> = books
                .drawn
                .get(&key(PRODUCTS_DRAWN, c.id))
                .map(|l| l.iter().filter(|(_, p)| *p == carriage).map(|(f, _)| *f).collect())
                .unwrap_or_default();
            let first = books.parties.first_cell_place();
            for party in carriers {
                let at = phx_rand::uniform::below_u64(&mut lot, total);
                let drawn = upto.iter().position(|u| at < *u);
                let Some(drawn) = drawn.and_then(|m| i64::try_from(m).ok()) else {
                    violation!(clause = "FRT.12", "a mode drawn beyond the modes");
                };
                let (place, slot) = books.parties.row(party);
                match place.checked_sub(first) {
                    None => books.parties.open_fact(party, mode, drawn),
                    Some(k) => {
                        let (tables, _, _) = books.parties.cells_mut();
                        Population::table_mut::<SystemBacking>(tables, usize::from(k)).open_fact(slot, mode, drawn);
                    }
                }
            }
        }
    }
}
