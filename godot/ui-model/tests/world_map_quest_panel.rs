//! The world map's "Map & Quest Log" window (Blizzard_WorldMap.lua:95-97,
//! QuestLogOwnerMixin.lua:162-172): Retail sizes, the docked quest list, details with
//! Back, and the maximize toggle, driven through the rendered buttons' actions.

use std::path::PathBuf;

use game_engine_ui_model::quest_log_frame_component::{
    QuestDifficulty, QuestLogDetails, QuestLogFrameState, QuestLogGroup, QuestLogObjectiveLine,
    QuestLogRow,
};
use game_engine_ui_model::world_map_frame_component::{
    QuestMapPanel, WorldMapClick, WorldMapDisplay, WorldMapFrameState, world_map_frame_screen,
};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn world_map_share_reflects_owned_eligibility_in_both_skins() {
    use ui_toolkit::widgets::button::ButtonState;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for can_share in [false, true] {
            let mut state = map_state(WorldMapDisplay::default());
            let panel = state.quest_panel.as_mut().unwrap();
            panel.details = true;
            panel.log.details.as_mut().unwrap().can_share = can_share;
            let registry = render(state);
            let button = frame(&registry, "QuestMapFrameShareButton").unwrap();
            let Some(WidgetData::Button(data)) = &button.widget_data else {
                panic!("Share button");
            };
            let expected = if can_share {
                ButtonState::Normal
            } else {
                ButtonState::Disabled
            };
            assert_eq!(data.state, expected);
        }
    }
}

fn configure_assets() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn row(quest_id: u32, title: &str, level: i32, difficulty: QuestDifficulty) -> QuestLogRow {
    QuestLogRow {
        quest_id,
        title: title.into(),
        level,
        difficulty,
        complete: false,
        watched: quest_id == 28762,
        selected: false,
    }
}

/// A level 3 paladin's log: two Northshire quests and one Elwynn Forest quest.
fn quest_log() -> QuestLogFrameState {
    QuestLogFrameState {
        visible: true,
        quest_count: 3,
        max_quests: 35,
        groups: vec![
            QuestLogGroup {
                sort_id: 9,
                name: "Northshire".into(),
                collapsed: false,
                quests: vec![
                    row(28762, "Beating Them Back!", 2, QuestDifficulty::Difficult),
                    row(28763, "Lions for Lambs", 8, QuestDifficulty::Impossible),
                ],
            },
            QuestLogGroup {
                sort_id: 12,
                name: "Elwynn Forest".into(),
                collapsed: false,
                quests: vec![row(60, "Kobold Candles", 7, QuestDifficulty::VeryDifficult)],
            },
        ],
        details: Some(QuestLogDetails {
            quest_id: 28762,
            title: "Beating Them Back!".into(),
            objectives_text: "Kill 6 Blackrock Worgs.".into(),
            objectives: vec![QuestLogObjectiveLine {
                text: "2/6 Blackrock Worg slain".into(),
                done: false,
            }],
            description: None,
            rewards: None,
            watched: true,
            can_share: false,
        }),
    }
}

fn map_state(display: WorldMapDisplay) -> WorldMapFrameState {
    WorldMapFrameState {
        visible: true,
        viewport: [1920.0, 1080.0],
        maximized: display.maximized,
        quest_panel: Some(QuestMapPanel {
            log: quest_log(),
            details: display.quest_details,
        }),
        map_name: "Northshire".into(),
        ..Default::default()
    }
}

fn render(state: WorldMapFrameState) -> FrameRegistry {
    configure_assets();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    Screen::new(world_map_frame_screen).sync(&shared, &mut registry);
    game_engine_ui_model::world_map_frame_component::apply_world_map_postsetup(
        &state,
        &mut registry,
    );
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> Option<&'a ui_toolkit::frame::Frame> {
    registry.get(registry.get_by_name(name)?)
}

fn shown(registry: &FrameRegistry, name: &str) -> bool {
    frame(registry, name).is_some_and(|frame| !frame.hidden)
}

fn size(registry: &FrameRegistry, name: &str) -> [f32; 2] {
    let frame = frame(registry, name).unwrap_or_else(|| panic!("{name} missing"));
    let fixed = |dimension: Dimension| match dimension {
        Dimension::Fixed(px) => px,
        other => panic!("{name}: {other:?} is not fixed"),
    };
    [fixed(frame.width), fixed(frame.height)]
}

fn text(registry: &FrameRegistry, name: &str) -> (String, [f32; 4]) {
    match frame(registry, name)
        .unwrap_or_else(|| panic!("{name} missing"))
        .widget_data
        .as_ref()
    {
        Some(WidgetData::FontString(data)) => (data.text.clone(), data.color),
        Some(WidgetData::Button(button)) => (button.text.clone(), [1.0; 4]),
        other => panic!("{name} is not text: {other:?}"),
    }
}

fn top(registry: &FrameRegistry, name: &str) -> f32 {
    let mut frame = frame(registry, name).unwrap_or_else(|| panic!("{name} missing"));
    let mut top = 0.0;
    while frame.name.as_deref() != Some("WorldMapBorderFrame") {
        top += match frame.position.top {
            Val::Px(px) => px,
            Val::Auto => 0.0,
            other => panic!("{name}: top {other:?}"),
        };
        let Some(parent) = frame.parent_id else {
            break;
        };
        frame = registry.get(parent).unwrap();
    }
    top
}

fn onclick(registry: &FrameRegistry, name: &str) -> String {
    frame(registry, name)
        .unwrap_or_else(|| panic!("{name} missing"))
        .onclick
        .clone()
        .unwrap_or_default()
}

/// Click `name` in the rendered frame and return the re-rendered frame.
fn click(
    display: &mut WorldMapDisplay,
    registry: &FrameRegistry,
    name: &str,
) -> (FrameRegistry, String) {
    let action = onclick(registry, name);
    let forwarded = match display.click(&action).unwrap() {
        WorldMapClick::QuestLog(action) => action.to_owned(),
        _ => String::new(),
    };
    (render(map_state(*display)), forwarded)
}

#[test]
fn windowed_map_is_retail_size_with_and_without_the_quest_log() {
    let with_log = render(map_state(WorldMapDisplay::default()));
    assert_eq!(size(&with_log, "WorldMapBorderFrame"), [1035.0, 534.0]);
    let (title, _) = text(&with_log, "WorldMapTitle");
    assert_eq!(title, "Map & Quest Log");
    let without_log = render(WorldMapFrameState {
        quest_panel: None,
        ..map_state(WorldMapDisplay::default())
    });
    assert_eq!(size(&without_log, "WorldMapBorderFrame"), [702.0, 534.0]);
    // The canvas fills the window left of the panel: 702 - 5 by 534 - 69.
    assert_eq!(size(&with_log, "WorldMapCanvas"), [697.0, 465.0]);
    assert_eq!(size(&without_log, "WorldMapCanvas"), [697.0, 465.0]);
}

#[test]
fn quest_panel_lists_log_quests_under_their_zone_headers() {
    let registry = render(map_state(WorldMapDisplay::default()));
    let northshire = top(&registry, "QuestLogHeader9");
    let beating = top(&registry, "QuestLogTitle28762");
    let lions = top(&registry, "QuestLogTitle28763");
    let elwynn = top(&registry, "QuestLogHeader12");
    let candles = top(&registry, "QuestLogTitle60");
    assert!(northshire < beating && beating < lions && lions < elwynn && elwynn < candles);
    assert_eq!(text(&registry, "QuestLogHeader9Text").0, "Northshire");
    assert_eq!(text(&registry, "QuestLogHeader12Text").0, "Elwynn Forest");
    let (title, color) = text(&registry, "QuestLogTitle28763Text");
    assert!(title.ends_with("Lions for Lambs"), "{title}");
    assert_eq!(color, [1.0, 0.1, 0.1, 1.0], "impossible quests are red");
    assert_eq!(
        text(&registry, "QuestLogTitle28762Text").1,
        [1.0, 0.82, 0.0, 1.0]
    );
    assert!(shown(&registry, "QuestLogTitle28762Check"), "tracked quest");
    assert!(!shown(&registry, "QuestLogTitle60Check"));
    assert!(!shown(&registry, "QuestLogDetailsTitle"));
}

#[test]
fn clicking_a_quest_shows_its_details_and_back_returns_to_the_list() {
    let mut display = WorldMapDisplay::default();
    let list = render(map_state(display));
    let (details, forwarded) = click(&mut display, &list, "QuestLogTitle28762");
    assert_eq!(forwarded, "quest_log:select:28762");
    assert_eq!(
        text(&details, "QuestLogDetailsTitle").0,
        "Beating Them Back!"
    );
    assert_eq!(
        text(&details, "QuestLogDetailsObjective0").0,
        "- 2/6 Blackrock Worg slain"
    );
    assert!(!shown(&details, "QuestLogTitle28763Text"));
    assert_eq!(text(&details, "QuestMapFrameTrackButton").0, "Untrack");
    assert_eq!(
        onclick(&details, "QuestMapFrameAbandonButton"),
        "quest_log:abandon"
    );

    let (list, forwarded) = click(&mut display, &details, "QuestMapFrameBackButton");
    assert_eq!(forwarded, "");
    assert!(shown(&list, "QuestLogTitle28763Text"));
    assert!(!shown(&list, "QuestLogDetailsTitle"));
}

#[test]
fn quest_map_geometry_and_selection_art_match_source_in_both_skins() {
    use ui_toolkit::atlas::resolve_region;
    use ui_toolkit::widgets::texture::BlendMode;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = map_state(WorldMapDisplay::default());
        state.quest_panel.as_mut().unwrap().log.groups[0].quests[0].selected = true;
        let list = render(state);
        assert_eq!(size(&list, "QuestMapFrame"), [330.0, 506.0]);
        assert_eq!(top(&list, "QuestMapFrame"), 25.0);
        assert_eq!(size(&list, "QuestScrollFrame"), [308.0, 477.0]);
        assert_eq!(top(&list, "QuestScrollFrame"), 54.0);
        assert_eq!(size(&list, "QuestLogHeader9"), [289.0, 22.0]);
        let selected = frame(&list, "QuestLogTitle28762Selected").unwrap();
        assert_eq!(selected.background_color, None);
        let Some(WidgetData::Texture(texture)) = &selected.widget_data else {
            panic!("selection art texture");
        };
        assert_eq!(texture.blend_mode, BlendMode::Additive);
        assert_eq!(texture.vertex_color[3], 0.4);
        let details = render(map_state(WorldMapDisplay {
            quest_details: true,
            ..Default::default()
        }));
        assert_eq!(size(&details, "QuestMapDetailsScrollFrame"), [298.0, 430.0]);
        assert_eq!(top(&details, "QuestMapDetailsScrollFrame"), 69.0);
        assert_eq!(
            size(&details, "QuestMapDetailsBackground"),
            [
                308.0,
                resolve_region("QuestDetailsBackgrounds", skin)
                    .unwrap()
                    .height
            ]
        );
        assert_eq!(size(&details, "QuestMapFrameBackButton"), [90.0, 22.0]);
        assert_eq!(
            onclick(&details, "QuestMapFrameShareButton"),
            "quest_log:share"
        );
    }
}

#[test]
fn maximize_toggles_the_frame_size_and_hides_the_quest_panel() {
    let mut display = WorldMapDisplay::default();
    let windowed = render(map_state(display));
    let (maximized, _) = click(&mut display, &windowed, "WorldMapMaximizeMinimizeButton");
    assert!(display.maximized);
    assert_eq!(size(&maximized, "WorldMapBorderFrame"), [1522.0, 1080.0]);
    assert_eq!(text(&maximized, "WorldMapTitle").0, "World Map");
    assert!(!shown(&maximized, "QuestLogTitle28762Text"));
    assert!(shown(&maximized, "WorldMapBlackout"));

    let (restored, _) = click(&mut display, &maximized, "WorldMapMaximizeMinimizeButton");
    assert!(!display.maximized);
    assert_eq!(size(&restored, "WorldMapBorderFrame"), [1035.0, 534.0]);
    assert!(shown(&restored, "QuestLogTitle28762Text"));
}

/// Forever prefixes the quest level (Camelot/QuestMapFrameOverrides.lua:13-16).
#[test]
fn forever_prefixes_quest_titles_with_their_level() {
    set_thread_skin(ActiveSkin::Forever);
    let forever = render(map_state(WorldMapDisplay::default()));
    set_thread_skin(ActiveSkin::Modern);
    let modern = render(map_state(WorldMapDisplay::default()));
    assert_eq!(
        text(&forever, "QuestLogTitle28762Text").0,
        "[2] Beating Them Back!"
    );
    assert_eq!(
        text(&modern, "QuestLogTitle28762Text").0,
        "Beating Them Back!"
    );
}
