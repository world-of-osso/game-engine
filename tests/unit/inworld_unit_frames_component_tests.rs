use super::*;
use crate::ui::layout::LayoutRect;
use crate::ui::registry::FrameRegistry;
use ui_toolkit::layout::recompute_layouts;
use ui_toolkit::screen::Screen;
use ui_toolkit::widgets::font_string::{GameFont, Outline};

#[test]
fn fill_width_clamps_to_bounds() {
    assert_eq!(fill_width(124.0, Some(50.0), Some(100.0)), 62.0);
    assert_eq!(fill_width(124.0, Some(200.0), Some(100.0)), 124.0);
    assert_eq!(fill_width(124.0, Some(-5.0), Some(100.0)), 0.0);
}

#[test]
fn format_value_text_handles_missing_values() {
    assert_eq!(format_value_text(Some(42.0), Some(80.0)), "42 / 80");
    assert_eq!(format_value_text(Some(42.0), None), "42");
    assert_eq!(format_value_text(None, Some(80.0)), "");
}

#[test]
fn unit_frames_match_wow_screen_rects() {
    let reg = unit_frames_registry();

    assert_eq!(
        rect_by_name(&reg, "PlayerFrame"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x,
            y: 850.0,
            width: FRAME_W,
            height: FRAME_H,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "TargetFrame"),
        LayoutRect {
            x: TARGET_FRAME_CONFIG.frame_x,
            y: 850.0,
            width: FRAME_W,
            height: FRAME_H,
        }
    );
}

#[test]
fn player_frame_key_geometry_matches_wow_spec() {
    let reg = unit_frames_registry();

    assert_eq!(
        rect_by_name(&reg, "PlayerPortrait"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x + PLAYER_FRAME_CONFIG.portrait.x,
            y: 869.0,
            width: PLAYER_FRAME_CONFIG.portrait.width,
            height: PLAYER_FRAME_CONFIG.portrait.height,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "PlayerName"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x + PLAYER_FRAME_CONFIG.name.x,
            y: 877.0,
            width: PLAYER_FRAME_CONFIG.name.width,
            height: 12.0,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "PlayerHealthBar"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x + PLAYER_FRAME_CONFIG.health_bar.x,
            y: 890.0,
            width: PLAYER_FRAME_CONFIG.health_bar.width,
            height: BAR_H,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "PlayerManaBar"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x + PLAYER_FRAME_CONFIG.mana_bar.x,
            y: 911.0,
            width: PLAYER_FRAME_CONFIG.mana_bar.width,
            height: MANA_H,
        }
    );
}

#[test]
fn target_frame_key_geometry_matches_wow_spec() {
    let reg = unit_frames_registry();

    assert_eq!(
        rect_by_name(&reg, "TargetPortrait"),
        LayoutRect {
            x: TARGET_FRAME_CONFIG.frame_x + TARGET_FRAME_CONFIG.portrait.x,
            y: 869.0,
            width: TARGET_FRAME_CONFIG.portrait.width,
            height: TARGET_FRAME_CONFIG.portrait.height,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "TargetName"),
        LayoutRect {
            x: TARGET_FRAME_CONFIG.frame_x + TARGET_FRAME_CONFIG.name.x,
            y: 876.0,
            width: TARGET_FRAME_CONFIG.name.width,
            height: 12.0,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "TargetHealthBar"),
        LayoutRect {
            x: TARGET_FRAME_CONFIG.frame_x + TARGET_FRAME_CONFIG.health_bar.x,
            y: 878.0,
            width: TARGET_FRAME_CONFIG.health_bar.width,
            height: BAR_H,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "TargetManaBar"),
        LayoutRect {
            x: TARGET_FRAME_CONFIG.frame_x + TARGET_FRAME_CONFIG.mana_bar.x,
            y: 889.0,
            width: TARGET_FRAME_CONFIG.mana_bar.width,
            height: MANA_H,
        }
    );
}

#[test]
fn unit_frame_text_uses_wow_font_styles() {
    let reg = unit_frames_registry();

    let player_name = reg.get(reg.get_by_name("PlayerName").unwrap()).unwrap();
    let Some(ui_toolkit::frame::WidgetData::FontString(name_font)) =
        player_name.widget_data.as_ref()
    else {
        panic!("expected PlayerName fontstring");
    };
    assert_eq!(name_font.font, GameFont::FrizQuadrata);
    assert_eq!(name_font.font_size, 10.0);
    assert_eq!(name_font.shadow_color, Some([0.0, 0.0, 0.0, 1.0]));
    assert_eq!(name_font.shadow_offset, [1.0, -1.0]);

    let player_health_text = reg
        .get(reg.get_by_name("PlayerHealthBarText").unwrap())
        .unwrap();
    let Some(ui_toolkit::frame::WidgetData::FontString(bar_font)) =
        player_health_text.widget_data.as_ref()
    else {
        panic!("expected PlayerHealthBarText fontstring");
    };
    assert_eq!(bar_font.font, GameFont::FrizQuadrata);
    assert_eq!(bar_font.font_size, 10.0);
    assert_eq!(bar_font.outline, Outline::Outline);
}

#[test]
fn explicit_size_icon_placeholders_match_wow_spec() {
    let reg = unit_frames_registry();

    assert_eq!(
        rect_by_name(&reg, "PlayerRoleIcon"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x + PLAYER_ROLE.x,
            y: 877.0,
            width: PLAYER_ROLE.width,
            height: PLAYER_ROLE.height,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "PlayerPrestigePortrait"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x + PLAYER_PRESTIGE.x,
            y: 888.0,
            width: PLAYER_PRESTIGE.width,
            height: PLAYER_PRESTIGE.height,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "TargetRaidTargetIcon"),
        LayoutRect {
            x: TARGET_FRAME_CONFIG.frame_x + TARGET_FRAME_CONFIG.portrait.x + 16.0,
            y: 856.0,
            width: TARGET_RAID_ICON.width,
            height: TARGET_RAID_ICON.height,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "TargetPetBattleIcon"),
        LayoutRect {
            x: TARGET_FRAME_CONFIG.frame_x + TARGET_PET_BATTLE.x,
            y: 902.0,
            width: TARGET_PET_BATTLE.width,
            height: TARGET_PET_BATTLE.height,
        }
    );
}

#[test]
fn player_resting_labels_render_when_enabled() {
    let reg = unit_frames_registry();

    let icon = reg.get(reg.get_by_name("PlayerRestingIcon").expect("resting icon"));
    let label = reg.get(
        reg.get_by_name("PlayerRestingLabel")
            .expect("resting label"),
    );
    let Some(ui_toolkit::frame::WidgetData::FontString(icon_font)) =
        icon.and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("expected PlayerRestingIcon fontstring");
    };
    let Some(ui_toolkit::frame::WidgetData::FontString(label_font)) =
        label.and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("expected PlayerRestingLabel fontstring");
    };

    assert_eq!(icon_font.text, "zzz");
    assert_eq!(label_font.text, "Resting");
}

#[test]
fn player_combat_icon_renders_when_enabled() {
    let reg = unit_frames_registry();

    let icon = reg.get(reg.get_by_name("PlayerCombatIcon").expect("combat icon"));
    let Some(ui_toolkit::frame::WidgetData::FontString(icon_font)) =
        icon.and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("expected PlayerCombatIcon fontstring");
    };

    assert_eq!(icon_font.text, "⚔");
}

fn unit_frames_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState {
            name: "Player".to_string(),
            level_text: "10".to_string(),
            resting_text: "Resting".to_string(),
            health_text: "100 / 100".to_string(),
            mana_text: "80 / 80".to_string(),
            health_fill_width: PLAYER_HEALTH_BAR_W,
            mana_fill_width: PLAYER_HEALTH_BAR_W,
            has_mana: true,
            show_combat_icon: true,
            show_resting_icon: true,
        },
        target: Some(UnitFrameState {
            name: "Target".to_string(),
            level_text: "12".to_string(),
            resting_text: String::new(),
            health_text: "90 / 90".to_string(),
            mana_text: "60 / 60".to_string(),
            health_fill_width: TARGET_HEALTH_BAR_W,
            mana_fill_width: TARGET_MANA_BAR_W,
            has_mana: true,
            show_combat_icon: false,
            show_resting_icon: false,
        }),
    });
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

fn rect_by_name(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .expect(name)
}
