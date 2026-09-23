use phx_macros::clause;

/// Physical stock per (tile, class) and, per (zone, class), the holders of that class there: the one writer of where
/// units stand. It is created empty and filled as units are placed.
#[clause("GEO.5")]
#[derive(Debug, Default)]
pub struct Stock {
    by_tile_class: Vec<(u32, u16, u64)>,
    holders: Vec<(u32, u16, u64)>,
}

impl Stock {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_tile_class.is_empty() && self.holders.is_empty()
    }
}
