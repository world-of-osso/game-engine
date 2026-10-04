//! Main action bar, end caps, micro menu and bags bar name Blizzard atlas elements.
//! Under Modern every frame is exactly what the components built from hand-copied crops
//! before (forever7 base, master 9343e578); under Forever the names Forever re-skins draw
//! their set-1 `c60` members, the main bar takes FlareUI's button scale and art, and the
//! end caps are the project's class shields.

use std::fmt::Write;
use std::path::PathBuf;

#[path = "fixtures/modern_action_bar_trees.rs"]
mod fixture;

use game_engine_ui_model::bags_bar_component::{
    BagBarState, FOREVER_ONLY_BAG_ATLASES, bags_bar_screen,
};
use game_engine_ui_model::hud_layout::{ActionBarLayout, MODERN};
use game_engine_ui_model::main_action_bar_component::{
    ActionBar, ActionButtonView, MainActionBarState, main_action_bar_screen, parse_action_button,
};
use game_engine_ui_model::micro_menu::{MicroMenuView, OpenWindows, micro_menu_screen};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

fn load_atlas_tables() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

/// `UiTextureAtlas` `AtlasWidth`×`AtlasHeight` of every sheet these frames draw from.
fn sheet_size(fdid: u32) -> [f32; 2] {
    match fdid {
        4_613_342 => [256.0, 1024.0], // uiactionbar
        7_948_328 => [512.0, 512.0],  // uiactionbarc60
        4_708_813 => [1024.0, 512.0], // uimicromenu2x
        8_200_846 => [1024.0, 512.0], // uimicromenuc602x
        4_691_255 => [512.0, 128.0],  // bag sheet (atlas 2098)
        other => panic!("no sheet size for {other}"),
    }
}

/// An atlas crop: sheet FileDataID and pixel `left, right, top, bottom`.
#[derive(Debug, PartialEq)]
struct Crop(u32, [f32; 4]);

/// The sheet and pixel crop a texture draws under `skin`: a FileDataID with its
/// `tex_coords`, or an atlas element's region narrowed by them.
fn crop_of(texture: &TextureData, skin: ActiveSkin) -> Crop {
    let [u0, u1, v0, v1] = texture.tex_coords;
    let (fdid, [left, right, top, bottom]) = match &texture.source {
        TextureSource::FileDataId(fdid) => (*fdid, [0.0, 1.0, 0.0, 1.0]),
        TextureSource::Atlas(name) => {
            let region =
                resolve_region(name, skin).unwrap_or_else(|| panic!("{name} under {skin:?}"));
            let AtlasSource::FileDataId(fdid) = region.source else {
                panic!("{name} is not a FileDataID sheet");
            };
            (fdid, [region.left, region.right, region.top, region.bottom])
        }
        other => panic!("not sheet art: {other:?}"),
    };
    let [width, height] = sheet_size(fdid);
    let x = |u: f32| (left + u * (right - left)) * width;
    let y = |v: f32| (top + v * (bottom - top)) * height;
    let round = |value: f32| (value * 1000.0).round() / 1000.0;
    Crop(fdid, [x(u0), x(u1), y(v0), y(v1)].map(round))
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

fn drawn(registry: &FrameRegistry, name: &str, skin: ActiveSkin) -> Crop {
    let Some(WidgetData::Texture(texture)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name} is not a Texture");
    };
    crop_of(texture, skin)
}

/// `(left, top, width, height)` of a frame placed with fixed pixels.
fn fixed_rect(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let f = frame(registry, name);
    let px = |value: Val| match value {
        Val::Px(px) => px,
        other => panic!("{name}: {other:?} is not pixels"),
    };
    let fixed = |dimension: Dimension| match dimension {
        Dimension::Fixed(px) => px,
        other => panic!("{name}: {other:?} is not fixed"),
    };
    (
        px(f.position.left),
        px(f.position.top),
        fixed(f.width),
        fixed(f.height),
    )
}

fn assert_close(actual: (f32, f32, f32, f32), expected: (f32, f32, f32, f32), what: &str) {
    let close = [
        (actual.0, expected.0),
        (actual.1, expected.1),
        (actual.2, expected.2),
        (actual.3, expected.3),
    ]
    .iter()
    .all(|(a, b)| (a - b).abs() < 1e-3);
    assert!(close, "{what}: {actual:?} != {expected:?}");
}

/// Every attribute the components set, one line per frame in tree order; texture art as
/// the sheet crop it draws under `skin`, whichever way the component named it.
fn dump(registry: &FrameRegistry, root: &str, skin: ActiveSkin) -> String {
    let mut out = String::new();
    dump_frame(
        registry,
        registry.get_by_name(root).expect(root),
        0,
        skin,
        &mut out,
    );
    out
}

fn dump_frame(registry: &FrameRegistry, id: u64, depth: usize, skin: ActiveSkin, out: &mut String) {
    let f = registry.get(id).unwrap();
    writeln!(
        out,
        "{:indent$}{:?} {:?} w={:?} h={:?} pos={:?} {:?} margin={:?} translate={:?} \
         hidden={} alpha={} strata={:?} level={} layer={:?} bg={:?} mouse={} click={:?} {}",
        "",
        f.name,
        f.widget_type,
        f.width,
        f.height,
        f.position,
        f.position_type,
        f.margin,
        f.translation,
        f.hidden,
        f.alpha,
        f.strata,
        f.frame_level,
        f.draw_layer,
        f.background_color,
        f.mouse_enabled,
        f.onclick,
        widget(f, skin),
        indent = depth * 2
    )
    .unwrap();
    for child in &f.children {
        dump_frame(registry, *child, depth + 1, skin, out);
    }
}

fn widget(f: &Frame, skin: ActiveSkin) -> String {
    match f.widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) if texture.source != TextureSource::None => {
            let art = match texture.source {
                TextureSource::FileDataId(fdid) if sheet_size_known(fdid) => {
                    format!("{:?}", crop_of(texture, skin))
                }
                TextureSource::Atlas(_) => format!("{:?}", crop_of(texture, skin)),
                ref other => format!("{other:?} {:?}", texture.tex_coords),
            };
            format!(
                "art={art} tile={},{} blend={:?} color={:?} desat={},{} rot={}",
                texture.horiz_tile,
                texture.vert_tile,
                texture.blend_mode,
                texture.vertex_color,
                texture.desaturated,
                texture.desaturation,
                texture.rotation
            )
        }
        other => format!("data={other:?}"),
    }
}

fn sheet_size_known(fdid: u32) -> bool {
    matches!(
        fdid,
        4_613_342 | 7_948_328 | 4_708_813 | 8_200_846 | 4_691_255
    )
}

fn build<T: 'static>(
    skin: ActiveSkin,
    state: T,
    screen: fn(&SharedContext) -> Element,
) -> FrameRegistry {
    load_atlas_tables();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(screen).sync(&shared, &mut registry);
    registry
}

/// A bar with a pushed spell on cooldown, a hovered button and empty slots.
fn bar_state() -> MainActionBarState {
    let mut state = MainActionBarState::default();
    state.buttons[0] = ActionButtonView {
        icon_fdid: 132_340,
        cooldown_fraction: 0.5,
        cooldown_text: "3".into(),
        pushed: true,
        hovered: false,
    };
    state.buttons[1].hovered = true;
    // A paladin: Modern's gryphons do not depend on the class.
    state.player_class = Some(2);
    state
}

/// Normal, hovered and pressed buttons; open windows; the game menu disabling the rest.
fn micro_views() -> Vec<MicroMenuView> {
    vec![
        MicroMenuView::default(),
        MicroMenuView {
            hovered: Some(1),
            pressed: Some(2),
            ..Default::default()
        },
        MicroMenuView {
            open: OpenWindows {
                character: true,
                quest_log: true,
                ..Default::default()
            },
            hovered: Some(9),
            ..Default::default()
        },
        MicroMenuView {
            open: OpenWindows {
                game_menu: true,
                ..Default::default()
            },
            ..Default::default()
        },
    ]
}

/// Filled and empty bag and reagent slots, expanded and collapsed.
fn bag_states() -> Vec<BagBarState> {
    vec![
        BagBarState {
            money: 1_234_567,
            free_slots: 12,
            bag_icons: [Some(133_633), None, Some(133_634), None, Some(4_549_254)],
            collapsed: false,
        },
        BagBarState {
            bag_icons: [None, None, None, None, None],
            collapsed: true,
            ..Default::default()
        },
    ]
}

fn trees(skin: ActiveSkin) -> String {
    let mut out = dump(
        &build(skin, bar_state(), main_action_bar_screen),
        "MainActionBar",
        skin,
    );
    for view in micro_views() {
        out += &dump(
            &build(skin, view, micro_menu_screen),
            "MicroMenuContainer",
            skin,
        );
    }
    for state in bag_states() {
        out += &dump(&build(skin, state, bags_bar_screen), "BagsBar", skin);
    }
    out
}

/// The Modern trees as the components built them from hand-copied crops (master 9343e578).
#[test]
fn modern_bars_draw_exactly_what_their_hand_copied_crops_drew() {
    let trees = trees(ActiveSkin::Modern);
    if std::env::var_os("PRINT_MODERN_TREES").is_some() {
        println!("<<<MODERN_TREES\n{trees}MODERN_TREES>>>");
    }
    for (line, (actual, expected)) in trees.lines().zip(fixture::MODERN_TREES.lines()).enumerate() {
        assert_eq!(actual, expected, "line {}", line + 1);
    }
    assert_eq!(trees.lines().count(), fixture::MODERN_TREES.lines().count());
}

/// The texture file a frame draws.
fn file_of(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::Texture(TextureData {
            source: TextureSource::File(path),
            ..
        })) => path.clone(),
        other => panic!("{name} draws no file: {other:?}"),
    }
}

/// Names of the atlas elements `root` and its descendants draw.
fn atlas_names(registry: &FrameRegistry, root: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut pending = vec![registry.get_by_name(root).expect(root)];
    while let Some(id) = pending.pop() {
        let f = registry.get(id).unwrap();
        if let Some(WidgetData::Texture(TextureData {
            source: TextureSource::Atlas(name),
            ..
        })) = f.widget_data.as_ref()
        {
            names.push(name.clone());
        }
        pending.extend(&f.children);
    }
    names
}

fn class_bar(class: Option<u8>) -> MainActionBarState {
    MainActionBarState {
        player_class: class,
        ..bar_state()
    }
}

const END_CAPS: [&str; 2] = ["MainActionBarLeftEndCap", "MainActionBarRightEndCap"];

type Rect = (f32, f32, f32, f32);

fn intersects(a: Rect, b: Rect) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Rect on the 1920×1080 canvas of a root frame anchored to its bottom centre.
fn bar_rect(registry: &FrameRegistry, name: &str) -> Rect {
    let f = frame(registry, name);
    let (Val::Percent(50.0), Val::Px(margin), Val::Px(bottom)) =
        (f.position.left, f.margin.left, f.position.bottom)
    else {
        panic!(
            "{name} is not anchored to the bottom centre: {:?}",
            f.position
        );
    };
    let (Dimension::Fixed(width), Dimension::Fixed(height)) = (f.width, f.height) else {
        panic!("{name} has no fixed size");
    };
    (960.0 + margin, 1080.0 - bottom - height, width, height)
}

/// Canvas rect of `child`, a direct child of root frame `bar`.
fn child_rect(registry: &FrameRegistry, bar: &str, child: &str) -> Rect {
    let (bar_x, bar_y, _, _) = bar_rect(registry, bar);
    let (x, y, width, height) = fixed_rect(registry, child);
    (bar_x + x, bar_y + y, width, height)
}

fn bar_name(bar: ActionBar) -> &'static str {
    match bar {
        ActionBar::Main => "MainActionBar",
        ActionBar::BottomLeft => "MultiBarBottomLeft",
        ActionBar::BottomRight => "MultiBarBottomRight",
    }
}

/// Name and canvas rect of every button `bar` shows, widened to its frame art (which
/// overhangs the button by a unit).
fn shown_buttons(registry: &FrameRegistry, bar: ActionBar) -> Vec<(String, Rect)> {
    (0..12)
        .map(|index| bar.button_name(index))
        .filter(|name| registry.get_by_name(name).is_some())
        .map(|name| {
            let (x, y, _, height) = child_rect(registry, bar_name(bar), &name);
            let art_width = fixed_rect(registry, &format!("{name}NormalTexture")).2;
            (name, (x, y, art_width, height))
        })
        .collect()
}

fn forever_bars() -> FrameRegistry {
    build(
        ActiveSkin::Forever,
        class_bar(Some(2)),
        main_action_bar_screen,
    )
}

/// User decision 2026-10-03: Forever's end caps are class shields standing outside the
/// buttons. They cover no button of any of the three bars.
#[test]
fn forever_end_caps_cover_no_action_button() {
    let registry = forever_bars();
    let caps = END_CAPS.map(|cap| child_rect(&registry, "MainActionBar", cap));
    let buttons: Vec<(String, Rect)> = ActionBar::ALL
        .into_iter()
        .flat_map(|bar| shown_buttons(&registry, bar))
        .collect();
    assert_eq!(buttons.len(), 28);
    for (name, button) in &buttons {
        for cap in caps {
            assert!(!intersects(*button, cap), "{name} {button:?} under {cap:?}");
        }
    }
    // One shield on each side of the lower rows.
    let (main_x, _, main_w, _) = bar_rect(&registry, "MainActionBar");
    assert!(caps[0].0 + caps[0].2 <= main_x);
    assert!(caps[1].0 >= main_x + main_w);
    let gryphons: Vec<String> = atlas_names(&registry, "MainActionBar")
        .into_iter()
        .filter(|name| name.contains("gryphon"))
        .collect();
    assert_eq!(gryphons, Vec::<String>::new());
}

/// The reference block: main bar and Action Bar 2 of 9 buttons each, Action Bar 3 of 10
/// smaller buttons on top, every bar centred, no two buttons overlapping.
#[test]
fn forever_stacks_three_centred_bars_of_9_9_and_10_buttons() {
    let registry = forever_bars();
    let bars = ActionBar::ALL.map(|bar| {
        (
            bar_rect(&registry, bar_name(bar)),
            shown_buttons(&registry, bar),
        )
    });
    assert_eq!(
        bars.each_ref().map(|(_, buttons)| buttons.len()),
        [9, 9, 10]
    );
    for ((x, _, width, _), _) in &bars {
        assert!(
            (x + width / 2.0 - 960.0).abs() < 1e-3,
            "centre {}",
            x + width / 2.0
        );
    }
    // Stacked upward from the main bar, each above the one below.
    let bottom = |index: usize| bars[index].0.1 + bars[index].0.3;
    assert!(bottom(1) < bars[0].0.1 && bottom(2) < bars[1].0.1);
    // The lower rows are the same size; the top row's buttons are smaller.
    let button_height = |index: usize| bars[index].1[0].1.3;
    assert_eq!(button_height(0), button_height(1));
    assert!(button_height(2) < button_height(0));
    let all: Vec<&(String, Rect)> = bars.iter().flat_map(|(_, buttons)| buttons).collect();
    for (index, (name, rect)) in all.iter().enumerate() {
        for (other_name, other) in &all[index + 1..] {
            // Frame art overhangs its button by a unit into the 2-unit gap.
            assert!(!intersects(*rect, *other), "{name} overlaps {other_name}");
        }
    }
}

/// Retail hides Action Bars 2 and 3 until the player enables them (MultiActionBars.xml:45,75).
#[test]
fn modern_shows_only_the_main_bar() {
    let registry = build(ActiveSkin::Modern, bar_state(), main_action_bar_screen);
    assert_eq!(shown_buttons(&registry, ActionBar::Main).len(), 12);
    for bar in ["MultiBarBottomLeft", "MultiBarBottomRight"] {
        assert_eq!(registry.get_by_name(bar), None, "{bar}");
    }
}

/// `ActionBarMixin:UpdateGridLayout` (ActionBar.lua:98-138): `ceil(icons / rows)` buttons
/// per row, rows filling left to right and stacking upward, a short last row on top.
#[test]
fn a_bar_lays_its_icons_out_in_rows_of_the_retail_stride() {
    let cases: [(usize, usize, &[usize]); 6] = [
        (12, 1, &[12]),
        (12, 2, &[6, 6]),
        (10, 2, &[5, 5]),
        (7, 2, &[4, 3]),
        (11, 3, &[4, 4, 3]),
        (12, 4, &[3, 3, 3, 3]),
    ];
    for (num_icons, num_rows, bottom_up) in cases {
        let layout = ActionBarLayout {
            anchor: MODERN.main_action_bar.anchor,
            num_icons,
            num_rows,
            icon_scale: 0.8,
        };
        let what = format!("{num_icons} icons in {num_rows} rows");
        let side = 45.0 * 0.8;
        let cells: Vec<Rect> = (0..num_icons)
            .map(|index| {
                let (x, y) = layout.button_origin(index, 1.0);
                (x, y, side, side)
            })
            .collect();
        let (width, height) = layout.size(1.0);
        // Buttons in index order fill the bottom row first.
        let mut rows: Vec<(f32, usize)> = Vec::new();
        for cell in &cells {
            match rows.last().copied() {
                Some((y, count)) if y == cell.1 => *rows.last_mut().unwrap() = (y, count + 1),
                Some((y, _)) => {
                    assert!(cell.1 < y, "{what}: rows stack upward");
                    rows.push((cell.1, 1));
                }
                None => rows.push((cell.1, 1)),
            }
        }
        let counts: Vec<usize> = rows.iter().map(|(_, count)| *count).collect();
        assert_eq!(counts, bottom_up, "{what}");
        for (index, cell) in cells.iter().enumerate() {
            assert!(cell.0 >= 0.0 && cell.1 >= 0.0, "{what}: {cell:?}");
            assert!(cell.0 + side <= width + 1e-3, "{what}: {cell:?} in {width}");
            assert!(
                cell.1 + side <= height + 1e-3,
                "{what}: {cell:?} in {height}"
            );
            for other in &cells[index + 1..] {
                assert!(!intersects(*cell, *other), "{what}: {cell:?} {other:?}");
            }
        }
        // A full row spans the bar; the bottom row touches its bottom edge.
        let last_in_bottom_row = cells[bottom_up[0] - 1];
        assert!((last_in_bottom_row.0 + side - width).abs() < 1e-3, "{what}");
        assert!((cells[0].1 + side - height).abs() < 1e-3, "{what}");
    }
}

/// Retail action slots (1-based): main bar 1-12, MultiBarBottomLeft page 6 = 61-72,
/// MultiBarBottomRight page 5 = 49-60 (MultiActionBars.xml:62,91, ActionButtonUtil.lua:66).
/// The protocol's slots are 0-based.
#[test]
fn each_bars_buttons_use_their_retail_action_slots() {
    let slots = |bar: ActionBar| [bar.action_slot(0), bar.action_slot(11)];
    assert_eq!(slots(ActionBar::Main), [0, 11]);
    assert_eq!(slots(ActionBar::BottomLeft), [60, 71]);
    assert_eq!(slots(ActionBar::BottomRight), [48, 59]);
    // A click on a drawn button names its own bar and button.
    let registry = forever_bars();
    let clicked = |name: &str| {
        parse_action_button(frame(&registry, name).onclick.as_deref().expect(name))
            .map(|(bar, index)| bar.action_slot(index))
    };
    assert_eq!(clicked("ActionButton3"), Some(2));
    assert_eq!(clicked("MultiBarBottomLeftButton3"), Some(62));
    assert_eq!(clicked("MultiBarBottomRightButton10"), Some(57));
    assert_eq!(
        ActionBar::of_button_name("MultiBarBottomRightButton10"),
        Some((ActionBar::BottomRight, 9))
    );
}

/// `ChrClasses` IDs 1-13 each draw their own class's shield on both sides.
#[test]
fn forever_end_caps_draw_the_players_class_shield() {
    let classes = [
        "warrior",
        "paladin",
        "hunter",
        "rogue",
        "priest",
        "deathknight",
        "shaman",
        "mage",
        "warlock",
        "monk",
        "druid",
        "demonhunter",
        "evoker",
    ];
    for (id, class) in (1..).zip(classes) {
        let registry = build(
            ActiveSkin::Forever,
            class_bar(Some(id)),
            main_action_bar_screen,
        );
        let [left, right] = END_CAPS.map(|name| file_of(&registry, name));
        assert!(left.ends_with(&format!("/{class}.ktx2")), "{id}: {left}");
        assert!(right.ends_with(&format!("/{class}.ktx2")), "{id}: {right}");
        assert_ne!(left, right, "{id}: mirrored art");
    }
}

/// No shield stands in for a class the client does not know (yet) or has no art for.
#[test]
fn forever_draws_no_end_cap_without_a_known_class() {
    for class in [None, Some(0), Some(14)] {
        let registry = build(
            ActiveSkin::Forever,
            class_bar(class),
            main_action_bar_screen,
        );
        for name in END_CAPS {
            assert_eq!(registry.get_by_name(name), None, "{name} for {class:?}");
        }
        assert!(registry.get_by_name("ActionButton1").is_some());
    }
}

#[test]
fn both_presets_show_the_micro_menu_and_bags_bar() {
    for skin in [ActiveSkin::Forever, ActiveSkin::Modern] {
        let micro = build(skin, MicroMenuView::default(), micro_menu_screen);
        let bags = build(skin, BagBarState::default(), bags_bar_screen);
        assert!(!frame(&micro, "MicroMenuContainer").hidden, "{skin:?}");
        assert!(!frame(&bags, "BagsBar").hidden, "{skin:?}");
    }
}

/// FlareUI (Core.lua:207,218; ActionBars.lua:141-157,186): buttons at scale 1.06, the
/// `SlotArt` shown and `SlotBackground` hidden, `UI-HUD-ActionBar-IconFrame` /
/// `-Down` at 46×45. Forever re-skins IconFrame and IconFrame-Slot, not -Down or
/// -Mouseover.
#[test]
fn forever_action_buttons_take_flareui_scale_and_c60_icon_frame() {
    let skin = ActiveSkin::Forever;
    let registry = build(skin, bar_state(), main_action_bar_screen);
    let scale = 1.06;
    assert_close(
        fixed_rect(&registry, "ActionButton2NormalTexture"),
        (0.0, 0.0, 46.0 * scale, 45.0 * scale),
        "NormalTexture",
    );
    assert!(frame(&registry, "ActionButton1SlotBackground").hidden);
    // UI-HUD-ActionBar-IconFrame (39247), -Slot (39246) on uiactionbarc60.
    assert_eq!(
        drawn(&registry, "ActionButton2NormalTexture", skin),
        Crop(7_948_328, [1.0, 47.0, 449.0, 494.0])
    );
    assert_eq!(
        drawn(&registry, "ActionButton1SlotArt", skin),
        Crop(7_948_328, [1.0, 65.0, 285.0, 347.0])
    );
    // -Down and -Mouseover have no set-1 member: Retail's (15802, 15805).
    assert_eq!(
        drawn(&registry, "ActionButton1PushedTexture", skin),
        Crop(4_613_342, [181.0, 227.0, 521.0, 566.0])
    );
    assert_eq!(
        drawn(&registry, "ActionButton2HighlightTexture", skin),
        Crop(4_613_342, [181.0, 227.0, 643.0, 688.0])
    );
}

/// Forever re-skins the micro buttons onto `uimicromenuc602x` (8200846); Housing and the
/// portrait shadow have no set-1 member and keep Retail's art.
#[test]
fn forever_micro_menu_draws_c60_buttons() {
    let skin = ActiveSkin::Forever;
    let registry = build(skin, MicroMenuView::default(), micro_menu_screen);
    let c60 = |left: f32, top: f32| Crop(8_200_846, [left, left + 64.0, top, top + 82.0]);
    let retail = |left: f32, top: f32| Crop(4_708_813, [left, left + 64.0, top, top + 82.0]);
    let cases = [
        // UI-HUD-MicroMenu-ButtonBG-Up
        ("ProfessionMicroButtonArt0", c60(1.0, 421.0)),
        // -Professions-Up, -SpecTalents-Up, -Questlog-Up, -GameMenu-Up
        ("ProfessionMicroButtonArt1", c60(331.0, 337.0)),
        ("PlayerSpellsMicroButtonArt1", c60(463.0, 337.0)),
        ("QuestLogMicroButtonArt1", c60(397.0, 169.0)),
        ("MainMenuMicroButtonArt1", c60(133.0, 85.0)),
        // -Housing-Up and -Portrait-Shadow: set 0 only.
        ("HousingMicroButtonArt1", retail(331.0, 337.0)),
        ("CharacterMicroButtonArt1", retail(397.0, 1.0)),
    ];
    for (name, expected) in cases {
        assert_eq!(drawn(&registry, name, skin), expected, "{name}");
    }
}

/// Forever has no set-1 member for the Retail bag atlases, so its bags bar draws Retail's
/// art; the bag button atlases Camelot's own bag bar uses exist only in Forever's set 1.
#[test]
fn forever_bags_keep_retail_art_and_camelot_bag_atlases_are_forever_only() {
    let state = bag_states().remove(0);
    let modern = dump(
        &build(ActiveSkin::Modern, state, bags_bar_screen),
        "BagsBar",
        ActiveSkin::Modern,
    );
    let forever = dump(
        &build(ActiveSkin::Forever, state, bags_bar_screen),
        "BagsBar",
        ActiveSkin::Forever,
    );
    // The anchors differ (hud_layout); everything inside the bar is the same.
    let inside = |tree: &str| tree.lines().skip(1).collect::<Vec<_>>().join("\n");
    assert_eq!(inside(&modern), inside(&forever));
    for name in FOREVER_ONLY_BAG_ATLASES {
        assert!(resolve_region(name, ActiveSkin::Modern).is_none(), "{name}");
        let region = resolve_region(name, ActiveSkin::Forever).expect(name);
        assert_eq!(region.source, AtlasSource::FileDataId(7_948_328), "{name}");
    }
}

/// One Screen re-synced with the other skin rebuilds the bar: the paladin shield on
/// Forever, then Modern's gryphon again; the class arriving late adds the shields.
#[test]
fn switching_skin_reskins_the_live_main_bar() {
    load_atlas_tables();
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(bar_state());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut screen = Screen::new(main_action_bar_screen);
    screen.sync(&shared, &mut registry);
    let cap = "MainActionBarLeftEndCap";
    let modern_cap = Crop(4_613_342, [1.0, 179.0, 136.0, 303.0]);
    assert_eq!(drawn(&registry, cap, ActiveSkin::Modern), modern_cap);

    shared.insert(ActiveSkin::Forever);
    screen.sync(&shared, &mut registry);
    assert!(file_of(&registry, cap).ends_with("/paladin.ktx2"));

    shared.insert(class_bar(None));
    screen.sync(&shared, &mut registry);
    assert_eq!(registry.get_by_name(cap), None);
    shared.insert(class_bar(Some(8)));
    screen.sync(&shared, &mut registry);
    assert!(file_of(&registry, cap).ends_with("/mage.ktx2"));

    shared.insert(ActiveSkin::Modern);
    screen.sync(&shared, &mut registry);
    assert_eq!(drawn(&registry, cap, ActiveSkin::Modern), modern_cap);
    assert_eq!(registry.get_by_name("MultiBarBottomLeft"), None);
}

/// Where the projection paints a button's region among its siblings: strata, then frame
/// level plus draw layer, then sibling order.
fn paint_order(registry: &FrameRegistry, button: &str, region: &str) -> (u8, i32, usize) {
    let f = frame(registry, region);
    let sibling = frame(registry, button)
        .children
        .iter()
        .position(|&id| id == f.id)
        .unwrap_or_else(|| panic!("{region} is not a region of {button}"));
    (
        f.strata as u8,
        f.frame_level + i32::from(f.draw_layer as u8),
        sibling,
    )
}

/// The bar is built empty and the server's action snapshot fills it afterwards. The icon
/// that arrives is the button's `BACKGROUND` layer (`ActionButtonTemplate.xml:22-33`): the
/// slot art stays under it and the border (`NormalTexture`, 46×45 over the 45×45 icon,
/// `:146-151`) frames it, on every bar and skin.
#[test]
fn an_icon_arriving_on_a_built_bar_is_painted_inside_and_under_the_border() {
    load_atlas_tables();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(MainActionBarState::default());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(main_action_bar_screen);
        screen.sync(&shared, &mut registry);

        let mut state = MainActionBarState::default();
        for bar in ActionBar::ALL {
            state.bar_mut(bar)[0].icon_fdid = 135_891;
        }
        shared.insert(state);
        screen.sync(&shared, &mut registry);

        for bar in ActionBar::ALL {
            let button = bar.button_name(0);
            if registry.get_by_name(&button).is_none() {
                continue;
            }
            let order =
                |region: &str| paint_order(&registry, &button, &format!("{button}{region}"));
            let icon = order("Icon");
            assert!(
                order("SlotArt") < icon,
                "{skin:?} {button}: slot under icon"
            );
            assert!(
                icon < order("NormalTexture"),
                "{skin:?} {button}: border over icon"
            );
            assert!(
                icon < order("PushedTexture"),
                "{skin:?} {button}: pushed border over icon"
            );

            let (x, y, width, height) = fixed_rect(&registry, &format!("{button}Icon"));
            let border = fixed_rect(&registry, &format!("{button}NormalTexture"));
            assert!(
                x >= border.0
                    && y >= border.1
                    && x + width <= border.0 + border.2
                    && y + height <= border.1 + border.3,
                "{skin:?} {button}: icon {:?} outside border {border:?}",
                (x, y, width, height)
            );
        }
    }
}
