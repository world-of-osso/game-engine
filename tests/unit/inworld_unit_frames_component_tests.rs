use super::*;
use crate::status::{SecondaryResourceEntry, SecondaryResourceKindEntry};
use crate::ui::layout::LayoutRect;
use crate::ui::registry::FrameRegistry;
use game_engine::ui::screens::inworld_unit_frames_component::TargetAuraIconState;
#[path = "../../src/ui/screens/menu_character_layout_test_support.rs"]
mod layout_test_support;
use layout_test_support::compute_layout;
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
            y: 844.0,
            width: 297.0,
            height: 106.0,
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
fn player_frame_key_geometry_matches_artwork_apertures() {
    let reg = unit_frames_registry();

    assert_eq!(
        rect_by_name(&reg, "PlayerPortrait"),
        LayoutRect {
            x: 282.0,
            y: 854.0,
            width: 83.0,
            height: 85.0,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "PlayerName"),
        LayoutRect {
            x: 369.0,
            y: 867.0,
            width: 142.0,
            height: 12.0,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "PlayerHealthBar"),
        LayoutRect {
            x: 369.0,
            y: 883.0,
            width: 187.0,
            height: 30.0,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "PlayerManaBar"),
        LayoutRect {
            x: 369.0,
            y: 915.0,
            width: 187.0,
            height: 15.0,
        }
    );
}

#[test]
fn player_contents_fit_the_scaled_artwork_openings() {
    let reg = unit_frames_registry();
    let shell = rect_by_name(&reg, "PlayerFrameTexture");
    let frame = rect_by_name(&reg, "PlayerFrame");
    // Centering an odd-width texture applies a half-pixel UiTransform after layout
    // rounding. Child aperture layout remains relative to the frame, not that transform.
    assert_eq!(
        shell,
        LayoutRect {
            x: frame.x + 0.5,
            ..frame
        }
    );
    assert_eq!((shell.width, shell.height), (297.0, 106.0));
    // At 1x, native layout rounds local positions and differences of absolute
    // edges separately: portrait width = round(364.75) - round(281.5) = 83.
    // The parent starts at y=844, so its rounded local y=10 places it at 854.
    for (name, (x, y, width, height)) in [
        ("PlayerPortrait", (282.0, 854.0, 83.0, 85.0)),
        ("PlayerHealthBar", (369.0, 883.0, 187.0, 30.0)),
        ("PlayerManaBar", (369.0, 915.0, 187.0, 15.0)),
    ] {
        assert_eq!(
            rect_by_name(&reg, name),
            LayoutRect {
                x,
                y,
                width,
                height,
            },
            "{name} must reach every edge of the pixel-snapped artwork aperture"
        );
    }
    for (name, opening) in [
        ("PlayerName", (369.0, 865.0, 142.0, 18.0)),
        ("PlayerLevelText", (529.0, 865.0, 27.0, 18.0)),
    ] {
        let rect = rect_by_name(&reg, name);
        let (x, y, width, height) = opening;
        let left = x;
        let top = y;
        assert!(
            rect.x >= left
                && rect.y >= top
                && rect.x + rect.width <= left + width
                && rect.y + rect.height <= top + height,
            "{name} {rect:?} must fit artwork opening {opening:?} at 75% scale"
        );
    }
}

#[test]
fn player_health_and_mana_updates_stay_inside_resized_bars() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = sample_unit_frames_context();
    let mut screen = Screen::new(inworld_unit_frames_screen);
    // Fill edges round independently: half fill is round(462.625) - round(369.25).
    for (percent, snapped_width) in [(100.0, 187.0), (50.0, 94.0), (0.0, 0.0), (100.0, 187.0)] {
        let mut state = shared.get::<InWorldUnitFramesState>().unwrap().clone();
        state.player.health_fill_width =
            fill_width(PLAYER_HEALTH_BAR_W, Some(percent), Some(100.0));
        state.player.mana_fill_width = fill_width(PLAYER_HEALTH_BAR_W, Some(percent), Some(100.0));
        shared.insert(state);
        screen.sync(&shared, &mut reg);
        compute_layout(&mut reg);
        for name in ["PlayerHealthBar", "PlayerManaBar"] {
            let bar = rect_by_name(&reg, name);
            let fill_name = format!("{name}Fill");
            let fill_frame = reg.get(reg.get_by_name(&fill_name).unwrap()).unwrap();
            assert_eq!(
                fill_frame.visible,
                percent > 0.0,
                "zero health must not draw an auto-sized fill"
            );
            if percent == 0.0 {
                continue;
            }
            let fill = rect_by_name(&reg, &fill_name);
            assert_eq!((fill.x, fill.y), (bar.x, bar.y));
            assert_eq!(fill.width, snapped_width);
            assert_eq!(fill.height, bar.height);
            assert!(fill.x + fill.width <= bar.x + bar.width);
        }
    }
}

#[test]
fn target_resting_label_keeps_original_offset_and_hidden_state() {
    let mut reg = unit_frames_registry();
    let label_id = reg.get_by_name("TargetRestingLabel").unwrap();
    assert!(!reg.get(label_id).unwrap().visible);
    reg.set_hidden(label_id, false);
    compute_layout(&mut reg);
    let target = rect_by_name(&reg, "TargetFrame");
    let label = rect_by_name(&reg, "TargetRestingLabel");
    assert_eq!(label.x, target.x + 85.0);
}

#[test]
fn player_status_widgets_fit_without_covering_resources() {
    let reg = unit_frames_registry();
    let portrait = rect_by_name(&reg, "PlayerPortrait");
    for name in ["PlayerCombatIcon", "PlayerRestingIcon"] {
        let rect = rect_by_name(&reg, name);
        assert!(
            rect.x >= portrait.x
                && rect.y >= portrait.y
                && rect.x + rect.width <= portrait.x + portrait.width
                && rect.y + rect.height <= portrait.y + portrait.height
        );
    }
    let mana = rect_by_name(&reg, "PlayerManaBar");
    let row = rect_by_name(&reg, "PlayerSecondaryResourceRow");
    let resting = rect_by_name(&reg, "PlayerRestingLabel");
    let frame = rect_by_name(&reg, "PlayerFrame");
    assert!(row.y >= mana.y + mana.height);
    assert!(resting.y >= row.y + row.height);
    assert!(resting.y + resting.height <= frame.y + frame.height);
    let player_portrait = reg.get(reg.get_by_name("PlayerPortrait").unwrap()).unwrap();
    assert_eq!(player_portrait.background_color, Some([0.0, 0.0, 0.0, 0.0]));
    let target_portrait = reg.get(reg.get_by_name("TargetPortrait").unwrap()).unwrap();
    assert_eq!(
        target_portrait.background_color,
        Some([0.02, 0.02, 0.02, 0.92])
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
fn explicit_size_icon_placeholders_match_wow_spec_when_shown() {
    let mut reg = unit_frames_registry();
    for name in [
        "PlayerRoleIcon",
        "PlayerPrestigePortrait",
        "TargetRaidTargetIcon",
        "TargetPetBattleIcon",
    ] {
        let id = reg.get_by_name(name).unwrap();
        assert!(reg.get(id).unwrap().hidden, "{name} starts hidden");
        reg.set_hidden(id, false);
    }
    compute_layout(&mut reg);

    assert_eq!(
        rect_by_name(&reg, "PlayerRoleIcon"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x + PLAYER_ROLE.x,
            y: 871.0,
            width: PLAYER_ROLE.width,
            height: PLAYER_ROLE.height,
        }
    );
    assert_eq!(
        rect_by_name(&reg, "PlayerPrestigePortrait"),
        LayoutRect {
            x: PLAYER_FRAME_CONFIG.frame_x + PLAYER_PRESTIGE.x,
            y: 882.0,
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

#[test]
fn unit_frame_portrait_textures_render() {
    let reg = unit_frames_registry();

    let player = reg
        .get(
            reg.get_by_name("PlayerPortraitTexture")
                .expect("player portrait texture"),
        )
        .expect("player portrait texture frame");
    let target = reg
        .get(
            reg.get_by_name("TargetPortraitTexture")
                .expect("target portrait texture"),
        )
        .expect("target portrait texture frame");

    let Some(ui_toolkit::frame::WidgetData::Texture(player_texture)) = player.widget_data.as_ref()
    else {
        panic!("expected PlayerPortraitTexture texture");
    };
    let Some(ui_toolkit::frame::WidgetData::Texture(target_texture)) = target.widget_data.as_ref()
    else {
        panic!("expected TargetPortraitTexture texture");
    };

    assert!(matches!(
        &player_texture.source,
        crate::ui::widgets::texture::TextureSource::File(path)
            if path.ends_with("ClassIcon_Paladin.blp")
    ));
    assert!(matches!(
        &target_texture.source,
        crate::ui::widgets::texture::TextureSource::File(path)
            if path.ends_with("INV_Misc_QuestionMark.blp")
    ));
}

#[test]
fn secondary_resource_pips_render_with_active_fill() {
    let reg = unit_frames_registry();

    for index in 0..5 {
        assert!(
            reg.get_by_name(&format!("PlayerSecondaryResourcePip{index}"))
                .is_some(),
            "PlayerSecondaryResourcePip{index} missing"
        );
    }

    let active = reg
        .get(
            reg.get_by_name("PlayerSecondaryResourcePip0")
                .expect("active pip"),
        )
        .expect("active pip frame");
    let inactive = reg
        .get(
            reg.get_by_name("PlayerSecondaryResourcePip4")
                .expect("inactive pip"),
        )
        .expect("inactive pip frame");

    assert_eq!(
        active.background_color,
        Some([1.0, 0.84, 0.28, 0.96]),
        "filled holy power pip color"
    );
    assert_eq!(
        inactive.background_color,
        Some([0.24, 0.19, 0.05, 0.92]),
        "empty holy power pip color"
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
        portrait_texture_file: "/home/osso/Projects/wow/Interface/ICONS/ClassIcon_Paladin.blp"
            .to_string(),
        name: "Player".to_string(),
        level_text: "10".to_string(),
        resting_text: "Resting".to_string(),
        health_text: "100 / 100".to_string(),
        mana_text: "80 / 80".to_string(),
        health_fill_width: PLAYER_HEALTH_BAR_W,
        mana_fill_width: PLAYER_HEALTH_BAR_W,
        secondary_resource: Some(SecondaryResourceEntry {
            kind: SecondaryResourceKindEntry::HolyPower,
            current: 4,
            max: 5,
        }),
        has_mana: true,
        show_combat_icon: true,
        show_resting_icon: true,
        target_buffs: Vec::new(),
        target_debuffs: Vec::new(),
    }
}

fn sample_target_frame_state() -> UnitFrameState {
    UnitFrameState {
        portrait_texture_file: "/home/osso/Projects/wow/Interface/ICONS/INV_Misc_QuestionMark.blp"
            .to_string(),
        name: "Target".to_string(),
        level_text: "12".to_string(),
        resting_text: String::new(),
        health_text: "90 / 90".to_string(),
        mana_text: "60 / 60".to_string(),
        health_fill_width: TARGET_HEALTH_BAR_W,
        mana_fill_width: TARGET_MANA_BAR_W,
        secondary_resource: None,
        has_mana: true,
        show_combat_icon: false,
        show_resting_icon: false,
        target_buffs: vec![TargetAuraIconState {
            icon_fdid: 136078,
            timer_text: "5m".to_string(),
            stacks: 1,
            border_color: "0.85,0.75,0.35,1.0".to_string(),
        }],
        target_debuffs: vec![TargetAuraIconState {
            icon_fdid: 136207,
            timer_text: "4s".to_string(),
            stacks: 3,
            border_color: "0.2,0.6,1.0,1.0".to_string(),
        }],
    }
}

fn unit_frames_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let shared = sample_unit_frames_context();
    sync_unit_frames_registry(&shared, &mut reg);
    reg
}

fn sample_unit_frames_context() -> SharedContext {
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: sample_player_frame_state(),
        target: Some(sample_target_frame_state()),
    });
    shared
}

fn sync_unit_frames_registry(shared: &SharedContext, reg: &mut FrameRegistry) {
    Screen::new(inworld_unit_frames_screen).sync(shared, reg);
    compute_layout(reg);
}

fn rect_by_name(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .expect(name)
}
