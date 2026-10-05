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
