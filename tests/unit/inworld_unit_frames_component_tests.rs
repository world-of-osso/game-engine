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

#[test]
fn cluster_flanks_docked_cast_bar_above_action_bars_at_1080p() {
    let reg = cluster_registry();
    let player = rect_by_name(&reg, "PlayerFrame");
    let target = rect_by_name(&reg, "TargetFrame");
    let cast = rect_by_name(&reg, "PlayerCastingBarFrame");

    assert_eq!(
        player,
        LayoutRect {
            x: 679.0,
            y: 877.0,
            width: 133.0,
            height: 51.0,
        }
    );
    assert_eq!(
        target,
        LayoutRect {
            x: 1108.0,
            y: 877.0,
            width: 133.0,
            height: 51.0,
        }
    );
    assert_eq!(cast.x, player.x + player.width + CAST_DOCK_GAP);
    assert_eq!(cast.x + cast.width, target.x - CAST_DOCK_GAP);
    assert_eq!(cast.y + cast.height, player.y + player.height);
    assert_eq!(
        player.x + player.width / 2.0 + target.x + target.width / 2.0,
        1920.0,
        "cluster is centred"
    );
    assert!(player.y + player.height < SECOND_ACTION_ROW_TOP);
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
fn target_auras_hang_below_the_frame() {
    let reg = unit_frames_registry();
    let frame = rect_by_name(&reg, "TargetFrame");
    let buffs = rect_by_name(&reg, "TargetBuffRow");
    let debuffs = rect_by_name(&reg, "TargetDebuffRow");
    assert!(buffs.y >= frame.y + frame.height);
    assert!(debuffs.y >= buffs.y + buffs.height);
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
fn target_aura_icons_render_with_timer_and_stacks() {
    let reg = unit_frames_registry();

    let buff = reg
        .get(
            reg.get_by_name("TargetBuffIcon0Texture")
                .expect("target buff texture"),
        )
        .expect("target buff texture frame");
    let debuff = reg
        .get(
            reg.get_by_name("TargetDebuffIcon0Texture")
                .expect("target debuff texture"),
        )
        .expect("target debuff texture frame");
    let buff_timer = reg
        .get(
            reg.get_by_name("TargetBuffIcon0Timer")
                .expect("target buff timer"),
        )
        .expect("target buff timer");
    let debuff_stack = reg
        .get(
            reg.get_by_name("TargetDebuffIcon0Stack")
                .expect("target debuff stack"),
        )
        .expect("target debuff stack");

    let Some(ui_toolkit::frame::WidgetData::Texture(buff_texture)) = buff.widget_data.as_ref()
    else {
        panic!("expected TargetBuffIcon0Texture texture");
    };
    let Some(ui_toolkit::frame::WidgetData::Texture(debuff_texture)) = debuff.widget_data.as_ref()
    else {
        panic!("expected TargetDebuffIcon0Texture texture");
    };
    let Some(ui_toolkit::frame::WidgetData::FontString(buff_timer_text)) =
        buff_timer.widget_data.as_ref()
    else {
        panic!("expected TargetBuffIcon0Timer fontstring");
    };
    let Some(ui_toolkit::frame::WidgetData::FontString(debuff_stack_text)) =
        debuff_stack.widget_data.as_ref()
    else {
        panic!("expected TargetDebuffIcon0Stack fontstring");
    };

    assert!(matches!(
        buff_texture.source,
        crate::ui::widgets::texture::TextureSource::FileDataId(136078)
    ));
    assert!(matches!(
        debuff_texture.source,
        crate::ui::widgets::texture::TextureSource::FileDataId(136207)
    ));
    assert_eq!(buff_timer_text.text, "5m");
    assert_eq!(debuff_stack_text.text, "3");
}

fn sample_player_frame_state() -> UnitFrameState {
    UnitFrameState {
        level_text: "70".into(),
        health_text: "80 / 100".into(),
        health_fraction: 0.8,
        ..UnitFrameState::named("Theron")
    }
}

fn sample_target_frame_state() -> UnitFrameState {
    UnitFrameState {
        level_text: "7".into(),
        reaction: Some(crate::faction_reaction::Reaction::Hostile),
        target_buffs: vec![TargetAuraIconState {
            icon_fdid: 136078,
            timer_text: "5m".to_string(),
            stacks: 1,
            border_color: "0.85,0.75,0.35,1.0".to_string(),
            mine: false,
        }],
        target_debuffs: vec![TargetAuraIconState {
            icon_fdid: 136207,
            timer_text: "12s".to_string(),
            stacks: 3,
            border_color: "0.2,0.6,1.0,1.0".to_string(),
            mine: false,
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
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: sample_player_frame_state(),
        target_of_target: Some(SmallUnitFrameState::from(&sample_player_frame_state())),
        focus: Some(SmallUnitFrameState::from(&target)),
        target: Some(target),
        menu: UnitFrameMenuState::default(),
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
