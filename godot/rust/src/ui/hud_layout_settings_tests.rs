//! A layout's settings (docs/specs/hud-edit-mode.md "Customisable layout settings") resize
//! the chat frame, the damage meter and the unit frames and restyle unit frame text. A
//! canvas takes them from its SharedContext, where `RegistryModel::sync` mirrors them.

use game_engine_core::ui_layout_data::{
    CHAT_WIDTH_RANGE, FrameSizeSettings, LayoutFont, LayoutSettings, UnitFrameSettings,
};
use game_engine_ui_model::chat_frame::add_system_line;
use game_engine_ui_model::chat_frame_component::{CHAT_MESSAGES, chat_frame_view};
use game_engine_ui_model::damage_meter_component::damage_meter_row_name;
use game_engine_ui_model::damage_meter_data::{DamageMeterRow, MeterType};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::widgets::font_string::{FontStringData, GameFont};

use super::*;

const SKINS: [ActiveSkin; 2] = [ActiveSkin::Modern, ActiveSkin::Forever];

fn sync_settings(hud: &mut [RegistryModel], skin: ActiveSkin, settings: LayoutSettings) {
    for model in hud.iter_mut() {
        model.shared.insert(settings);
    }
    sync(hud, skin);
}

/// `name`'s rect as `(x, y, width, height)`.
fn at(hud: &[RegistryModel], name: &str) -> (f32, f32, f32, f32) {
    let rect = rect(hud, name);
    (rect.x, rect.y, rect.width, rect.height)
}

fn assert_inside(hud: &[RegistryModel], child: &str, parent: &str) {
    let (c, p) = (at(hud, child), at(hud, parent));
    let inside = c.0 >= p.0 - 0.001
        && c.1 >= p.1 - 0.001
        && c.0 + c.2 <= p.0 + p.2 + 0.001
        && c.1 + c.3 <= p.1 + p.3 + 0.001;
    assert!(inside, "{child} {c:?} is not inside {parent} {p:?}");
}

fn text<'a>(hud: &'a [RegistryModel], name: &str) -> &'a FontStringData {
    let frame = hud
        .iter()
        .find_map(|model| model.registry.get(model.registry.get_by_name(name)?))
        .unwrap_or_else(|| panic!("no canvas draws {name}"));
    let Some(WidgetData::FontString(text)) = frame.widget_data.as_ref() else {
        panic!("{name} is not text");
    };
    text
}

fn size(width: u16, height: u16) -> FrameSizeSettings {
    FrameSizeSettings {
        width: Some(width),
        height: Some(height),
    }
}

#[test]
fn chat_size_setting_resizes_the_frame_from_its_corner_and_keeps_its_parts_inside() {
    for skin in SKINS {
        let mut hud = hud();
        sync(&mut hud, skin);
        let preset = rect(&hud, CHAT_FRAME.0);
        let settings = LayoutSettings {
            chat: size(640, 360),
            ..Default::default()
        };
        sync_settings(&mut hud, skin, settings);
        let chat = rect(&hud, CHAT_FRAME.0);
        assert_eq!((chat.width, chat.height), (640.0, 360.0));
        assert!(chat.width > preset.width && chat.height > preset.height);
        // The bottom-left corner is the frame's anchor.
        assert_eq!(chat.x, preset.x);
        assert_eq!(chat.y + chat.height, preset.y + preset.height);
        for part in [
            CHAT_MESSAGES,
            "ChatFrame1EditBox",
            "ChatFrame1ScrollToBottomButton",
        ] {
            assert_inside(&hud, part, CHAT_FRAME.0);
        }
        if skin == ActiveSkin::Forever {
            assert_inside(&hud, CHAT_MESSAGES, CHAT_FLARE_SKIN);
        }
        // Back on the preset the frame has its preset size again.
        sync_settings(&mut hud, skin, LayoutSettings::default());
        assert_eq!(
            at(&hud, CHAT_FRAME.0),
            (preset.x, preset.y, preset.width, preset.height)
        );
    }
}

#[test]
fn chat_width_outside_its_range_is_clamped() {
    let mut hud = hud();
    sync(&mut hud, ActiveSkin::Modern);
    let preset = rect(&hud, CHAT_FRAME.0);
    let settings = LayoutSettings {
        chat: FrameSizeSettings {
            width: Some(5000),
            height: None,
        },
        ..Default::default()
    };
    sync_settings(&mut hud, ActiveSkin::Modern, settings);
    let chat = rect(&hud, CHAT_FRAME.0);
    assert_eq!(chat.width, f32::from(CHAT_WIDTH_RANGE.max));
    // The unset height keeps the preset's.
    assert_eq!(chat.height, preset.height);
}

#[test]
fn chat_lines_wrap_to_the_frame_width_and_fill_its_height() {
    set_data_root();
    let mut model = crate::chat::ChatModel::default();
    for index in 0..40 {
        add_system_line(
            &mut model.log,
            &format!("{index}: the quick brown fox jumps over the lazy dog and runs far away"),
        );
    }
    let view = |size| {
        chat_frame_view(
            &model.state,
            &model.log,
            &model.combat,
            |_| String::new(),
            size,
        )
    };
    let (narrow, wide, tall) = (
        view((250.0, 280.0)),
        view((800.0, 280.0)),
        view((800.0, 600.0)),
    );
    let rows =
        |message: &game_engine_ui_model::chat_frame_component::ChatMessageView| message.rows.len();
    // The newest line needs more rows in a narrow frame than in a wide one.
    assert!(rows(narrow.messages.last().unwrap()) > rows(wide.messages.last().unwrap()));
    // A taller frame shows more of the log.
    assert!(tall.messages.len() > wide.messages.len());
    assert!(wide.messages.len() > narrow.messages.len());
}

fn meter_rows(count: usize) -> DamageMeterView {
    DamageMeterView {
        rows: (0..count)
            .map(|index| DamageMeterRow {
                name_text: format!("{}. Fbmeter{index}", index + 1),
                value_text: "1.2K (40.0)".into(),
                fraction: 1.0 - index as f32 / count as f32,
                color: [0.25, 0.78, 0.92],
                class_id: 8,
                is_local_player: index == 0,
            })
            .collect(),
        ..DamageMeterView::default()
    }
}

fn shown_meter_rows(hud: &[RegistryModel]) -> usize {
    (0..)
        .take_while(|&index| {
            hud[0]
                .registry
                .get_by_name(&damage_meter_row_name(index))
                .is_some()
        })
        .count()
}

/// Distance from `name`'s right edge to the meter window's.
fn right_gap(hud: &[RegistryModel], name: &str) -> f32 {
    let (window, part) = (rect(hud, DAMAGE_METER_ROOT.0), rect(hud, name));
    window.x + window.width - part.x - part.width
}

#[test]
fn damage_meter_size_setting_resizes_the_window_its_rows_and_how_many_fit() {
    set_data_root();
    for skin in SKINS {
        let mut hud = vec![model(meter_rows(30), damage_meter_screen)];
        sync(&mut hud, skin);
        let preset = rect(&hud, DAMAGE_METER_ROOT.0);
        let preset_row = rect(&hud, &damage_meter_row_name(0));
        let preset_rows = shown_meter_rows(&hud);
        let preset_gap = right_gap(&hud, "DamageMeterSettings");
        let settings = LayoutSettings {
            damage_meter: size(600, 400),
            ..Default::default()
        };
        sync_settings(&mut hud, skin, settings);
        let meter = rect(&hud, DAMAGE_METER_ROOT.0);
        assert_eq!((meter.width, meter.height), (600.0, 400.0));
        assert!(rect(&hud, &damage_meter_row_name(0)).width > preset_row.width);
        let rows = shown_meter_rows(&hud);
        assert!(rows > preset_rows, "{skin:?}: {rows} rows");
        for index in 0..rows {
            assert_inside(&hud, &damage_meter_row_name(index), DAMAGE_METER_ROOT.0);
            assert_inside(
                &hud,
                &format!("{}Value", damage_meter_row_name(index)),
                &damage_meter_row_name(index),
            );
        }
        // The header buttons keep their distance from the window's right edge.
        assert!((right_gap(&hud, "DamageMeterSettings") - preset_gap).abs() < 0.001);
        // A smaller window than the preset shows fewer rows, all still inside it.
        let settings = LayoutSettings {
            damage_meter: size(200, 120),
            ..Default::default()
        };
        sync_settings(&mut hud, skin, settings);
        let small = rect(&hud, DAMAGE_METER_ROOT.0);
        assert!(small.width < preset.width && small.height <= preset.height);
        let rows = shown_meter_rows(&hud);
        assert!(rows >= 1 && rows <= preset_rows, "{skin:?}: {rows} rows");
        for index in 0..rows {
            assert_inside(&hud, &damage_meter_row_name(index), DAMAGE_METER_ROOT.0);
        }
    }
}

/// Left and right edges of `name`.
fn span(hud: &[RegistryModel], name: &str) -> (f32, f32) {
    let rect = rect(hud, name);
    (rect.x, rect.x + rect.width)
}

#[test]
fn damage_meter_type_menu_and_clickable_death_rows_fit_the_window_at_any_size() {
    set_data_root();
    for skin in SKINS {
        // Modern opens the type menu alone; Forever's one menu lists types and sessions.
        let view = DamageMeterView {
            meter_type: MeterType::Deaths,
            type_menu_open: skin == ActiveSkin::Modern,
            menu_open: skin == ActiveSkin::Forever,
            ..meter_rows(30)
        };
        let mut hud = vec![model(view, damage_meter_screen)];
        for (width, height) in [(600, 400), (200, 120)] {
            let settings = LayoutSettings {
                damage_meter: size(width, height),
                ..Default::default()
            };
            sync_settings(&mut hud, skin, settings);
            let window = span(&hud, DAMAGE_METER_ROOT.0);
            let rows = shown_meter_rows(&hud);
            assert!(rows >= 1, "{skin:?} {width}x{height}");
            for index in 0..rows {
                let row = damage_meter_row_name(index);
                assert_inside(&hud, &row, DAMAGE_METER_ROOT.0);
                // The click target covers its row.
                assert_eq!(at(&hud, &format!("{row}Button")), at(&hud, &row));
            }
            let menu = span(&hud, "DamageMeterTypeMenu");
            assert!(
                menu.0 >= window.0 - 0.001 && menu.1 <= window.1 + 0.001,
                "{skin:?} {width}x{height}: type menu {menu:?} outside {window:?}"
            );
            if skin == ActiveSkin::Forever {
                assert_inside(&hud, "DamageMeterOtherTypeName", DAMAGE_METER_ROOT.0);
                let (types, sessions) = (
                    rect(&hud, "DamageMeterTypeMenu"),
                    rect(&hud, "DamageMeterSessionMenu"),
                );
                let apart =
                    types.x + types.width <= sessions.x || sessions.y + sessions.height <= types.y;
                assert!(apart, "{width}x{height}: the menus overlap");
            }
        }
        // In the roomy window the whole menu is inside it.
        let settings = LayoutSettings {
            damage_meter: size(600, 400),
            ..Default::default()
        };
        sync_settings(&mut hud, skin, settings);
        assert_inside(&hud, "DamageMeterTypeMenu", DAMAGE_METER_ROOT.0);
    }
}

fn frame_size(percent: u16) -> UnitFrameSettings {
    UnitFrameSettings {
        frame_size: Some(percent),
        ..Default::default()
    }
}

#[test]
fn unit_frame_size_setting_scales_the_frame_and_its_parts_about_its_anchor() {
    for skin in SKINS {
        let mut hud = vec![model(unit_frames(), inworld_unit_frames_screen)];
        sync(&mut hud, skin);
        let (player, health, pet) = (
            rect(&hud, "PlayerFrame"),
            rect(&hud, "PlayerHealthBar"),
            rect(&hud, "PetFrame"),
        );
        let (target, focus) = (at(&hud, "TargetFrame"), at(&hud, "FocusFrame"));
        let name_size = text(&hud, "PlayerName").font_size;
        let settings = LayoutSettings {
            player_frame: frame_size(150),
            ..Default::default()
        };
        sync_settings(&mut hud, skin, settings);
        let big = rect(&hud, "PlayerFrame");
        assert!((big.width - player.width * 1.5).abs() < 0.001);
        assert!((big.height - player.height * 1.5).abs() < 0.001);
        let big_health = rect(&hud, "PlayerHealthBar");
        assert!((big_health.width - health.width * 1.5).abs() < 0.001);
        assert_inside(&hud, "PlayerHealthBar", "PlayerFrame");
        assert_inside(&hud, "PlayerName", "PlayerFrame");
        assert!((text(&hud, "PlayerName").font_size - name_size * 1.5).abs() < 0.001);
        // Modern anchors the frame's bottom-right corner, Forever its centre.
        match skin {
            ActiveSkin::Modern => assert_eq!(
                (big.x + big.width, big.y + big.height),
                (player.x + player.width, player.y + player.height)
            ),
            ActiveSkin::Forever => assert!(
                (big.x + big.width / 2.0 - player.x - player.width / 2.0).abs() < 0.001
                    && (big.y + big.height / 2.0 - player.y - player.height / 2.0).abs() < 0.001
            ),
        }
        // The other systems keep their size; the pet keeps its own size and still clears
        // the player frame it hangs from as it did.
        assert_eq!(at(&hud, "TargetFrame"), target);
        assert_eq!(at(&hud, "FocusFrame"), focus);
        let big_pet = rect(&hud, "PetFrame");
        assert_eq!((big_pet.width, big_pet.height), (pet.width, pet.height));
        assert!(!intersects(&big, &rect(&hud, "TargetFrame")));
        if !intersects(&player, &pet) {
            assert!(
                !intersects(&big, &big_pet),
                "{skin:?}: pet under the player"
            );
        }
    }
}

#[test]
fn target_of_target_and_the_target_cast_bar_follow_the_target_frame_size() {
    set_data_root();
    for skin in SKINS {
        let mut state = unit_frames();
        state.target_cast = Some(CastingBarState {
            visible: true,
            spell_name: "Frostbolt".into(),
            icon_fdid: Some(135846),
            timer_text: "1.5".into(),
            progress: 0.25,
            ..Default::default()
        });
        let mut hud = vec![model(state, inworld_unit_frames_screen)];
        sync(&mut hud, skin);
        let (target, tot, cast) = (
            rect(&hud, "TargetFrame"),
            rect(&hud, "TargetOfTargetFrame"),
            rect(&hud, "TargetFrameSpellBar"),
        );
        let settings = LayoutSettings {
            target_frame: frame_size(200),
            ..Default::default()
        };
        sync_settings(&mut hud, skin, settings);
        let (big, big_tot, big_cast) = (
            rect(&hud, "TargetFrame"),
            rect(&hud, "TargetOfTargetFrame"),
            rect(&hud, "TargetFrameSpellBar"),
        );
        assert!((big.width - target.width * 2.0).abs() < 0.001);
        assert!((big_tot.width - tot.width * 2.0).abs() < 0.001);
        assert!((big_tot.height - tot.height * 2.0).abs() < 0.001);
        assert!((big_cast.width - cast.width * 2.0).abs() < 0.001);
        // Target of target stays beside the target, the cast bar the same side of it.
        assert_eq!(intersects(&big, &big_tot), intersects(&target, &tot));
        assert!(big_tot.x >= big.x + big.width - 0.001 || intersects(&target, &tot));
        assert_eq!(
            big_cast.y >= big.y + big.height - 0.001,
            cast.y >= target.y + target.height - 0.001
        );
        assert!(
            big_cast.x >= big.x - 0.001 && big_cast.x + big_cast.width <= big.x + big.width + 0.001
        );
    }
}

#[test]
fn unit_frame_font_settings_restyle_that_frame_text_only() {
    for skin in SKINS {
        let mut hud = vec![model(unit_frames(), inworld_unit_frames_screen)];
        sync(&mut hud, skin);
        let player_name = text(&hud, "PlayerName").clone();
        let target_name = text(&hud, "TargetName").clone();
        let player = at(&hud, "PlayerFrame");
        assert_eq!(player_name.font, GameFont::FrizQuadrata);
        let settings = LayoutSettings {
            player_frame: UnitFrameSettings {
                frame_size: None,
                font: Some(LayoutFont::ArialNarrow),
                text_size: Some(150),
            },
            ..Default::default()
        };
        sync_settings(&mut hud, skin, settings);
        for name in ["PlayerName", "PlayerLevelText"] {
            assert_eq!(
                text(&hud, name).font,
                GameFont::ArialNarrow,
                "{skin:?} {name}"
            );
        }
        assert!((text(&hud, "PlayerName").font_size - player_name.font_size * 1.5).abs() < 0.001);
        // Text size alone leaves the frame its size, and the target its text.
        assert_eq!(at(&hud, "PlayerFrame"), player);
        assert_eq!(*text(&hud, "TargetName"), target_name);
        // A smaller text size shrinks the text.
        let settings = LayoutSettings {
            player_frame: UnitFrameSettings {
                text_size: Some(50),
                ..Default::default()
            },
            ..Default::default()
        };
        sync_settings(&mut hud, skin, settings);
        assert!(text(&hud, "PlayerName").font_size < player_name.font_size);
        assert_eq!(text(&hud, "PlayerName").font, GameFont::FrizQuadrata);
    }
}
