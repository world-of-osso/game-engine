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

fn shown_micro_menu() -> RegistryModel {
    let mut model = model(MicroMenuView::default(), micro_menu_screen);
    model
        .shared
        .insert(game_engine_core::ui_layout_data::LayoutSettings {
            show_micro_menu: Some(true),
            ..Default::default()
        });
    model
}

/// Every HUD canvas with each positioned frame shown (micro menu explicitly opted in).
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
        shown_micro_menu(),
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
    // Flush right (user decision 2026-10-05); Retail is TOPRIGHT (-110, -275),
    // Mainline/EditModePresetLayouts.lua:549-554.
    assert_eq!(top_right(hud, TRACKER_FRAME), (1366.0, 275.0));
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

#[test]
fn raidoverlap_two_member_raid_clears_player_in_both_presets_and_viewports() {
    for viewport in [(1920.0, 1080.0), (1280.0, 720.0)] {
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            let groups = GroupFramesState {
                raid: vec![vec![member("Mailalpha"), member("Mailbeta")]],
                ..Default::default()
            };
            let mut frames = vec![
                model(unit_frames(), inworld_unit_frames_screen),
                model(groups, group_frames_screen),
            ];
            for frame in &mut frames {
                frame.registry = UiParent::for_viewport(viewport.0, viewport.1).registry();
            }
            sync(&mut frames, skin);
            let player = rect(&frames, "PlayerFrame");
            let raid = rect(&frames, RAID_FRAME);
            assert!(
                !intersects(&raid, &player),
                "{skin:?} {viewport:?}: raid {raid:?} overlaps player {player:?}"
            );
            for name in ["CompactRaidGroup1Member1", "CompactRaidGroup1Member2"] {
                assert!(!intersects(&rect(&frames, name), &player));
            }
        }
    }
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
    // Reference: pet 6px below player, right-aligned. ToT and focus: the next test.
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
    // Size still matches the chat skin; placement follows the user's top-left choice.
    let meter = rect(&hud, DAMAGE_METER_ROOT.0);
    let minimap = rect(&hud, MINIMAP_CLUSTER);
    assert_eq!(meter.x, 1366.0 - minimap.x - minimap.width);
    assert_eq!(meter.y, minimap.y);
    assert_eq!((meter.width, meter.height), (450.0, 214.0));
    // User decision 2026-10-06: visible bronze edge corner-flush, input inside it.
    assert_rect(&hud, CHAT_FRAME.0, (-26.0, 563.0, 469.0, 235.0));
    assert_rect(&hud, CHAT_FLARE_SKIN, (-2.0, 556.0, 450.0, 214.0));
    assert_rect(&hud, "ChatFrame1Messages", (8.0, 590.0, 430.0, 138.0));
    assert_rect(&hud, "ChatFrame1EditBox", (1.0, 735.0, 442.0, 32.0));
    assert_rect(
        &hud,
        "ChatFrame1ScrollToBottomButton",
        (410.0, 695.0, 26.0, 28.0),
    );
    assert_modern_edit_mode_systems(&hud);

    sync(&mut hud, ActiveSkin::Modern);
    assert_modern(&hud);
}

#[test]
fn chatflush_forever_visible_corner_and_input_clear_approved_frames() {
    let mut canvases = default_canvas_hud();
    sync(&mut canvases, ActiveSkin::Forever);
    let panel = rect(&canvases, CHAT_FLARE_SKIN);
    let input = rect(&canvases, "ChatFrame1EditBox");
    println!("CHATFLUSH panel {panel:?}; input {input:?}");
    for name in [
        MAIN_ACTION_BAR.0,
        "MultiBarBottomLeft",
        "MultiBarBottomRight",
        "MainActionBarLeftEndCap",
        "MainActionBarRightEndCap",
        "PetActionBar",
        "PlayerFrame",
        "BagsBar",
        "DamageMeterFlareSkin",
    ] {
        let other = rect(&canvases, name);
        println!("CHATFLUSH {name} {other:?}");
        assert!(
            !intersects(&panel, &other),
            "chat {panel:?} overlaps {name} {other:?}"
        );
        assert!(
            !intersects(&input, &other),
            "input {input:?} overlaps {name} {other:?}"
        );
    }
    // Reference's bronze border reaches x=0 and the last screen row. The authored
    // tooltip-border sheet's line is two units inside its outer rectangle.
    assert!((panel.x + 2.0).abs() < 0.001, "left edge: {panel:?}");
    assert!(
        (panel.y + panel.height - 2.0 - 1080.0).abs() < 0.001,
        "bottom edge: {panel:?}"
    );
    assert!(input.x >= 0.0 && input.y >= 0.0, "input {input:?}");
    assert!(
        input.x + input.width <= 1920.0 && input.y + input.height <= 1080.0,
        "input {input:?}"
    );
    let messages = rect(&canvases, "ChatFrame1Messages");
    assert!(
        messages.y + messages.height <= input.y,
        "messages {messages:?} under input {input:?}"
    );
    for index in 0..3 {
        let tab = rect(&canvases, &format!("ChatFrame1TabsTab{index}"));
        assert!(tab.x >= 0.0 && tab.y >= 0.0, "tab {tab:?}");
    }
}

fn default_canvas_hud() -> Vec<RegistryModel> {
    let mut canvases = hud();
    for canvas in &mut canvases {
        canvas.registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
    }
    canvases
}

#[test]
fn launcher_clears_buff_area_on_default_1920x1080_canvas() {
    let mut canvases = default_canvas_hud();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        sync(&mut canvases, skin);
        let launcher = rect(&canvases, "MinimapLauncherButton");
        assert_eq!((launcher.width, launcher.height), (30.0, 30.0));
        // Every buff row, and the first debuff row: up to eight debuffs (DEBUFFS_PER_ROW).
        let buffs = rect(&canvases, BUFF_FRAME.0);
        let mut first_debuff_row = rect(&canvases, DEBUFF_FRAME.0);
        first_debuff_row.height = 40.0;
        for auras in [buffs, first_debuff_row] {
            assert!(
                !intersects(&launcher, &auras),
                "{skin:?}: {launcher:?} overlaps {auras:?}"
            );
        }
    }
}

#[test]
fn minimapgaps_forever_badge_preserves_approved_corner_placements() {
    let mut canvases = default_canvas_hud();
    sync(&mut canvases, ActiveSkin::Forever);
    assert_rect(&canvases, MINIMAP_CLUSTER, (1660.0, 0.0, 260.0, 260.0));
    assert_rect(
        &canvases,
        "MinimapLauncherButton",
        (1624.0, 230.0, 30.0, 30.0),
    );
    assert_rect(
        &canvases,
        "MinimapDayNightBadge",
        (1662.0, 226.0, 33.0, 32.0),
    );
    let badge = rect(&canvases, "MinimapDayNightBadge");
    let launcher = rect(&canvases, "MinimapLauncherButton");
    assert!(!intersects(&badge, &launcher));
    let minimap = rect(&canvases, MINIMAP_CLUSTER);
    for name in [TRACKER_FRAME, "ObjectiveTrackerFrameHeaderBackground"] {
        let tracker = rect(&canvases, name);
        assert!(!intersects(&tracker, &minimap), "{name}: {tracker:?}");
        assert!(!intersects(&tracker, &badge), "{name}: {tracker:?}");
    }
}

#[test]
fn bosslayout_managed_tracker_clears_bosses_and_returns_for_both_skins() {
    use game_engine_ui_model::objective_tracker_component::BossFrameCount;
    let mut canvases = default_canvas_hud();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        sync(&mut canvases, skin);
        let resting = rect(&canvases, TRACKER_FRAME);
        for count in [0, 1, 3, 0] {
            let mut units = unit_frames();
            units.bosses = (0..count)
                .map(|i| UnitFrameState::named(format!("Boss {i}")))
                .collect();
            for canvas in &mut canvases {
                if canvas.shared.get::<InWorldUnitFramesState>().is_some() {
                    canvas.shared.insert(units.clone());
                }
                if canvas.shared.get::<ObjectiveTrackerState>().is_some() {
                    canvas.shared.insert(BossFrameCount(count));
                }
            }
            sync(&mut canvases, skin);
            let tracker = rect(&canvases, TRACKER_FRAME);
            // Forever's frame ends 14 tracker units short so its header line meets the
            // minimap's right edge; that canvas snaps to whole pixels.
            let right = match skin {
                ActiveSkin::Modern => 1920.0,
                ActiveSkin::Forever => (1920.0f32 - 14.0 * 260.0 / 288.0).round(),
            };
            assert!((tracker.x + tracker.width - right).abs() < 0.001);
            if count == 0 {
                assert_eq!(tracker, resting, "{skin:?}: hidden bosses restore tracker");
            }
            let minimap = rect(&canvases, MINIMAP_CLUSTER);
            assert!(!intersects(&tracker, &minimap));
            let mut previous = None;
            for i in 0..count {
                let boss = rect(&canvases, &format!("Boss{}TargetFrame", i + 1));
                assert_eq!((boss.width, boss.height), (133.0, 51.0));
                assert!(!intersects(&boss, &minimap));
                assert!(
                    !intersects(&boss, &tracker),
                    "{skin:?}: {boss:?} overlaps {tracker:?}"
                );
                assert!(tracker.y > boss.y + boss.height);
                if let Some(last) = &previous {
                    assert!(!intersects(last, &boss));
                }
                previous = Some(boss);
            }
        }
    }
}

#[test]
fn tracker_clears_minimap_on_default_1920x1080_canvas() {
    let mut canvases = default_canvas_hud();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        sync(&mut canvases, skin);
        let minimap = rect(&canvases, MINIMAP_CLUSTER);
        let launcher = rect(&canvases, "MinimapLauncherButton");
        for name in [TRACKER_FRAME, "ObjectiveTrackerFrameHeaderBackground"] {
            let tracker = rect(&canvases, name);
            assert!(
                !intersects(&tracker, &minimap),
                "{skin:?}: {name} {tracker:?} overlaps {minimap:?}"
            );
            assert!(
                !intersects(&tracker, &launcher),
                "{skin:?}: {name} {tracker:?} overlaps {launcher:?}"
            );
        }
    }
}

#[test]
fn forever_tracker_matches_minimap_width_and_modern_restores_geometry() {
    let mut hud = hud();
    sync(&mut hud, ActiveSkin::Modern);
    assert_rect(&hud, TRACKER_FRAME, (1106.0, 275.0, 260.0, 32.0));
    let modern_header = rect(&hud, "ObjectiveTrackerFrameHeaderBackground");
    println!("MODERN_TRACKER_EMPTY_HASH {}", tracker_canvas_hash(&hud));
    sync(&mut hud, ActiveSkin::Forever);
    let map = rect(&hud, MINIMAP_CLUSTER);
    let tracker = rect(&hud, TRACKER_FRAME);
    let header = rect(&hud, "ObjectiveTrackerFrameHeaderBackground");
    // FlareUI Modules/Minimap.lua:371-379,401-406: SetScale(frame outer width / 288), the
    // 288 visible pixels of the 300-pixel header line; here 260 / 288. SetScale also scales
    // the TOPRIGHT offsets, so the header line's right end meets the minimap's: frame right
    // edge 1366 - 14 * 260/288, top 268 + 4 * 260/288.
    let actual = [
        tracker.x,
        tracker.y,
        tracker.width,
        tracker.height,
        header.width,
    ];
    let expected = [1118.6389, 271.61111, 234.72223, 28.88889, 270.83334];
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
    }
    assert!((header.width * 288.0 / 300.0 - map.width).abs() < 0.001);
    sync(&mut hud, ActiveSkin::Modern);
    assert_rect(&hud, TRACKER_FRAME, (1106.0, 275.0, 260.0, 32.0));
    assert_eq!(
        rect(&hud, "ObjectiveTrackerFrameHeaderBackground"),
        modern_header
    );
}

#[test]
fn forevertracker_header_line_sits_under_minimap_at_both_resolutions() {
    // 260/288 (FlareUI Modules/Minimap.lua:366-406): the header art is 300 wide, its line
    // fades over 6 units at each end, so 288 * scale = 260 shows, the minimap's width.
    let scale = 260.0 / 288.0;
    // The 1920 canvas has ui_scale 1, so layout snaps every edge to a whole pixel.
    for (mut canvases, width, tolerance) in
        [(default_canvas_hud(), 1920.0, 0.5), (hud(), 1366.0, 0.001)]
    {
        sync(&mut canvases, ActiveSkin::Modern);
        assert_rect(
            &canvases,
            TRACKER_FRAME,
            (width - 260.0, 275.0, 260.0, 32.0),
        );
        let modern_header = rect(&canvases, "ObjectiveTrackerFrameHeaderBackground");

        sync(&mut canvases, ActiveSkin::Forever);
        assert_rect(
            &canvases,
            MINIMAP_CLUSTER,
            (width - 260.0, 0.0, 260.0, 260.0),
        );
        // Header art top 8 below the minimap's bottom (260), its visible line spanning the
        // minimap's 260 columns exactly; the frame starts 4 tracker units below the art and
        // ends 14 tracker units left of the line's right end.
        assert_edges(
            &canvases,
            "ObjectiveTrackerFrameHeaderBackground",
            (
                width - 260.0 - 6.0 * scale,
                268.0,
                300.0 * scale,
                40.0 * scale,
            ),
            tolerance,
        );
        assert_edges(
            &canvases,
            TRACKER_FRAME,
            (
                width - 14.0 * scale - 260.0 * scale,
                268.0 + 4.0 * scale,
                260.0 * scale,
                32.0 * scale,
            ),
            tolerance,
        );
        let art = rect(&canvases, "ObjectiveTrackerFrameHeaderBackground");
        let map = rect(&canvases, MINIMAP_CLUSTER);
        let line_right = art.x + art.width - 6.0 * scale;
        assert!(
            (line_right - (map.x + map.width)).abs() <= tolerance,
            "{line_right}"
        );
        assert!(
            (art.y - (map.y + map.height) - 8.0).abs() <= tolerance,
            "{}",
            art.y
        );

        sync(&mut canvases, ActiveSkin::Modern);
        assert_rect(
            &canvases,
            TRACKER_FRAME,
            (width - 260.0, 275.0, 260.0, 32.0),
        );
        assert_eq!(
            rect(&canvases, "ObjectiveTrackerFrameHeaderBackground"),
            modern_header
        );
    }
}

/// `name`'s left, top, right and bottom edges each within `tolerance` of `expected`'s.
fn assert_edges(
    hud: &[RegistryModel],
    name: &str,
    (x, y, width, height): (f32, f32, f32, f32),
    tolerance: f32,
) {
    let r = rect(hud, name);
    let actual = [r.x, r.y, r.x + r.width, r.y + r.height];
    let expected = [x, y, x + width, y + height];
    for (a, b) in actual.into_iter().zip(expected) {
        assert!(
            (a - b).abs() <= tolerance,
            "{name}: {actual:?} != {expected:?}"
        );
    }
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

/// FlareUI's focus frame sits right of the target, top-aligned 10 units apart, and its
/// target of target left-aligned under the target's cast bar (the relations its edge-pinned
/// defaults, UnitFrames.lua:1890-1892, give on its own canvas), so on the 1080p canvas and
/// on the 1366x768 UIParent none of the four frames covers another.
#[test]
fn forever_focus_right_of_target_and_tot_under_its_cast_bar_overlap_nothing() {
    set_data_root();
    let mut state = unit_frames();
    state.target_cast = Some(CastingBarState {
        visible: true,
        spell_name: "Frostbolt".into(),
        icon_fdid: Some(135846),
        timer_text: "1.5".into(),
        progress: 0.25,
        ..Default::default()
    });
    for (width, height) in [(1920.0, 1080.0), (1366.0, 768.0)] {
        let mut hud = vec![model(state.clone(), inworld_unit_frames_screen)];
        hud[0].registry = ui_toolkit::registry::FrameRegistry::new(width, height);
        sync(&mut hud, ActiveSkin::Forever);
        let names = [
            "TargetFrame",
            "FocusFrame",
            "TargetOfTargetFrame",
            "TargetFrameSpellBar",
        ];
        let [target, focus, tot, cast] = names.map(|name| rect(&hud, name));
        assert_eq!(
            focus.y, target.y,
            "{width}: focus top-aligned with the target"
        );
        assert_eq!(
            focus.x - (target.x + target.width),
            10.0,
            "{width}: focus 10 right of the target"
        );
        assert_eq!(tot.x, target.x, "{width}: ToT left-aligned with the target");
        assert!(
            tot.y >= cast.y + cast.height,
            "{width}: ToT at {} above the cast bar's bottom {}",
            tot.y,
            cast.y + cast.height
        );
        let rects = [&target, &focus, &tot, &cast];
        for (i, a) in rects.iter().enumerate() {
            for (j, b) in rects.iter().enumerate().skip(i + 1) {
                assert!(
                    !intersects(a, b),
                    "{width}: {} {a:?} intersects {} {b:?}",
                    names[i],
                    names[j]
                );
            }
        }
    }
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

/// The old launcher covered half of Forever's player name ("Launchpo...").
#[test]
fn launcher_panel_clears_player_name_on_default_1080p_layout() {
    use game_engine_ui_model::launcher::{LauncherView, launcher_screen};

    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_data_root();
        let mut units = unit_frames();
        units.player.name = "Launchpolish".into();
        let mut canvases = [
            model(units, inworld_unit_frames_screen),
            model(
                LauncherView {
                    open: true,
                    ..Default::default()
                },
                launcher_screen,
            ),
        ];
        for canvas in &mut canvases {
            canvas.registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
        }
        sync(&mut canvases, skin);
        let panel = rect(&canvases, "LauncherPanel");
        let player = rect(&canvases, "PlayerFrame");
        assert!(
            !intersects(&panel, &player),
            "{skin:?}: launcher {panel:?} clips player {player:?}"
        );
    }
}

/// Launcher chrome must contain its controls, not just the title's baseline.
#[test]
fn launcher_header_contains_close_and_centres_title_in_both_skins() {
    use game_engine_ui_model::launcher::{LauncherView, launcher_screen};
    use ui_toolkit::frame::WidgetData;

    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_data_root();
        let mut canvases = [model(
            LauncherView {
                open: true,
                ..Default::default()
            },
            launcher_screen,
        )];
        sync(&mut canvases, skin);
        let header = rect(&canvases, "LauncherHeaderBar");
        let close = rect(&canvases, "LauncherCloseButton");
        let title = rect(&canvases, "LauncherTitleText");
        assert!(
            close.x >= header.x && close.x + close.width <= header.x + header.width,
            "{skin:?}: close {close:?} outside header {header:?}"
        );
        assert!(
            close.y >= header.y && close.y + close.height <= header.y + header.height,
            "{skin:?}: close {close:?} outside header {header:?}"
        );
        let header_centre = header.y + header.height / 2.0;
        assert!((close.y + close.height / 2.0 - header_centre).abs() <= 1.0);
        assert!((title.y + title.height / 2.0 - header_centre).abs() <= 1.0);
        let registry = &canvases[0].registry;
        let frame = registry
            .get(registry.get_by_name("LauncherTitleText").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(text)) = &frame.widget_data else {
            panic!("Launcher title must be text");
        };
        assert!(text.font_size >= 18.0, "{skin:?}: title still too small");
        assert_eq!(
            text.justify_v,
            ui_toolkit::widgets::font_string::JustifyV::Middle
        );
    }
}

/// Preset defaults mirror the minimap's corner margins without covering other HUD frames.
#[test]
fn preset_meter_mirrors_minimap_and_clears_hud_at_both_resolutions() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (width, height) in [(1920.0, 1080.0), (1366.0, 768.0)] {
            let mut canvases = hud();
            let mut units = unit_frames();
            units.target_cast = Some(CastingBarState {
                visible: true,
                spell_name: "Frostbolt".into(),
                progress: 0.25,
                ..Default::default()
            });
            canvases[0].shared.insert(units);
            for canvas in &mut canvases {
                canvas.registry = ui_toolkit::registry::FrameRegistry::new(width, height);
            }
            sync(&mut canvases, skin);
            let meter = rect(&canvases, DAMAGE_METER_ROOT.0);
            let minimap = rect(&canvases, MINIMAP_CLUSTER);
            println!("HUD_RECT {skin:?} {width} {height} DamageMeter {meter:?}");
            let map_right_margin = width - minimap.x - minimap.width;
            assert_eq!(
                meter.x, map_right_margin,
                "{skin:?} {width}: mirrored margin"
            );
            assert_eq!(meter.y, minimap.y, "{skin:?} {width}: top margin");
            for name in [
                MINIMAP_CLUSTER,
                TRACKER_FRAME,
                BUFF_FRAME.0,
                DEBUFF_FRAME.0,
                CHAT_FRAME.0,
                "PlayerFrame",
                "TargetFrame",
                "FocusFrame",
                "TargetOfTargetFrame",
                "TargetFrameSpellBar",
                MICRO_MENU,
                "BagsBar",
                "MinimapLauncherButton",
            ] {
                let other = rect(&canvases, name);
                println!("HUD_RECT {skin:?} {width} {height} {name} {other:?}");
                assert!(
                    !intersects(&meter, &other),
                    "{skin:?} {width}x{height}: meter {meter:?} overlaps {name} {other:?}"
                );
            }
        }
    }
}

#[test]
fn preset_meter_menus_open_below_header_and_stay_on_screen() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (width, height) in [(1920.0, 1080.0), (1366.0, 768.0)] {
            let view = DamageMeterView {
                menu_open: true,
                type_menu_open: true,
                ..Default::default()
            };
            let mut canvases = [model(view, damage_meter_screen)];
            canvases[0].registry = ui_toolkit::registry::FrameRegistry::new(width, height);
            sync(&mut canvases, skin);
            let meter = rect(&canvases, DAMAGE_METER_ROOT.0);
            for name in ["DamageMeterTypeMenu", "DamageMeterSessionMenu"] {
                let menu = rect(&canvases, name);
                assert!(menu.x >= 0.0, "{skin:?} {width}: {name} left");
                assert!(menu.y > meter.y, "{skin:?} {width}: {name} opens down");
                assert!(
                    menu.x + menu.width <= width,
                    "{skin:?} {width}: {name} right"
                );
                assert!(
                    menu.y + menu.height <= height,
                    "{skin:?} {width}: {name} bottom"
                );
            }
        }
    }
}

#[path = "hud_layout_settings_tests.rs"]
mod settings;
