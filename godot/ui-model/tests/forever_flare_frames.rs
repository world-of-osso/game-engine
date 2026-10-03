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
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
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
        health_fraction: 0.5,
        reaction: Some(Reaction::Neutral),
    }
}

fn unit_frames(skin: ActiveSkin) -> FrameRegistry {
    load_atlas_tables();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
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
