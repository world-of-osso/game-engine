use super::*;
use crate::ui::layout::LayoutRect;
use crate::ui::registry::FrameRegistry;
use crate::ui::screens::casting_bar_frame_component::{CastingBarState, casting_bar_frame_screen};
#[path = "../../src/ui/screens/menu_character_layout_test_support.rs"]
mod layout_test_support;
use crate::ui::widgets::texture::{TextureData, TextureSource};
use inworld_unit_frames_art::FRAME_PORTRAIT_OFF;
use layout_test_support::compute_layout;
use ui_toolkit::screen::Screen;

/// Action bar row 2 top edge at 1080p: row 1 top (1080 - 52 - 45) minus one 47px slot step.
const SECOND_ACTION_ROW_TOP: f32 = 1080.0 - 52.0 - 45.0 - 47.0;

/// Retail Modern preset PlayerFrame BOTTOMRIGHT at BOTTOM (-300, 250), health bar TOPLEFT
/// (85, -41) in the 232×100 frame: the portrait-off art sits 450 left of centre and 272 up.
#[test]
fn player_frame_sits_at_the_modern_preset() {
    let reg = cluster_registry();
    assert_eq!(
        rect_by_name(&reg, "PlayerFrame"),
        LayoutRect {
            x: 960.0 - 450.0,
            y: 1080.0 - 272.0 - 51.0,
            width: 133.0,
            height: 51.0,
        }
    );
    let cast = rect_by_name(&reg, "PlayerCastingBarFrame");
    assert_eq!(cast.x + cast.width / 2.0, 960.0, "cast bar is centred");
    assert!(
        cast.y + cast.height < SECOND_ACTION_ROW_TOP,
        "cast bar clears two bar rows"
    );
}

/// Retail Modern preset TargetFrame BOTTOMLEFT at BOTTOM (300, 250); for a normal unit its
/// health bar's BOTTOMRIGHT is at LEFT + (149, -10) (TargetFrame.lua:419), so the
/// portrait-off art sits 320 right of centre and 273 up.
#[test]
fn target_frame_sits_at_the_modern_preset() {
    let reg = cluster_registry();
    assert_eq!(
        rect_by_name(&reg, "TargetFrame"),
        LayoutRect {
            x: 960.0 + 320.0,
            y: 1080.0 - 273.0 - 51.0,
            width: 133.0,
            height: 51.0,
        }
    );
}

/// Retail draws the player health bar 41..61 below its frame top and a normal target's
/// 40..60 below: the health bars line up within the 1 px Retail itself leaves.
#[test]
fn player_and_target_health_bars_line_up() {
    let reg = cluster_registry();
    let player = rect_by_name(&reg, "PlayerHealthBar");
    let target = rect_by_name(&reg, "TargetHealthBar");
    assert_eq!(target.y + target.height, player.y + player.height - 1.0);
    assert_eq!(
        target.x + target.width / 2.0 - 960.0,
        960.0 - (player.x + player.width / 2.0),
        "mirrored about the screen centre"
    );
}

#[test]
fn small_frames_sit_right_of_target_top_aligned() {
    let reg = cluster_registry();
    let target = rect_by_name(&reg, "TargetFrame");
    let tot = rect_by_name(&reg, "TargetOfTargetFrame");
    let focus = rect_by_name(&reg, "FocusFrame");

    assert_eq!(
        (tot.x, tot.y),
        (target.x + target.width + SMALL_FRAME_GAP, target.y)
    );
    assert_eq!(
        (focus.x, focus.y),
        (tot.x + tot.width + SMALL_FRAME_GAP, target.y)
    );
    assert!(
        focus.x + focus.width <= 1920.0 - 50.0,
        "clears the side bars"
    );
}

#[test]
fn frames_draw_portrait_off_art_with_bars_in_its_slots() {
    let reg = cluster_registry();
    for root in [
        "PlayerFrame",
        "TargetFrame",
        "TargetOfTargetFrame",
        "FocusFrame",
    ] {
        let frame = rect_by_name(&reg, root);
        let art_name = format!("{root}Art");
        assert_eq!(
            rect_by_name(&reg, &art_name),
            frame,
            "{art_name} fills {root}"
        );
        let art = texture(&reg, &art_name);
        assert_eq!(
            art.source,
            TextureSource::FileDataId(FRAME_PORTRAIT_OFF.fdid),
            "{art_name}"
        );
    }
    // Health and power sit in the 124×20 and 124×10 slots of the 133×51 art.
    let frame = rect_by_name(&reg, "TargetFrame");
    let health = rect_by_name(&reg, "TargetHealthBar");
    assert_eq!(
        (
            health.x - frame.x,
            health.y - frame.y,
            health.width,
            health.height
        ),
        (3.0, 14.0, 124.0, 20.0)
    );
    assert!(
        reg.get_by_name("TargetFrameBacking").is_none(),
        "no flat backing behind the art"
    );
}

#[test]
fn hostile_target_auras_hang_off_the_retail_frame_texture_debuffs_first() {
    let reg = unit_frames_registry();
    let frame = rect_by_name(&reg, "TargetFrame");
    let container = rect_by_name(&reg, "TargetFrameAuras");
    // FrameTexture BOTTOMLEFT + (5, 9): 5 px in, 48.5 px down the portrait-off art (the
    // layout snaps to whole pixels).
    assert_eq!(container.x - frame.x, 5.0);
    assert!(
        (container.y - frame.y - 48.5).abs() <= 0.5,
        "{}",
        container.y - frame.y
    );
    let debuff = rect_by_name(&reg, "TargetDebuffIcon0");
    let buff = rect_by_name(&reg, "TargetBuffIcon0");
    assert_eq!((debuff.x, debuff.y), (container.x, container.y));
    // The buff group starts a new line: 17 px debuff line + 3 px line spacing.
    assert_eq!((buff.x, buff.y), (container.x, container.y + 20.0));
}

#[test]
fn only_focus_of_the_small_frames_carries_a_reaction_strip() {
    let reg = unit_frames_registry();
    let strip = texture(&reg, "FocusReputationColor");
    assert_eq!(
        strip.vertex_color,
        rgba(reaction_color(crate::faction_reaction::Reaction::Hostile))
    );
    assert!(reg.get_by_name("TargetOfTargetReputationColor").is_none());
}

#[test]
fn player_combat_and_resting_icons_follow_state() {
    let mut shared = sample_unit_frames_context();
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut reg);
    assert!(
        reg.get(reg.get_by_name("PlayerCombatIcon").unwrap())
            .unwrap()
            .hidden
    );

    let mut state = shared.get::<InWorldUnitFramesState>().unwrap().clone();
    state.player.show_combat_icon = true;
    state.player.show_resting_icon = true;
    shared.insert(state);
    let mut screen = Screen::new(inworld_unit_frames_screen);
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    screen.sync(&shared, &mut reg);
    assert!(
        !reg.get(reg.get_by_name("PlayerCombatIcon").unwrap())
            .unwrap()
            .hidden
    );
    assert!(
        !reg.get(reg.get_by_name("PlayerRestingIcon").unwrap())
            .unwrap()
            .hidden
    );
}

#[test]
fn target_aura_buttons_show_icon_count_dispel_border_and_swipe_frame() {
    let reg = unit_frames_registry();
    let texture = |name: &str| match reg
        .get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.widget_data.as_ref())
    {
        Some(ui_toolkit::frame::WidgetData::Texture(texture)) => texture.clone(),
        _ => panic!("{name} is not a texture"),
    };
    let text = |name: &str| match reg
        .get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.widget_data.as_ref())
    {
        Some(ui_toolkit::frame::WidgetData::FontString(text)) => text.text.clone(),
        _ => panic!("{name} is not a fontstring"),
    };
    assert_eq!(
        texture("TargetBuffIcon0Texture").source,
        TextureSource::FileDataId(136078)
    );
    assert_eq!(
        texture("TargetDebuffIcon0Texture").source,
        TextureSource::FileDataId(136207)
    );
    assert_eq!(text("TargetDebuffIcon0Count"), "3");
    assert_eq!(text("TargetBuffIcon0Count"), "");
    let border = texture("TargetDebuffIcon0Border");
    assert_eq!(border.source, TextureSource::FileDataId(130759));
    assert_eq!(border.vertex_color, [0.2, 0.6, 1.0, 1.0]);
    assert!(reg.get_by_name("TargetBuffIcon0Border").is_none());
    let icon = rect_by_name(&reg, "TargetDebuffIcon0");
    let border = rect_by_name(&reg, "TargetDebuffIcon0Border");
    assert_eq!(
        (border.x, border.y, border.width),
        (icon.x - 1.0, icon.y - 1.0, 19.0)
    );
    let swipe = rect_by_name(&reg, "TargetDebuffIcon0Cooldown");
    assert_eq!((swipe.x, swipe.y), (icon.x, icon.y + 1.0));
}

fn sample_player_frame_state() -> UnitFrameState {
    UnitFrameState {
        level_text: "70".into(),
        health_text: StatusBarText {
            center: "80 / 100".into(),
            ..Default::default()
        },
        health_fraction: 0.8,
        ..UnitFrameState::named("Theron")
    }
}

fn sample_target_frame_state() -> UnitFrameState {
    UnitFrameState {
        level_text: "7".into(),
        reaction: Some(crate::faction_reaction::Reaction::Hostile),
        target_buffs: vec![TargetAuraIconState {
            spell_id: 1126,
            icon_fdid: 136078,
            stacks: 1,
            dispel_color: None,
            large: false,
            elapsed: Some(0.25),
        }],
        target_debuffs: vec![TargetAuraIconState {
            spell_id: 589,
            icon_fdid: 136207,
            stacks: 3,
            dispel_color: Some("0.2,0.6,1.0,1.0".to_string()),
            large: false,
            elapsed: Some(0.5),
        }],
        ..UnitFrameState::named("Timber Wolf")
    }
}

fn unit_frames_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&sample_unit_frames_context(), &mut reg);
    compute_layout(&mut reg);
    reg
}

fn cluster_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&sample_unit_frames_context(), &mut reg);
    let mut cast = SharedContext::new();
    cast.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    cast.insert(CastingBarState {
        visible: true,
        ..CastingBarState::default()
    });
    Screen::new(casting_bar_frame_screen).sync(&cast, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn sample_unit_frames_context() -> SharedContext {
    let target = sample_target_frame_state();
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: sample_player_frame_state(),
        target_of_target: Some(SmallUnitFrameState::from(&sample_player_frame_state())),
        focus: Some(SmallUnitFrameState::from(&target)),
        pet: None,
        target: Some(target),
        bosses: vec![UnitFrameState {
            level_text: "32".into(),
            health_text: StatusBarText {
                center: "153 K / 153 K".into(),
                ..Default::default()
            },
            health_fraction: 1.0,
            reaction: Some(crate::faction_reaction::Reaction::Hostile),
            ..UnitFrameState::named("Hogger")
        }],
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    shared
}

fn texture<'a>(reg: &'a FrameRegistry, name: &str) -> &'a TextureData {
    match reg
        .get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.widget_data.as_ref())
    {
        Some(ui_toolkit::frame::WidgetData::Texture(texture)) => texture,
        _ => panic!("{name} is not a texture"),
    }
}

fn rgba(color: &str) -> [f32; 4] {
    let parts: Vec<f32> = color.split(',').map(|part| part.parse().unwrap()).collect();
    [parts[0], parts[1], parts[2], parts[3]]
}

fn rect_by_name(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .expect(name)
}

fn open_menu_state(shared: &mut SharedContext, difficulty_menu: Option<DifficultyMenuState>) {
    let mut state = sample_unit_frames_context()
        .get::<InWorldUnitFramesState>()
        .unwrap()
        .clone();
    state.menu = UnitFrameMenuState {
        visible: true,
        title: "Xpbar".into(),
        x: 746.0,
        y: 850.0,
        player_items: vec![UnitMenuItem {
            name: "UnitFrameContextMenuDungeonDifficulty".into(),
            label: "Dungeon Difficulty".into(),
            action: ACTION_UNIT_MENU_DUNGEON_DIFFICULTY.into(),
        }],
        difficulty_menu,
    };
    shared.insert(state);
}

/// The live client keeps one registry and lays it out incrementally: the submenu opened
/// after the menu must get geometry on the following frames.
#[test]
fn the_difficulty_submenu_is_laid_out_when_it_opens_on_a_settled_menu() {
    let mut app = layout_test_support::layout_app(1920.0, 1080.0);
    app.finish();
    app.cleanup();
    let mut screen = Screen::new(inworld_unit_frames_screen);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    open_menu_state(&mut shared, None);
    {
        let mut ui = app.world_mut().resource_mut::<crate::ui::plugin::UiState>();
        screen.sync(&shared, &mut ui.registry);
    }
    for _ in 0..3 {
        app.update();
    }
    let submenu = DifficultyMenuState {
        x: 886.0,
        y: 928.0,
        entries: [
            (1, "Normal", true),
            (2, "Heroic", false),
            (23, "Mythic", false),
        ]
        .into_iter()
        .map(|(difficulty_id, label, checked)| DifficultyMenuEntry {
            difficulty_id,
            label: label.into(),
            checked,
            enabled: true,
        })
        .collect(),
    };
    open_menu_state(&mut shared, Some(submenu));
    {
        let mut ui = app.world_mut().resource_mut::<crate::ui::plugin::UiState>();
        screen.sync(&shared, &mut ui.registry);
    }
    for _ in 0..3 {
        app.update();
    }
    let ui = app.world().resource::<crate::ui::plugin::UiState>();
    let heroic = rect_by_name(&ui.registry, "UnitFrameDifficultyMenu2");
    assert_eq!((heroic.x, heroic.width), (892.0, 108.0));
    assert!(heroic.y > 928.0, "{heroic:?}");
}
