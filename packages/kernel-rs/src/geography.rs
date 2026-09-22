//! THE PHYSICAL WORLD: one tiled surface, its jurisdictions, sites, network capital, routes and
//! cargo. Economic mechanisms read this store; they cannot maintain a presentation copy of it.
//!
//! @spec 49 A1 · 49 A2 · 49 A3 · 49 A4 · 49 A5 · 49 B1 · 49 B2 · 49 B3 · 49 B4 · 49 B5 ·
//! @spec 49 C1 · 49 C2 · 49 C3 · 49 C4 · 49 C5 · 49 C6 · 49 D1 · 49 D2 · 49 D3 · 49 D4 ·
//! @spec 49 D5 · 49 E1 · 49 E2 · 49 E3 · 49 E4 · 49 E5 · 49 F1 · 49 F2 · 49 F3 · 49 F4 ·
//! @spec 49 F5 · 49 F6 · 49 G1 · 49 G2 · 49 G3 · 49 G4 · 49 G5 · 49 G6 · 49 H1 · 49 H2 ·
//! @spec 49 H3 · 49 H4 · Law 2, Law 4, Law 5, Law 6, Law 8, Law 19 · Appendix B

use crate::audit::{Contribution, Family, Sources, Violation};
use crate::calendar::Week;
use crate::ids::{CountryId, InstrumentId, PartyId, RegionId};
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
    UnknownRegion(RegionId),
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
    party_sites: BTreeMap<PartyId, SiteId>,
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
            party_sites: BTreeMap::new(),
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

    /// 49 C1: a country exists here as ground it can hold. What its money is belongs to the
    /// registry, which is the one writer of it.
    pub fn declare_country(&mut self, id: CountryId) {
        self.countries.insert(
            id,
            Country {
                id,
                regions: BTreeSet::new(),
            },
        );
    }

    /// 49 C2: a region is a PLACE — ground under one country. It comes into existence with the
    /// first tile assigned to it, because a region with no ground is not a place.
    pub fn region(&mut self, country: CountryId, tile: TileId) -> Result<RegionId, GeographyError> {
        let region = RegionId::at(self.region_tiles.len() as u32);
        self.assign(tile, country, region)?;
        Ok(region)
    }

    /// How many regions have ground, which is how many there are.
    pub fn regions(&self) -> usize {
        self.region_tiles.len()
    }

    /// The first land nobody has claimed. Water is not a place to put a region.
    pub fn unclaimed_land(&self) -> Option<TileId> {
        self.tiles
            .iter()
            .find(|tile| tile.surface == Surface::Land && self.territory[tile.id.row()].is_none())
            .map(|tile| tile.id)
    }

    /// Whose law a region is under, read from the ground it IS.
    pub fn country_of(&self, region: RegionId) -> Option<CountryId> {
        let tile = self.region_tiles.get(&region)?.iter().next()?;
        match self.territory[tile.row()] {
            Some(Territory::Assigned { country, .. }) => Some(country),
            _ => None,
        }
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

    /// 49 C4: a party stands on an exact tile of its region, so its jurisdiction is READ BACK
    /// through that ground rather than kept beside it.
    pub fn stand(
        &mut self,
        who: PartyId,
        region: RegionId,
        kind: SiteKind,
    ) -> Result<SiteId, GeographyError> {
        let tile = self
            .tiles_of(region)
            .and_then(|tiles| {
                tiles
                    .iter()
                    .copied()
                    .find(|tile| self.tiles[tile.row()].surface == Surface::Land)
            })
            .ok_or(GeographyError::UnknownRegion(region))?;
        let site = self.site(tile, kind)?;
        self.party_sites.insert(who, site);
        Ok(site)
    }

    pub fn site_of(&self, who: PartyId) -> Option<SiteId> {
        self.party_sites.get(&who).copied()
    }

    /// Whose law a party is under, and where it is — both read through its one site.
    pub fn where_is(&self, who: PartyId) -> Option<(CountryId, RegionId)> {
        self.jurisdiction_of(self.site_of(who)?).ok()
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

/// 49 H1: nine contributions and not one geography check, because a single finding that covers
/// topology, territory and cargo alike says nothing about which of the nine broke.
#[derive(Default)]
pub struct TopologyIsReciprocal {
    found: Vec<Violation>,
}

impl Contribution for TopologyIsReciprocal {
    fn family(&self) -> Family {
        Family::Names
    }
    fn contributor(&self) -> &'static str {
        "geography.topology"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            topology_is_reciprocal(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct TerritoryIsExclusive {
    found: Vec<Violation>,
}

impl Contribution for TerritoryIsExclusive {
    fn family(&self) -> Family {
        Family::Ownership
    }
    fn contributor(&self) -> &'static str {
        "geography.territory"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            territory_is_exclusive(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct SitesStandWhereTheyAre {
    found: Vec<Violation>,
}

impl Contribution for SitesStandWhereTheyAre {
    fn family(&self) -> Family {
        Family::Ownership
    }
    fn contributor(&self) -> &'static str {
        "geography.sites"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            sites_stand_where_they_are(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct PathLegsAreCompatible {
    found: Vec<Violation>,
}

impl Contribution for PathLegsAreCompatible {
    fn family(&self) -> Family {
        Family::Names
    }
    fn contributor(&self) -> &'static str {
        "geography.path-compatibility"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            path_legs_are_compatible(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct DistanceIsPhysical {
    found: Vec<Violation>,
}

impl Contribution for DistanceIsPhysical {
    fn family(&self) -> Family {
        Family::Units
    }
    fn contributor(&self) -> &'static str {
        "geography.distance"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            distance_is_physical(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct SegmentCapacityIsShared {
    found: Vec<Violation>,
}

impl Contribution for SegmentCapacityIsShared {
    fn family(&self) -> Family {
        Family::Units
    }
    fn contributor(&self) -> &'static str {
        "geography.segment-capacity"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            segment_capacity_is_shared(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct CargoHasAnOwner {
    found: Vec<Violation>,
}

impl Contribution for CargoHasAnOwner {
    fn family(&self) -> Family {
        Family::Ownership
    }
    fn contributor(&self) -> &'static str {
        "geography.cargo-ownership"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            cargo_has_an_owner(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct FreightIsPaidFor {
    found: Vec<Violation>,
}

impl Contribution for FreightIsPaidFor {
    fn family(&self) -> Family {
        Family::Flows
    }
    fn contributor(&self) -> &'static str {
        "geography.freight-payments"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            freight_is_paid_for(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

#[derive(Default)]
pub struct DeliveriesLandOnce {
    found: Vec<Violation>,
}

impl Contribution for DeliveriesLandOnce {
    fn family(&self) -> Family {
        Family::Liveness
    }
    fn contributor(&self) -> &'static str {
        "geography.delivery-flows"
    }
    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        if let Some(map) = from.geography {
            deliveries_land_once(map, from, &mut self.found);
        }
    }
    fn finish(&mut self, _period: u32) -> Vec<Violation> {
        std::mem::take(&mut self.found)
    }
}

/// Every contribution 49 H1 asks for, in the order its clause names them.
pub fn contributions() -> Vec<Box<dyn Contribution>> {
    vec![
        Box::<TopologyIsReciprocal>::default(),
        Box::<TerritoryIsExclusive>::default(),
        Box::<SitesStandWhereTheyAre>::default(),
        Box::<PathLegsAreCompatible>::default(),
        Box::<DistanceIsPhysical>::default(),
        Box::<SegmentCapacityIsShared>::default(),
        Box::<CargoHasAnOwner>::default(),
        Box::<FreightIsPaidFor>::default(),
        Box::<DeliveriesLandOnce>::default(),
    ]
}

fn found(
    found: &mut Vec<Violation>,
    family: Family,
    spec: &'static str,
    owner: String,
    size: f64,
    unit: &'static str,
    week: u32,
    message: &str,
) {
    found.push(Violation {
        family,
        spec,
        owner,
        size,
        unit,
        week,
        message: message.to_string(),
    });
}

fn topology_is_reciprocal(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    for tile in &map.tiles {
        let Ok(near) = map.neighbors(tile.id) else {
            continue;
        };
        for other in near {
            if map
                .neighbors(other)
                .is_ok_and(|back| back.contains(&tile.id))
            {
                continue;
            }
            found(
                out,
                Family::Names,
                "49 A3",
                format!("tile {}", tile.id.0),
                1.0,
                "one-way adjacencies",
                from.week,
                "a tile is its neighbour's neighbour in one direction only",
            );
        }
    }
}

fn territory_is_exclusive(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    // Unclaimed ground is a state of its own, not a defect, so only assigned tiles are measured.
    for tile in &map.tiles {
        if let Some(Territory::Assigned { country, region }) = map.territory_of(tile.id) {
            let under = map
                .countries
                .get(&country)
                .is_some_and(|it| it.regions.contains(&region));
            let inside = map
                .region_tiles
                .get(&region)
                .is_some_and(|it| it.contains(&tile.id));
            if !under || !inside {
                found(
                    out,
                    Family::Ownership,
                    "49 C2",
                    format!("tile {}", tile.id.0),
                    1.0,
                    "misplaced tiles",
                    from.week,
                    "an assigned tile is not in the region or country it says it is under",
                );
            }
        }
    }
    for (region, tiles) in &map.region_tiles {
        if !tiles
            .iter()
            .any(|tile| map.tiles[tile.row()].surface == Surface::Land)
        {
            found(
                out,
                Family::Ownership,
                "49 C6",
                format!("region {}", region.0),
                tiles.len() as f64,
                "tiles of water",
                from.week,
                "a region nobody can stand in",
            );
        }
    }
}

fn sites_stand_where_they_are(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    for site in &map.sites {
        let on_land = map
            .tiles
            .get(site.tile.row())
            .is_some_and(|tile| tile.surface == Surface::Land);
        if on_land && map.jurisdiction_of(site.id).is_ok() {
            continue;
        }
        found(
            out,
            Family::Ownership,
            "49 C6",
            format!("site {}", site.id.0),
            1.0,
            "sites out of jurisdiction",
            from.week,
            "a site does not read through to the ground it stands on",
        );
    }
    // A party's region column and the ground it stands on are written from one argument, and this
    // is what would catch them coming apart.
    for (who, _) in map.party_sites.iter() {
        let Some((_, region)) = map.where_is(*who) else {
            continue;
        };
        if who.row() >= from.parties.len() || from.parties.region_of(*who) == region {
            continue;
        }
        found(
            out,
            Family::Ownership,
            "49 C4",
            format!("party {}", who.0),
            1.0,
            "parties in two places",
            from.week,
            "a party's region and the ground its site stands on disagree",
        );
    }
}

fn path_legs_are_compatible(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    for segment in &map.segments {
        let joins = map
            .neighbors(segment.from)
            .is_ok_and(|near| near.contains(&segment.to));
        let over_water = [segment.from, segment.to].iter().any(|tile| {
            map.tiles
                .get(tile.row())
                .is_none_or(|it| it.surface == Surface::Water)
        });
        let carried =
            matches!(segment.mode, Mode::Maritime | Mode::Transfer) || segment.asset.is_some();
        if joins && (!over_water || carried) {
            continue;
        }
        found(
            out,
            Family::Names,
            "49 B5",
            format!("segment {}", segment.id.0),
            1.0,
            "impassable legs",
            from.week,
            "a leg either does not join neighbours or crosses water nothing was built over",
        );
    }
    for route in &map.routes {
        if map.route_length(route.id).is_ok() && route.legs.len() == route.modes.len() {
            continue;
        }
        found(
            out,
            Family::Names,
            "49 F1",
            format!("route {}", route.id.0),
            route.legs.len() as f64,
            "legs with no stated mode",
            from.week,
            "a route's legs and the modes they are travelled in do not correspond",
        );
    }
}

fn distance_is_physical(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    for tile in &map.tiles {
        if map.distance(tile.id, tile.id) != Ok(Kilometres(0.0)) {
            found(
                out,
                Family::Units,
                "49 B4",
                format!("tile {}", tile.id.0),
                1.0,
                "tiles away from themselves",
                from.week,
                "a place is some distance from itself",
            );
        }
        let Ok(near) = map.neighbors(tile.id) else {
            continue;
        };
        for other in near {
            let (Ok(there), Ok(back)) =
                (map.distance(tile.id, other), map.distance(other, tile.id))
            else {
                continue;
            };
            let gap = there.0 - back.0;
            if gap.abs() <= crate::num::dust(2, &[there.0, back.0]) {
                continue;
            }
            found(
                out,
                Family::Units,
                "49 B4",
                format!("tile {} to tile {}", tile.id.0, other.0),
                gap.abs(),
                "km of asymmetry",
                from.week,
                "the way back is not as long as the way there",
            );
        }
    }
    // A path that goes around an obstacle is longer than the line through it; one that is shorter
    // went through it.
    for route in &map.routes {
        let (Ok(length), Some(origin), Some(destination)) = (
            map.route_length(route.id),
            map.sites.get(route.origin.row()),
            map.sites.get(route.destination.row()),
        ) else {
            continue;
        };
        let Ok(straight) = map.distance(origin.tile, destination.tile) else {
            continue;
        };
        let short = straight.0 - length.0;
        if short <= crate::num::dust(route.legs.len() + 1, &[straight.0, length.0]) {
            continue;
        }
        found(
            out,
            Family::Units,
            "49 B2",
            format!("route {}", route.id.0),
            short,
            "km cut off the path",
            from.week,
            "a route is shorter than the straight line between the sites it joins",
        );
    }
}

fn segment_capacity_is_shared(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    for ((week, asset), used) in &map.reservations {
        let Some(row) = map.assets.get(asset.row()) else {
            found(
                out,
                Family::Units,
                "49 E4",
                format!("asset {}", asset.0),
                *used,
                "units reserved on nothing",
                from.week,
                "a reservation names capital that does not exist",
            );
            continue;
        };
        let over = used - row.capacity_per_week;
        if over > crate::num::dust(2, &[*used, row.capacity_per_week]) {
            found(
                out,
                Family::Units,
                "49 E4",
                format!("asset {} in week {}", asset.0, week.0),
                over,
                "units over capacity",
                from.week,
                "two routes each consumed the whole of one segment",
            );
        }
        if row.state != AssetState::Operating && *used > 0.0 {
            found(
                out,
                Family::Units,
                "49 E1",
                format!("asset {} in week {}", asset.0, week.0),
                *used,
                "units on closed capital",
                from.week,
                "capacity was taken on capital that is not operating",
            );
        }
    }
}

fn cargo_has_an_owner(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    let live =
        |who: PartyId| who.some() && who.row() < from.parties.len() && from.parties.alive(who);
    for shipment in &map.shipments {
        if !matches!(
            shipment.state,
            ShipmentState::Booked | ShipmentState::InTransit
        ) {
            continue;
        }
        if live(shipment.owner) && live(shipment.carrier) {
            continue;
        }
        found(
            out,
            Family::Ownership,
            "49 G2",
            format!("shipment {}", shipment.id.0),
            shipment.units,
            "units in transit",
            from.week,
            "cargo is in the air with no live owner or no live carrier",
        );
    }
}

fn freight_is_paid_for(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    for shipment in &map.shipments {
        if !matches!(shipment.state, ShipmentState::Delivered { .. }) {
            continue;
        }
        let parts = [
            shipment.settled_freight,
            shipment.settled_tolls,
            shipment.settled_handling,
        ];
        if parts.iter().all(|part| part.is_finite() && *part >= 0.0) && shipment.landed_cost() > 0.0
        {
            continue;
        }
        found(
            out,
            Family::Flows,
            "49 G3",
            format!("shipment {}", shipment.id.0),
            shipment.units,
            "units landed on unsettled freight",
            from.week,
            "goods arrived without consideration anybody settled",
        );
    }
}

fn deliveries_land_once(map: &Geography, from: &Sources<'_>, out: &mut Vec<Violation>) {
    let now = Week(i64::from(from.week));
    for shipment in &map.shipments {
        if matches!(
            shipment.state,
            ShipmentState::Booked | ShipmentState::InTransit
        ) && shipment.promised_arrival < now
        {
            found(
                out,
                Family::Liveness,
                "49 G4",
                format!("shipment {}", shipment.id.0),
                (now.0 - shipment.promised_arrival.0) as f64,
                "weeks overdue",
                from.week,
                "a shipment is past its promise and is neither delivered nor failed",
            );
        }
        if let ShipmentState::Delivered { at } = &shipment.state {
            if *at >= shipment.promised_arrival {
                continue;
            }
            found(
                out,
                Family::Liveness,
                "49 F4",
                format!("shipment {}", shipment.id.0),
                (shipment.promised_arrival.0 - at.0) as f64,
                "weeks early",
                from.week,
                "a shipment landed before the physical travel it was promised after",
            );
        }
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
        map.declare_country(CountryId(0));
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
