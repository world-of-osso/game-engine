//! Reference chrome and a byte-for-byte Modern tree captured at c699d4e7.
#[path = "fixtures/modern_chat_meter.rs"]
mod baseline;
use std::fmt::Write;
use std::path::PathBuf;

use game_engine_core::ui_layout_data::{FrameSizeSettings, LayoutSettings};
use game_engine_ui_model::chat_frame::ChatTab;
use game_engine_ui_model::chat_frame_component::{ChatFrameView, chat_frame_screen};
use game_engine_ui_model::damage_meter_component::damage_meter_screen;
use game_engine_ui_model::damage_meter_data::{
    DamageMeterRow, DamageMeterView, DamageMeterWindow, MeterSessionType, MeterType,
};
use game_engine_ui_model::flare_panel::{FLARE_BRONZE_PANEL_STYLE, flare_bronze_style};
use shared::protocol::{DamageMeterSession, DamageMeterSnapshot, DamageMeterSource};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::{FontStringData, JustifyH};
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

fn canvas<T: 'static>(
    skin: ActiveSkin,
    view: T,
    build: fn(&SharedContext) -> Element,
) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    registry.register_panel_style(
        FLARE_BRONZE_PANEL_STYLE,
        flare_bronze_style(TextureSource::FileDataId(137_057)),
    );
    Screen::new(build).sync(&shared, &mut registry);
    registry
}

fn meter_view() -> DamageMeterView {
    let session = DamageMeterSession {
        session_id: 1,
        duration_secs: 20.0,
        active: true,
        total_amount: 2400,
        sources: vec![
            DamageMeterSource {
                unit: 42,
                name: "Fbmage".into(),
                class_id: 8,
                is_local_player: true,
                total_amount: 2400,
                amount_per_second: 120.0,
                spells: vec![],
            },
            DamageMeterSource {
                unit: 43,
                name: "Fbwarrior".into(),
                class_id: 1,
                is_local_player: false,
                total_amount: 1200,
                amount_per_second: 60.0,
                spells: vec![],
            },
        ],
    };
    DamageMeterWindow {
        snapshot: Some(DamageMeterSnapshot {
            current: Some(session.clone()),
            overall: session,
        }),
        session: MeterSessionType::Current,
        menu_open: true,
        ..Default::default()
    }
    .view(true, 45.0)
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("no {name}")),
        )
        .unwrap()
}

fn rect(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let f = frame(registry, name);
    let px = |v| match v {
        Val::Px(n) => n,
        other => panic!("{name}: {other:?}"),
    };
    let (Dimension::Fixed(w), Dimension::Fixed(h)) = (f.width, f.height) else {
        panic!("{name}: size")
    };
    (px(f.position.left), px(f.position.top), w, h)
}

fn font<'a>(registry: &'a FrameRegistry, name: &str) -> &'a FontStringData {
    let Some(WidgetData::FontString(text)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name}: text")
    };
    text
}

fn texture<'a>(registry: &'a FrameRegistry, name: &str) -> &'a TextureData {
    let Some(WidgetData::Texture(tex)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name}: texture")
    };
    tex
}

fn dump(registry: &FrameRegistry, name: &str, out: &mut String) {
    let f = frame(registry, name);
    writeln!(out, "{:?} {:?} size={:?},{:?} pos={:?} translate={:?} margin={:?} hidden={} alpha={} scale={} strata={:?} level={} layer={:?} bg={:?} slice={:?} border={:?} style={:?} mouse={} click={:?} data={:?}",
        f.name, f.widget_type, f.width, f.height, f.position, f.translation, f.margin,
        f.hidden, f.alpha, f.scale, f.strata, f.frame_level, f.draw_layer, f.background_color,
        f.nine_slice, f.border, f.panel_style, f.mouse_enabled, f.onclick, f.widget_data).unwrap();
    for child in &f.children {
        dump(
            registry,
            registry.get(*child).unwrap().name.as_deref().unwrap(),
            out,
        );
    }
}

#[test]
fn modern_chat_and_meter_are_byte_identical_to_base() {
    let mut tree = String::new();
    for tab in ChatTab::ALL {
        let view = ChatFrameView {
            tab,
            input_open: true,
            scrolled_up: true,
            flashing: vec![ChatTab::Whispers],
            ..Default::default()
        };
        dump(
            &canvas(ActiveSkin::Modern, view, chat_frame_screen),
            "ChatFrame1",
            &mut tree,
        );
    }
    dump(
        &canvas(ActiveSkin::Modern, meter_view(), damage_meter_screen),
        "DamageMeter",
        &mut tree,
    );
    if baseline::MODERN_TREE.is_empty() || std::env::var_os("CAPTURE_FLAREPANELS_BASE").is_some() {
        println!("<<<FLAREPANELS_BASE\n{tree}FLAREPANELS_BASE>>>");
    }
    assert_eq!(tree.as_bytes(), baseline::MODERN_TREE.as_bytes());
}

#[test]
fn forever_meter_header_has_text_tabs_and_bronze_icons() {
    let registry = canvas(ActiveSkin::Forever, meter_view(), damage_meter_screen);
    assert_eq!(
        rect(&registry, "DamageMeterFlareHeader"),
        (-2.0, -2.0, 454.0, 24.0)
    );
    assert_eq!(
        rect(&registry, "DamageMeterFlareSeparator"),
        (1.0, 22.0, 448.0, 1.0)
    );
    assert_eq!(
        frame(&registry, "DamageMeterFlareSeparator").background_color,
        Some([0.65, 0.49, 0.27, 1.0])
    );
    assert_eq!(
        rect(&registry, "DamageMeterTypeName"),
        (15.0, 7.0, 90.0, 12.0)
    );
    assert_eq!(font(&registry, "DamageMeterTypeName").text, "DPS");
    assert_eq!(
        font(&registry, "DamageMeterTypeName").color,
        [0.80, 0.60, 0.34, 1.0]
    );
    assert_eq!(font(&registry, "DamageMeterHpsTabName").text, "HPS");
    assert_eq!(
        font(&registry, "DamageMeterHpsTabName").color,
        [0.56, 0.51, 0.46, 1.0]
    );
    assert!(registry.get_by_name("DamageMeterThreatTab").is_none());
    assert_eq!(
        rect(&registry, "DamageMeterSessionDropdown"),
        (398.5, -1.0, 22.0, 22.0)
    );
    assert_eq!(
        rect(&registry, "DamageMeterSettings"),
        (419.5, -1.0, 22.0, 22.0)
    );
    for (index, left, top, height) in [(0, 0.0, 7.7, 5.5), (1, 5.5, 4.4, 8.8), (2, 11.0, 0.0, 13.2)]
    {
        let name = format!("DamageMeterChartColumn{index}");
        assert_eq!(rect(&registry, &name), (left, top, 2.2, height));
        assert_eq!(
            frame(&registry, &name).background_color,
            Some([0.61, 0.48, 0.29, 1.0])
        );
    }
    assert_eq!(
        texture(&registry, "DamageMeterSettingsIcon").vertex_color,
        [0.61, 0.48, 0.29, 1.0]
    );
    for removed in [
        "DamageMeterSessionTimer",
        "DamageMeterTypeArrow",
        "DamageMeterMinimize",
        "DamageMeterSessionDropdownBackground",
    ] {
        assert!(registry.get_by_name(removed).is_none(), "{removed}");
    }
    let panel = frame(&registry, "DamageMeterFlareSkin");
    assert_eq!(
        rect(&registry, "DamageMeterFlareSkin"),
        (-2.0, -2.0, 454.0, 218.0)
    );
    assert_eq!(panel.panel_style.as_deref(), Some("flare_bronze"));
    assert_eq!(
        panel.nine_slice.as_ref().unwrap().bg_color,
        [1.0, 1.0, 1.0, 0.6]
    );
}

#[test]
fn forever_meter_rows_have_class_icons_gradient_borders_and_shadowed_text() {
    let registry = canvas(ActiveSkin::Forever, meter_view(), damage_meter_screen);
    assert_ne!(
        texture(&registry, "DamageMeterEntry1Icon").source,
        texture(&registry, "DamageMeterEntry2Icon").source,
        "a mage and a warrior show different class icons"
    );
    assert_eq!(
        frame(&registry, "DamageMeterEntry1Bar").background_color,
        Some([0.1, 0.1, 0.1, 0.9])
    );
    // Class-coloured fill, as long as damage over the top damage.
    assert_eq!(
        texture(&registry, "DamageMeterEntry1StatusBar").vertex_color,
        [0.25, 0.78, 0.92, 1.0]
    );
    let fill = |name: &str| rect(&registry, name).2;
    assert_eq!(
        fill("DamageMeterEntry2StatusBar") * 2.0,
        fill("DamageMeterEntry1StatusBar")
    );
    let gradient = texture(&registry, "DamageMeterEntry1Gradient");
    assert_eq!(gradient.tex_coords, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(gradient.vertex_color, [0.0, 0.0, 0.0, 0.6]);
    assert_eq!(
        texture(&registry, "DamageMeterEntry1BarBorderTopLeft").vertex_color,
        [0.65, 0.49, 0.27, 1.0]
    );
    for (name, label, justify) in [
        ("DamageMeterEntry1Name", "1. Fbmage", JustifyH::Left),
        ("DamageMeterEntry1Value", "2,400 (120)", JustifyH::Right),
    ] {
        let text = font(&registry, name);
        assert_eq!(text.text, label);
        assert_eq!(text.color, [1.0; 4]);
        assert_eq!(text.shadow_color, Some([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(text.shadow_offset, [1.0, -1.0]);
        assert_eq!(text.justify_h, justify);
    }
    // The fill and its gradient are drawn before the text (text above the fill).
    let bar = &frame(&registry, "DamageMeterEntry1Bar").children;
    let order = |name: &str| {
        let id = registry.get_by_name(name).unwrap();
        bar.iter().position(|child| *child == id).unwrap()
    };
    assert!(order("DamageMeterEntry1Gradient") < order("DamageMeterEntry1Name"));
}

/// `(left, top, right, bottom)` of `name` inside its parent.
fn edges(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = rect(registry, name);
    (x, y, x + w, y + h)
}

fn window_size(registry: &FrameRegistry) -> (f32, f32) {
    let root = frame(registry, "DamageMeter");
    let (Dimension::Fixed(w), Dimension::Fixed(h)) = (root.width, root.height) else {
        panic!("DamageMeter: size")
    };
    (w, h)
}

fn shown_rows(registry: &FrameRegistry) -> usize {
    (1..)
        .take_while(|index| {
            registry
                .get_by_name(&format!("DamageMeterEntry{index}"))
                .is_some()
        })
        .count()
}

/// `count` ranked rows of `meter_type` (a recap's lines when `recap_open`).
fn many_rows(count: usize, meter_type: MeterType, recap_open: bool) -> DamageMeterView {
    DamageMeterView {
        rows: (0..count)
            .map(|index| DamageMeterRow {
                name_text: format!("{}. Fbsylvanas{index}", index + 1),
                value_text: "123.4K (1,234)".into(),
                fraction: 1.0 - index as f32 / count as f32,
                color: [0.67, 0.83, 0.45],
                class_id: if recap_open { 0 } else { 3 },
                is_local_player: index == 0,
            })
            .collect(),
        ..typed_view(meter_type, recap_open)
    }
}

fn sized_canvas(view: DamageMeterView, height: Option<u16>) -> FrameRegistry {
    let settings = LayoutSettings {
        damage_meter: FrameSizeSettings {
            width: None,
            height,
        },
        ..Default::default()
    };
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Forever);
    shared.insert(settings);
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    registry.register_panel_style(
        FLARE_BRONZE_PANEL_STYLE,
        flare_bronze_style(TextureSource::FileDataId(137_057)),
    );
    Screen::new(damage_meter_screen).sync(&shared, &mut registry);
    registry
}

/// Rows, their icons, bars and text keep their places relative to each other, and the
/// window shows as many rows as fit its height: the reference (FlareUI over Retail's
/// meter, 450x214) shows five; a taller layout setting shows more.
#[test]
fn forever_meter_rows_fit_the_window_height_without_overlapping() {
    let views = [
        many_rows(12, MeterType::DamageDone, false),
        many_rows(12, MeterType::Deaths, false),
        many_rows(12, MeterType::Deaths, true),
    ];
    for view in views {
        for (height, expected) in [(None, 5), (Some(320), 8)] {
            let registry = sized_canvas(view.clone(), height);
            let (window_w, window_h) = window_size(&registry);
            let rows = shown_rows(&registry);
            assert_eq!(rows, expected, "{:?} {height:?}", view.meter_type);
            let row = |index: usize| edges(&registry, &format!("DamageMeterEntry{index}"));
            let gap = row(2).1 - row(1).3;
            assert!(gap > 0.0, "rows touch or overlap: gap {gap}");
            for index in 1..=rows {
                let (left, top, right, bottom) = row(index);
                assert!(left >= 0.0 && right <= window_w, "row {index} width");
                assert!(top >= 0.0 && bottom <= window_h, "row {index} height");
                if index > 1 {
                    assert!(
                        (top - row(index - 1).3 - gap).abs() < 0.001,
                        "row {index} gap"
                    );
                }
            }
            // One more row at the same pitch would leave the window.
            let (_, last_top, _, last_bottom) = row(rows);
            assert!(last_bottom + gap + (last_bottom - last_top) > window_h);
            check_row_parts(&registry, view.recap_open);
        }
    }
}

fn check_row_parts(registry: &FrameRegistry, recap: bool) {
    let row = edges(registry, "DamageMeterEntry1");
    let row_h = row.3 - row.1;
    let bar = edges(registry, "DamageMeterEntry1Bar");
    assert!(bar.1 >= 0.0 && bar.3 <= row_h, "bar inside its row");
    if !recap {
        let icon = edges(registry, "DamageMeterEntry1Icon");
        let (icon_w, icon_h) = (icon.2 - icon.0, icon.3 - icon.1);
        assert_eq!(icon_w, icon_h, "square icon");
        assert!(icon.1 >= 0.0 && icon.3 <= row_h, "icon inside its row");
        assert!(icon.0 >= 0.0 && icon.2 < bar.0, "icon left of its bar");
        // A row's icon is most of its height (reference: ~24 of ~30).
        assert!(icon_h >= 0.75 * row_h, "icon {icon_h} in a {row_h} row");
    }
    let bar_w = bar.2 - bar.0;
    let bar_h = bar.3 - bar.1;
    let name = edges(registry, "DamageMeterEntry1Name");
    let value = edges(registry, "DamageMeterEntry1Value");
    for (label, text) in [("Name", name), ("Value", value)] {
        assert!(text.0 >= 0.0 && text.2 <= bar_w, "{label} inside its bar");
        assert!(text.1 >= 0.0 && text.3 <= bar_h, "{label} inside its bar");
        let size = font(registry, &format!("DamageMeterEntry1{label}")).font_size;
        assert!(
            size <= text.3 - text.1,
            "{label} font {size} taller than its box"
        );
        // Text fills most of the bar's height, as in the reference.
        assert!(size >= 0.5 * bar_h, "{label} font {size} in a {bar_h} bar");
    }
    assert!(name.2 <= value.0, "name and value do not overlap");
}

#[test]
fn forever_chat_has_plain_text_tabs_separator_and_four_header_icons() {
    let registry = canvas(
        ActiveSkin::Forever,
        ChatFrameView::default(),
        chat_frame_screen,
    );
    assert_eq!(
        rect(&registry, "ChatFrame1FlareSkin"),
        (24.0, -7.0, 450.0, 214.0)
    );
    assert_eq!(
        rect(&registry, "ChatFrame1FlareHeader"),
        (24.0, -7.0, 450.0, 24.0)
    );
    assert_eq!(
        rect(&registry, "ChatFrame1FlareSeparator"),
        (27.0, 17.0, 444.0, 1.0)
    );
    for index in 0..3 {
        let name = format!("ChatFrame1TabsTab{index}");
        assert_eq!(frame(&registry, &name).height, Dimension::Fixed(24.0));
        assert_eq!(
            frame(&registry, &name).onclick.as_deref(),
            Some(ChatTab::ALL[index].action())
        );
        let label = font(&registry, &format!("{name}Text"));
        assert_eq!(label.font_size, 12.0);
        assert_eq!(
            label.color,
            if index == 0 {
                [0.80, 0.60, 0.34, 1.0]
            } else {
                [0.56, 0.51, 0.46, 1.0]
            }
        );
        assert!(registry.get_by_name(&format!("{name}Left")).is_none());
    }
    for (name, x) in [
        ("Channel", 321.0),
        ("Menu", 356.0),
        ("Social", 391.0),
        ("Volume", 426.0),
    ] {
        let name = format!("ChatFrame1Flare{name}");
        assert_eq!(rect(&registry, &name), (x, -6.0, 22.0, 22.0));
    }
    assert!(registry.get_by_name("ChatFrame1CopyButton").is_none());
    assert_eq!(
        rect(&registry, "ChatFrame1Messages"),
        (34.0, 27.0, 430.0, 170.0)
    );
}

fn assert_header_icons(
    registry: &FrameRegistry,
    header: &str,
    separator: &str,
    icons: &[(&str, f32)],
    button_top: f32,
    spacing: f32,
) {
    let (_, header_top, _, header_height) = rect(registry, header);
    let (_, separator_top, _, _) = rect(registry, separator);
    let center = header_top + header_height / 2.0;
    for (index, &(name, left)) in icons.iter().enumerate() {
        let button = rect(registry, name);
        assert_eq!(button, (left, button_top, 22.0, 22.0), "{name}");
        assert_eq!(button.1 + button.3 / 2.0, center, "{name}: button centre");
        assert!(
            button.1 + button.3 < separator_top,
            "{name}: separator overlap"
        );
        let icon_name = format!("{name}Icon");
        let icon = rect(registry, &icon_name);
        assert_eq!(icon, (4.4, 4.4, 13.2, 13.2), "{icon_name}");
        assert!((button.1 + icon.1 + icon.3 / 2.0 - center).abs() < 0.00001);
        assert!(button.1 + icon.1 + icon.3 < separator_top);
        if index > 0 {
            assert_eq!(left - icons[index - 1].1, spacing);
        }
    }
}

#[test]
fn forever_header_icons_are_centred_equal_sized_and_clear_of_separator() {
    let chat = canvas(
        ActiveSkin::Forever,
        ChatFrameView::default(),
        chat_frame_screen,
    );
    assert_header_icons(
        &chat,
        "ChatFrame1FlareHeader",
        "ChatFrame1FlareSeparator",
        &[
            ("ChatFrame1FlareChannel", 321.0),
            ("ChatFrame1FlareMenu", 356.0),
            ("ChatFrame1FlareSocial", 391.0),
            ("ChatFrame1FlareVolume", 426.0),
        ],
        -6.0,
        35.0,
    );
    let meter = canvas(ActiveSkin::Forever, meter_view(), damage_meter_screen);
    assert_header_icons(
        &meter,
        "DamageMeterFlareHeader",
        "DamageMeterFlareSeparator",
        &[
            ("DamageMeterSessionDropdown", 398.5),
            ("DamageMeterSettings", 419.5),
        ],
        -1.0,
        21.0,
    );
}

#[test]
fn forever_chrome_atlases_resolve_to_assets_already_in_data() {
    canvas(
        ActiveSkin::Forever,
        ChatFrameView::default(),
        chat_frame_screen,
    );
    for name in ["classicon-mage", "classicon-warrior"] {
        let art = resolve_region(name, ActiveSkin::Forever).unwrap_or_else(|| panic!("no {name}"));
        let AtlasSource::FileDataId(fdid) = art.source else {
            panic!("{name}: not Blizzard art")
        };
        assert!(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../data/textures/{fdid}.blp"))
                .exists(),
            "missing {name}: {fdid}"
        );
    }
}

/// One row of `meter_type`: a paladin, or a recap line (no class) while a recap is open.
fn typed_view(meter_type: MeterType, recap_open: bool) -> DamageMeterView {
    DamageMeterView {
        meter_type,
        recap_open,
        rows: vec![DamageMeterRow {
            name_text: "Shot".into(),
            value_text: "12s".into(),
            fraction: 1.0,
            color: [0.96, 0.55, 0.73],
            class_id: if recap_open { 0 } else { 2 },
            is_local_player: !recap_open,
        }],
        ..Default::default()
    }
}

fn click(registry: &FrameRegistry, name: &str) -> Option<String> {
    frame(registry, name).onclick.clone()
}

const ACTIVE: [f32; 4] = [0.80, 0.60, 0.34, 1.0];
const INACTIVE: [f32; 4] = [0.56, 0.51, 0.46, 1.0];

#[test]
fn forever_dps_and_hps_tabs_select_damage_and_healing() {
    let view = typed_view(MeterType::HealingDone, false);
    let registry = canvas(ActiveSkin::Forever, view, damage_meter_screen);
    assert_eq!(font(&registry, "DamageMeterTypeName").color, INACTIVE);
    assert_eq!(font(&registry, "DamageMeterHpsTabName").color, ACTIVE);
    assert_eq!(
        click(&registry, "DamageMeterDpsTab").as_deref(),
        Some(MeterType::DamageDone.action())
    );
    assert_eq!(
        click(&registry, "DamageMeterHpsTab").as_deref(),
        Some(MeterType::HealingDone.action())
    );
    assert!(registry.get_by_name("DamageMeterOtherTypeName").is_none());
    // Healing rows look like damage rows and are not clickable.
    assert_eq!(font(&registry, "DamageMeterEntry1Name").text, "Shot");
    assert_eq!(
        texture(&registry, "DamageMeterEntry1Icon").source,
        TextureSource::Atlas("classicon-paladin".into())
    );
    assert!(registry.get_by_name("DamageMeterEntry1Button").is_none());
}

#[test]
fn forever_other_types_get_their_own_label_and_death_rows_are_clickable() {
    let view = typed_view(MeterType::Deaths, false);
    let registry = canvas(ActiveSkin::Forever, view, damage_meter_screen);
    assert_eq!(font(&registry, "DamageMeterTypeName").color, INACTIVE);
    assert_eq!(font(&registry, "DamageMeterHpsTabName").color, INACTIVE);
    assert_eq!(font(&registry, "DamageMeterOtherTypeName").text, "Deaths");
    assert_eq!(font(&registry, "DamageMeterOtherTypeName").color, ACTIVE);
    assert_eq!(
        click(&registry, "DamageMeterEntry1Button").as_deref(),
        Some("damage_meter:row:0")
    );

    // A recap line has no class icon and closes the recap when clicked.
    let view = typed_view(MeterType::Deaths, true);
    let registry = canvas(ActiveSkin::Forever, view, damage_meter_screen);
    assert_eq!(
        font(&registry, "DamageMeterOtherTypeName").text,
        "Death Recap"
    );
    assert!(registry.get_by_name("DamageMeterEntry1Icon").is_none());
    assert_eq!(font(&registry, "DamageMeterEntry1Value").text, "12s");
    assert_eq!(
        click(&registry, "DamageMeterEntry1Button").as_deref(),
        Some("damage_meter:row:0")
    );
}

#[test]
fn the_type_menu_lists_every_type_in_both_skins() {
    let options = |registry: &FrameRegistry| -> Vec<(Option<String>, String)> {
        (1..=MeterType::ALL.len())
            .map(|index| {
                (
                    click(registry, &format!("DamageMeterTypeMenuOption{index}")),
                    font(registry, &format!("DamageMeterTypeMenuText{index}"))
                        .text
                        .trim_start_matches(['\u{2022}', ' '])
                        .to_string(),
                )
            })
            .collect()
    };
    let expected: Vec<_> = MeterType::ALL
        .into_iter()
        .map(|t| (Some(t.action().to_string()), t.label().to_string()))
        .collect();
    assert_eq!(
        expected.iter().map(|(_, l)| l.as_str()).collect::<Vec<_>>(),
        [
            "Damage Done",
            "Healing Done",
            "Interrupts",
            "Dispels",
            "Deaths"
        ]
    );

    // Modern: the type dropdown opens it; the session menu stays closed.
    let view = DamageMeterView {
        type_menu_open: true,
        ..typed_view(MeterType::Interrupts, false)
    };
    let modern = canvas(ActiveSkin::Modern, view, damage_meter_screen);
    assert_eq!(
        click(&modern, "DamageMeterTypeDropdown").as_deref(),
        Some("damage_meter:type_menu")
    );
    assert_eq!(font(&modern, "DamageMeterTypeName").text, "Interrupts");
    assert_eq!(options(&modern), expected);
    // The selected type is the marked one.
    assert!(
        font(&modern, "DamageMeterTypeMenuText3")
            .text
            .starts_with('\u{2022}')
    );
    assert!(
        !font(&modern, "DamageMeterTypeMenuText1")
            .text
            .starts_with('\u{2022}')
    );
    assert!(modern.get_by_name("DamageMeterSessionMenu").is_none());
    // The menu is drawn over the row it covers: a higher strata.
    let strata = |name: &str| format!("{:?}", frame(&modern, name).strata);
    assert_eq!(
        (strata("DamageMeterTypeMenu"), strata("DamageMeterEntry1")),
        ("Dialog".to_string(), "Medium".to_string())
    );

    // Forever: the chart button's one menu lists the types beside the sessions.
    let view = DamageMeterView {
        menu_open: true,
        ..Default::default()
    };
    let forever = canvas(ActiveSkin::Forever, view, damage_meter_screen);
    assert_eq!(options(&forever), expected);
    assert_eq!(
        click(&forever, "DamageMeterSessionMenuOption2").as_deref(),
        Some("damage_meter:overall")
    );
    let closed = canvas(
        ActiveSkin::Forever,
        DamageMeterView::default(),
        damage_meter_screen,
    );
    assert!(closed.get_by_name("DamageMeterTypeMenu").is_none());
    assert!(closed.get_by_name("DamageMeterSessionMenu").is_none());
}
