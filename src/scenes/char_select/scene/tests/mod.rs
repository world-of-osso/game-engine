pub(super) use super::camera::{
    CHAR_SELECT_CAMERA_GROUND_CLEARANCE, camera_params, char_select_fog, char_select_orbit_camera,
    clamp_char_select_eye, orbit_eye, orbit_from_eye_focus, orbit_input_debug_state,
    should_log_orbit_input,
};
pub(super) use super::*;
pub(super) use crate::networking_auth::CharacterList;
pub(super) use bevy::app::App;
pub(super) use bevy::ecs::message::Messages;
pub(super) use bevy::ecs::system::RunSystemOnce;
pub(super) use bevy::input::keyboard::KeyboardInput;
pub(super) use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
pub(super) use bevy::pbr::{DistanceFog, FogFalloff};
pub(super) use bevy::state::app::StatesPlugin;
pub(super) use bevy::window::PrimaryWindow;
pub(super) use game_engine::ui::automation::UiAutomationPlugin;
pub(super) use game_engine::ui::event::EventBus;
pub(super) use game_engine::ui::plugin::UiState;
pub(super) use game_engine::ui::registry::FrameRegistry;
pub(super) use shared::components::{CharacterAppearance, EquipmentAppearance};
pub(super) use shared::protocol::CharacterListEntry;

mod camera_tests;
mod model_sync_tests;
mod render_path_tests;

pub(super) fn character(character_id: u64, race: u8, sex: u8, name: &str) -> CharacterListEntry {
    CharacterListEntry {
        character_id,
        name: name.to_string(),
        level: 1,
        race,
        class: 1,
        appearance: CharacterAppearance {
            sex,
            ..Default::default()
        },
        equipment_appearance: EquipmentAppearance::default(),
    }
}

pub(super) fn render_path_test_app() -> App {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    app.init_resource::<Assets<crate::sky_material::SkyMaterial>>();
    app.init_resource::<Assets<crate::m2_effect_material::M2EffectMaterial>>();
    app.init_resource::<Assets<crate::skybox_m2_material::SkyboxM2Material>>();
    app.init_resource::<Assets<crate::terrain_material::TerrainMaterial>>();
    app.init_resource::<Assets<crate::water_material::WaterMaterial>>();
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>();
    app.init_resource::<crate::terrain_heightmap::TerrainHeightmap>();
    app.init_resource::<scene_types::DisplayedCharacterId>();
    app.init_resource::<scene_types::PendingSupplementalWarbandScene>();
    app.init_resource::<scene_tree::ActiveWarbandSceneId>();
    app.insert_resource(crate::creature_display::CreatureDisplayMap);
    app.insert_resource(game_engine::customization_data::CustomizationDb::load(
        std::path::Path::new("data"),
    ));
    app.insert_resource(CharacterList(vec![character(6, 1, 0, "Theron")]));
    app.insert_resource(crate::scenes::char_select::SelectedCharIndex(Some(0)));
    app.insert_resource(crate::scenes::char_select::warband::WarbandScenes::load());
    app.insert_resource(crate::scenes::char_select::warband::SelectedWarbandScene { scene_id: 1 });
    let cloud_maps = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        crate::sky::cloud_texture::create_procedural_cloud_maps(&mut images)
    };
    app.insert_resource(cloud_maps);
    app
}
