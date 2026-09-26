//! The firm, the vocabulary every system that reads or writes a firm shares: what it makes, the ways it knows, and
//! what it holds and expects.
//! A large firm keeps them as facts of its row; a small firm as its agent's attributes and positions.

pub mod consts;
pub mod facts;
pub mod known;

use phx_core::{FactDef, ItemDecl};

/// Every item the crate exports.
pub const ITEMS: &[ItemDecl] = &[
    <known::Industry as FactDef>::ITEM,
    <known::Known as FactDef>::ITEM,
    <facts::Stock as FactDef>::ITEM,
    <facts::ExpectedSales as FactDef>::ITEM,
    <facts::SalesWidth as FactDef>::ITEM,
    <facts::SalesSince as FactDef>::ITEM,
    <facts::LastReview as FactDef>::ITEM,
    <facts::UnitCost as FactDef>::ITEM,
    <facts::Markup as FactDef>::ITEM,
    <facts::Price as FactDef>::ITEM,
    <facts::OutputRate as FactDef>::ITEM,
    <facts::PriceAttention as FactDef>::ITEM,
    <facts::WagePerHour as FactDef>::ITEM,
];
