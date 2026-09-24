use phx_id::PartyId;
use phx_ledger::books::Books;
use phx_macros::clause;
use phx_num::Missing;
use phx_store::Backing;

use crate::accounts::{Accounts, Tally};
use crate::equity::EquityAccount;

/// A party's statement for its period to date, as read: the income and capital its records show, its assets less
/// liabilities at carrying values (absent where unreadable), its equity account, and what it is owed and owes on
/// recognised claims. Nothing in it is stored; each field is read when the statement is.
#[clause("ACC.9", "ACC.16")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Statement {
    pub party: PartyId,
    pub income: Missing<i128>,
    pub capital: Missing<i128>,
    pub net_assets: Missing<i128>,
    pub equity: Missing<i64>,
    pub receivable: i128,
    pub payable: i128,
}

/// The statement over what was read: a party without a tally or an account has none to show.
fn read(
    party: PartyId,
    tally: Missing<Tally>,
    net_assets: Missing<i128>,
    account: Missing<EquityAccount>,
    (receivable, payable): (i128, i128),
) -> Statement {
    let (income, capital) = match tally {
        Missing::Present(t) => (Missing::Present(t.income), Missing::Present(t.capital)),
        Missing::Absent => (Missing::Absent, Missing::Absent),
    };
    let equity = match account {
        Missing::Present(a) => Missing::Present(a.balance()),
        Missing::Absent => Missing::Absent,
    };
    Statement { party, income, capital, net_assets, equity, receivable, payable }
}

/// A party's statement, read from its accounts and the books.
#[clause("ACC.9")]
#[must_use]
pub fn statement<B: Backing>(accounts: &Accounts, books: &Books<B>, party: PartyId) -> Statement {
    let net_assets = match accounts.net_assets(books, party) {
        Ok(v) => Missing::Present(v),
        Err(_) => Missing::Absent,
    };
    read(party, accounts.tally_of(party), net_assets, accounts.equity.of(party), accounts.claims.of(party))
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_num::{Ccy, Missing};

    use super::read;
    use crate::accounts::Tally;
    use crate::equity::EquityAccounts;

    #[test]
    fn statement_is_a_read() {
        let firm = PartyId::new(4);
        let mut accounts = EquityAccounts::default();
        accounts.open(firm, Ccy::new(0), 900);
        let tally = Tally { opened: 900, income: 40, capital: -10 };
        let s = read(firm, Missing::Present(tally), Missing::Present(930), accounts.of(firm), (5, 0));
        assert_eq!((s.income, s.capital), (Missing::Present(40), Missing::Present(-10)));
        assert_eq!((s.net_assets, s.equity, s.receivable), (Missing::Present(930), Missing::Present(900), 5));
        let household = PartyId::new(9);
        let none = read(household, Missing::Absent, Missing::Absent, accounts.of(household), (0, 0));
        assert_eq!((none.income, none.equity), (Missing::Absent, Missing::Absent), "no account, nothing shown as zero");
    }
}
