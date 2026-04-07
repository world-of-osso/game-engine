use bevy::prelude::*;

pub mod textures {
    // --- Frame chrome (world map border pieces) ---
    /// World map top-left border.
    pub const MAP_FRAME_TOP1: u32 = 137185;
    /// World map top-right border.
    pub const MAP_FRAME_TOP2: u32 = 137186;
    /// World map middle-left border.
    pub const MAP_FRAME_MID1: u32 = 137181;
    /// World map bottom-left border.
    pub const MAP_FRAME_BOT1: u32 = 137177;

    // --- Player/party indicators ---
    /// Player icon on world map.
    pub const PLAYER_ICON: u32 = 137198;
    /// Party member icon on world map.
    pub const PARTY_ICON: u32 = 137197;
    /// Minimap player arrow (rotating).
    pub const PLAYER_ARROW: u32 = 136443;
    /// World map globe icon.
    pub const WORLD_ICON: u32 = 137176;

    // --- Pin icons ---
    /// Quest exclamation mark (active quest).
    pub const PIN_QUEST: u32 = 132048;
    /// Taxi/gossip icon (flight master).
    pub const PIN_FLIGHT_MASTER: u32 = 136456;
    /// POI icons atlas (minimap).
    pub const PIN_POI_ATLAS: u32 = 136441;
    /// World map icon (generic).
    pub const MAP_ICON: u32 = 137195;

    // --- Fog of war ---
    /// Fog of war overlay texture.
    pub const FOG_OVERLAY: u32 = 2002950;
    /// Fog hard edge mask.
    pub const FOG_MASK_HARD: u32 = 2015282;
    /// Fog soft edge mask.
    pub const FOG_MASK_SOFT: u32 = 2015284;
}

// --- Pin types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PinType {
    #[default]
    Quest,
    FlightPath,
    PointOfInterest,
    Vendor,
    Innkeeper,
}

impl PinType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Quest => "Quest",
            Self::FlightPath => "Flight Path",
            Self::PointOfInterest => "Point of Interest",
            Self::Vendor => "Vendor",
            Self::Innkeeper => "Innkeeper",
        }
    }
}

// --- Map pin ---

#[derive(Clone, Debug, PartialEq)]
pub struct WorldMapPin {
    pub pin_type: PinType,
    pub label: String,
    /// Position on the zone map as fractions (0.0–1.0).
    pub x: f32,
    pub y: f32,
    pub icon_fdid: u32,
}

// --- Flight path connection ---

#[derive(Clone, Debug, PartialEq)]
pub struct FlightConnection {
    pub from_name: String,
    pub to_name: String,
    pub from_x: f32,
    pub from_y: f32,
    pub to_x: f32,
    pub to_y: f32,
    pub discovered: bool,
}

impl FlightConnection {
    pub fn midpoint(&self) -> (f32, f32) {
        (
            (self.from_x + self.to_x) / 2.0,
            (self.from_y + self.to_y) / 2.0,
        )
    }
}

// --- Zone map ---

#[derive(Clone, Debug, PartialEq)]
pub struct ZoneMapData {
    pub zone_id: u32,
    pub name: String,
    pub texture_fdid: u32,
    pub pins: Vec<WorldMapPin>,
    pub flight_connections: Vec<FlightConnection>,
}

// --- Continent ---

#[derive(Clone, Debug, PartialEq)]
pub struct ContinentData {
    pub name: String,
    pub zones: Vec<ZoneMapEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZoneMapEntry {
    pub zone_id: u32,
    pub name: String,
    /// Bounding box on continent map (fractions 0.0–1.0).
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

// --- Fog of war ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FogOfWar {
    /// Set of explored zone IDs.
    pub explored_zones: Vec<u32>,
}

impl FogOfWar {
    pub fn is_explored(&self, zone_id: u32) -> bool {
        self.explored_zones.contains(&zone_id)
    }

    pub fn explore(&mut self, zone_id: u32) {
        if !self.explored_zones.contains(&zone_id) {
            self.explored_zones.push(zone_id);
        }
    }

    pub fn explored_count(&self) -> usize {
        self.explored_zones.len()
    }
}

// --- Player position ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MapPlayerPosition {
    pub zone_id: u32,
    pub continent_name: String,
    pub zone_name: String,
    /// Position on current zone map (fractions 0.0–1.0).
    pub x: f32,
    pub y: f32,
    /// Facing direction in radians.
    pub facing: f32,
}

impl MapPlayerPosition {
    pub fn coord_text(&self) -> String {
        format!("{:.1}, {:.1}", self.x * 100.0, self.y * 100.0)
    }
}

// --- Runtime resource ---

/// Runtime world map state, held as a Bevy Resource.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct WorldMapState {
    pub player: MapPlayerPosition,
    pub fog: FogOfWar,
    pub continents: Vec<ContinentData>,
    pub current_zone: Option<ZoneMapData>,
    pub selected_continent_idx: usize,
}

impl WorldMapState {
    pub fn current_zone_name(&self) -> &str {
        self.current_zone
            .as_ref()
            .map(|z| z.name.as_str())
            .unwrap_or("")
    }

    pub fn current_zone_pins(&self) -> &[WorldMapPin] {
        self.current_zone
            .as_ref()
            .map(|z| z.pins.as_slice())
            .unwrap_or(&[])
    }

    pub fn quest_pin_count(&self) -> usize {
        self.current_zone_pins()
            .iter()
            .filter(|p| p.pin_type == PinType::Quest)
            .count()
    }

    pub fn continent_names(&self) -> Vec<&str> {
        self.continents.iter().map(|c| c.name.as_str()).collect()
    }

    /// Filter current zone pins by type.
    pub fn pins_by_type(&self, pin_type: PinType) -> Vec<&WorldMapPin> {
        self.current_zone_pins()
            .iter()
            .filter(|p| p.pin_type == pin_type)
            .collect()
    }

    /// Discovered flight connections in the current zone.
    pub fn discovered_flights(&self) -> Vec<&FlightConnection> {
        self.current_zone
            .as_ref()
            .map(|z| {
                z.flight_connections
                    .iter()
                    .filter(|f| f.discovered)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether the current zone is explored (in fog of war).
    pub fn is_current_zone_explored(&self) -> bool {
        self.current_zone
            .as_ref()
            .is_some_and(|z| self.fog.is_explored(z.zone_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(pin_type: PinType, x: f32, y: f32) -> WorldMapPin {
        WorldMapPin {
            pin_type,
            label: "Test".into(),
            x,
            y,
            icon_fdid: 1,
        }
    }

    // --- PinType ---

    #[test]
    fn pin_type_labels() {
        assert_eq!(PinType::Quest.label(), "Quest");
        assert_eq!(PinType::FlightPath.label(), "Flight Path");
        assert_eq!(PinType::Vendor.label(), "Vendor");
    }

    // --- FlightConnection ---

    #[test]
    fn flight_connection_midpoint() {
        let fc = FlightConnection {
            from_name: "A".into(),
            to_name: "B".into(),
            from_x: 0.2,
            from_y: 0.4,
            to_x: 0.8,
            to_y: 0.6,
            discovered: true,
        };
        let (mx, my) = fc.midpoint();
        assert!((mx - 0.5).abs() < 0.01);
        assert!((my - 0.5).abs() < 0.01);
    }

    // --- FogOfWar ---

    #[test]
    fn fog_explore_and_query() {
        let mut fog = FogOfWar::default();
        assert!(!fog.is_explored(42));
        fog.explore(42);
        assert!(fog.is_explored(42));
        assert_eq!(fog.explored_count(), 1);
        // Duplicate explore is idempotent
        fog.explore(42);
        assert_eq!(fog.explored_count(), 1);
    }

    // --- MapPlayerPosition ---

    #[test]
    fn player_coord_text() {
        let pos = MapPlayerPosition {
            x: 0.425,
            y: 0.637,
            ..Default::default()
        };
        assert_eq!(pos.coord_text(), "42.5, 63.7");
    }

    // --- WorldMapState ---

    #[test]
    fn current_zone_name_empty_when_none() {
        let state = WorldMapState::default();
        assert_eq!(state.current_zone_name(), "");
    }

    #[test]
    fn current_zone_name_from_zone() {
        let state = WorldMapState {
            current_zone: Some(ZoneMapData {
                zone_id: 1,
                name: "Elwynn Forest".into(),
                texture_fdid: 100,
                pins: vec![],
                flight_connections: vec![],
            }),
            ..Default::default()
        };
        assert_eq!(state.current_zone_name(), "Elwynn Forest");
    }

    #[test]
    fn quest_pin_count() {
        let state = WorldMapState {
            current_zone: Some(ZoneMapData {
                zone_id: 1,
                name: "Z".into(),
                texture_fdid: 100,
                pins: vec![
                    pin(PinType::Quest, 0.1, 0.2),
                    pin(PinType::FlightPath, 0.3, 0.4),
                    pin(PinType::Quest, 0.5, 0.6),
                ],
                flight_connections: vec![],
            }),
            ..Default::default()
        };
        assert_eq!(state.quest_pin_count(), 2);
    }

    #[test]
    fn continent_names() {
        let state = WorldMapState {
            continents: vec![
                ContinentData {
                    name: "Eastern Kingdoms".into(),
                    zones: vec![],
                },
                ContinentData {
                    name: "Kalimdor".into(),
                    zones: vec![],
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            state.continent_names(),
            vec!["Eastern Kingdoms", "Kalimdor"]
        );
    }

    // --- Pin filtering ---

    fn zone_with_pins() -> ZoneMapData {
        ZoneMapData {
            zone_id: 10,
            name: "Westfall".into(),
            texture_fdid: 100,
            pins: vec![
                pin(PinType::Quest, 0.1, 0.2),
                pin(PinType::FlightPath, 0.5, 0.5),
                pin(PinType::Quest, 0.3, 0.4),
                pin(PinType::Vendor, 0.7, 0.8),
                pin(PinType::Innkeeper, 0.6, 0.3),
            ],
            flight_connections: vec![
                FlightConnection {
                    from_name: "Sentinel Hill".into(),
                    to_name: "Stormwind".into(),
                    from_x: 0.5,
                    from_y: 0.5,
                    to_x: 0.9,
                    to_y: 0.1,
                    discovered: true,
                },
                FlightConnection {
                    from_name: "Sentinel Hill".into(),
                    to_name: "Darkshire".into(),
                    from_x: 0.5,
                    from_y: 0.5,
                    to_x: 0.8,
                    to_y: 0.7,
                    discovered: false,
                },
            ],
        }
    }

    #[test]
    fn pins_by_type_quest() {
        let state = WorldMapState {
            current_zone: Some(zone_with_pins()),
            ..Default::default()
        };
        let quests = state.pins_by_type(PinType::Quest);
        assert_eq!(quests.len(), 2);
    }

    #[test]
    fn pins_by_type_flight_path() {
        let state = WorldMapState {
            current_zone: Some(zone_with_pins()),
            ..Default::default()
        };
        assert_eq!(state.pins_by_type(PinType::FlightPath).len(), 1);
    }

    #[test]
    fn pins_by_type_no_zone() {
        let state = WorldMapState::default();
        assert!(state.pins_by_type(PinType::Quest).is_empty());
    }

    #[test]
    fn pins_by_type_no_match() {
        let state = WorldMapState {
            current_zone: Some(zone_with_pins()),
            ..Default::default()
        };
        assert!(state.pins_by_type(PinType::PointOfInterest).is_empty());
    }

    // --- Flight path connectivity ---

    #[test]
    fn discovered_flights_only() {
        let state = WorldMapState {
            current_zone: Some(zone_with_pins()),
            ..Default::default()
        };
        let flights = state.discovered_flights();
        assert_eq!(flights.len(), 1);
        assert_eq!(flights[0].to_name, "Stormwind");
    }

    #[test]
    fn discovered_flights_no_zone() {
        let state = WorldMapState::default();
        assert!(state.discovered_flights().is_empty());
    }

    // --- Discovery fog ---

    #[test]
    fn is_current_zone_explored_true() {
        let mut state = WorldMapState {
            current_zone: Some(zone_with_pins()),
            ..Default::default()
        };
        state.fog.explore(10); // zone_id matches
        assert!(state.is_current_zone_explored());
    }

    #[test]
    fn is_current_zone_explored_false() {
        let state = WorldMapState {
            current_zone: Some(zone_with_pins()),
            ..Default::default()
        };
        assert!(!state.is_current_zone_explored());
    }

    #[test]
    fn is_current_zone_explored_no_zone() {
        let state = WorldMapState::default();
        assert!(!state.is_current_zone_explored());
    }

    #[test]
    fn fog_multiple_zones() {
        let mut fog = FogOfWar::default();
        fog.explore(1);
        fog.explore(5);
        fog.explore(10);
        assert_eq!(fog.explored_count(), 3);
        assert!(fog.is_explored(5));
        assert!(!fog.is_explored(7));
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        // Frame chrome
        assert_ne!(textures::MAP_FRAME_TOP1, 0);
        assert_ne!(textures::MAP_FRAME_TOP2, 0);
        assert_ne!(textures::MAP_FRAME_MID1, 0);
        assert_ne!(textures::MAP_FRAME_BOT1, 0);
        // Player/party
        assert_ne!(textures::PLAYER_ICON, 0);
        assert_ne!(textures::PARTY_ICON, 0);
        assert_ne!(textures::PLAYER_ARROW, 0);
        assert_ne!(textures::WORLD_ICON, 0);
        // Pin icons
        assert_ne!(textures::PIN_QUEST, 0);
        assert_ne!(textures::PIN_FLIGHT_MASTER, 0);
        assert_ne!(textures::PIN_POI_ATLAS, 0);
        assert_ne!(textures::MAP_ICON, 0);
        // Fog of war
        assert_ne!(textures::FOG_OVERLAY, 0);
        assert_ne!(textures::FOG_MASK_HARD, 0);
        assert_ne!(textures::FOG_MASK_SOFT, 0);
    }
}
