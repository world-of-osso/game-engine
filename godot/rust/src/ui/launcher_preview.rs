//! Offline native candidate capture, composing the production launcher and HUD screens.
use game_engine_ui_model::chat_frame_component::{ChatFrameView, chat_frame_screen};
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, UnitFrameState, inworld_unit_frames_screen,
};
use game_engine_ui_model::launcher::{
    LauncherIconStyle, LauncherView, SEARCH_FIELD, launcher_screen,
};
use game_engine_ui_model::minimap::{MinimapClusterState, minimap_cluster_screen};
use godot::prelude::*;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;

use super::{RegistryModel, RegistryUi, ScreenPostsetup};

fn candidate_screen(ctx: &SharedContext) -> Element {
    let mut elements = inworld_unit_frames_screen(ctx);
    elements.extend(chat_frame_screen(ctx));
    elements.extend(minimap_cluster_screen(ctx));
    elements.extend(launcher_screen(ctx));
    elements
}

fn candidate_context(style: LauncherIconStyle) -> SharedContext {
    let mut shared = SharedContext::new();
    shared.insert(style);
    shared.insert(LauncherView {
        open: true,
        ..Default::default()
    });
    shared.insert(MinimapClusterState {
        zone_text: "Elwynn Forest".into(),
        clock_text: "12:00".into(),
        ..Default::default()
    });
    shared.insert(candidate_player());
    shared.insert(ChatFrameView::default());
    shared
}

fn candidate_player() -> InWorldUnitFramesState {
    InWorldUnitFramesState {
        player: UnitFrameState::named("Launchpolish"),
        show_player_frame: true,
        show_target_frame: false,
        target_cast: None,
        target: None,
        target_of_target: None,
        focus: None,
        pet: None,
        bosses: Vec::new(),
        menu: Default::default(),
        personal_resource: None,
    }
}

fn register_candidate_panels(registry: &mut FrameRegistry) -> Result<(), String> {
    super::register_metal_frame_style(
        registry,
        game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
    )?;
    super::register_flare_bronze_style(registry, |fdid| {
        let image = super::assets::decode_blp(&format!("data/textures/{fdid}.blp"))?;
        Ok((image.pixels, image.width))
    })
}

impl RegistryUi {
    pub(super) fn initialize_launcher_candidate(
        &mut self,
        skin: ActiveSkin,
        style: LauncherIconStyle,
    ) -> Result<(), String> {
        if self.model.is_some() {
            return Err("RegistryUi already has a screen".into());
        }
        super::party_preview::load_data_root()?;
        ui_toolkit::atlas::set_active_skin(skin);
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut registry = FrameRegistry::new(size.x, size.y);
        register_candidate_panels(&mut registry)?;
        let mut model = RegistryModel {
            screen: Screen::new(candidate_screen),
            shared: candidate_context(style),
            registry,
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::None,
        };
        model.sync_skin(skin);
        self.initialize_model(model, size.x, size.y)?;
        self.focus_frame_named(SEARCH_FIELD)
    }
}
