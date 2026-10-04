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
use game_engine_ui_model::main_action_bar_component::{
    ActionButtonView, MainActionBarState, main_action_bar_screen,
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

/// User decision 2026-10-03: Forever's end caps are the project's narrow class shields,
/// 95 units tall at the art's 68:256 aspect, bottoms on the bar's bottom edge, 6 units
/// outside the first and last button.
#[test]
fn forever_end_caps_are_class_shields_outside_the_buttons() {
    let registry = build(
        ActiveSkin::Forever,
        class_bar(Some(2)),
        main_action_bar_screen,
    );
    let (bar_w, bar_h) = (562.0 * 1.06, 45.0 * 1.06);
    let cap_w = 95.0 * 68.0 / 256.0;
    let caps = [
        (-6.0 - cap_w, bar_h - 95.0, cap_w, 95.0),
        (bar_w + 6.0, bar_h - 95.0, cap_w, 95.0),
    ];
    assert_eq!(cap_w, 25.234375);
    for (name, expected) in END_CAPS.into_iter().zip(caps) {
        assert_close(fixed_rect(&registry, name), expected, name);
    }
    // Buttons and their 46-wide frame art (the last one's overhangs the bar) stay clear.
    for index in 1..=12 {
        let (x, y, _, h) = fixed_rect(&registry, &format!("ActionButton{index}"));
        let art_w = fixed_rect(&registry, &format!("ActionButton{index}NormalTexture")).2;
        for (cap_x, cap_y, cap_w, cap_h) in caps {
            let apart_x = cap_x + cap_w <= x || x + art_w <= cap_x;
            let apart_y = cap_y + cap_h <= y || y + h <= cap_y;
            assert!(apart_x || apart_y, "ActionButton{index} under an end cap");
        }
    }
    let gryphons: Vec<String> = atlas_names(&registry, "MainActionBar")
        .into_iter()
        .filter(|name| name.contains("gryphon"))
        .collect();
    assert_eq!(gryphons, Vec::<String>::new());
}

/// `ChrClasses` IDs 1-13 name the shield files under `data/ui/endcaps/`.
#[test]
fn forever_end_caps_draw_the_players_class_shield() {
    let classes = [
        (1, "warrior"),
        (2, "paladin"),
        (3, "hunter"),
        (4, "rogue"),
        (5, "priest"),
        (6, "deathknight"),
        (7, "shaman"),
        (8, "mage"),
        (9, "warlock"),
        (10, "monk"),
        (11, "druid"),
        (12, "demonhunter"),
        (13, "evoker"),
    ];
    for (id, class) in classes {
        let registry = build(
            ActiveSkin::Forever,
            class_bar(Some(id)),
            main_action_bar_screen,
        );
        assert_eq!(
            END_CAPS.map(|name| file_of(&registry, name)),
            [
                format!("data/ui/endcaps/left/{class}.ktx2"),
                format!("data/ui/endcaps/right/{class}.ktx2"),
            ]
        );
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
        assert!(registry.get_by_name("ActionButton12").is_some());
    }
}

#[test]
fn forever_centres_main_bar() {
    let registry = build(ActiveSkin::Forever, bar_state(), main_action_bar_screen);
    let bar = frame(&registry, "MainActionBar");
    assert_eq!(bar.position.left, Val::Percent(50.0));
    assert_eq!(bar.position.bottom, Val::Px(2.0));
    assert_eq!(bar.margin.left, Val::Px(-562.0 * 1.06 / 2.0));
}

#[test]
fn forever_hides_reference_utility_bars() {
    for skin in [ActiveSkin::Forever, ActiveSkin::Modern] {
        let micro = build(skin, MicroMenuView::default(), micro_menu_screen);
        let bags = build(skin, BagBarState::default(), bags_bar_screen);
        assert_eq!(
            frame(&micro, "MicroMenuContainer").hidden,
            skin == ActiveSkin::Forever
        );
        assert_eq!(frame(&bags, "BagsBar").hidden, skin == ActiveSkin::Forever);
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
    let bar = frame(&registry, "MainActionBar");
    assert_eq!(bar.width, Dimension::Fixed(562.0 * scale));
    assert_eq!(bar.height, Dimension::Fixed(45.0 * scale));
    assert_close(
        fixed_rect(&registry, "ActionButton12"),
        (11.0 * 47.0 * scale, 0.0, 45.0 * scale, 45.0 * scale),
        "ActionButton12",
    );
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
    assert_eq!(fixed_rect(&registry, cap).2, 104.5);

    shared.insert(ActiveSkin::Forever);
    screen.sync(&shared, &mut registry);
    assert_eq!(file_of(&registry, cap), "data/ui/endcaps/left/paladin.ktx2");
    assert_eq!(fixed_rect(&registry, cap).2, 25.234375);

    shared.insert(class_bar(None));
    screen.sync(&shared, &mut registry);
    assert_eq!(registry.get_by_name(cap), None);
    shared.insert(class_bar(Some(8)));
    screen.sync(&shared, &mut registry);
    assert_eq!(file_of(&registry, cap), "data/ui/endcaps/left/mage.ktx2");

    shared.insert(ActiveSkin::Modern);
    screen.sync(&shared, &mut registry);
    assert_eq!(drawn(&registry, cap, ActiveSkin::Modern), modern_cap);
    assert_eq!(fixed_rect(&registry, cap).2, 104.5);
}
