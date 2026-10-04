//! A layout's settings (docs/specs/hud-edit-mode.md "Customisable layout settings") resize
//! the chat frame, the damage meter and the unit frames and restyle unit frame text. A
//! canvas takes them from its SharedContext, where `RegistryModel::sync` mirrors them.

use game_engine_core::ui_layout_data::{
    self, CHAT_WIDTH_RANGE, FrameSizeSettings, LayoutFont, LayoutSettings, LayoutSkin,
    UnitFrameSettings,
};
use game_engine_ui_model::chat_frame::add_system_line;
use game_engine_ui_model::chat_frame_component::{CHAT_MESSAGES, chat_frame_view};
use game_engine_ui_model::damage_meter_component::damage_meter_row_name;
use game_engine_ui_model::damage_meter_data::{DamageMeterRow, MeterType};
use game_engine_ui_model::options_menu_component::{LayoutOptionsView, LayoutSystem};
use game_engine_ui_model::options_menu_data::{
    LayoutAction, SliderField, apply_layout_action, apply_layout_slider, parse_layout_action,
    parse_slider_action,
};
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

const CHARACTER: u64 = 17;

/// The menu's layout after the host saved or selected `layout`, as
/// `GameClient::ui_layout_options` builds it.
fn shown(
    path: &std::path::Path,
    layout: ui_layout_data::ActiveLayout,
    system: LayoutSystem,
) -> LayoutOptionsView {
    LayoutOptionsView {
        active: layout.name,
        names: ui_layout_data::layout_names(path).unwrap(),
        skin: layout.skin,
        settings: layout.settings,
        system,
    }
}

/// A slider event of the Options HUD page: store the value, save the layout, draw it.
fn slide(
    path: &std::path::Path,
    hud: &mut [RegistryModel],
    layout: &mut LayoutOptionsView,
    (action, value): (&str, f32),
) {
    let Some(SliderField::Layout(slider)) = parse_slider_action(action) else {
        panic!("{action} is not a layout slider");
    };
    apply_layout_slider(slider, value, layout);
    save_and_draw(path, hud, layout);
}

/// A click of the Options HUD page's layout controls.
fn click(
    path: &std::path::Path,
    hud: &mut [RegistryModel],
    layout: &mut LayoutOptionsView,
    action: &str,
) {
    let action = parse_layout_action(action).unwrap_or_else(|| panic!("{action}"));
    apply_layout_action(action, layout);
    if let LayoutAction::Select(index) = action {
        let name = layout.names[index].clone();
        let selected = ui_layout_data::set_active_layout(path, CHARACTER, &name).unwrap();
        *layout = shown(path, selected, layout.system);
        draw(hud, layout);
    } else {
        save_and_draw(path, hud, layout);
    }
}

fn save_and_draw(
    path: &std::path::Path,
    hud: &mut [RegistryModel],
    layout: &mut LayoutOptionsView,
) {
    let saved = ui_layout_data::save_layout_settings(path, CHARACTER, layout.settings).unwrap();
    *layout = shown(path, saved, layout.system);
    draw(hud, layout);
}

fn draw(hud: &mut [RegistryModel], layout: &LayoutOptionsView) {
    let skin = match layout.skin {
        LayoutSkin::Modern => ActiveSkin::Modern,
        LayoutSkin::Forever => ActiveSkin::Forever,
    };
    sync_settings(hud, skin, layout.settings);
}

/// What the settings change on the HUD: frame sizes and the player name's text.
#[derive(Debug, PartialEq)]
struct Drawn {
    player: (f32, f32),
    name_size: f32,
    name_font: GameFont,
    chat: (f32, f32),
    meter: (f32, f32),
}

fn drawn(hud: &[RegistryModel]) -> Drawn {
    let size = |name| {
        let rect = rect(hud, name);
        (rect.width, rect.height)
    };
    let name = text(hud, "PlayerName");
    Drawn {
        player: size("PlayerFrame"),
        name_size: name.font_size,
        name_font: name.font,
        chat: size(CHAT_FRAME.0),
        meter: size(DAMAGE_METER_ROOT.0),
    }
}

/// The Options HUD controls end to end under both presets: each control's action stores
/// its setting, the first change of a preset saves "Layout 1" and switches to it, the HUD
/// redraws larger, the file brings the values back, the Layout dropdown switches between
/// the preset and the player layout, and "Reset to Preset" clears the settings.
#[test]
fn options_layout_controls_save_a_player_layout_and_redraw_the_hud() {
    for (preset, skin) in ui_layout_data::SYSTEM_PRESETS {
        let path = std::env::temp_dir().join(format!(
            "ui-layout-options-{preset}-{}.ron",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let active = ui_layout_data::set_active_layout(&path, CHARACTER, preset).unwrap();
        let mut layout = shown(&path, active, LayoutSystem::PlayerFrame);
        assert_eq!(layout.names, ["Modern", "Forever"]);
        let mut hud = hud();
        draw(&mut hud, &layout);
        let preset_hud = drawn(&hud);
        assert_eq!(preset_hud.name_font, GameFont::FrizQuadrata);

        // Frame Size 150 %: the preset is left alone, "Layout 1" is saved and active.
        slide(
            &path,
            &mut hud,
            &mut layout,
            ("options_slider:layout_frame_size", 150.0),
        );
        assert_eq!(layout.active, "Layout 1");
        assert_eq!(layout.skin, skin);
        assert_eq!(layout.names, ["Modern", "Forever", "Layout 1"]);
        assert_eq!(layout.settings.player_frame.frame_size, Some(150));
        let sized = drawn(&hud);
        assert!(sized.player.0 > preset_hud.player.0 && sized.player.1 > preset_hud.player.1);
        assert!(sized.name_size > preset_hud.name_size);

        // Text Size 130 % and Arial Narrow change the text, not the frame.
        slide(
            &path,
            &mut hud,
            &mut layout,
            ("options_slider:layout_text_size", 130.0),
        );
        click(&path, &mut hud, &mut layout, "options_toggle:layout_font:1");
        let styled = drawn(&hud);
        assert!(styled.name_size > sized.name_size);
        assert_eq!(styled.name_font, GameFont::ArialNarrow);
        assert_eq!(styled.player, sized.player);

        // Chat 600x300 and meter 550x300, each larger than either preset's.
        for (action, value) in [
            ("options_slider:layout_chat_width", 600.0),
            ("options_slider:layout_chat_height", 300.0),
            ("options_slider:layout_meter_width", 550.0),
            ("options_slider:layout_meter_height", 300.0),
        ] {
            slide(&path, &mut hud, &mut layout, (action, value));
        }
        let custom = drawn(&hud);
        assert_eq!(custom.chat, (600.0, 300.0));
        assert_eq!(custom.meter, (550.0, 300.0));
        assert!(custom.chat.0 > preset_hud.chat.0 && custom.chat.1 > preset_hud.chat.1);
        assert!(custom.meter.0 > preset_hud.meter.0 && custom.meter.1 > preset_hud.meter.1);
        // Every change after the first went to the same player layout.
        assert_eq!(layout.active, "Layout 1");
        assert_eq!(layout.names.len(), 3);

        // Restart: the file gives the character the same layout and the same HUD.
        let reloaded = ui_layout_data::active_layout(&path, CHARACTER).unwrap();
        assert_eq!(reloaded.name, "Layout 1");
        assert_eq!(reloaded.settings, layout.settings);
        let mut restarted = self::hud();
        draw(
            &mut restarted,
            &shown(&path, reloaded, LayoutSystem::PlayerFrame),
        );
        assert_eq!(drawn(&restarted), custom);

        // The Layout dropdown: back to the preset and its values, then to the layout's.
        let preset_index = layout.names.iter().position(|name| name == preset).unwrap();
        click(
            &path,
            &mut hud,
            &mut layout,
            &format!("options_toggle:ui_layout:{preset_index}"),
        );
        assert_eq!(layout.active, preset);
        assert_eq!(layout.settings, LayoutSettings::default());
        assert_eq!(drawn(&hud), preset_hud);
        click(&path, &mut hud, &mut layout, "options_toggle:ui_layout:2");
        assert_eq!(layout.active, "Layout 1");
        assert_eq!(drawn(&hud), custom);

        // Reset to Preset clears the layout's settings; it stays the active layout.
        click(
            &path,
            &mut hud,
            &mut layout,
            "options_reset_layout_settings",
        );
        assert_eq!(layout.active, "Layout 1");
        assert_eq!(layout.settings, LayoutSettings::default());
        assert_eq!(drawn(&hud), preset_hud);
        assert_eq!(
            ui_layout_data::active_layout(&path, CHARACTER)
                .unwrap()
                .settings,
            LayoutSettings::default()
        );
        std::fs::remove_file(&path).unwrap();
    }
}

/// The Options HUD page with `system`'s layout settings shown.
fn options_hud_view(
    system: LayoutSystem,
) -> game_engine_ui_model::game_menu_component::GameMenuViewModel {
    use game_engine_core::client_options_data::{
        CameraOptionsFile, GraphicsOptionsFile, HudOptionsFile, SoundOptionsFile,
    };
    use game_engine_ui_model::options_menu_data as policy;
    let graphics = policy::graphics_draft_from_file(&GraphicsOptionsFile::default());
    let sound = policy::sound_draft_from_file(&SoundOptionsFile::default());
    let camera = policy::camera_draft_from_file(&CameraOptionsFile::default());
    let hud = policy::hud_draft_from_file(&HudOptionsFile::default());
    policy::build_view_model(&policy::OptionsModel {
        logged_in: true,
        view: game_engine_ui_model::game_menu_component::GameMenuView::Options,
        category: game_engine_ui_model::options_menu_component::OptionsCategory::Hud,
        modal_position: [0.0, 0.0],
        draft_graphics: graphics.clone(),
        committed_graphics: graphics,
        draft_sound: sound.clone(),
        committed_sound: sound,
        draft_camera: camera.clone(),
        committed_camera: camera,
        draft_hud: hud.clone(),
        committed_hud: hud,
        draft_bindings: Default::default(),
        committed_bindings: Default::default(),
        binding_section: game_engine_core::input_bindings_data::BindingSection::Movement,
        binding_capture: policy::BindingCapture::None,
        layout: LayoutOptionsView {
            system,
            ..Default::default()
        },
    })
}

/// Switching the shown system keeps its controls between the "Layout Settings" selector
/// and "Reset to Preset", as on a freshly opened page.
#[test]
fn switching_layout_systems_keeps_their_controls_under_the_selector() {
    use game_engine_ui_model::game_menu_component::game_menu_screen;
    let mut menu = model(
        options_hud_view(LayoutSystem::PlayerFrame),
        game_menu_screen,
    );
    for system in [
        LayoutSystem::ChatFrame,
        LayoutSystem::PlayerFrame,
        LayoutSystem::DamageMeter,
        LayoutSystem::TargetFrame,
    ] {
        menu.shared.insert(options_hud_view(system));
        menu.screen.sync(&menu.shared, &mut menu.registry);
        let menu = std::slice::from_ref(&menu);
        let first_control = match system {
            LayoutSystem::ChatFrame | LayoutSystem::DamageMeter => "LayoutFrameSize",
            _ => "LayoutUnitFrameSizes",
        };
        let selector = rect(menu, "ChoiceRowlayout_system");
        let control = rect(menu, first_control);
        let reset = rect(menu, "ActionRowreset_layout_settings");
        let minimap = rect(menu, "ToggleRowshow_minimap");
        assert!(
            selector.y < control.y && control.y < reset.y && reset.y < minimap.y,
            "{system:?}: selector {} control {} reset {} minimap {}",
            selector.y,
            control.y,
            reset.y,
            minimap.y
        );
    }
}
