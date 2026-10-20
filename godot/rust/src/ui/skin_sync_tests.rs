//! A canvas mirrors the active skin into its `SharedContext`, so a preset switch
//! rebuilds every Screen that read the skin.

use std::path::PathBuf;

use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::TextureSource;

use super::{RegistryModel, ScreenPostsetup};

const PORTRAIT: &str = "UI-HUD-UnitFrame-Player-PortraitOn";

/// The sheet `PORTRAIT` resolves to under `skin`.
fn skin_portrait_fdid(skin: ActiveSkin) -> u32 {
    let AtlasSource::FileDataId(fdid) = resolve_region(PORTRAIT, skin).unwrap().source else {
        panic!("{PORTRAIT} is DB2 art");
    };
    fdid
}

/// A player portrait drawn from the sheet its atlas name resolves to under the skin.
fn portrait_screen(ctx: &SharedContext) -> Element {
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let fdid = skin_portrait_fdid(skin);
    rsx! {
        texture {
            name: "SkinPortrait",
            width: 198.0,
            height: 71.0,
            texture_fdid: fdid,
        }
    }
}

fn portrait_fdid(model: &RegistryModel) -> u32 {
    let id = model.registry.get_by_name("SkinPortrait").unwrap();
    let Some(WidgetData::Texture(texture)) = &model.registry.get(id).unwrap().widget_data else {
        panic!("SkinPortrait is a texture");
    };
    let TextureSource::FileDataId(fdid) = texture.source else {
        panic!("SkinPortrait source {:?}", texture.source);
    };
    fdid
}

/// Production order: `apply_ui_layout` sets the thread skin, then resyncs every canvas.
fn apply_layout_skin(model: &mut RegistryModel, skin: ActiveSkin) {
    ui_toolkit::atlas::set_thread_skin(skin);
    model.sync_skin(skin);
}

/// A Forever CharacterFrame canvas survives into a glue screen, which draws Modern
/// (forced disconnect to Login): the frame must rebuild before Modern chrome postsetup.
#[test]
fn character_frame_rebuilds_modern_chrome_after_forever_layout_switch() {
    use game_engine_ui_model::character_frame::{CharacterFrameView, character_frame_screen};
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(CharacterFrameView {
        visible: true,
        ..CharacterFrameView::default()
    });
    let mut model = RegistryModel {
        screen: Screen::new(character_frame_screen),
        shared,
        registry: FrameRegistry::new(1920.0, 1080.0),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::CharacterFrame,
    };
    apply_layout_skin(&mut model, ActiveSkin::Forever);
    assert!(
        model
            .registry
            .get_by_name("CharacterFrameTopTileStreaks")
            .is_none(),
        "Forever uses the portrait border, not Modern window chrome"
    );

    apply_layout_skin(&mut model, ActiveSkin::Modern);
    let id = model
        .registry
        .get_by_name("CharacterFrameTopTileStreaks")
        .expect("Modern layout rebuilds CharacterFrame window chrome");
    let Some(WidgetData::Texture(texture)) = &model.registry.get(id).unwrap().widget_data else {
        panic!("CharacterFrameTopTileStreaks is a texture");
    };
    let AtlasSource::FileDataId(streaks) =
        resolve_region("_UI-Frame-TopTileStreaks", ActiveSkin::Modern)
            .unwrap()
            .source
    else {
        panic!("_UI-Frame-TopTileStreaks is DB2 art");
    };
    assert_eq!(texture.source, TextureSource::FileDataId(streaks));
}

#[test]
fn switching_skin_rebuilds_screens_that_read_it_with_forever_art() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut model = RegistryModel {
        screen: Screen::new(portrait_screen),
        shared: SharedContext::new(),
        registry: FrameRegistry::new(1280.0, 720.0),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    model.sync_skin(ActiveSkin::Modern);
    let modern_generation = model.shared.generation::<ActiveSkin>();
    let modern_art = skin_portrait_fdid(ActiveSkin::Modern);
    assert_eq!(portrait_fdid(&model), modern_art);

    model.sync_skin(ActiveSkin::Modern);
    assert_eq!(model.shared.generation::<ActiveSkin>(), modern_generation);

    model.sync_skin(ActiveSkin::Forever);
    assert!(model.shared.generation::<ActiveSkin>() > modern_generation);
    assert_eq!(
        portrait_fdid(&model),
        skin_portrait_fdid(ActiveSkin::Forever)
    );
    assert_ne!(
        portrait_fdid(&model),
        modern_art,
        "Forever draws its own art"
    );
}
