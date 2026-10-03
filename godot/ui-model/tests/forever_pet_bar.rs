use std::fmt::Write;
use std::path::PathBuf;

use game_engine_ui_model::pet_action_bar_component::{
    PetActionBarState, PetActionButtonView, PetAutocast, apply_pet_action_bar_postsetup,
    pet_action_bar_screen,
};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

fn state() -> PetActionBarState {
    PetActionBarState {
        visible: true,
        buttons: std::array::from_fn(|i| PetActionButtonView {
            icon_fdid: 132_152,
            checked: i == 0,
            checked_alpha: 0.5,
            flash: i == 0,
            hotkey: format!("c-{}", i + 1),
            pushed: i == 1,
            hovered: i == 2,
            autocast: if i == 3 {
                PetAutocast::On
            } else {
                PetAutocast::Off
            },
        }),
        shine_texture: None,
    }
}

fn setup(skin: ActiveSkin) -> (SharedContext, Screen, FrameRegistry) {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut screen = Screen::new(pet_action_bar_screen);
    screen.sync(&shared, &mut registry);
    apply_pet_action_bar_postsetup(&state(), &mut registry);
    (shared, screen, registry)
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

/// Normalize named and hand-copied regions to the sheet pixels actually drawn.
fn crop(texture: &TextureData, skin: ActiveSkin) -> (u32, [f32; 4]) {
    let (fdid, bounds) = match &texture.source {
        TextureSource::FileDataId(fdid) => (*fdid, [0.0, 1.0, 0.0, 1.0]),
        TextureSource::Atlas(name) => {
            let region = resolve_region(name, skin).expect(name);
            let AtlasSource::FileDataId(fdid) = region.source else {
                panic!("not a sheet")
            };
            (fdid, [region.left, region.right, region.top, region.bottom])
        }
        other => panic!("not sheet art: {other:?}"),
    };
    let (w, h) = match fdid {
        4_613_342 => (256.0, 1024.0),
        7_948_328 => (512.0, 512.0),
        5_199_404 => (2048.0, 1024.0),
        132_152 => (64.0, 64.0),
        other => panic!("unknown sheet {other}"),
    };
    let [u0, u1, v0, v1] = texture.tex_coords;
    let [l, r, t, b] = bounds;
    let round = |v: f32| (v * 1000.0).round() / 1000.0;
    (
        fdid,
        [
            round((l + u0 * (r - l)) * w),
            round((l + u1 * (r - l)) * w),
            round((t + v0 * (b - t)) * h),
            round((t + v1 * (b - t)) * h),
        ],
    )
}

fn dump_frame(registry: &FrameRegistry, id: u64, skin: ActiveSkin, out: &mut String) {
    let f = registry.get(id).unwrap();
    let data = match f.widget_data.as_ref() {
        Some(WidgetData::Texture(t)) if t.source != TextureSource::None => format!(
            "crop={:?} tile={},{} blend={:?} color={:?} desat={},{} rot={}",
            crop(t, skin),
            t.horiz_tile,
            t.vert_tile,
            t.blend_mode,
            t.vertex_color,
            t.desaturated,
            t.desaturation,
            t.rotation
        ),
        other => format!("{other:?}"),
    };
    writeln!(out, "{:?} {:?} w={:?} h={:?} pos={:?} {:?} margin={:?} translate={:?} hidden={} alpha={} strata={:?} level={} layer={:?} bg={:?} mouse={} click={:?} {data}",
        f.name, f.widget_type, f.width, f.height, f.position, f.position_type, f.margin,
        f.translation, f.hidden, f.alpha, f.strata, f.frame_level, f.draw_layer,
        f.background_color, f.mouse_enabled, f.onclick).unwrap();
    for child in &f.children {
        dump_frame(registry, *child, skin, out);
    }
}

fn dump(registry: &FrameRegistry) -> String {
    let mut out = String::new();
    dump_frame(
        registry,
        registry.get_by_name("PetActionBar").unwrap(),
        ActiveSkin::Modern,
        &mut out,
    );
    out
}

/// Screen-space rect on the concrete 1920x1080 test viewport.
fn rect(registry: &FrameRegistry) -> (f32, f32, f32, f32) {
    let f = frame(registry, "PetActionBar");
    let Dimension::Fixed(w) = f.width else {
        panic!("width")
    };
    let Dimension::Fixed(h) = f.height else {
        panic!("height")
    };
    let Val::Percent(50.0) = f.position.left else {
        panic!("left")
    };
    let Val::Px(x) = f.margin.left else {
        panic!("margin")
    };
    let Val::Px(bottom) = f.position.bottom else {
        panic!("bottom")
    };
    (960.0 + x, 1080.0 - bottom - h, w, h)
}

fn assert_rect(registry: &FrameRegistry, expected: (f32, f32, f32, f32)) {
    let actual = rect(registry);
    for (a, b) in [
        (actual.0, expected.0),
        (actual.1, expected.1),
        (actual.2, expected.2),
        (actual.3, expected.3),
    ] {
        assert!((a - b).abs() < 0.001, "{actual:?} != {expected:?}");
    }
}

#[test]
fn modern_pet_bar_matches_base_fixture() {
    let (_, _, registry) = setup(ActiveSkin::Modern);
    assert_eq!(dump(&registry), include_str!("fixtures/modern_pet_bar.txt"));
    assert_rect(&registry, (679.0, 955.0, 318.0, 30.0));
}

#[test]
fn forever_pet_bar_follows_main_bar_bottom_left_with_camelot_indent_and_gap() {
    let (_, _, registry) = setup(ActiveSkin::Forever);
    // Main BOTTOMRIGHT = (907.5,1078), width 595.72, height 47.7;
    // pet BOTTOMLEFT on main BOTTOMLEFT +(30,51.7), per Camelot's stack.
    // FlareUI Core.lua:102-107,218 and ActionBars.lua:43,179-190 scale pet by 1.06.
    assert_rect(&registry, (341.78, 994.5, 337.08, 31.8));
}

#[test]
fn switching_preset_moves_the_mounted_pet_bar_and_restores_modern() {
    let (mut shared, mut screen, mut registry) = setup(ActiveSkin::Modern);
    assert_rect(&registry, (679.0, 955.0, 318.0, 30.0));
    shared.insert(ActiveSkin::Forever);
    screen.sync(&shared, &mut registry);
    assert_rect(&registry, (341.78, 994.5, 337.08, 31.8));
    shared.insert(ActiveSkin::Modern);
    screen.sync(&shared, &mut registry);
    assert_rect(&registry, (679.0, 955.0, 318.0, 30.0));
}
