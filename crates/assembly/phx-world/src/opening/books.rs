use core::any::Any;

use phx_core::{
    Contribution, Declarations, GenReport, KindTableRef, Opening, OpeningCountry, OpeningCtx, OpeningPhase,
};
use phx_geo::GeoState;
use phx_id::{CountryId, TileId};
use phx_ledger::books::{Books, BooksSize};

use crate::consts::{
    BOOK_ROWS_PER_CHUNK, CELL_ROWS, CELL_ROWS_PER_CHUNK, HOLDER_BLOCKS, INSTRUMENTS, KIND_ROWS, KIND_ROWS_PER_CHUNK,
    LINES, WHOLE,
};
use crate::opening::newgame::NewGame;

/// Each country as the opening's contributions read it: its people, from the setup's split of the world's population;
/// its derived values; and its land tiles, where its parties are sited.
#[must_use]
pub fn countries(game: &NewGame, geo: &GeoState, population: u64, units: &[u64]) -> Vec<OpeningCountry> {
    let map = &geo.map;
    let region_of = |i: usize| {
        map.tiles
            .get(i)
            .and_then(phx_geo::tile::Tile::zone)
            .and_then(|z| map.zones.get(usize::try_from(z.get()).ok()?))
            .map(|z| z.region.get())
            .and_then(|r| map.regions.get(usize::from(r)).map(|x| (u32::from(r), x.country)))
    };
    let mut sites: Vec<Vec<TileId>> = vec![Vec::new(); game.countries.len()];
    let mut regions: Vec<Vec<(u32, Vec<TileId>)>> = vec![Vec::new(); game.countries.len()];
    for i in 0..map.tiles.len() {
        if let Some((r, c)) = region_of(i)
            && let (Some(s), Some(rs)) = (sites.get_mut(usize::from(c.get())), regions.get_mut(usize::from(c.get())))
        {
            let tile = map.grid.tile(i);
            s.push(tile);
            match rs.iter_mut().find(|(id, _)| *id == r) {
                Some((_, land)) => land.push(tile),
                None => rs.push((r, vec![tile])),
            }
        }
    }
    for rs in &mut regions {
        rs.sort_by_key(|(r, _)| *r);
    }
    game.countries
        .iter()
        .zip(&game.setup.split)
        .zip(sites.into_iter().zip(regions))
        .zip(0_u8..)
        .map(|(((country, share), (sites, regions)), i)| {
            let people = population * share / WHOLE;
            let per_head = country.derived.iter().find(|(n, _)| n == "GEN.gdp_per_head").map(|(_, v)| *v);
            let unit = units.get(usize::from(i)).copied().map(phx_rand::float::from_u64);
            let (Some(per_head), Some(unit)) = (per_head, unit) else {
                phx_num::violation!(clause = "GEN.15", "a country with no GDP per head or no unit", country = i);
            };
            OpeningCountry {
                id: CountryId::new(i),
                people,
                gdp: phx_rand::float::from_u64(people) * per_head * unit,
                derived: country.derived.clone(),
                sites,
                regions,
            }
        })
        .collect()
}

/// The sizes the world's books are made at.
#[must_use]
pub fn size() -> BooksSize {
    BooksSize {
        rows: KIND_ROWS,
        rows_per_chunk: KIND_ROWS_PER_CHUNK,
        instruments: INSTRUMENTS,
        lines: LINES,
        per_chunk: BOOK_ROWS_PER_CHUNK,
        blocks: HOLDER_BLOCKS,
    }
}

/// A maker of the population's empty cell tables in the books' address space, their identities after the kind tables'.
pub fn cell_tables(
    pop: &[phx_pop::kind::PopKindDecl],
) -> impl FnOnce(&mut phx_store::AddressSpace, u16) -> Vec<Box<dyn phx_ledger::holder::CellHolders>> + '_ {
    move |space, first| {
        phx_pop::population::Population::tables::<phx_store::SystemBacking>(
            pop,
            space,
            first,
            CELL_ROWS,
            CELL_ROWS_PER_CHUNK,
        )
    }
}

/// The world's books opened: a kind table for each kind of individual the systems declare and a cell table for each
/// population kind, then each of the given contributing phases in order, its contributions in the order of their
/// systems and names, each handed the books and the population.
pub fn open_books(
    d: &mut Declarations,
    pop: &[phx_pop::population::KindSetup],
    compiled: &crate::compile::Compiled,
    countries: &[OpeningCountry],
    date: phx_id::Date,
    phases: &[OpeningPhase],
) -> (Books, phx_pop::population::Population, GenReport) {
    let kinds: Vec<&'static str> =
        d.kinds.iter().filter(|(_, k)| k.table == KindTableRef::Individuals).map(|(_, k)| k.name).collect();
    let decls: Vec<phx_pop::kind::PopKindDecl> = pop.iter().map(|s| s.decl.clone()).collect();
    let mut books: Books = Books::with_cells(&kinds, pop.len(), cell_tables(&decls), size());
    let first = books.parties.first_cell_place();
    let mut population =
        phx_pop::population::Population::new(pop.to_vec(), first, compiled.day_zero, books.parties.cells_mut().2);
    let mut report = GenReport::default();
    let mut contributions: Vec<(&'static str, Box<dyn Contribution>)> = std::mem::take(&mut d.contributions);
    let mut attachments = std::mem::take(&mut d.attachments);
    attachments.sort_by_key(|(system, _)| *system);
    contributions.sort_by(|(a, x), (b, y)| (x.phase().0, *a, x.name()).cmp(&(y.phase().0, *b, y.name())));
    for phase in phases.iter().copied() {
        for (_, c) in contributions.iter().filter(|(_, c)| c.phase() == phase) {
            let mut opening = Opening {
                ctx: OpeningCtx::new(&compiled.streams, phase),
                day: compiled.day_zero,
                date,
                calendar: &compiled.calendar,
                register: &compiled.register,
                countries,
                report: &mut report,
                books: {
                    let any: &mut dyn Any = &mut books;
                    any
                },
                population: {
                    let any: &mut dyn Any = &mut population;
                    any
                },
                attachments: &attachments,
            };
            c.contribute(&mut opening);
            books.parties.compact_arenas();
        }
    }
    for kind in &kinds {
        report.equity.extend(books.parties.of_kind(kind).map(|p| (p, books.equity(p))));
    }
    let cells = books.parties.cells_mut().0;
    let phx_pop::population::Population { kinds: pop_kinds, agenda, .. } = &mut population;
    for (i, k) in pop_kinds.iter().enumerate() {
        let table = phx_pop::population::Population::table_mut::<phx_store::SystemBacking>(cells, i);
        phx_pop::population::book_changed(k, table, agenda, compiled.day_zero.succ());
    }
    (books, population, report)
}
