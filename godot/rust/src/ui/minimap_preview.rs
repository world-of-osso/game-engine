//! Offline minimap capture through the production screens, without a server.
use game_engine_ui_model::minimap::{
    MinimapClusterState, apply_minimap_postsetup, minimap_cluster_screen,
};
use game_engine_ui_model::objective_tracker_component::objective_tracker_screen;
use godot::prelude::*;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;

use super::{RegistryModel, RegistryUi, ScreenPostsetup};

fn preview_screen(ctx: &SharedContext) -> Element {
    let mut elements = minimap_cluster_screen(ctx);
    elements.extend(objective_tracker_screen(ctx));
    elements
}

impl RegistryUi {
    pub(super) fn initialize_minimap_preview(&mut self) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        super::party_preview::load_data_root()?;
        ui_toolkit::atlas::set_thread_skin(ActiveSkin::Forever);
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut model = load_preview_model(size.x, size.y)?;
        model.sync_skin(ActiveSkin::Forever);
        let state = model.shared.get::<MinimapClusterState>().unwrap().clone();
        apply_minimap_postsetup(&state, &mut model.registry);
        self.initialize_model(model, size.x, size.y)
    }
}

fn load_preview_model(width: f32, height: f32) -> Result<RegistryModel, String> {
    let mut registry = FrameRegistry::new(width, height);
    let tile = super::assets::decode_blp("data/minimap/map32_48.blp")?;
    let map_texture = registry.create_dynamic_texture(tile.width, tile.height, tile.pixels)?;
    let mut shared = SharedContext::new();
    shared.insert(MinimapClusterState {
        zone_text: "Elwynn Forest".into(),
        clock_text: "12:00".into(),
        calendar_day: Some(6),
        map_texture: Some(map_texture),
        ..Default::default()
    });
    shared.insert(super::dungeon_preview::tracker());
    Ok(RegistryModel {
        screen: Screen::new(preview_screen),
        shared,
        registry,
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    })
}
