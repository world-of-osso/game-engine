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
            vignettes: &[],
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
        objectives: vec![shared::protocol::QuestObjectiveSnapshot {
            text: "Objective".into(),
            current: 0,
            required: 1,
            completed: false,
            kind: shared::protocol::QuestObjectiveKind::Monster,
            object_id: 49871,
        }],
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
            vignettes: &[],
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
                vignettes: &[],
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

/// Doomwalker (creature_template 167749, Vignette 6520 `ShowOnMap`, elite) at its
/// world.db spawn in Tanaris, map 1.
fn doomwalker(hide_on_continent_maps: bool) -> MapVignette {
    MapVignette {
        name: "Doomwalker".into(),
        elite: true,
        hide_on_continent_maps,
        map_id: 1,
        position: [-8502.32, -4474.17, 11.2417],
    }
}

fn vignette_pins_on(data: &WorldMapData, map_id: u32, vignettes: &[MapVignette]) -> Vec<MapPin> {
    world_map_frame_state(
        data,
        WorldMapRequest {
            visible: true,
            viewport: [1280.0, 720.0],
            map_id,
            hovered: None,
            player: None,
            quests: &[],
            quest_areas: &[],
            vignettes,
        },
    )
    .pins
    .into_iter()
    .filter(|pin| matches!(pin.pin_type, MapPinType::Vignette { .. }))
    .collect()
}

#[test]
fn doomwalkers_vignette_pins_tanaris_and_kalimdor() {
    let data = data();
    let shown = [doomwalker(false)];
    let tanaris = vignette_pins_on(data, 71, &shown);
    assert_eq!(tanaris.len(), 1);
    assert_eq!(tanaris[0].pin_type, MapPinType::Vignette { elite: true });
    assert_eq!(tanaris[0].label, "Doomwalker");
    assert!((0.0..1.0).contains(&tanaris[0].x) && (0.0..1.0).contains(&tanaris[0].y));
    let kalimdor = vignette_pins_on(data, 12, &shown);
    assert_eq!(kalimdor.len(), 1);
    assert_eq!(
        zoom_in(data, 12, [kalimdor[0].x, kalimdor[0].y]),
        Some(71),
        "the Kalimdor pin lies on Tanaris"
    );
    assert!(
        vignette_pins_on(data, 947, &shown).is_empty(),
        "not on Azeroth"
    );

    let zone_only = [doomwalker(true)];
    assert_eq!(vignette_pins_on(data, 71, &zone_only).len(), 1);
    assert!(vignette_pins_on(data, 12, &zone_only).is_empty());
}

#[test]
fn giver_offer_pin_projects_marshal_mcbride_only_when_authoritatively_available() {
    use shared::protocol::{QuestGiverStatus, QuestMarkerClass};
    let catalog = &data().catalog;
    // Actual content_creature197 spawn, world XYZ (not engine X/height/Z).
    let location = (0, [-8913.42, -137.542, 80.8928]);
    let pin = quest_offer_pin(
        catalog,
        37,
        "Marshal McBride",
        QuestGiverStatus::Available(QuestMarkerClass::Normal),
        location,
    )
    .unwrap();
    assert_eq!(pin.pin_type, MapPinType::QuestAvailable);
    assert_eq!(pin.label, "Marshal McBride");
    assert!((pin.x - (1535.420044 + 137.542) / 3470.840088).abs() < 0.00001);
    assert!((pin.y - (-7939.580078 + 8913.42) / 2314.620117).abs() < 0.00001);
    assert!(
        quest_offer_pin(
            catalog,
            37,
            "Marshal McBride",
            QuestGiverStatus::Incomplete(QuestMarkerClass::Normal),
            location
        )
        .is_none()
    );
    assert!(
        quest_offer_pin(
            catalog,
            947,
            "Marshal McBride",
            QuestGiverStatus::Available(QuestMarkerClass::Normal),
            location
        )
        .is_none()
    );
}

#[test]
fn completed_objective_does_not_keep_an_objective_pin() {
    let mut entry = quest("Beating Them Back!", false, vec![poi(0, [-8894.0, -138.0])]);
    entry.quest_id = 28766;
    entry.objectives = vec![shared::protocol::QuestObjectiveSnapshot {
        text: "Blackrock Worg slain".into(),
        current: 6,
        required: 6,
        completed: true,
        kind: shared::protocol::QuestObjectiveKind::Monster,
        object_id: 49871,
    }];
    let view = state_with_quests(data(), 37, &player(0.0), &[entry]);
    assert!(
        !view
            .pins
            .iter()
            .any(|p| p.pin_type == MapPinType::QuestObjective)
    );
}

#[test]
fn real_worg_polygon_projects_into_elwynn_and_completion_replaces_it_with_turnin() {
    let catalog = crate::quest_poi::QuestPoiCatalog::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933"),
    )
    .unwrap();
    let mut entry = quest("Beating Them Back!", false, Vec::new());
    entry.quest_id = 28766;
    let mut runtime = crate::quest_runtime::QuestRuntime::default();
    runtime.log.push(entry);
    runtime.watched.push(28766);
    catalog.apply(&mut runtime.log);
    let view = |runtime: &crate::quest_runtime::QuestRuntime, selected| {
        world_map_frame_state(
            data(),
            WorldMapRequest {
                visible: true,
                viewport: [1280.0, 720.0],
                map_id: 37,
                hovered: None,
                player: None,
                quests: &runtime.map_entries(),
                quest_areas: &runtime.selected_objective_areas(selected),
                vignettes: &[],
            },
        )
    };
    let unselected = view(&runtime, None);
    assert!(unselected.quest_areas.is_empty());
    assert_eq!(
        unselected.pins[0].badge, "1",
        "selection changes blobs, not pins"
    );
    assert!(view(&runtime, Some(7)).quest_areas.is_empty());
    let active = view(&runtime, Some(28766));
    assert_eq!(active.quest_areas.len(), 1);
    assert_eq!(active.quest_areas[0].len(), 7);
    // Independent UiMapAssignment39462 bounds, local DB2 point(-8894,-138).
    let [u, v] = active.quest_areas[0][0];
    assert!((u - 1673.420044 / 3470.840088).abs() < 0.00001);
    assert!((v - 954.419922 / 2314.620117).abs() < 0.00001);
    assert_eq!(active.pins[0].badge, "1");
    let mut finished = runtime.log[0].clone();
    finished.completed = true;
    finished.objectives[0].completed = true;
    runtime.apply_update(shared::protocol::QuestLogUpdate {
        changed: vec![finished],
        removed: Vec::new(),
        watched_quest_ids: vec![28766],
    });
    let complete = view(&runtime, Some(28766));
    assert!(complete.quest_areas.is_empty());
    assert_eq!(complete.pins[0].pin_type, MapPinType::QuestTurnIn);
    assert!(complete.pins[0].badge.is_empty());
}
