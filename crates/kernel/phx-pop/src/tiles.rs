use phx_id::TileId;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::Draws;

use crate::pick::weighted;

/// The tile a unit of a class stands on, drawn when something depends on it — a flood, a fire, a sale — from the
/// zone's stock of that class per tile, each tile in proportion to its units. Nothing records it until then.
#[clause("REP.24")]
pub fn tile_of_unit(d: &mut Draws, stock: &[(TileId, u64)]) -> TileId {
    let units: Vec<u64> = stock.iter().map(|(_, n)| *n).collect();
    let Some((tile, _)) = stock.get(weighted(d, &units)) else {
        violation!(clause = "REP.24", "a unit's tile drawn from outside the zone's stock");
    };
    *tile
}

#[cfg(test)]
mod tests {
    use phx_id::TileId;

    use super::tile_of_unit;
    use crate::fixture::draws;

    #[test]
    fn a_unit_stands_where_the_stock_is() {
        let stock = [(TileId::new(4), 30_u64), (TileId::new(9), 0), (TileId::new(12), 10)];
        let mut on_four = 0_u32;
        for i in 0..8_000 {
            let t = tile_of_unit(&mut draws("GEO.flood", i), &stock);
            assert_ne!(t, TileId::new(9), "no unit stands on a tile with none");
            on_four += u32::from(t == TileId::new(4));
        }
        let share = f64::from(on_four) / 8_000.0;
        assert!((share - 0.75).abs() < 0.02, "{share}");
    }
}
