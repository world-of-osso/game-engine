//! Under the Forever preset the player, target, target-of-target, focus and pet frames take
//! FlareUI 1.3's shape (`Core.lua:281-302`, `Modules/UnitFrames.lua`) and the player cast
//! bar its standalone bar; Modern keeps its frame trees exactly.

#[path = "fixtures/modern_unit_frame_trees.rs"]
mod fixture;

use std::fmt::Write as _;
use std::path::PathBuf;

use game_engine_ui_model::casting_bar_frame_component::{
    CastingBarState, casting_bar_frame_screen,
};
use game_engine_ui_model::faction_reaction::Reaction;
use game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_art::{
    atlas_size, sized_atlas_texture,
};
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PetFrameState, PowerBarState, SmallUnitFrameState, UnitFrameMenuState,
    UnitFrameState, inworld_unit_frames_screen,
};
use shared::components::{CreatureClassification, PowerType};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Dimension, Frame, WidgetData, WidgetType};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn load_atlas_tables() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn unit(name: &str, power: PowerType) -> UnitFrameState {
    UnitFrameState {
        level_text: "60".into(),
        health_fraction: 0.75,
        reaction: Some(Reaction::Hostile),
        power: Some(PowerBarState {
            power,
            current: 40,
            max: 100,
        }),
        show_combat_icon: true,
        show_resting_icon: true,
        ..UnitFrameState::named(name)
    }
}

fn small(name: &str) -> SmallUnitFrameState {
    SmallUnitFrameState {
        name: name.into(),
        level: None,
        health_fraction: 0.5,
        reaction: Some(Reaction::Neutral),
        class_id: None,
    }
}

fn unit_frames(skin: ActiveSkin) -> FrameRegistry {
    load_atlas_tables();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        target_cast: None,
        player: unit("Fbflare", PowerType::Mana),
        target: Some(UnitFrameState {
            classification: CreatureClassification::Elite,
            ..unit("Hogger", PowerType::Rage)
        }),
        target_of_target: Some(small("Fbflare")),
        focus: Some(small("Mother Fang")),
        pet: Some(PetFrameState {
            name: "Wolf".into(),
            health_fraction: 1.0,
            reaction: None,
            health_text: Default::default(),
            power: Some(PowerBarState {
                power: PowerType::Focus,
                current: 100,
                max: 100,
            }),
            power_text: Default::default(),
        }),
        bosses: vec![unit("Edwin VanCleef", PowerType::Mana)],
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}

fn cast_bar(skin: ActiveSkin, state: CastingBarState) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(casting_bar_frame_screen).sync(&shared, &mut registry);
    registry
}

fn casting(is_channel: bool) -> CastingBarState {
    CastingBarState {
        visible: true,
        spell_name: "Fireball".into(),
        timer_text: "1.5".into(),
        progress: 0.5,
        is_channel,
        ..Default::default()
    }
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

/// Every attribute the components set, one line per frame in tree order.
fn dump(registry: &FrameRegistry, root: &str) -> String {
    let mut out = String::new();
    dump_frame(
        registry,
        registry.get_by_name(root).expect(root),
        0,
        &mut out,
    );
    out
}

fn dump_frame(registry: &FrameRegistry, id: u64, depth: usize, out: &mut String) {
    let f = registry.get(id).unwrap();
    writeln!(
        out,
        "{:indent$}{} {}",
        "",
        placement(f),
        appearance(f),
        indent = depth * 2
    )
    .unwrap();
    for child in &f.children {
        dump_frame(registry, *child, depth + 1, out);
    }
}

fn placement(f: &Frame) -> String {
    format!(
        "{:?} {:?} w={:?} h={:?} pos={:?} {:?} margin={:?} translate={:?} hidden={}",
        f.name,
        f.widget_type,
        f.width,
        f.height,
        f.position,
        f.position_type,
        f.margin,
        f.translation,
        f.hidden,
    )
}

fn appearance(f: &Frame) -> String {
    format!(
        "strata={:?} level={} layer={:?} bg={:?} slice={:?} border={:?} style={:?} mouse={} \
         click={:?} data={:?}",
        f.strata,
        f.frame_level,
        f.draw_layer,
        f.background_color,
        f.nine_slice,
        f.border,
        f.panel_style,
        f.mouse_enabled,
        f.onclick,
        f.widget_data,
    )
}

fn modern_trees() -> String {
    let skin = ActiveSkin::Modern;
    [
        dump(&unit_frames(skin), "InWorldUnitFramesRoot"),
        dump(&cast_bar(skin, casting(false)), "PlayerCastingBarFrame"),
        dump(&cast_bar(skin, casting(true)), "PlayerCastingBarFrame"),
    ]
    .concat()
}

/// The Modern trees as the components built them before the Forever branch (forever5,
/// 43cde936).
#[test]
fn modern_unit_frames_and_cast_bar_keep_their_frame_trees() {
    let trees = modern_trees();
    if std::env::var_os("PRINT_MODERN_TREES").is_some() {
        println!("<<<MODERN_TREES\n{trees}MODERN_TREES>>>");
    }
    assert!(trees == fixture::MODERN_TREES, "Modern frame trees changed");
}

fn texture_source(registry: &FrameRegistry, name: &str) -> (TextureSource, [f32; 4]) {
    let Some(WidgetData::Texture(texture)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name} is not a Texture");
    };
    (texture.source.clone(), texture.vertex_color)
}

fn fixed_rect(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let f = frame(registry, name);
    let (Dimension::Fixed(width), Dimension::Fixed(height)) = (f.width, f.height) else {
        panic!("{name} is not fixed-size");
    };
    let px = |value: ui_toolkit::layout_values::Val| match value {
        ui_toolkit::layout_values::Val::Px(px) => px,
        other => panic!("{name} is placed at {other:?}"),
    };
    (px(f.position.left), px(f.position.top), width, height)
}

/// `Interface\Tooltips\UI-Tooltip-Border` in FlareUI's fixed bronze `ns.BORDER_COLOR`
/// #A67D45 (`Core.lua:8`).
const BRONZE_BORDER: (u32, [f32; 4]) = (137_057, [0.65, 0.49, 0.27, 1.0]);

fn colour_frames(target_reaction: Reaction, target_class: Option<u8>) -> FrameRegistry {
    load_atlas_tables();
    let mut player = unit("Shot", PowerType::Mana);
    player.class_id = Some(2);
    player.level_text = "1".into();
    player.power.as_mut().unwrap().current = 78;
    let mut target = unit("Stormwind Army Registrar", PowerType::Rage);
    target.reaction = Some(target_reaction);
    target.class_id = target_class;
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Forever);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        target_cast: None,
        target_of_target: Some(SmallUnitFrameState::from(&player)),
        focus: Some(SmallUnitFrameState::from(&target)),
        player,
        target: Some(target),
        pet: Some(PetFrameState {
            name: "Wolf".into(),
            health_fraction: 0.5,
            reaction: Some(target_reaction),
            health_text: Default::default(),
            power: None,
            power_text: Default::default(),
        }),
        bosses: vec![],
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}

fn label<'a>(
    registry: &'a FrameRegistry,
    name: &str,
) -> &'a ui_toolkit::widgets::font_string::FontStringData {
    let Some(WidgetData::FontString(text)) = frame(registry, name).widget_data.as_ref() else {
        panic!("{name} is not a FontString");
    };
    text
}

#[test]
fn forever_players_use_unmodified_retail_class_colours() {
    let registry = colour_frames(Reaction::Hostile, Some(8));
    // GetHealthColor returns C_ClassColor:GetRGB directly, no multiplier (:254-261).
    for (name, rgb) in [
        ("PlayerHealthBarFill", [0.96, 0.55, 0.73, 1.0]),
        ("TargetOfTargetHealthBarFill", [0.96, 0.55, 0.73, 1.0]),
        ("TargetHealthBarFill", [0.25, 0.78, 0.92, 1.0]),
        ("FocusHealthBarFill", [0.25, 0.78, 0.92, 1.0]),
    ] {
        assert_eq!(frame(&registry, name).background_color, Some(rgb), "{name}");
    }
}

#[test]
fn forever_npcs_use_flareui_reaction_colours_not_saturated_selection_colours() {
    // UnitFrames.lua:63-72,254-266: NPCs, including pets, use reaction, not owner class.
    for (reaction, rgb) in [
        (Reaction::Friendly, [0.30, 0.78, 0.30, 1.0]),
        (Reaction::Hostile, [0.87, 0.27, 0.27, 1.0]),
        (Reaction::Neutral, [0.93, 0.78, 0.25, 1.0]),
    ] {
        let registry = colour_frames(reaction, None);
        for name in [
            "TargetHealthBarFill",
            "FocusHealthBarFill",
            "PetFrameHealthBarFill",
        ] {
            assert_eq!(frame(&registry, name).background_color, Some(rgb), "{name}");
        }
    }
}

#[test]
fn forever_names_and_percentages_are_white_with_outline_and_black_shadow() {
    let registry = colour_frames(Reaction::Friendly, None);
    for (name, content, rgb) in [
        ("PlayerName", "Shot", [1.0, 1.0, 1.0, 1.0]),
        ("PlayerHealthBarText", "75%", [1.0, 1.0, 1.0, 1.0]),
        ("PlayerLevelText", "1", [1.0, 0.82, 0.0, 1.0]),
        (
            "TargetName",
            "Stormwind Army Registrar",
            [1.0, 1.0, 1.0, 1.0],
        ),
        ("TargetHealthBarText", "75%", [1.0, 1.0, 1.0, 1.0]),
        ("TargetLevelText", "60", [1.0, 0.82, 0.0, 1.0]),
    ] {
        let text = label(&registry, name);
        assert_eq!(text.text, content, "{name}");
        assert_eq!(text.color, rgb, "{name}");
        assert_eq!(text.font_size, 12.0, "{name}");
        assert_eq!(format!("{:?}", text.outline), "Outline", "{name}");
        assert_eq!(text.shadow_color, Some([0.0, 0.0, 0.0, 1.0]), "{name}");
        assert_eq!(text.shadow_offset, [1.0, -1.0], "{name}");
    }
}

#[test]
fn forever_player_power_shows_current_value_in_white_on_the_right() {
    let registry = colour_frames(Reaction::Friendly, None);
    let text = label(&registry, "PlayerManaBarText");
    assert_eq!(text.text, "78");
    assert_eq!(text.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(text.font_size, 10.0);
    assert_eq!(format!("{:?}", text.justify_h), "Right");
    assert_eq!(format!("{:?}", text.outline), "Outline");
    assert_eq!(text.shadow_color, Some([0.0, 0.0, 0.0, 1.0]));
    assert_eq!(text.shadow_offset, [1.0, -1.0]);
    assert!(!frame(&registry, "PlayerManaBarText").hidden);
    assert!(registry.get_by_name("TargetManaBarText").is_none());
}

#[test]
fn forever_partial_bars_show_dark_background_without_an_opaque_frame_backdrop() {
    let registry = colour_frames(Reaction::Friendly, None);
    // bar.bg (:240); outer backdrop BG_OPACITY=0 (:42,1768).
    for name in ["PlayerHealthBar", "PlayerManaBar", "TargetHealthBar"] {
        assert_eq!(
            frame(&registry, name).background_color,
            Some([0.15, 0.15, 0.15, 0.9])
        );
    }
    assert_eq!(fixed_rect(&registry, "PlayerHealthBarFill").2, 174.0);
    assert_eq!(fixed_rect(&registry, "TargetHealthBarFill").0, 58.0);
    for name in ["PlayerFrameBackground", "TargetFrameBackground"] {
        assert_eq!(texture_source(&registry, name).1, [0.0, 0.0, 0.0, 0.0]);
    }
}

#[test]
fn forever_player_frame_is_flareui_thin_frame_with_bronze_border() {
    let registry = unit_frames(ActiveSkin::Forever);
    let player = frame(&registry, "PlayerFrame");
    assert_eq!(
        (player.width, player.height),
        (Dimension::Fixed(240.0), Dimension::Fixed(60.0))
    );
    // INSET 4 (UnitFrames.lua:40,1705-1722): health above a 14-high power bar.
    assert_eq!(
        fixed_rect(&registry, "PlayerHealthBar"),
        (4.0, 4.0, 232.0, 38.0)
    );
    assert_eq!(
        fixed_rect(&registry, "PlayerManaBar"),
        (4.0, 42.0, 232.0, 14.0)
    );
    for piece in [
        "TopLeft",
        "TopRight",
        "BottomLeft",
        "BottomRight",
        "Top",
        "Left",
    ] {
        let (source, color) = texture_source(&registry, &format!("PlayerFrameBorder{piece}"));
        assert_eq!(
            (source, color),
            (TextureSource::FileDataId(BRONZE_BORDER.0), BRONZE_BORDER.1),
            "{piece}"
        );
    }
    // 16-px edge (`BORDER_SIZE`): corners 16×16 at the frame's corners.
    assert_eq!(
        fixed_rect(&registry, "PlayerFrameBorderBottomRight"),
        (224.0, 44.0, 16.0, 16.0)
    );
    let (background, _) = texture_source(&registry, "PlayerFrameBackground");
    assert_eq!(background, TextureSource::FileDataId(312_922));
    // Health as a percentage (`healthText = "percent"`), no portrait.
    let text = frame(&registry, "PlayerHealthBarText");
    let Some(WidgetData::FontString(text)) = text.widget_data.as_ref() else {
        panic!("PlayerHealthBarText is not a FontString");
    };
    assert_eq!(text.text, "75%");
    assert!(registry.get_by_name("PlayerPortrait").is_none());
    assert!(registry.get_by_name("PlayerFrameArt").is_none());
    // Target 240×60 without power, ToT 120×28, focus 160×36, pet 160×28.
    for (name, size) in [
        ("TargetFrame", (240.0, 60.0)),
        ("TargetOfTargetFrame", (120.0, 28.0)),
        ("FocusFrame", (160.0, 36.0)),
        ("PetFrame", (160.0, 28.0)),
    ] {
        let f = frame(&registry, name);
        assert_eq!(
            (f.width, f.height),
            (Dimension::Fixed(size.0), Dimension::Fixed(size.1)),
            "{name}"
        );
    }
    assert!(registry.get_by_name("TargetManaBar").is_none());
}

#[test]
fn forever_cast_bar_is_flareui_steel_blue_292_by_26_bar() {
    let registry = cast_bar(ActiveSkin::Forever, casting(false));
    // `playerCastbar` 292×26 inside its INSET-4 holder (`Core.lua:281`,
    // UnitFrames.lua:2224-2227).
    let bar = frame(&registry, "CastingBarBackground");
    assert_eq!(
        (bar.width, bar.height),
        (Dimension::Fixed(292.0), Dimension::Fixed(26.0))
    );
    let holder = frame(&registry, "PlayerCastingBarFrame");
    assert_eq!(
        (holder.width, holder.height),
        (Dimension::Fixed(326.0), Dimension::Fixed(34.0))
    );
    // `PLAYER_CAST_COLOR` #5C8FC7 (UnitFrames.lua:78).
    let fill = frame(&registry, "CastingBarFill");
    assert_eq!(fill.background_color, Some([0.36, 0.56, 0.78, 1.0]));
    let (source, color) = texture_source(&registry, "PlayerCastingBarFrameBorderTopLeft");
    assert_eq!(
        (source, color),
        (TextureSource::FileDataId(BRONZE_BORDER.0), BRONZE_BORDER.1)
    );
    // `PLAYER_CAST_CHANNEL` #80BFE0 for channels.
    let channel = cast_bar(ActiveSkin::Forever, casting(true));
    let fill = frame(&channel, "CastingBarFill");
    assert_eq!(fill.background_color, Some([0.50, 0.75, 0.88, 1.0]));
}

/// `root`'s visible frames in the order the client paints them: a frame's z is its strata,
/// frame level and draw layer (godot/rust/src/ui/projection.rs `update_node`), and Godot
/// paints equal z in tree order.
fn paint_order<'a>(registry: &'a FrameRegistry, root: &str) -> Vec<&'a Frame> {
    fn visit<'a>(registry: &'a FrameRegistry, id: u64, out: &mut Vec<&'a Frame>) {
        let f = registry.get(id).unwrap();
        if f.visible {
            out.push(f);
        }
        for child in &f.children {
            visit(registry, *child, out);
        }
    }
    let mut frames = Vec::new();
    visit(
        registry,
        registry.get_by_name(root).expect(root),
        &mut frames,
    );
    frames.sort_by_key(|f| {
        i32::from(f.strata as u8) * 100 + f.frame_level + i32::from(f.draw_layer as u8)
    });
    frames
}

/// Asserts `root` paints its bar fills, then its bronze border, then its texts.
fn assert_fills_under_border_under_texts(registry: &FrameRegistry, root: &str) {
    let order = paint_order(registry, root);
    let name = |f: &Frame| f.name.clone().unwrap_or_default();
    let indices = |pick: &dyn Fn(&Frame) -> bool| -> Vec<usize> {
        (0..order.len()).filter(|&at| pick(order[at])).collect()
    };
    let fills = indices(&|f| name(f).ends_with("Fill"));
    let border = indices(&|f| f.widget_type == WidgetType::Texture && name(f).contains("Border"));
    let texts = indices(&|f| f.widget_type == WidgetType::FontString);
    let painted: Vec<String> = order.iter().map(|f| name(f)).collect();
    assert!(!fills.is_empty(), "{root} has no fill: {painted:?}");
    // Edges a short frame has no room for are hidden; the four corners always show.
    assert!(border.len() >= 4, "{root} border pieces: {painted:?}");
    assert!(!texts.is_empty(), "{root} has no text: {painted:?}");
    assert!(
        fills.last() < border.first(),
        "{root} paints a fill over its border: {painted:?}"
    );
    assert!(
        border.last() < texts.first(),
        "{root} paints its border over a text: {painted:?}"
    );
}

/// FlareUI raises a unit frame's border and texts above its bars: bars at level +1, the
/// `Border` frame at +4, the `Overlay` frame holding the texts at +5
/// (UnitFrames.lua:2084-2106); a cast bar's `Border` at +2 and text overlay at +3
/// (UnitFrames.lua:495-509).
#[test]
fn forever_frames_paint_fills_under_the_border_under_the_texts() {
    let registry = unit_frames(ActiveSkin::Forever);
    for root in [
        "PlayerFrame",
        "TargetFrame",
        "TargetOfTargetFrame",
        "FocusFrame",
        "PetFrame",
    ] {
        assert_fills_under_border_under_texts(&registry, root);
    }
    let cast = cast_bar(ActiveSkin::Forever, casting(false));
    assert_fills_under_border_under_texts(&cast, "PlayerCastingBarFrame");
}

/// `UI-HUD-UnitFrame-SmallCircle` has only a Forever set-1 member (UiTextureAtlasMember,
/// no Retail row): under Modern it is art the data lacks.
#[test]
fn atlas_without_a_member_for_the_skin_is_an_error_and_draws_nothing() {
    load_atlas_tables();
    let error = atlas_size("UI-HUD-UnitFrame-SmallCircle", ActiveSkin::Modern).unwrap_err();
    assert!(error.contains("UI-HUD-UnitFrame-SmallCircle"), "{error}");
    assert!(atlas_size("UI-HUD-UnitFrame-SmallCircle", ActiveSkin::Forever).is_ok());

    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(|_| {
        [
            sized_atlas_texture(
                "MissingCircle".into(),
                "UI-HUD-UnitFrame-SmallCircle",
                ActiveSkin::Modern,
                |(width, height)| (0.0, 0.0, width, height),
                "1.0,1.0,1.0,1.0",
                false,
            ),
            sized_atlas_texture(
                "PresentIcon".into(),
                "UI-HUD-UnitFrame-Player-CombatIcon",
                ActiveSkin::Modern,
                |(width, height)| (0.0, 0.0, width, height),
                "1.0,1.0,1.0,1.0",
                false,
            ),
        ]
        .into_iter()
        .flatten()
        .collect()
    })
    .sync(&SharedContext::new(), &mut registry);
    assert!(registry.get_by_name("MissingCircle").is_none());
    let icon = frame(&registry, "PresentIcon");
    assert_eq!(
        (icon.width, icon.height),
        (Dimension::Fixed(16.0), Dimension::Fixed(16.0))
    );
}
