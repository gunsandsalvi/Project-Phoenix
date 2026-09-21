//! THE PHYSICAL WORLD: one tiled surface, its jurisdictions, sites, network capital, routes and
//! cargo. Economic mechanisms read this store; they cannot maintain a presentation copy of it.
//!
//! @spec 49 A1 · 49 A2 · 49 A3 · 49 A4 · 49 A5 · 49 B1 · 49 B2 · 49 B3 · 49 B4 · 49 B5 ·
//! @spec 49 C1 · 49 C2 · 49 C3 · 49 C4 · 49 C5 · 49 C6 · 49 D1 · 49 D2 · 49 D3 · 49 D4 ·
//! @spec 49 D5 · 49 E1 · 49 E2 · 49 E3 · 49 E4 · 49 E5 · 49 F1 · 49 F2 · 49 F3 · 49 F4 ·
//! @spec 49 F5 · 49 F6 · 49 G1 · 49 G2 · 49 G3 · 49 G4 · 49 G5 · 49 G6 · 49 H1 · 49 H2 ·
//! @spec 49 H3 · 49 H4 · Law 2, Law 4, Law 5, Law 6, Law 8, Law 19 · Appendix B

use crate::calendar::Week;
use crate::ids::{CurrencyCode, InstrumentId, PartyId, RegionId};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! physical_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u32);
        impl $name {
            pub const fn at(row: u32) -> Self {
                Self(row)
            }
            pub const fn row(self) -> usize {
                self.0 as usize
            }
        }
    };
}

physical_id!(TileId);
physical_id!(CountryId);
physical_id!(SiteId);
physical_id!(AssetId);
physical_id!(SegmentId);
physical_id!(RouteId);
physical_id!(ShipmentId);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kilometres(pub f64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SquareKilometres(pub f64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metres(pub f64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coordinate {
    pub east_km: Kilometres,
    pub north_km: Kilometres,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Land,
    Water,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Road,
    Maritime,
    Transfer,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tile {
    pub id: TileId,
    pub coordinate: Coordinate,
    pub area: SquareKilometres,
    pub elevation: Metres,
    pub surface: Surface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Territory {
    InternationalWater,
    Assigned {
        country: CountryId,
        region: RegionId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Country {
    pub id: CountryId,
    pub currency: CurrencyCode,
    pub regions: BTreeSet<RegionId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteKind {
    Party,
    Establishment,
    Dwelling,
    Plant,
    Warehouse,
    Port,
    Infrastructure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    pub id: SiteId,
    pub tile: TileId,
    pub kind: SiteKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkKind {
    Road,
    Bridge,
    Tunnel,
    Port,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetState {
    Operating,
    Closed,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetworkAsset {
    pub id: AssetId,
    pub owner: PartyId,
    pub instrument: InstrumentId,
    pub site: SiteId,
    pub kind: NetworkKind,
    pub capacity_per_week: f64,
    pub remaining_life_weeks: u32,
    pub maintenance_per_week: f64,
    pub state: AssetState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub id: SegmentId,
    pub from: TileId,
    pub to: TileId,
    pub mode: Mode,
    pub asset: Option<AssetId>,
    pub length: Kilometres,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub id: RouteId,
    pub origin: SiteId,
    pub destination: SiteId,
    pub legs: Vec<SegmentId>,
    pub modes: Vec<Mode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub run_seed: u64,
    pub substream: String,
    pub algorithm_version: u32,
    pub projection: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationShape {
    pub width: u32,
    pub height: u32,
    pub tile_side: Kilometres,
    pub sea_level: Metres,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub attempt: u32,
    pub condition: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarrierOffer {
    pub carrier: PartyId,
    pub route: RouteId,
    pub vehicle_capacity: f64,
    pub vehicle_cost: f64,
    pub labour_cost: f64,
    pub energy_cost: f64,
    pub tolls: f64,
    pub port_charges: f64,
    pub capital_cost: f64,
    pub available_capacity: f64,
    pub price: f64,
}

impl CarrierOffer {
    pub fn derived_price(&self, distance: Kilometres) -> f64 {
        (self.vehicle_cost + self.labour_cost + self.energy_cost) * distance.0
            + self.tolls
            + self.port_charges
            + self.capital_cost
    }
    pub fn is_derived(&self, distance: Kilometres) -> bool {
        self.price == self.derived_price(distance)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShipmentState {
    Booked,
    InTransit,
    Delivered { at: Week },
    Failed { at: Week, claim_on: PartyId },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shipment {
    pub id: ShipmentId,
    pub goods: InstrumentId,
    pub units: f64,
    pub owner: PartyId,
    pub carrier: PartyId,
    pub route: RouteId,
    pub destination: SiteId,
    pub dispatched: Week,
    pub expected_arrival: Week,
    pub promised_arrival: Week,
    pub state: ShipmentState,
    pub settled_freight: f64,
    pub settled_tolls: f64,
    pub settled_handling: f64,
}

impl Shipment {
    pub fn landed_cost(&self) -> f64 {
        self.settled_freight + self.settled_tolls + self.settled_handling
    }
    pub fn destination_inventory(&self) -> bool {
        matches!(self.state, ShipmentState::Delivered { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeographyError {
    InvalidShape,
    UnknownTile(TileId),
    UnknownSite(SiteId),
    UnknownAsset(AssetId),
    UnknownSegment(SegmentId),
    UnknownRoute(RouteId),
    DuplicateAssignment(TileId),
    LandWithoutTerritory(TileId),
    SiteOnWater(TileId),
    NonAdjacentLeg(TileId, TileId),
    ObstructedLeg(TileId, TileId),
    PortNotBesideWater(TileId),
    NoCapacity(SegmentId),
    InvalidRoute,
    DuplicateDelivery(ShipmentId),
}

#[derive(Clone, Debug)]
pub struct Geography {
    shape: GenerationShape,
    provenance: Provenance,
    tiles: Vec<Tile>,
    territory: Vec<Option<Territory>>,
    countries: BTreeMap<CountryId, Country>,
    region_tiles: BTreeMap<RegionId, BTreeSet<TileId>>,
    sites: Vec<Site>,
    assets: Vec<NetworkAsset>,
    segments: Vec<Segment>,
    routes: Vec<Route>,
    shipments: Vec<Shipment>,
    reservations: BTreeMap<(Week, AssetId), f64>,
    rejections: Vec<Rejection>,
}

impl Geography {
    pub fn generate(
        shape: GenerationShape,
        provenance: Provenance,
    ) -> Result<Self, GeographyError> {
        if shape.width == 0
            || shape.height == 0
            || shape.tile_side.0 <= 0.0
            || !shape.tile_side.0.is_finite()
        {
            return Err(GeographyError::InvalidShape);
        }
        let mut state = provenance.run_seed
            ^ stable_name(&provenance.substream)
            ^ u64::from(provenance.algorithm_version);
        let mut tiles = Vec::new();
        for north in 0..shape.height {
            for east in 0..shape.width {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let elevation = Metres(((state >> 32) as i32) as f64 / 2_147_483_648.0);
                let surface = if elevation.0 > shape.sea_level.0 {
                    Surface::Land
                } else {
                    Surface::Water
                };
                let id = TileId::at(tiles.len() as u32);
                tiles.push(Tile {
                    id,
                    coordinate: Coordinate {
                        east_km: Kilometres(f64::from(east) * shape.tile_side.0),
                        north_km: Kilometres(f64::from(north) * shape.tile_side.0),
                    },
                    area: SquareKilometres(shape.tile_side.0 * shape.tile_side.0),
                    elevation,
                    surface,
                });
            }
        }
        let territory = vec![None; tiles.len()];
        Ok(Self {
            shape,
            provenance,
            tiles,
            territory,
            countries: BTreeMap::new(),
            region_tiles: BTreeMap::new(),
            sites: Vec::new(),
            assets: Vec::new(),
            segments: Vec::new(),
            routes: Vec::new(),
            shipments: Vec::new(),
            reservations: BTreeMap::new(),
            rejections: Vec::new(),
        })
    }

    pub fn shape(&self) -> &GenerationShape {
        &self.shape
    }
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    pub fn rejections(&self) -> &[Rejection] {
        &self.rejections
    }
    pub fn reject(&mut self, attempt: u32, condition: String) {
        self.rejections.push(Rejection { attempt, condition });
    }

    pub fn neighbors(&self, tile: TileId) -> Result<Vec<TileId>, GeographyError> {
        self.tile(tile)?;
        let x = tile.0 % self.shape.width;
        let y = tile.0 / self.shape.width;
        let mut result = Vec::new();
        if x > 0 {
            result.push(TileId(tile.0 - 1));
        }
        if x + 1 < self.shape.width {
            result.push(TileId(tile.0 + 1));
        }
        if y > 0 {
            result.push(TileId(tile.0 - self.shape.width));
        }
        if y + 1 < self.shape.height {
            result.push(TileId(tile.0 + self.shape.width));
        }
        Ok(result)
    }

    pub fn distance(&self, from: TileId, to: TileId) -> Result<Kilometres, GeographyError> {
        let a = self.tile(from)?.coordinate;
        let b = self.tile(to)?.coordinate;
        let east = a.east_km.0 - b.east_km.0;
        let north = a.north_km.0 - b.north_km.0;
        Ok(Kilometres((east * east + north * north).sqrt()))
    }

    pub fn declare_country(&mut self, id: CountryId, currency: CurrencyCode) {
        self.countries.insert(
            id,
            Country {
                id,
                currency,
                regions: BTreeSet::new(),
            },
        );
    }

    pub fn assign(
        &mut self,
        tile: TileId,
        country: CountryId,
        region: RegionId,
    ) -> Result<(), GeographyError> {
        let row = tile.row();
        if self.tiles.get(row).is_none() {
            return Err(GeographyError::UnknownTile(tile));
        }
        if self.territory[row].is_some() {
            return Err(GeographyError::DuplicateAssignment(tile));
        }
        self.territory[row] = Some(Territory::Assigned { country, region });
        self.countries
            .get_mut(&country)
            .expect("49 C1: territory needs a declared country")
            .regions
            .insert(region);
        self.region_tiles.entry(region).or_default().insert(tile);
        Ok(())
    }

    pub fn international_water(&mut self, tile: TileId) -> Result<(), GeographyError> {
        let row = tile.row();
        if self.tiles.get(row).is_none() {
            return Err(GeographyError::UnknownTile(tile));
        }
        if self.territory[row].is_some() {
            return Err(GeographyError::DuplicateAssignment(tile));
        }
        self.territory[row] = Some(Territory::InternationalWater);
        Ok(())
    }

    pub fn territory_of(&self, tile: TileId) -> Option<Territory> {
        self.territory.get(tile.row()).copied().flatten()
    }
    pub fn tiles_of(&self, region: RegionId) -> Option<&BTreeSet<TileId>> {
        self.region_tiles.get(&region)
    }

    pub fn site(&mut self, tile: TileId, kind: SiteKind) -> Result<SiteId, GeographyError> {
        if self.tile(tile)?.surface != Surface::Land {
            return Err(GeographyError::SiteOnWater(tile));
        }
        let id = SiteId::at(self.sites.len() as u32);
        self.sites.push(Site { id, tile, kind });
        Ok(id)
    }

    pub fn jurisdiction_of(&self, site: SiteId) -> Result<(CountryId, RegionId), GeographyError> {
        let tile = self.site_row(site)?.tile;
        match self.territory_of(tile) {
            Some(Territory::Assigned { country, region }) => Ok((country, region)),
            _ => Err(GeographyError::LandWithoutTerritory(tile)),
        }
    }

    pub fn add_asset(&mut self, mut asset: NetworkAsset) -> Result<AssetId, GeographyError> {
        self.site_row(asset.site)?;
        asset.id = AssetId::at(self.assets.len() as u32);
        if asset.kind == NetworkKind::Port {
            let tile = self.site_row(asset.site)?.tile;
            let beside_water = self
                .neighbors(tile)?
                .iter()
                .any(|other| self.tiles[other.row()].surface == Surface::Water);
            if !beside_water {
                return Err(GeographyError::PortNotBesideWater(tile));
            }
        }
        let id = asset.id;
        self.assets.push(asset);
        Ok(id)
    }

    pub fn add_segment(
        &mut self,
        from: TileId,
        to: TileId,
        mode: Mode,
        asset: Option<AssetId>,
    ) -> Result<SegmentId, GeographyError> {
        if !self.neighbors(from)?.contains(&to) {
            return Err(GeographyError::NonAdjacentLeg(from, to));
        }
        let a = self.tile(from)?;
        let b = self.tile(to)?;
        let permitted = match mode {
            Mode::Road => {
                a.surface == Surface::Land && b.surface == Surface::Land && asset.is_some()
            }
            Mode::Maritime => a.surface == Surface::Water && b.surface == Surface::Water,
            Mode::Transfer => asset
                .and_then(|id| self.assets.get(id.row()))
                .is_some_and(|row| row.kind == NetworkKind::Port),
        };
        if !permitted {
            return Err(GeographyError::ObstructedLeg(from, to));
        }
        if let Some(id) = asset {
            self.asset(id)?;
        }
        let id = SegmentId::at(self.segments.len() as u32);
        let length = self.distance(from, to)?;
        self.segments.push(Segment {
            id,
            from,
            to,
            mode,
            asset,
            length,
        });
        Ok(id)
    }

    pub fn add_route(
        &mut self,
        origin: SiteId,
        destination: SiteId,
        legs: Vec<SegmentId>,
    ) -> Result<RouteId, GeographyError> {
        let origin_tile = self.site_row(origin)?.tile;
        let destination_tile = self.site_row(destination)?.tile;
        if legs.is_empty() {
            return Err(GeographyError::InvalidRoute);
        }
        let mut at = origin_tile;
        let mut modes = Vec::new();
        for leg in &legs {
            let segment = self.segment(*leg)?;
            if segment.from != at {
                return Err(GeographyError::InvalidRoute);
            }
            at = segment.to;
            modes.push(segment.mode);
        }
        if at != destination_tile {
            return Err(GeographyError::InvalidRoute);
        }
        let id = RouteId::at(self.routes.len() as u32);
        self.routes.push(Route {
            id,
            origin,
            destination,
            legs,
            modes,
        });
        Ok(id)
    }

    pub fn route_length(&self, route: RouteId) -> Result<Kilometres, GeographyError> {
        let mut length = 0.0;
        for leg in &self.route(route)?.legs {
            length += self.segment(*leg)?.length.0;
        }
        Ok(Kilometres(length))
    }

    pub fn reserve(
        &mut self,
        week: Week,
        route: RouteId,
        units: f64,
    ) -> Result<(), GeographyError> {
        if units <= 0.0 || !units.is_finite() {
            return Err(GeographyError::InvalidRoute);
        }
        let assets: Vec<(SegmentId, AssetId)> = self
            .route(route)?
            .legs
            .iter()
            .filter_map(|leg| {
                let segment = &self.segments[leg.row()];
                segment.asset.map(|asset| (*leg, asset))
            })
            .collect();
        for (segment, asset) in &assets {
            let row = self.asset(*asset)?;
            let used = match self.reservations.get(&(week, *asset)) {
                Some(used) => *used,
                None => 0.0,
            };
            if row.state != AssetState::Operating || used + units > row.capacity_per_week {
                return Err(GeographyError::NoCapacity(*segment));
            }
        }
        for (_, asset) in assets {
            *self.reservations.entry((week, asset)).or_default() += units;
        }
        Ok(())
    }

    pub fn dispatch(&mut self, mut shipment: Shipment) -> Result<ShipmentId, GeographyError> {
        self.route(shipment.route)?;
        if shipment.promised_arrival <= shipment.dispatched
            || shipment.expected_arrival <= shipment.dispatched
            || shipment.units <= 0.0
            || !shipment.owner.some()
        {
            return Err(GeographyError::InvalidRoute);
        }
        self.reserve(shipment.dispatched, shipment.route, shipment.units)?;
        shipment.id = ShipmentId::at(self.shipments.len() as u32);
        shipment.state = ShipmentState::InTransit;
        let id = shipment.id;
        self.shipments.push(shipment);
        Ok(id)
    }

    pub fn deliver(&mut self, shipment: ShipmentId, at: Week) -> Result<(), GeographyError> {
        let row = self
            .shipments
            .get_mut(shipment.row())
            .ok_or(GeographyError::DuplicateDelivery(shipment))?;
        if !matches!(row.state, ShipmentState::InTransit) || at < row.promised_arrival {
            return Err(GeographyError::DuplicateDelivery(shipment));
        }
        row.state = ShipmentState::Delivered { at };
        Ok(())
    }

    pub fn fail(
        &mut self,
        shipment: ShipmentId,
        at: Week,
        claim_on: PartyId,
    ) -> Result<(), GeographyError> {
        let row = self
            .shipments
            .get_mut(shipment.row())
            .ok_or(GeographyError::DuplicateDelivery(shipment))?;
        if !matches!(row.state, ShipmentState::InTransit) {
            return Err(GeographyError::DuplicateDelivery(shipment));
        }
        row.state = ShipmentState::Failed { at, claim_on };
        Ok(())
    }

    pub fn audit(&self) -> Vec<String> {
        let mut findings = Vec::new();
        for tile in &self.tiles {
            if tile.surface == Surface::Land
                && !matches!(self.territory_of(tile.id), Some(Territory::Assigned { .. }))
            {
                findings.push(format!("territory: land tile {} is unassigned", tile.id.0));
            }
        }
        for (region, tiles) in &self.region_tiles {
            if !tiles
                .iter()
                .any(|tile| self.tiles[tile.row()].surface == Surface::Land)
            {
                findings.push(format!(
                    "territory: inhabited region {} has no land",
                    region.0
                ));
            }
        }
        for route in &self.routes {
            if self.route_length(route.id).is_err() {
                findings.push(format!("path: route {} has incompatible legs", route.id.0));
            }
        }
        for ((week, asset), used) in &self.reservations {
            if self
                .assets
                .get(asset.row())
                .is_some_and(|row| *used > row.capacity_per_week)
            {
                findings.push(format!(
                    "capacity: asset {} is over-reserved in week {}",
                    asset.0, week.0
                ));
            }
        }
        findings
    }

    pub fn observe(&self) -> GeographySnapshot<'_> {
        GeographySnapshot {
            tiles: &self.tiles,
            territory: &self.territory,
            sites: &self.sites,
            assets: &self.assets,
            shipments: &self.shipments,
        }
    }

    fn tile(&self, id: TileId) -> Result<&Tile, GeographyError> {
        self.tiles
            .get(id.row())
            .ok_or(GeographyError::UnknownTile(id))
    }
    fn site_row(&self, id: SiteId) -> Result<&Site, GeographyError> {
        self.sites
            .get(id.row())
            .ok_or(GeographyError::UnknownSite(id))
    }
    fn asset(&self, id: AssetId) -> Result<&NetworkAsset, GeographyError> {
        self.assets
            .get(id.row())
            .ok_or(GeographyError::UnknownAsset(id))
    }
    fn segment(&self, id: SegmentId) -> Result<&Segment, GeographyError> {
        self.segments
            .get(id.row())
            .ok_or(GeographyError::UnknownSegment(id))
    }
    fn route(&self, id: RouteId) -> Result<&Route, GeographyError> {
        self.routes
            .get(id.row())
            .ok_or(GeographyError::UnknownRoute(id))
    }
}

pub struct GeographySnapshot<'a> {
    pub tiles: &'a [Tile],
    pub territory: &'a [Option<Territory>],
    pub sites: &'a [Site],
    pub assets: &'a [NetworkAsset],
    pub shipments: &'a [Shipment],
}

fn stable_name(name: &str) -> u64 {
    let mut value = 14_695_981_039_346_656_037_u64;
    for byte in name.bytes() {
        value ^= u64::from(byte);
        value = value.wrapping_mul(1_099_511_628_211);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> Geography {
        Geography::generate(
            GenerationShape {
                width: 2,
                height: 2,
                tile_side: Kilometres(3.0),
                sea_level: Metres(-2.0),
            },
            Provenance {
                run_seed: 7,
                substream: "terrain".into(),
                algorithm_version: 1,
                projection: "local equal-distance".into(),
            },
        )
        .unwrap()
    }

    #[test]
    fn generation_is_reproducible_and_distance_has_units() {
        let a = world();
        let b = world();
        assert_eq!(a.tiles(), b.tiles());
        assert_eq!(a.distance(TileId(0), TileId(1)), Ok(Kilometres(3.0)));
        assert_eq!(a.distance(TileId(1), TileId(0)), Ok(Kilometres(3.0)));
        assert_eq!(a.distance(TileId(0), TileId(0)), Ok(Kilometres(0.0)));
    }

    #[test]
    fn topology_is_authoritative() {
        let map = world();
        assert_eq!(
            map.neighbors(TileId(0)).unwrap(),
            vec![TileId(1), TileId(2)]
        );
        assert_eq!(
            map.neighbors(TileId(3)).unwrap(),
            vec![TileId(2), TileId(1)]
        );
    }

    #[test]
    fn territory_has_one_writer() {
        let mut map = world();
        map.declare_country(CountryId(0), CurrencyCode(0));
        assert_eq!(map.assign(TileId(0), CountryId(0), RegionId(0)), Ok(()));
        assert_eq!(
            map.assign(TileId(0), CountryId(0), RegionId(1)),
            Err(GeographyError::DuplicateAssignment(TileId(0)))
        );
    }

    #[test]
    fn an_obstacle_is_not_a_route() {
        let mut map = world();
        map.tiles[1].surface = Surface::Water;
        assert_eq!(
            map.add_segment(TileId(0), TileId(1), Mode::Road, Some(AssetId(0))),
            Err(GeographyError::ObstructedLeg(TileId(0), TileId(1)))
        );
    }

    #[test]
    fn a_carrier_offer_exposes_its_physical_cost() {
        let offer = CarrierOffer {
            carrier: PartyId(1),
            route: RouteId(1),
            vehicle_capacity: 9.0,
            vehicle_cost: 2.0,
            labour_cost: 3.0,
            energy_cost: 1.0,
            tolls: 4.0,
            port_charges: 5.0,
            capital_cost: 6.0,
            available_capacity: 8.0,
            price: 75.0,
        };
        assert!(offer.is_derived(Kilometres(10.0)));
    }
}
