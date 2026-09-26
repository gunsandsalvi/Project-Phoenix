use phx_core::{ItemDecl, SystemEntry};

/// Every system of the world, one line each; the order carries no meaning.
pub const SYSTEMS: &[fn() -> SystemEntry] = &[
    SystemEntry::of::<phx_geo::Geo>,
    SystemEntry::of::<sys_cb::Cb>,
    SystemEntry::of::<sys_bnk::Bnk>,
    SystemEntry::of::<sys_frm::Frm>,
    SystemEntry::of::<sys_dem::Dem>,
    SystemEntry::of::<sys_lab::Lab>,
    SystemEntry::of::<sys_hsg::Hsg>,
    SystemEntry::of::<sys_soc::Soc>,
    SystemEntry::of::<sys_tec::Tec>,
    SystemEntry::of::<sys_cap::Cap>,
    SystemEntry::of::<sys_gds::Gds>,
];

/// Every interface crate's items, one line each.
pub const INTERFACES: &[&[ItemDecl]] = &[phx_geo::ITEMS, if_firm::ITEMS];
