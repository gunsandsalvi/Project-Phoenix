//! The site a country's central bank and treasury share.

use phx_core::{StreamDef, opening_subject};

use crate::OpeningStream;

/// The site a country's central bank and treasury share, drawn among its land.
pub fn site(ctx: &phx_core::OpeningCtx<'_>, c: &phx_core::OpeningCountry) -> phx_id::TileId {
    let mut draws = ctx.draws(&OpeningStream::DECL, opening_subject(u32::from(c.id.get()), 0));
    c.site(&mut draws)
}
