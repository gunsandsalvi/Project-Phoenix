//! Tests over a real kind table, whose arenas hold the rows, holdings, lots and named units under test.
#![cfg(test)]

use phx_core::kind_tables::{KindTable, NewIndividual};
use phx_id::{Day, InstrumentId, PartyId, Slot, TableId, TileId};
use phx_num::{Missing, Qty, UnitId};
use phx_store::{AddressSpace, HeapBacking};

use crate::covered::{Covers, Uncovered};
use crate::holding::{Disposal, Lot, LotOrder, acquire, basis, dispose, holding, lots};
use crate::lien::Liens;
use crate::units::{NamedUnit, named, transfer};

pub(crate) type Heap = HeapBacking<4096>;

pub(crate) fn table(space: &mut AddressSpace, kind: &'static str, id: u16) -> KindTable<Heap> {
    KindTable::new(space, kind, TableId::new(id), 64, 16, 0)
}

pub(crate) fn holder(space: &mut AddressSpace, t: &mut KindTable<Heap>, party: u64) -> Slot {
    t.add(
        space,
        NewIndividual { party: PartyId::new(party), site: TileId::new(0), created: Day::new(1), weight: 1, types: &[] },
    )
}

const BOND: InstrumentId = InstrumentId::new(3);

#[test]
fn holdings_keep_their_lots_and_basis() {
    let mut space = AddressSpace::empty();
    let mut firms = table(&mut space, "firm", 0);
    let a = holder(&mut space, &mut firms, 1);
    assert!(acquire(&mut firms, a, BOND, Lot::new(Day::new(1), 100, 1_000)), "the first lot begins the holding");
    assert!(!acquire(&mut firms, a, BOND, Lot::new(Day::new(5), 50, 800)));
    assert!(acquire(&mut firms, a, InstrumentId::new(4), Lot::new(Day::new(6), 7, 70)));
    let Missing::Present(h) = holding(&firms, a, BOND) else { panic!("held") };
    assert_eq!((h.quantity.raw(), h.lots, basis(&firms, a, BOND)), (150, 2, Missing::Present(1_800)));
    let gone = dispose(&mut firms, a, BOND, Disposal { units: 120, bound: 0, order: LotOrder::FirstIn });
    assert_eq!(gone.cost, 1_000 + 320, "the first lot whole, then twenty of the second's fifty");
    assert!(!gone.emptied);
    assert_eq!(lots(&firms, a, BOND), vec![Lot::new(Day::new(5), 30, 480)]);
    assert_eq!(
        lots(&firms, a, InstrumentId::new(4)),
        vec![Lot::new(Day::new(6), 7, 70)],
        "the other holding's lots untouched"
    );
    assert!(dispose(&mut firms, a, BOND, Disposal { units: 30, bound: 0, order: LotOrder::FirstIn }).emptied);
    assert_eq!(holding(&firms, a, BOND), Missing::Absent);
}

#[test]
fn only_free_units_leave() {
    let mut space = AddressSpace::empty();
    let mut firms = table(&mut space, "firm", 0);
    let a = holder(&mut space, &mut firms, 1);
    acquire(&mut firms, a, BOND, Lot::new(Day::new(1), 100, 1_000));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispose(&mut firms, a, BOND, Disposal { units: 80, bound: 30, order: LotOrder::FirstIn });
    }));
    let payload = caught.expect_err("seventy are free, eighty cannot leave");
    assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("REG.2"));
}

#[test]
fn pledge_refuses_more_than_free() {
    let mut liens = Liens::default();
    let (holder, bank, dealer) = (PartyId::new(1), PartyId::new(2), PartyId::new(3));
    let first = liens.pledge(holder, BOND, 60, bank, 100, 0);
    liens.pledge(holder, BOND, 40, bank, 100, 0);
    assert_eq!(liens.pledged(holder, BOND), 100);
    let onward = liens.repledge(first, 50, dealer);
    assert_eq!(liens.pledged(holder, BOND), 100, "a re-pledge pledges the same units again, not more");
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        liens.pledge(holder, BOND, 1, bank, 100, 0);
    }));
    let payload = caught.expect_err("no unit pledged twice");
    assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("REG.16"));
    liens.release(onward);
    liens.release(first);
    assert_eq!(liens.pledged(holder, BOND), 40);
}

#[test]
fn covered_refuses_committed_units() {
    let mut covers = Covers::default();
    let holder = PartyId::new(1);
    let units = |n| Qty::new(n, UnitId::new(0));
    let first = covers.cover(holder, BOND, units(70), 100, 20).expect("seventy of eighty free units");
    assert_eq!(
        covers.cover(holder, BOND, units(20), 100, 20),
        Err(Uncovered { free: 10 }),
        "those seventy cover one offer"
    );
    covers.release(first);
    assert!(covers.cover(holder, BOND, units(80), 100, 20).is_ok(), "released when its order lapses or settles");
}

#[test]
fn named_units_keep_their_site_when_sold() {
    let mut space = AddressSpace::empty();
    let mut firms = table(&mut space, "firm", 0);
    let mut banks = table(&mut space, "bank", 1);
    let seller = holder(&mut space, &mut firms, 1);
    let buyer = holder(&mut space, &mut banks, 2);
    crate::units::add(&mut firms, seller, NamedUnit::new(9, TileId::new(77), Day::new(3), 2, 90));
    let sold = transfer(&mut firms, seller, &mut banks, buyer, 9);
    assert!(named(&firms, seller).is_empty());
    assert_eq!(named(&banks, buyer), vec![sold]);
    assert_eq!(sold.site, TileId::new(77), "a sale changes title, not location");
}

mod lines {
    use phx_id::LineId;
    use phx_num::{Ccy, Missing, Money, UnitId};
    use phx_store::AddressSpace;

    use super::{Heap, holder, table};
    use crate::algebra::Side;
    use crate::holder::HolderKeys;
    use crate::line::{LineKindDecl, Lines, NewRow, SideDecl};
    use crate::rows::{BALANCE, Optional, PENDING, rows};
    use crate::terms::TermsId;

    const LOAN: LineKindDecl = LineKindDecl {
        name: "loan",
        asset: SideDecl {
            holder_kinds: &["bank"],
            words: 0,
            holder_list: true,
            holder_roles: &[],
            exclusive: false,
            many: false,
        },
        liability: SideDecl {
            holder_kinds: &["firm"],
            words: BALANCE,
            holder_list: true,
            holder_roles: &[],
            exclusive: false,
            many: false,
        },
        transfer_requesters: &["BNK"],
        dated: false,
    };

    fn lines(space: &mut AddressSpace) -> Lines<Heap> {
        Lines::new(space, 64, 16, 64, HolderKeys::new(2))
    }

    #[test]
    fn line_side_counts_track_rows() {
        let mut space = AddressSpace::empty();
        let (mut banks, mut firms) = (table(&mut space, "bank", 0), table(&mut space, "firm", 1));
        let mut all = lines(&mut space);
        let loan = all.declare_money(LOAN);
        let line = all.open(loan.index(), TermsId::new(0), Missing::Absent);
        let bank = holder(&mut space, &mut banks, 1);
        let (f1, f2) = (holder(&mut space, &mut firms, 2), holder(&mut space, &mut firms, 3));
        all.add_row(
            &mut banks,
            0,
            bank,
            line,
            NewRow { side: Side::Asset, within: 0, count: 1, point: 0, optional: Optional::NONE },
        );
        let owed = |b| Optional { balance: Missing::Present(b), ..Optional::NONE };
        all.add_row(
            &mut firms,
            1,
            f1,
            line,
            NewRow { side: Side::Liability, within: 0, count: 1, point: 0, optional: owed(-500) },
        );
        all.add_row(
            &mut firms,
            1,
            f2,
            line,
            NewRow { side: Side::Liability, within: 0, count: 3, point: 0, optional: owed(-900) },
        );
        assert_eq!((all.side_count(line, Side::Asset), all.side_count(line, Side::Liability)), (1, 4));
        all.set_count(&mut firms, f2, line, Side::Liability, 2);
        assert_eq!(all.side_count(line, Side::Liability), 3);
        let r = rows(&firms, f2);
        assert_eq!(loan.balance(&r[0], Ccy::new(0)), Missing::Present(Money::new(-900, Ccy::new(0))));
        assert_eq!(all.holders(line).count(), 3);
        all.remove_row(&mut firms, 1, f1, line, Side::Liability);
        assert_eq!((all.side_count(line, Side::Liability), all.holders(line).count()), (2, 2));
        assert!(rows(&firms, f1).is_empty());
    }

    #[test]
    fn holder_kinds_refused() {
        let mut space = AddressSpace::empty();
        let mut firms = table(&mut space, "firm", 1);
        let mut all = lines(&mut space);
        let loan = all.declare_money(LOAN);
        let line = all.open(loan.index(), TermsId::new(0), Missing::Absent);
        let f = holder(&mut space, &mut firms, 2);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            all.add_row(
                &mut firms,
                1,
                f,
                line,
                NewRow { side: Side::Asset, within: 0, count: 1, point: 0, optional: Optional::NONE },
            );
        }));
        let payload = caught.expect_err("a firm may not hold a loan's asset side");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("REG.8"));
    }

    #[test]
    fn holder_list_per_shard_pool_deterministic() {
        // The same inserts made line by line, and made shard by shard as parallel workers would, give the same lists.
        let build = |shard_major: bool| -> Vec<Vec<u32>> {
            let mut space = AddressSpace::empty();
            let mut firms = table(&mut space, "firm", 1);
            let mut all = lines(&mut space);
            let loan = all.declare_money(LOAN);
            let ids: Vec<LineId> = (0..12).map(|_| all.open(loan.index(), TermsId::new(0), Missing::Absent)).collect();
            let holders: Vec<_> = (0..6).map(|p| holder(&mut space, &mut firms, 10 + p)).collect();
            let mut order: Vec<(usize, LineId)> = ids.iter().enumerate().map(|(i, l)| (i, *l)).collect();
            if shard_major {
                order.sort_by_key(|(i, l)| (phx_exec::mix64(u64::from(l.get())) % 8, *i));
            }
            for (i, line) in order {
                for h in holders.iter().skip(i % 3) {
                    let owed = Optional { balance: Missing::Present(-1), ..Optional::NONE };
                    all.add_row(
                        &mut firms,
                        1,
                        *h,
                        line,
                        NewRow { side: Side::Liability, within: 0, count: 1, point: 0, optional: owed },
                    );
                }
            }
            ids.iter().map(|l| all.holders(*l).collect()).collect()
        };
        assert_eq!(build(false), build(true));
    }

    #[test]
    fn rows_keep_their_optional_words() {
        let mut space = AddressSpace::empty();
        let mut firms = table(&mut space, "firm", 1);
        let mut all = lines(&mut space);
        let deposit = all.declare_money(LineKindDecl {
            liability: SideDecl {
                holder_kinds: &["firm"],
                words: BALANCE | PENDING,
                holder_list: true,
                holder_roles: &[],
                exclusive: false,
                many: false,
            },
            ..LOAN
        });
        let plain = all.declare_money(LineKindDecl {
            liability: SideDecl {
                holder_kinds: &["firm"],
                words: 0,
                holder_list: true,
                holder_roles: &[],
                exclusive: false,
                many: false,
            },
            ..LOAN
        });
        let (with_words, bare) = (
            all.open(deposit.index(), TermsId::new(0), Missing::Absent),
            all.open(plain.index(), TermsId::new(0), Missing::Absent),
        );
        let f = holder(&mut space, &mut firms, 2);
        let words = Optional { balance: Missing::Present(-7), pending: Missing::Present(3), amount: Missing::Absent };
        all.add_row(
            &mut firms,
            1,
            f,
            with_words,
            NewRow { side: Side::Liability, within: 0, count: 1, point: 0, optional: words },
        );
        all.add_row(
            &mut firms,
            1,
            f,
            bare,
            NewRow { side: Side::Liability, within: 0, count: 1, point: 0, optional: Optional::NONE },
        );
        let r = rows(&firms, f);
        assert_eq!((r[0].optional, r[1].optional), (words, Optional::NONE));
        assert_eq!(r[1].row.line, bare, "each row read past the words of the one before");
        let pension = Lines::<Heap>::new(&mut space, 4, 4, 4, HolderKeys::new(1)).declare_in_unit(LOAN, UnitId::new(9));
        let Missing::Present(q) = pension.balance(&r[0], Ccy::new(0)) else { panic!("a balance") };
        assert_eq!(q.unit(), UnitId::new(9));
    }

    #[test]
    fn declared_words_only() {
        let mut space = AddressSpace::empty();
        let mut firms = table(&mut space, "firm", 1);
        let mut all = lines(&mut space);
        let loan = all.declare_money(LOAN);
        let line = all.open(loan.index(), TermsId::new(0), Missing::Absent);
        let f = holder(&mut space, &mut firms, 2);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            all.add_row(
                &mut firms,
                1,
                f,
                line,
                NewRow { side: Side::Liability, within: 0, count: 1, point: 0, optional: Optional::NONE },
            );
        }));
        let payload = caught.expect_err("a loan's borrower row carries its balance");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("REP.3"));
    }

    #[test]
    fn listed_on_either_side() {
        // A holder first on a side that keeps no list, then on one that does, is listed once and leaves with its
        // last listed row.
        let mut space = AddressSpace::empty();
        let mut firms = table(&mut space, "firm", 1);
        let mut all = lines(&mut space);
        let swap = all.declare_money(LineKindDecl {
            asset: SideDecl {
                holder_kinds: &["firm"],
                words: 0,
                holder_list: false,
                holder_roles: &[],
                exclusive: false,
                many: false,
            },
            liability: SideDecl {
                holder_kinds: &["firm"],
                words: 0,
                holder_list: true,
                holder_roles: &[],
                exclusive: false,
                many: false,
            },
            ..LOAN
        });
        let line = all.open(swap.index(), TermsId::new(0), Missing::Absent);
        let f = holder(&mut space, &mut firms, 2);
        all.add_row(
            &mut firms,
            1,
            f,
            line,
            NewRow { side: Side::Asset, within: 0, count: 1, point: 0, optional: Optional::NONE },
        );
        assert_eq!(all.holders(line).count(), 0);
        all.add_row(
            &mut firms,
            1,
            f,
            line,
            NewRow { side: Side::Liability, within: 0, count: 1, point: 0, optional: Optional::NONE },
        );
        assert_eq!(all.holders(line).count(), 1);
        all.remove_row(&mut firms, 1, f, line, Side::Liability);
        assert_eq!(all.holders(line).count(), 0);
        all.remove_row(&mut firms, 1, f, line, Side::Asset);
        assert!(rows(&firms, f).is_empty());
    }
}

mod books {
    use phx_id::{Day, InstrumentId, PartyId};
    use phx_num::{Ccy, Missing, Qty, UnitId};
    use phx_store::AddressSpace;

    use super::{Heap, holder, table};
    use crate::algebra::Side;
    use crate::audit::{contracts, ownership};
    use crate::events::{InstrumentEventDecl, InstrumentEvents, refusals};
    use crate::holder::{HolderArenas, HolderKeys};
    use crate::holding::{Disposal, Lot, LotOrder};
    use crate::instrument::{InstrumentFamily, InstrumentState, Instruments, IssueChange, NewInstrument};
    use crate::line::{LineKindDecl, Lines, NewRow, SideDecl};
    use crate::rows::Optional;
    use crate::terms::TermsId;

    fn bond(all: &mut Instruments<Heap>) -> InstrumentId {
        let new = NewInstrument {
            family: InstrumentFamily::Debt,
            issuer: Missing::Present(PartyId::new(1)),
            unit: UnitId::new(0),
            ccy: Ccy::new(0),
            terms: TermsId::new(0),
        };
        all.issue(new)
    }

    #[test]
    fn holdings_sum_to_issued() {
        let mut space = AddressSpace::empty();
        let (mut banks, mut firms) = (table(&mut space, "bank", 0), table(&mut space, "firm", 1));
        let mut all: Instruments<Heap> = Instruments::new(&mut space, 16, 16, 16, HolderKeys::new(2));
        let id = bond(&mut all);
        all.change_issued(id, Qty::new(150, UnitId::new(0)), IssueChange::Issuance);
        let (b, f) = (holder(&mut space, &mut banks, 2), holder(&mut space, &mut firms, 3));
        all.acquire(&mut banks, 0, b, id, Lot::new(Day::new(1), 100, 100));
        all.acquire(&mut firms, 1, f, id, Lot::new(Day::new(1), 50, 50));
        assert_eq!(all.holders(id).count(), 2);
        let tables: [&dyn HolderArenas; 2] = [&banks, &firms];
        assert!(ownership(&all, &tables, id).is_empty());
        all.change_issued(id, Qty::new(10, UnitId::new(0)), IssueChange::Reopening);
        let tables: [&dyn HolderArenas; 2] = [&banks, &firms];
        let gaps = ownership(&all, &tables, id);
        assert_eq!(gaps.iter().map(|g| g.size).collect::<Vec<_>>(), vec![-10], "ten issued that nobody holds");
        let gone = all.dispose(&mut firms, 1, f, id, Disposal { units: 50, bound: 0, order: LotOrder::FirstIn });
        assert!(gone.emptied);
        assert_eq!(all.holders(id).count(), 1, "the firm leaves the list with its last unit");
    }

    #[test]
    fn sides_against_each_other_and_their_rows() {
        let mut space = AddressSpace::empty();
        let (mut banks, mut firms) = (table(&mut space, "bank", 0), table(&mut space, "firm", 1));
        let mut all: Lines<Heap> = Lines::new(&mut space, 16, 16, 16, HolderKeys::new(2));
        let loan = all.declare_money(LineKindDecl {
            name: "loan",
            asset: SideDecl {
                holder_kinds: &["bank"],
                words: 0,
                holder_list: true,
                holder_roles: &[],
                exclusive: false,
                many: false,
            },
            liability: SideDecl {
                holder_kinds: &["firm"],
                words: 0,
                holder_list: true,
                holder_roles: &[],
                exclusive: false,
                many: false,
            },
            transfer_requesters: &["BNK"],
            dated: false,
        });
        let line = all.open(loan.index(), TermsId::new(0), Missing::Absent);
        let b = holder(&mut space, &mut banks, 2);
        let (f1, f2) = (holder(&mut space, &mut firms, 3), holder(&mut space, &mut firms, 4));
        all.add_row(
            &mut banks,
            0,
            b,
            line,
            NewRow { side: Side::Asset, within: 0, count: 4, point: 0, optional: Optional::NONE },
        );
        all.add_row(
            &mut firms,
            1,
            f1,
            line,
            NewRow { side: Side::Liability, within: 0, count: 1, point: 0, optional: Optional::NONE },
        );
        all.add_row(
            &mut firms,
            1,
            f2,
            line,
            NewRow { side: Side::Liability, within: 0, count: 3, point: 0, optional: Optional::NONE },
        );
        let tables: [&dyn HolderArenas; 2] = [&banks, &firms];
        assert!(contracts(&all, &tables, line).is_empty());
        all.set_count(&mut firms, f2, line, Side::Liability, 2);
        let tables: [&dyn HolderArenas; 2] = [&banks, &firms];
        let gaps = contracts(&all, &tables, line);
        assert_eq!(gaps.iter().map(|g| g.size).collect::<Vec<_>>(), vec![1], "four lent against three owing");
    }

    #[test]
    fn state_moves_only_by_declared_events() {
        const DEFAULT: InstrumentEventDecl =
            InstrumentEventDecl { name: "default", from: &[InstrumentState::Live], to: InstrumentState::Defaulted };
        let mut space = AddressSpace::empty();
        let mut all: Instruments<Heap> = Instruments::new(&mut space, 16, 16, 16, HolderKeys::new(1));
        let id = bond(&mut all);
        let mut events = InstrumentEvents::default();
        let default = events.declare(DEFAULT);
        assert_eq!(events.apply(&mut all, id, default), InstrumentState::Defaulted);
        assert_eq!(all.get(id).state, InstrumentState::Defaulted);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = events.apply(&mut all, id, default);
        }));
        let payload = caught.expect_err("a defaulted instrument cannot default again");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("REG.3"));
        assert_eq!(refusals(&[InstrumentEventDecl { from: &[], ..DEFAULT }]).len(), 1);
    }
}
