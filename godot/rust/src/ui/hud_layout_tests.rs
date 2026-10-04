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
    MAIN_ACTION_BAR, MainActionBarState, main_action_bar_screen,
};
use game_engine_ui_model::micro_menu::{MICRO_MENU, MicroMenuView, micro_menu_screen};
use game_engine_ui_model::minimap::{MINIMAP_CLUSTER, MinimapClusterState, minimap_cluster_screen};
use game_engine_ui_model::objective_tracker_component::{
    ObjectiveTrackerState, TRACKER_FRAME, objective_tracker_screen,
};
use game_engine_ui_model::status::{ClassBar, ClassBarResource};
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
        model(MainActionBarState::default(), main_action_bar_screen),
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
        model(ObjectiveTrackerState::default(), objective_tracker_screen),
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
    // 329×40 micro menu BOTTOMRIGHT (-6, 6); 368×47 bags bar TOPRIGHT (-6, 96).
    assert_eq!(top_left(hud, MICRO_MENU), (1031.0, 722.0));
    assert_eq!(top_right(hud, "BagsBar"), (1360.0, 672.0));
    assert_modern_edit_mode_systems(hud);
}

/// Systems whose anchors the Forever preset shares with Modern.
fn assert_modern_edit_mode_systems(hud: &[RegistryModel]) {
    assert_eq!(top_right(hud, BUFF_FRAME.0), (1111.0, 10.0));
    assert_eq!(top_right(hud, DEBUFF_FRAME.0), (1096.0, 155.0));
    assert_eq!(top_right(hud, MINIMAP_CLUSTER), (1366.0, 0.0));
    assert_eq!(top_left(hud, PARTY_FRAME), (22.0, 147.0));
    let raid = rect(hud, RAID_FRAME);
    assert_eq!((raid.x, raid.y + raid.height), (395.0, 553.0));
    assert_eq!(top_left(hud, DAMAGE_METER_ROOT.0), (0.0, 0.0));
    assert_eq!(top_right(hud, TRACKER_FRAME), (1256.0, 275.0));
}

#[test]
fn modern_preset_keeps_the_retail_modern_hud_positions() {
    let mut hud = hud();
    sync(&mut hud, ActiveSkin::Modern);
    assert_modern(&hud);
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
    // Camelot: micro menu BOTTOM (116.5, 6); main bar BOTTOMRIGHT on its BOTTOMLEFT
    // (-4.5, -4); bags bar BOTTOMLEFT on its BOTTOMRIGHT (7, -4).
    assert_eq!(top_left(&hud, MICRO_MENU), (635.0, 722.0));
    // The main bar's buttons take FlareUI's scale 1.06: 562×45 grows to 595.72×47.7.
    let bar = rect(&hud, MAIN_ACTION_BAR.0);
    let edges = [bar.x, bar.y, bar.x + bar.width, bar.y + bar.height];
    let expected = [630.5 - 562.0 * 1.06, 766.0 - 45.0 * 1.06, 630.5, 766.0];
    assert!(
        edges
            .iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1e-3),
        "MainActionBar {edges:?} != {expected:?}"
    );
    assert_eq!(top_left(&hud, "BagsBar"), (971.0, 719.0));
    // Forever's Mainline preset keeps Modern's minimap, aura, group, meter and tracker
    // anchors.
    assert_modern_edit_mode_systems(&hud);

    sync(&mut hud, ActiveSkin::Modern);
    assert_modern(&hud);
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
