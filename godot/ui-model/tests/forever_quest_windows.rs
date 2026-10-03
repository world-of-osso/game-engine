//! Quest chrome resolves by name; Modern trees retain their base 818e14b8 bytes.
#[path = "fixtures/modern_quest_options_menu_trees.rs"]
mod fixture;
use std::fmt::Write;
use std::path::PathBuf;

use game_engine_ui_model::client_options_data::{
    CameraOptionsFile, GraphicsOptionsFile, HudOptionsFile, SoundOptionsFile,
};
use game_engine_ui_model::game_menu_component::{GameMenuView, game_menu_screen};
use game_engine_ui_model::input_bindings_data::{BindingSection, InputBindingsData};
use game_engine_ui_model::options_menu_component::OptionsCategory;
use game_engine_ui_model::options_menu_data::*;
use game_engine_ui_model::panel_style_data::{MetalTopLeft, metal_frame_style};
use game_engine_ui_model::quest_frame_component::{
    QuestFramePage, QuestFrameState, RewardView, quest_frame_screen,
};
use game_engine_ui_model::quest_log_frame_component::{QuestLogFrameState, quest_log_frame_screen};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region, set_active_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

fn load_tables() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn dump_frame(registry: &FrameRegistry, id: u64, out: &mut String) {
    let f = registry.get(id).unwrap();
    writeln!(out, "{:?} {:?} size={:?},{:?} pos={:?},{:?} anchor={:?} translate={:?} margin={:?} layout={:?} visibility={},{} alpha={},{} scale={},{} strata={:?} level={} raise={} layer={:?},{} input={},{},{:?} bg={:?} backdrop={:?} nine={:?} three={:?} border={:?} style={:?},{:?} behavior={},{},{} click={:?} flex={:?} data={:?}",
        f.name, f.widget_type, f.width, f.height, f.position, f.position_type, f.anchor,
        f.translation, f.margin, f.layout_rect, f.hidden, f.visible, f.alpha, f.effective_alpha,
        f.scale, f.effective_scale, f.strata, f.frame_level, f.raise_order, f.draw_layer,
        f.draw_sub_layer, f.mouse_enabled, f.keyboard_enabled, f.hit_rect_insets,
        f.background_color, f.backdrop, f.nine_slice, f.three_slice, f.border,
        f.panel_style, f.three_slice_style, f.clamped_to_screen, f.movable, f.resizable,
        f.onclick, f.flex_layout, f.widget_data).unwrap();
    for child in &f.children {
        dump_frame(registry, *child, out);
    }
}

fn registry<T: 'static>(state: T, screen: fn(&SharedContext) -> Element) -> FrameRegistry {
    let mut ctx = SharedContext::new();
    ctx.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    for corner in [MetalTopLeft::Portrait, MetalTopLeft::Plain] {
        registry.register_panel_style(
            corner.style_name(),
            metal_frame_style(TextureSource::Dynamic(DynamicTextureId(1))),
        );
    }
    Screen::new(screen).sync(&ctx, &mut registry);
    registry
}

fn dump<T: 'static>(state: T, screen: fn(&SharedContext) -> Element, root: &str) -> String {
    let registry = registry(state, screen);
    let mut out = String::new();
    dump_frame(&registry, registry.get_by_name(root).unwrap(), &mut out);
    out
}

fn options_model() -> OptionsModel {
    let graphics = graphics_draft_from_file(&GraphicsOptionsFile::default());
    let sound = sound_draft_from_file(&SoundOptionsFile::default());
    let camera = camera_draft_from_file(&CameraOptionsFile::default());
    let hud = hud_draft_from_file(&HudOptionsFile::default());
    OptionsModel {
        logged_in: true,
        view: GameMenuView::Options,
        category: OptionsCategory::Graphics,
        modal_position: [0.0, 0.0],
        draft_graphics: graphics.clone(),
        committed_graphics: graphics,
        draft_sound: sound.clone(),
        committed_sound: sound,
        draft_camera: camera.clone(),
        committed_camera: camera,
        draft_hud: hud.clone(),
        committed_hud: hud,
        draft_bindings: InputBindingsData::default(),
        committed_bindings: InputBindingsData::default(),
        binding_section: BindingSection::Movement,
        binding_capture: BindingCapture::None,
        active_layout: "Modern".into(),
    }
}

fn modern_trees() -> String {
    let mut out = String::new();
    for page in [
        QuestFramePage::Greeting {
            text: "Welcome, traveler.".into(),
            options: vec![],
            quests: vec![],
        },
        QuestFramePage::Detail {
            title: "A Threat Within".into(),
            description: "Defend Northshire.".into(),
            objectives_text: "Speak to McBride.".into(),
            rewards: RewardView::default(),
        },
        QuestFramePage::Progress {
            title: "A Threat Within".into(),
            text: "Have you finished?".into(),
            required: vec![],
            can_complete: true,
        },
        QuestFramePage::Reward {
            title: "A Threat Within".into(),
            text: "Well done.".into(),
            rewards: RewardView::default(),
        },
    ] {
        out += &dump(
            QuestFrameState {
                visible: true,
                npc_name: "Marshal McBride".into(),
                page,
            },
            quest_frame_screen,
            "QuestFrame",
        );
    }
    out += &dump(
        QuestLogFrameState {
            visible: true,
            ..Default::default()
        },
        quest_log_frame_screen,
        "QuestLogFrame",
    );
    let mut view = build_view_model(&options_model());
    for logged_in in [false, true] {
        view.logged_in = logged_in;
        view.view = GameMenuView::MainMenu;
        out += &dump(view.clone(), game_menu_screen, "GameMenuRoot");
    }
    view.view = GameMenuView::Options;
    for category in OptionsCategory::ALL {
        view.options.category = category;
        out += &dump(view.clone(), game_menu_screen, "GameMenuRoot");
    }
    out
}

fn assert_region(name: &str, skin: ActiveSkin, fdid: u32, size: [u32; 2], rect: [f32; 4]) {
    let region = resolve_region(name, skin).unwrap();
    assert_eq!(
        region.source,
        AtlasSource::FileDataId(fdid),
        "{name} {skin:?}"
    );
    let actual = region.rect_pixels(size[0], size[1]);
    assert_eq!(
        [actual.min[0], actual.max[0], actual.min[1], actual.max[1]],
        rect,
        "{name} {skin:?}"
    );
}

fn assert_close_texture(skin: ActiveSkin, fdid: u32, coords: [f32; 4]) {
    set_active_skin(skin);
    for (root, registry) in [
        (
            "QuestFrame",
            registry(QuestFrameState::default(), quest_frame_screen),
        ),
        (
            "QuestLogFrame",
            registry(QuestLogFrameState::default(), quest_log_frame_screen),
        ),
    ] {
        let frame = registry
            .get(
                registry
                    .get_by_name(&format!("{root}CloseButtonNormal"))
                    .unwrap(),
            )
            .unwrap();
        let Some(WidgetData::Texture(data)) = &frame.widget_data else {
            panic!("expected texture")
        };
        assert_eq!(
            data.source,
            TextureSource::FileDataId(fdid),
            "{root} {skin:?}"
        );
        assert_eq!(data.tex_coords, coords, "{root} {skin:?}");
    }
}

#[test]
#[ignore = "base fixture capture only"]
fn capture_base_trees() {
    load_tables();
    set_active_skin(ActiveSkin::Modern);
    println!("BEGIN_BASE_TREES\n{}END_BASE_TREES", modern_trees());
    println!(
        "Modern close {:?}",
        resolve_region("RedButton-Exit", ActiveSkin::Modern)
    );
    println!(
        "Forever close {:?}",
        resolve_region("RedButton-Exit", ActiveSkin::Forever)
    );
}

#[test]
fn quest_chrome_preserves_modern_and_resolves_forever() {
    load_tables();
    set_active_skin(ActiveSkin::Modern);
    assert_eq!(modern_trees().as_bytes(), fixture::MODERN_TREES.as_bytes());
    assert_region(
        "RedButton-Exit",
        ActiveSkin::Modern,
        5_262_907,
        [128, 64],
        [21.0, 39.0, 1.0, 20.0],
    );
    assert_region(
        "RedButton-Exit",
        ActiveSkin::Forever,
        8_107_307,
        [256, 128],
        [35.0, 67.0, 1.0, 33.0],
    );
    assert_close_texture(
        ActiveSkin::Modern,
        5_262_907,
        [21.0 / 128.0, 39.0 / 128.0, 1.0 / 64.0, 20.0 / 64.0],
    );
    assert_close_texture(
        ActiveSkin::Forever,
        8_107_307,
        [35.0 / 256.0, 67.0 / 256.0, 1.0 / 128.0, 33.0 / 128.0],
    );
    set_active_skin(ActiveSkin::Modern);
}
