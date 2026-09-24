use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{Ccy, Missing, UnitId};

use crate::instrument::{InstrumentFamily, NewInstrument};
use crate::line::{LineKindDecl, SideDecl};
use crate::rows::{BALANCE, PENDING};
use crate::terms::TermsId;

/// The kinds of party that hold each side of the money lines, and the systems that may move their rows, as the
/// systems that own those parties declare them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoneyHolders {
    pub central_banks: &'static [&'static str],
    pub banks: &'static [&'static str],
    pub treasuries: &'static [&'static str],
    pub depositors: &'static [&'static str],
    pub requesters: &'static [&'static str],
}

/// Money is a liability of the issuer on a line's liability side to the holder on its asset side; both sides carry
/// the balance, the holder's owed to it and the issuer's owed by it, so each line's balances sum to nothing and the
/// issuer's side is its recorded money liability.
#[clause("MON.1", "MON.2", "MON.11")]
impl MoneyHolders {
    fn side(kinds: &'static [&'static str], words: u8) -> SideDecl {
        SideDecl { holder_kinds: kinds, words, holder_list: true }
    }

    /// Reserves: a central bank's liability to a bank.
    #[must_use]
    pub fn reserves(&self) -> LineKindDecl {
        LineKindDecl {
            name: "reserves",
            asset: Self::side(self.banks, BALANCE),
            liability: Self::side(self.central_banks, BALANCE),
            transfer_requesters: self.requesters,
            dated: false,
        }
    }

    /// A deposit kind: a bank's liability to a named depositor, with the amounts awaiting settlement on the
    /// depositor's row.
    #[must_use]
    pub fn deposits(&self, name: &'static str) -> LineKindDecl {
        LineKindDecl {
            name,
            asset: Self::side(self.depositors, BALANCE | PENDING),
            liability: Self::side(self.banks, BALANCE),
            transfer_requesters: self.requesters,
            dated: true,
        }
    }

    /// The treasury's account: a central bank's liability to its own treasury.
    #[must_use]
    pub fn treasury_account(&self) -> LineKindDecl {
        LineKindDecl {
            name: "treasury account",
            asset: Self::side(self.treasuries, BALANCE),
            liability: Self::side(self.central_banks, BALANCE),
            transfer_requesters: self.requesters,
            dated: false,
        }
    }
}

/// A central bank's banknotes in its currency: an instrument it issues, counted in the currency's smallest units, each
/// note held by a named holder.
#[clause("MON.4", "MON.14")]
#[must_use]
pub fn banknotes(central_bank: PartyId, ccy: Ccy, face: UnitId, terms: TermsId) -> NewInstrument {
    NewInstrument { family: InstrumentFamily::Banknote, issuer: Missing::Present(central_bank), unit: face, ccy, terms }
}
