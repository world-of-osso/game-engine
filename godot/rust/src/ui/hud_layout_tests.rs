//! HUD frames sit where the active preset puts them: Modern at the Retail Modern Edit Mode
//! anchors, Forever at FlareUI's unit-frame positions and Forever's Camelot constants. A
//! preset switch goes through the canvas skin mirror, as `GameClient::apply_ui_layout` does.

use std::collections::HashMap;

use game_engine_ui_model::bags_bar_component::BagBarState;
use game_engine_ui_model::buff_frame_component::{
    BUFF_FRAME, BuffFrameState, DEBUFF_FRAME, buff_frame_screen,
};
use game_engine_ui_model::casting_bar_frame_component::{
    CastingBarState, casting_bar_frame_screen,
};
use game_engine_ui_model::chat_frame_component::{
    CHAT_FLARE_SKIN, CHAT_FRAME, ChatFrameView, chat_frame_screen,
};
use game_engine_ui_model::compact_unit_frame_component::{CompactUnitView, UnitStatus};
use game_engine_ui_model::damage_meter_component::{DAMAGE_METER_ROOT, damage_meter_screen};
use game_engine_ui_model::damage_meter_data::DamageMeterView;
use game_engine_ui_model::group_frames_component::{
    GroupFramesState, PARTY_FRAME, RAID_FRAME, group_frames_screen,
};
use game_engine_ui_model::inworld_unit_frames_component::class_bars::settled_view;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PetFrameState, PowerBarState, SmallUnitFrameState, UnitFrameMenuState,
    UnitFrameState, inworld_unit_frames_screen,
};
use game_engine_ui_model::main_action_bar_component::{
    ActionBar, MAIN_ACTION_BAR, MainActionBarState, main_action_bar_screen,
};
use game_engine_ui_model::micro_menu::{MICRO_MENU, MicroMenuView, micro_menu_screen};
use game_engine_ui_model::minimap::{MINIMAP_CLUSTER, MinimapClusterState, minimap_cluster_screen};
use game_engine_ui_model::objective_tracker_component::{
    ObjectiveLine, ObjectiveLineStyle, ObjectiveTrackerState, TRACKER_FRAME, TrackedQuest,
    objective_tracker_screen,
};
use game_engine_ui_model::pet_action_bar_component::{PetActionBarState, pet_action_bar_screen};
use game_engine_ui_model::status::{ClassBar, ClassBarResource};
use game_engine_ui_model::xp_bar_component::{XpBarState, xp_bar_screen};
use shared::components::PowerType;
use shared::protocol::GroupRoleSnapshot;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;

use super::ui_parent::UiParent;
use super::{RegistryModel, ScreenPostsetup};
use crate::ui::layout::compute_layout_with_intrinsics;

/// A 16:9 viewport whose UIParent canvas is 1366×768 UI units (scale 2).
const VIEWPORT: (f32, f32) = (2732.0, 1536.0);

fn model<T: 'static>(state: T, build: fn(&SharedContext) -> Element) -> RegistryModel {
    let mut shared = SharedContext::new();
    shared.insert(state);
    RegistryModel {
        screen: Screen::new(build),
        shared,
        registry: UiParent::for_viewport(VIEWPORT.0, VIEWPORT.1).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    }
}

fn member(name: &str) -> CompactUnitView {
    CompactUnitView {
        name: name.into(),
        class_rgb: [1.0, 1.0, 1.0],
        health_fraction: Some(1.0),
        power: None,
        role: GroupRoleSnapshot::None,
        status: UnitStatus::Online,
        in_range: true,
        selected: false,
        ready: None,
        debuffs: Vec::new(),
    }
}

/// Player, target, target of target, focus and pet frames, all shown.
fn unit_frames() -> InWorldUnitFramesState {
    let target = UnitFrameState::named("Fbhudtarget");
    InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        target_cast: None,
        player: UnitFrameState::named("Fbhud"),
        target_of_target: Some(SmallUnitFrameState::from(&target)),
        focus: Some(SmallUnitFrameState::from(&target)),
        target: Some(target),
        pet: Some(PetFrameState {
            name: "Wolf".into(),
            health_fraction: 1.0,
            reaction: None,
            health_text: Default::default(),
            power: None,
            power_text: Default::default(),
        }),
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    }
}

/// Every HUD canvas with each positioned frame shown.
fn hud() -> Vec<RegistryModel> {
    // The objective tracker measures its header text.
    set_data_root();
    let groups = GroupFramesState {
        party: vec![member("Fbhud")],
        raid: vec![vec![member("Fbhud")]],
        ..Default::default()
    };
    let cast = CastingBarState {
        visible: true,
        ..Default::default()
    };
    vec![
        model(unit_frames(), inworld_unit_frames_screen),
        model(cast, casting_bar_frame_screen),
        // A paladin's bar.
        model(
            MainActionBarState {
                player_class: Some(2),
                ..Default::default()
            },
            main_action_bar_screen,
        ),
        model(
            PetActionBarState {
                visible: true,
                ..Default::default()
            },
            pet_action_bar_screen,
        ),
        model(MicroMenuView::default(), micro_menu_screen),
        // The bag bar as the client mounts it: inside the bags canvas's composed screen.
        model(
            crate::bags::BagsView::closed(BagBarState::default()),
            crate::bags::bags_screen,
        ),
        model(BuffFrameState::default(), buff_frame_screen),
        model(MinimapClusterState::default(), minimap_cluster_screen),
        model(groups, group_frames_screen),
        model(DamageMeterView::default(), damage_meter_screen),
        model(
            ChatFrameView {
                input_open: true,
                scrolled_up: true,
                ..Default::default()
            },
            chat_frame_screen,
        ),
        model(ObjectiveTrackerState::default(), objective_tracker_screen),
        model(
            XpBarState {
                xp: 250,
                next_level_xp: 1000,
                rested_xp: 400,
                hovered: false,
            },
            xp_bar_screen,
        ),
    ]
}

/// Atlas tables and fonts load from the checkout's `data/`.
fn set_data_root() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn sync(hud: &mut [RegistryModel], skin: ActiveSkin) {
    for model in hud {
        model.sync_skin(skin);
    }
}

/// `name`'s rect on the canvas, in UI units.
fn rect(hud: &[RegistryModel], name: &str) -> LayoutRect {
    let model = hud
        .iter()
        .find(|model| model.registry.get_by_name(name).is_some())
        .unwrap_or_else(|| panic!("no canvas draws {name}"));
    let bounds = compute_layout_with_intrinsics(&model.registry, &HashMap::new()).unwrap();
    bounds[&model.registry.get_by_name(name).unwrap()].clone()
}

fn top_left(hud: &[RegistryModel], name: &str) -> (f32, f32) {
    let rect = rect(hud, name);
    (rect.x, rect.y)
}

/// Right edge and top.
fn top_right(hud: &[RegistryModel], name: &str) -> (f32, f32) {
    let rect = rect(hud, name);
    (rect.x + rect.width, rect.y)
}

fn assert_rect(hud: &[RegistryModel], name: &str, expected: (f32, f32, f32, f32)) {
    let r = rect(hud, name);
    let actual = (r.x, r.y, r.width, r.height);
    for (a, b) in [
        (actual.0, expected.0),
        (actual.1, expected.1),
        (actual.2, expected.2),
        (actual.3, expected.3),
    ] {
        assert!((a - b).abs() < 0.001, "{name}: {actual:?} != {expected:?}");
    }
}

fn assert_modern(hud: &[RegistryModel]) {
    // Centre x 683, bottom 768. PlayerFrame BOTTOMRIGHT and TargetFrame BOTTOMLEFT at BOTTOM
    // (∓300, 250), 232×100.
    assert_eq!(top_left(hud, "PlayerFrame"), (151.0, 418.0));
    assert_eq!(top_left(hud, "TargetFrame"), (983.0, 418.0));
    // 99.75×38.25 small frames right of the target, tops on its 67-high FrameTexture.
    assert_eq!(top_left(hud, "TargetOfTargetFrame"), (1223.0, 434.5));
    assert_eq!(top_left(hud, "FocusFrame"), (1330.75, 434.5));
    // 120×49 PetFrame under PlayerFrame's BOTTOM + (30, 25).
    assert_eq!(top_left(hud, "PetFrame"), (244.5, 493.0));
    // 264×28 cast bar BOTTOM 152; 562×45 main bar BOTTOM 45.
    assert_eq!(top_left(hud, "PlayerCastingBarFrame"), (551.0, 588.0));
    assert_eq!(top_left(hud, MAIN_ACTION_BAR.0), (402.0, 678.0));
    // 329×40 micro menu BOTTOMRIGHT (-6, 6); 208×47 bags bar TOPRIGHT (-6, 96).
    assert_eq!(top_left(hud, MICRO_MENU), (1031.0, 722.0));
    assert_eq!(top_right(hud, "BagsBar"), (1360.0, 672.0));
    assert_rect(hud, MAIN_ACTION_BAR.0, (402.0, 678.0, 562.0, 45.0));
    assert_rect(hud, "MainActionBarLeftEndCap", (306.5, 647.0, 104.5, 98.0));
    assert_rect(hud, "MainActionBarRightEndCap", (956.0, 647.0, 104.5, 98.0));
    assert_rect(hud, MICRO_MENU, (1031.0, 722.0, 329.0, 40.0));
    assert_rect(hud, "BagsBar", (1152.0, 672.0, 208.0, 47.0));
    assert_rect(hud, "PetActionBar", (402.0, 643.0, 318.0, 30.0));
    assert_rect(hud, DAMAGE_METER_ROOT.0, (0.0, 0.0, 400.0, 140.0));
    assert_rect(hud, CHAT_FRAME.0, (0.0, 448.0, 500.0, 280.0));
    assert_rect(hud, "ChatFrame1EditBox", (0.0, 696.0, 500.0, 32.0));
    assert_rect(
        hud,
        "ChatFrame1ScrollToBottomButton",
        (467.0, 657.0, 26.0, 28.0),
    );
    assert_modern_edit_mode_systems(hud);
    // Mainline/EditModePresetLayouts.lua:549-554: TOPRIGHT (-110, -275).
    assert_eq!(top_right(hud, TRACKER_FRAME), (1256.0, 275.0));
}

/// Systems whose anchors the Forever preset shares with Modern.
fn assert_modern_edit_mode_systems(hud: &[RegistryModel]) {
    assert_eq!(top_right(hud, BUFF_FRAME.0), (1111.0, 10.0));
    assert_eq!(top_right(hud, DEBUFF_FRAME.0), (1096.0, 155.0));
    assert_eq!(top_right(hud, MINIMAP_CLUSTER), (1366.0, 0.0));
    assert_eq!(top_left(hud, PARTY_FRAME), (22.0, 147.0));
    let raid = rect(hud, RAID_FRAME);
    assert_eq!((raid.x, raid.y + raid.height), (395.0, 553.0));
}

#[test]
fn xp_bar_canvas_mirrors_skin_and_repositions_with_the_hud() {
    let mut hud = hud();
    sync(&mut hud, ActiveSkin::Modern);
    assert_rect(&hud, "ExperienceBar", (397.5, 751.0, 571.0, 17.0));
    sync(&mut hud, ActiveSkin::Forever);
    assert_rect(&hud, "ExperienceBar", (385.0, 6.0, 596.0, 17.0));
    sync(&mut hud, ActiveSkin::Modern);
    assert_rect(&hud, "ExperienceBar", (397.5, 751.0, 571.0, 17.0));
}

#[test]
fn modern_preset_keeps_the_retail_modern_hud_positions() {
    let mut hud = hud();
    sync(&mut hud, ActiveSkin::Modern);
    assert_modern(&hud);
}

fn intersects(a: &LayoutRect, b: &LayoutRect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// The reference block as the layout engine places it: three bars of 9, 9 and 10 buttons
/// centred on the 1366-wide canvas and stacked upward, class shields beside the lower rows
/// covering no button, the pet bar above all of it.
fn assert_forever_action_bars(hud: &[RegistryModel]) {
    let bar_names = [
        MAIN_ACTION_BAR.0,
        "MultiBarBottomLeft",
        "MultiBarBottomRight",
    ];
    let bars = bar_names.map(|name| rect(hud, name));
    for (name, bar) in bar_names.iter().zip(&bars) {
        assert!((bar.x + bar.width / 2.0 - 683.0).abs() < 0.001, "{name}");
    }
    assert!(bars[1].y + bars[1].height < bars[0].y);
    assert!(bars[2].y + bars[2].height < bars[1].y);
    let model = hud
        .iter()
        .find(|model| model.registry.get_by_name(MAIN_ACTION_BAR.0).is_some())
        .unwrap();
    let buttons: Vec<(String, LayoutRect)> = ActionBar::ALL
        .into_iter()
        .flat_map(|bar| (0..12).map(move |index| bar.button_name(index)))
        .filter(|name| model.registry.get_by_name(name).is_some())
        .map(|name| {
            let button = rect(hud, &name);
            (name, button)
        })
        .collect();
    assert_eq!(buttons.len(), 9 + 9 + 10);
    let caps = ["MainActionBarLeftEndCap", "MainActionBarRightEndCap"].map(|cap| rect(hud, cap));
    assert!(caps[0].x + caps[0].width <= bars[0].x);
    assert!(caps[1].x >= bars[0].x + bars[0].width);
    let pet = rect(hud, "PetActionBar");
    for (index, (name, button)) in buttons.iter().enumerate() {
        for cap in &caps {
            assert!(!intersects(button, cap), "{name} under an end cap");
        }
        assert!(!intersects(button, &pet), "{name} under the pet bar");
        for (other_name, other) in &buttons[index + 1..] {
            assert!(!intersects(button, other), "{name} overlaps {other_name}");
        }
    }
    assert!(pet.y + pet.height <= bars[2].y);
}

/// Micro menu and bags bar are shown inside the 1366×768 canvas, clear of each other and
/// of the meter, the chat panel, the action bars, their end caps and the pet bar.
fn assert_forever_utility_bars(hud: &[RegistryModel]) {
    let others = [
        DAMAGE_METER_ROOT.0,
        CHAT_FRAME.0,
        CHAT_FLARE_SKIN,
        MAIN_ACTION_BAR.0,
        "MultiBarBottomLeft",
        "MultiBarBottomRight",
        "MainActionBarLeftEndCap",
        "MainActionBarRightEndCap",
        "PetActionBar",
    ];
    let utility = [MICRO_MENU, "BagsBar"].map(|name| (name, rect(hud, name)));
    for (name, bar) in &utility {
        let model = hud
            .iter()
            .find(|model| model.registry.get_by_name(name).is_some())
            .unwrap();
        let id = model.registry.get_by_name(name).unwrap();
        assert!(model.registry.get(id).unwrap().visible, "{name} not shown");
        assert!(bar.width > 0.0 && bar.height > 0.0, "{name} is empty");
        assert!(
            bar.x >= 0.0
                && bar.y >= 0.0
                && bar.x + bar.width <= 1366.0
                && bar.y + bar.height <= 768.0,
            "{name} leaves the canvas: {bar:?}"
        );
        for other in others {
            let other_rect = rect(hud, other);
            assert!(
                !intersects(bar, &other_rect),
                "{name} {bar:?} overlaps {other} {other_rect:?}"
            );
        }
    }
    assert!(!intersects(&utility[0].1, &utility[1].1));
}

#[test]
fn forever_preset_moves_the_hud_and_modern_restores_it() {
    let mut hud = hud();
    sync(&mut hud, ActiveSkin::Modern);
    sync(&mut hud, ActiveSkin::Forever);
    // FlareUI UnitFrames.lua:1888-1894 with its frame sizes (Core.lua:281-302):
    // 240×60 player/target CENTER (∓330, -270); centre y 384.
    assert_eq!(top_left(&hud, "PlayerFrame"), (233.0, 624.0));
    assert_eq!(top_left(&hud, "TargetFrame"), (893.0, 624.0));
    // Reference: ToT top-aligned 8px right of target; pet 6px below player, right-aligned.
    // Source sizes remain 120×28 ToT, 160×28 pet, 160×36 focus (RIGHT -453,-258).
    assert_eq!(top_left(&hud, "TargetOfTargetFrame"), (1141.0, 624.0));
    assert_eq!(top_left(&hud, "FocusFrame"), (753.0, 624.0));
    assert_eq!(top_left(&hud, "PetFrame"), (313.0, 690.0));
    // The 292×26 cast bar plus flush 26px icon inside its 326×34 holder at BOTTOM (0,268).
    assert_eq!(top_left(&hud, "PlayerCastingBarFrame"), (520.0, 466.0));
    let bar = rect(&hud, "CastingBarBackground");
    assert_eq!(
        (bar.x, bar.y, bar.width, bar.height),
        (550.0, 470.0, 292.0, 26.0)
    );
    assert_forever_action_bars(&hud);
    assert_forever_utility_bars(&hud);
    // FlareUI matches the meter root to the chat skin; mirror it across UIParent.
    assert_rect(&hud, DAMAGE_METER_ROOT.0, (891.0, 419.0, 450.0, 214.0));
    // Forever chat messages 430x170 at BOTTOMLEFT(35,145), padding 10 + header 24.
    assert_rect(&hud, CHAT_FRAME.0, (1.0, 426.0, 469.0, 235.0));
    assert_rect(&hud, CHAT_FLARE_SKIN, (25.0, 419.0, 450.0, 214.0));
    assert_rect(&hud, "ChatFrame1Messages", (35.0, 453.0, 430.0, 170.0));
    assert_rect(&hud, "ChatFrame1EditBox", (1.0, 629.0, 469.0, 32.0));
    assert_rect(
        &hud,
        "ChatFrame1ScrollToBottomButton",
        (437.0, 590.0, 26.0, 28.0),
    );
    assert_modern_edit_mode_systems(&hud);

    sync(&mut hud, ActiveSkin::Modern);
    assert_modern(&hud);
}

#[test]
fn forever_tracker_matches_minimap_width_and_modern_restores_geometry() {
    let mut hud = hud();
    sync(&mut hud, ActiveSkin::Modern);
    assert_rect(&hud, TRACKER_FRAME, (996.0, 275.0, 260.0, 32.0));
    let modern_header = rect(&hud, "ObjectiveTrackerFrameHeaderBackground");
    println!("MODERN_TRACKER_EMPTY_HASH {}", tracker_canvas_hash(&hud));
    sync(&mut hud, ActiveSkin::Forever);
    let map = rect(&hud, MINIMAP_CLUSTER);
    let tracker = rect(&hud, TRACKER_FRAME);
    let header = rect(&hud, "ObjectiveTrackerFrameHeaderBackground");
    // FlareUI Modules/Minimap.lua:371-379,401-406: SetScale(frame outer width / 288), the
    // 288 visible pixels of the 300-pixel header line; here 260 / 288. SetScale also scales
    // the Mainline TOPRIGHT (-110, -275) offsets: right edge 1366 - 99.306, top 248.264.
    let actual = [
        tracker.x,
        tracker.y,
        tracker.width,
        tracker.height,
        header.width,
    ];
    let expected = [1031.9722, 248.26389, 234.72223, 28.88889, 270.83334];
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
    }
    assert!((header.width * 288.0 / 300.0 - map.width).abs() < 0.001);
    sync(&mut hud, ActiveSkin::Modern);
    assert_rect(&hud, TRACKER_FRAME, (996.0, 275.0, 260.0, 32.0));
    assert_eq!(
        rect(&hud, "ObjectiveTrackerFrameHeaderBackground"),
        modern_header
    );
}

/// Snapshot the external frame data and computed bounds, not the RSX construction.
fn tracker_canvas_hash(hud: &[RegistryModel]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let registry = &hud.last().unwrap().registry;
    let rects = compute_layout_with_intrinsics(registry, &HashMap::new()).unwrap();
    let mut frames: Vec<_> = registry.frames_iter().collect();
    frames.sort_by_key(|frame| frame.id);
    let mut hash = DefaultHasher::new();
    for frame in frames {
        format!(
            "{:?} {:?} {:?} {} {} {:?} {:?} {:?} {:?}",
            frame.name,
            frame.parent_id,
            rects[&frame.id],
            frame.visible,
            frame.alpha,
            frame.background_color,
            frame.widget_data,
            frame.strata,
            frame.onclick,
        )
        .hash(&mut hash);
    }
    hash.finish()
}

#[test]
fn forever_tracker_scales_quest_text_art_and_hit_rects_without_changing_modern() {
    set_data_root();
    let state = ObjectiveTrackerState {
        quests: vec![TrackedQuest {
            quest_id: 783,
            title: "A Threat Within".into(),
            complete: false,
            lines: vec![
                ObjectiveLine {
                    text: "2/8 Defias slain".into(),
                    style: ObjectiveLineStyle::InProgress,
                },
                ObjectiveLine {
                    text: "Marshal McBride found".into(),
                    style: ObjectiveLineStyle::Completed,
                },
            ],
        }],
        ..Default::default()
    };
    let mut hud = [model(state, objective_tracker_screen)];
    sync(&mut hud, ActiveSkin::Modern);
    let modern = tracker_canvas_hash(&hud);
    println!("MODERN_TRACKER_POPULATED_HASH {modern}");
    sync(&mut hud, ActiveSkin::Forever);
    let registry = &hud[0].registry;
    for (name, expected) in [
        ("ObjectiveTrackerFrameHeaderText", 12.638889),
        ("QuestBlock783HeaderText", 10.833333),
        ("QuestBlock783Line0Text", 10.833333),
    ] {
        let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let Some(ui_toolkit::frame::WidgetData::FontString(font)) = &frame.widget_data else {
            panic!("{name} is not text");
        };
        assert!((font.font_size - expected).abs() < 0.001, "{name}");
    }
    let hit = rect(&hud, "QuestBlock783POIButton");
    assert!((hit.width - 18.055555).abs() < 0.001);
    assert!((hit.height - 18.055555).abs() < 0.001);
    let check = rect(&hud, "QuestBlock783Line1Check");
    assert!((check.width - 14.444445).abs() < 0.001);
    sync(&mut hud, ActiveSkin::Modern);
    assert_eq!(tracker_canvas_hash(&hud), modern);
}

/// Retail Arcane Charges: `PlayerFrameBottomManagedFramesContainer` top 4 px below the
/// 124-wide mana bar (PlayerFrame.lua:716-718,758) plus the bar's `topPadding` 7
/// (MageArcaneChargesBar.xml:134), centred 1 px left of the mana bar; four 21 px charges
/// 10 px apart (MageArcaneChargesBar.xml:6,125).
#[test]
fn arcane_charges_hang_below_the_mana_bar_clear_of_its_text() {
    set_data_root();
    let arcane = ClassBarResource {
        bar: ClassBar::ArcaneCharges,
        current: 0,
        max: 4,
        tenths: 0,
        dynamics: Default::default(),
        spec: None,
        in_combat: false,
    };
    let mut frames = unit_frames();
    frames.player.power = Some(PowerBarState {
        power: PowerType::Mana,
        current: 1000,
        max: 1000,
    });
    frames.player.class_bar = settled_view(&arcane);
    let mut hud = [model(frames, inworld_unit_frames_screen)];
    sync(&mut hud, ActiveSkin::Modern);
    let mana = rect(&hud, "PlayerManaBar");
    let row = rect(&hud, "PlayerSecondaryResourceRow");
    assert_eq!(row.y, mana.y + mana.height + 11.0);
    assert_eq!(row.x + row.width / 2.0, mana.x + mana.width / 2.0 - 1.0);
    assert_eq!((row.width, row.height), (4.0 * 21.0 + 3.0 * 10.0, 21.0));
}

#[test]
fn target_cast_follows_target_rect_and_switches_preset_without_moving_player_cast() {
    set_data_root();
    let mut state = unit_frames();
    state.target_of_target = None;
    state.focus = None;
    state.target_cast = Some(CastingBarState {
        visible: true,
        spell_name: "Frostbolt".into(),
        icon_fdid: Some(135846),
        timer_text: "1.5".into(),
        progress: 0.25,
        ..Default::default()
    });
    let mut hud = vec![model(state, inworld_unit_frames_screen)];
    sync(&mut hud, ActiveSkin::Modern);
    assert_rect(&hud, "TargetFrameSpellBar", (1026.0, 513.0, 150.0, 10.0));
    assert_rect(&hud, "TargetCastingBarIcon", (1004.0, 513.0, 20.0, 20.0));
    sync(&mut hud, ActiveSkin::Forever);
    assert_rect(&hud, "TargetFrameSpellBar", (893.0, 684.0, 240.0, 24.0));
    assert_rect(
        &hud,
        "TargetCastingBarBackground",
        (913.0, 688.0, 216.0, 16.0),
    );
    sync(&mut hud, ActiveSkin::Modern);
    assert_rect(&hud, "TargetFrameSpellBar", (1026.0, 513.0, 150.0, 10.0));
}

/// Party and raid members stack without overlapping inside their group frame under both
/// presets. FlareUI's hidden party title (`Core.lua:392`, `Modules/Tweaks.lua:869-874`) is a
/// `Hide()`: the members stay where Modern has them.
#[test]
fn group_members_do_not_overlap_and_keep_their_place_without_the_party_title() {
    set_data_root();
    let five = || {
        (1..=5)
            .map(|n| member(&format!("Fbhud{n}")))
            .collect::<Vec<_>>()
    };
    let groups = GroupFramesState {
        party: five(),
        raid: vec![five(), five()],
        ..Default::default()
    };
    let mut hud = [model(groups, group_frames_screen)];
    let members: Vec<String> = (1..=5)
        .flat_map(|n| {
            [
                format!("CompactPartyFrameMember{n}"),
                format!("CompactRaidGroup1Member{n}"),
                format!("CompactRaidGroup2Member{n}"),
            ]
        })
        .collect();
    let mut rects = Vec::new();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        sync(&mut hud, skin);
        let placed: Vec<_> = members.iter().map(|name| rect(&hud, name)).collect();
        for (at, a) in placed.iter().enumerate() {
            for (other, b) in placed.iter().enumerate().skip(at + 1) {
                // The party and the raid are separate frames, never shown together.
                if members[at].contains("Party") != members[other].contains("Party") {
                    continue;
                }
                let apart = a.x + a.width <= b.x
                    || b.x + b.width <= a.x
                    || a.y + a.height <= b.y
                    || b.y + b.height <= a.y;
                assert!(
                    apart,
                    "{} overlaps {} under {skin:?}",
                    members[at], members[other]
                );
            }
        }
        let party = rect(&hud, "CompactPartyFrame");
        let last = rect(&hud, "CompactPartyFrameMember5");
        assert_eq!(last.y + last.height, party.y + party.height, "{skin:?}");
        rects.push(
            placed
                .iter()
                .map(|r| (r.x, r.y, r.width, r.height))
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(rects[0], rects[1]);
}

#[path = "hud_layout_settings_tests.rs"]
mod settings;
