use std::path::PathBuf;
use std::sync::OnceLock;

use shared::protocol::{QuestPoiPoint, QuestPoiSnapshot, QuestRepeatability};

use super::*;

/// Goldshire flight master (TaxiNodes 582), Retail axes.
const GOLDSHIRE: [f32; 3] = [-9433.99, 85.149, 57.0];

fn data() -> &'static WorldMapData {
    static DATA: OnceLock<WorldMapData> = OnceLock::new();
    DATA.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dir = [
            root.join("data/db2/12.1.0.69933"),
            root.join("../../data/db2/12.1.0.69933"),
        ]
        .into_iter()
        .find(|dir| dir.join("UiMapArtTile.csv").exists())
        .expect("UiMapArtTile.csv export (scripts/export_db2_csv.py)");
        WorldMapData::load(&dir).unwrap()
    })
}

fn player(yaw: f32) -> WorldMapPlayer {
    WorldMapPlayer {
        map_id: 0,
        position: GOLDSHIRE,
        yaw,
        faction: Some(Faction::Alliance),
    }
}

fn state(data: &WorldMapData, map_id: u32, player: &WorldMapPlayer) -> WorldMapFrameState {
    state_with_quests(data, map_id, player, &[])
}

fn state_with_quests(
    data: &WorldMapData,
    map_id: u32,
    player: &WorldMapPlayer,
    quests: &[QuestEntrySnapshot],
) -> WorldMapFrameState {
    world_map_frame_state(
        data,
        WorldMapRequest {
            visible: true,
            viewport: [1280.0, 720.0],
            map_id,
            hovered: None,
            player: Some(player),
            quests,
            quest_areas: &[],
        },
    )
}

fn quest(title: &str, completed: bool, pois: Vec<QuestPoiSnapshot>) -> QuestEntrySnapshot {
    QuestEntrySnapshot {
        quest_id: 1,
        title: title.into(),
        zone: String::new(),
        completed,
        repeatability: QuestRepeatability::Normal,
        objectives: Vec::new(),
        level: 1,
        sort_id: 12,
        objectives_text: String::new(),
        completion_text: String::new(),
        watched: false,
        pois,
    }
}

fn poi(objective_index: i32, around: [f32; 2]) -> QuestPoiSnapshot {
    let [x, y] = around.map(|value| value as i32);
    QuestPoiSnapshot {
        objective_index,
        map_id: 0,
        world_map_area_id: 0,
        floor: 0,
        priority: 0,
        flags: 0,
        points: vec![
            QuestPoiPoint {
                x: x - 20,
                y: y - 20,
            },
            QuestPoiPoint {
                x: x + 20,
                y: y - 20,
            },
            QuestPoiPoint {
                x: x + 20,
                y: y + 20,
            },
            QuestPoiPoint {
                x: x - 20,
                y: y + 20,
            },
        ],
    }
}

#[test]
fn opens_on_the_players_zone_with_its_art_and_arrow() {
    let data = data();
    let player = player(0.0);
    let zone = player_map(data, &player).unwrap();
    let view = state(data, zone, &player);
    assert_eq!(view.map_name, "Elwynn Forest");
    let names: Vec<_> = view.breadcrumbs.iter().map(|c| c.name.as_str()).collect();
    assert!(
        names.ends_with(&["Azeroth", "Eastern Kingdoms", "Elwynn Forest"]),
        "{names:?}"
    );
    assert_eq!(view.tiles.len(), 12);
    let arrow = view.player.unwrap();
    assert!((arrow.x - 0.4178).abs() < 0.002 && (arrow.y - 0.6456).abs() < 0.002);
}

#[test]
fn zooming_out_reaches_continent_then_world_and_keeps_the_player() {
    let data = data();
    let player = player(0.0);
    let zone = player_map(data, &player).unwrap();
    let continent = zoom_out(data, zone).unwrap();
    let world = zoom_out(data, continent).unwrap();
    assert_eq!(state(data, continent, &player).map_name, "Eastern Kingdoms");
    let world_view = state(data, world, &player);
    assert_eq!(world_view.map_name, "Azeroth");
    let arrow = world_view.player.unwrap();
    assert!((arrow.x - 0.8555).abs() < 0.002 && (arrow.y - 0.6381).abs() < 0.002);
    // Clicking the player's spot zooms back in, one level at a time.
    assert_eq!(zoom_in(data, world, [arrow.x, arrow.y]), Some(continent));
    let on_continent = state(data, continent, &player).player.unwrap();
    assert_eq!(
        zoom_in(data, continent, [on_continent.x, on_continent.y]),
        Some(zone)
    );
}

#[test]
fn arrow_points_where_forward_movement_goes_on_the_map() {
    let data = data();
    for yaw in [0.0, 0.7, 2.0, std::f32::consts::PI, 4.4] {
        let start = player(yaw);
        // Engine forward is (sin yaw, 0, cos yaw).
        let engine = [GOLDSHIRE[0], GOLDSHIRE[2], -GOLDSHIRE[1]];
        let ahead = [
            engine[0] + 30.0 * yaw.sin(),
            engine[1],
            engine[2] + 30.0 * yaw.cos(),
        ];
        let moved = WorldMapPlayer {
            position: engine_to_world(ahead),
            ..start
        };
        let a = state(data, 37, &start).player.unwrap();
        let b = state(data, 37, &moved).player.unwrap();
        // Canvas pixels: Elwynn is 1002×668 for an equal yards-per-pixel scale.
        let (dx, dy) = ((b.x - a.x) * 1002.0, (b.y - a.y) * 668.0);
        let length = dx.hypot(dy);
        // The north-up arrow turned counter-clockwise by `rotation` points at
        // (-sin r, -cos r) in y-down screen space.
        let (ax, ay) = (-a.rotation.sin(), -a.rotation.cos());
        assert!(
            (dx / length - ax).abs() < 0.01 && (dy / length - ay).abs() < 0.01,
            "yaw {yaw}: moved ({dx}, {dy}), arrow ({ax}, {ay})"
        );
    }
}

#[test]
fn flight_masters_follow_the_players_faction() {
    let data = data();
    let alliance = state(data, 37, &player(0.0));
    let goldshire = alliance
        .pins
        .iter()
        .find(|pin| pin.label == "Goldshire, Elwynn")
        .expect("Goldshire flight master on Elwynn");
    assert_eq!(goldshire.pin_type, MapPinType::FlightAlliance);
    assert!((goldshire.x - 0.4178).abs() < 0.002);
    let horde = state(
        data,
        37,
        &WorldMapPlayer {
            faction: Some(Faction::Horde),
            ..player(0.0)
        },
    );
    assert!(
        !horde
            .pins
            .iter()
            .any(|pin| pin.label == "Goldshire, Elwynn")
    );
    assert_eq!(data.race_faction(1), Some(Faction::Alliance));
    assert_eq!(data.race_faction(2), Some(Faction::Horde));
}

#[test]
fn quest_log_areas_pin_objectives_and_turn_ins() {
    let data = data();
    let quests = [
        quest(
            "Active",
            false,
            vec![poi(-1, [-9000.0, 0.0]), poi(0, [-9433.99, 85.149])],
        ),
        quest(
            "Done",
            true,
            vec![poi(0, [-9000.0, 0.0]), poi(-1, [-9433.99, 85.149])],
        ),
        quest("Elsewhere", false, vec![poi(0, [1000.0, 1000.0])]),
    ];
    let view = state_with_quests(data, 37, &player(0.0), &quests);
    let quest_pins: Vec<_> = view
        .pins
        .iter()
        .filter(|pin| {
            matches!(
                pin.pin_type,
                MapPinType::QuestObjective | MapPinType::QuestTurnIn
            )
        })
        .collect();
    assert_eq!(quest_pins.len(), 2);
    assert_eq!(quest_pins[0].pin_type, MapPinType::QuestObjective);
    assert_eq!(quest_pins[0].badge, "1");
    assert!((quest_pins[0].x - 0.4178).abs() < 0.002);
    assert_eq!(quest_pins[1].pin_type, MapPinType::QuestTurnIn);
    assert!((quest_pins[1].x - 0.4178).abs() < 0.002);
    // The world map shows no pins.
    assert!(
        state_with_quests(data, 947, &player(0.0), &quests)
            .pins
            .is_empty()
    );
}

#[test]
fn hovering_a_zone_on_the_continent_highlights_it() {
    let data = data();
    let player = player(0.0);
    let on_continent = state(data, 13, &player).player.unwrap();
    let view = world_map_frame_state(
        data,
        WorldMapRequest {
            visible: true,
            viewport: [1280.0, 720.0],
            map_id: 13,
            hovered: Some([on_continent.x, on_continent.y]),
            player: Some(&player),
            quests: &[],
            quest_areas: &[],
        },
    );
    let highlight = view.highlight.unwrap();
    assert_eq!(highlight.name, "Elwynn Forest");
    assert_ne!(highlight.fdid, 0);
}

#[test]
fn objective_areas_outline_on_zone_and_continent_maps_only() {
    let data = data();
    let area = poi(0, [-9433.99, 85.149]);
    let elsewhere = poi(0, [1000.0, 1000.0]);
    let areas = [&area, &elsewhere];
    let view = |map_id| {
        world_map_frame_state(
            data,
            WorldMapRequest {
                visible: true,
                viewport: [1280.0, 720.0],
                map_id,
                hovered: None,
                player: None,
                quests: &[],
                quest_areas: &areas,
            },
        )
    };
    // Elwynn Forest (37): the 40-yard square around Goldshire, centred on the
    // Goldshire UV the turn-in pin test resolves; the far-away area is off the map.
    let elwynn = view(37);
    assert_eq!(elwynn.quest_areas.len(), 1);
    let square = &elwynn.quest_areas[0];
    assert_eq!(square.len(), 4);
    let centre_u = square.iter().map(|[u, _]| u).sum::<f32>() / 4.0;
    assert!((centre_u - 0.4178).abs() < 0.002, "centre u {centre_u}");
    let width = square.iter().map(|[u, _]| *u).fold(f32::MIN, f32::max)
        - square.iter().map(|[u, _]| *u).fold(f32::MAX, f32::min);
    assert!(width > 0.0 && width < 0.05, "40 yd span {width} of Elwynn");
    // The world map (947) draws no objective areas.
    assert!(view(947).quest_areas.is_empty());
}
