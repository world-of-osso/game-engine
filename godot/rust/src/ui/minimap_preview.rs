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

fn boss_preview_screen(ctx: &SharedContext) -> Element {
    let mut elements = preview_screen(ctx);
    elements.extend(
        game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_screen(ctx),
    );
    elements
}

impl RegistryUi {
    fn initialize_boss_preview(&mut self, skin: ActiveSkin) -> Result<(), String> {
        use game_engine_ui_model::inworld_unit_frames_component::{
            InWorldUnitFramesState, UnitFrameState,
        };
        use game_engine_ui_model::objective_tracker_component::BossFrameCount;
        super::party_preview::load_data_root()?;
        ui_toolkit::atlas::set_thread_skin(skin);
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("RegistryUi has no viewport")?;
        let size = viewport.get_visible_rect().size;
        let mut model = load_preview_model(size.x, size.y)?;
        model.screen = Screen::new(boss_preview_screen);
        model.shared.insert(InWorldUnitFramesState {
            bosses: ["Hogger", "Lord Overheat"]
                .into_iter()
                .map(|name| UnitFrameState {
                    health_fraction: 0.75,
                    level_text: "60".into(),
                    reaction: Some(game_engine_ui_model::faction_reaction::Reaction::Hostile),
                    power: Some(
                        game_engine_ui_model::inworld_unit_frames_component::PowerBarState {
                            power: shared::components::PowerType::Mana,
                            current: 40,
                            max: 100,
                        },
                    ),
                    ..UnitFrameState::named(name)
                })
                .collect(),
            ..Default::default()
        });
        model.shared.insert(BossFrameCount(2));
        model.sync_skin(skin);
        let state = model.shared.get::<MinimapClusterState>().unwrap().clone();
        apply_minimap_postsetup(&state, &mut model.registry);
        self.initialize_model(model, size.x, size.y)
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn show_bosslayout_preview(&mut self) -> GString {
        GString::from(
            self.initialize_boss_preview(ActiveSkin::Modern)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    pub fn show_forever_bosslayout_preview(&mut self) -> GString {
        GString::from(
            self.initialize_boss_preview(ActiveSkin::Forever)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    /// Offline Forever minimap chrome, badge and tracker for capture_ui_screen.gd.
    #[func]
    pub fn show_forever_minimap_preview(&mut self) -> GString {
        GString::from(
            self.initialize_minimap_preview()
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }
}
