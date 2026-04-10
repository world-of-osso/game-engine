use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationDb, ModelPresentation};
use game_engine::outfit_data::OutfitData;
use shared::protocol::CharacterListEntry;

use crate::creature_display;
use crate::equipment::EquipmentItem;
use crate::m2_effect_material::M2EffectMaterial;
use crate::networking_auth::CharacterList;
use crate::scenes::char_select::SelectedCharIndex;
use crate::scenes::char_select::scene::camera;
use crate::scenes::char_select::scene_tree::ActiveWarbandSceneId;
use crate::scenes::char_select::warband::{
    SelectedWarbandScene, WarbandSceneEntry, WarbandScenePlacement, WarbandScenes,
};
use crate::sky::cloud_texture::ProceduralCloudMaps;
use crate::sky_material::SkyMaterial;
use crate::skybox_m2_material::SkyboxM2Material;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::water_material::WaterMaterial;
use game_engine::scene_tree::SceneTree;

use super::{
    CharSelectModelCharacter, CharSelectModelRoot, CharSelectModelWrapper, CharSelectScene,
};

#[derive(Resource, Default)]
pub(super) struct DisplayedCharacterId(pub(super) Option<u64>);

#[derive(Resource, Default)]
pub(super) struct DisplayedCharacterAppearance(pub(super) Option<AppliedCharacterAppearance>);

#[derive(Resource, Default)]
pub(super) struct PendingSupplementalWarbandScene {
    pub(super) scene_id: Option<u32>,
    pub(super) wait_for_next_frame: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AppliedCharacterAppearance {
    pub(super) character_id: u64,
    pub(super) race: u8,
    pub(super) class: u8,
    pub(super) appearance: shared::components::CharacterAppearance,
    pub(super) equipment_appearance: shared::components::EquipmentAppearance,
}

#[derive(SystemParam)]
pub(super) struct CharSelectRenderAssets<'w> {
    pub(super) meshes: ResMut<'w, Assets<Mesh>>,
    pub(super) materials: ResMut<'w, Assets<StandardMaterial>>,
    pub(super) sky_materials: ResMut<'w, Assets<SkyMaterial>>,
    pub(super) effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    pub(super) skybox_materials: ResMut<'w, Assets<SkyboxM2Material>>,
    pub(super) terrain_materials: ResMut<'w, Assets<TerrainMaterial>>,
    pub(super) water_materials: ResMut<'w, Assets<WaterMaterial>>,
    pub(super) images: ResMut<'w, Assets<Image>>,
    pub(super) inv_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
}

pub(super) struct SceneSetupSelection {
    pub(super) scene_entry: Option<WarbandSceneEntry>,
    pub(super) placement: Option<WarbandScenePlacement>,
    pub(super) presentation: ModelPresentation,
}

pub(super) struct ModelSyncSelection {
    pub(super) desired_id: Option<u64>,
    pub(super) scene_entry: Option<WarbandSceneEntry>,
    pub(super) placement: Option<WarbandScenePlacement>,
    pub(super) presentation: ModelPresentation,
    pub(super) char_tf: Transform,
}

pub(super) struct AppearanceSyncSelection {
    pub(super) root: Entity,
    pub(super) desired: AppliedCharacterAppearance,
    pub(super) character: CharacterListEntry,
}

pub(super) struct SceneSetupLighting {
    pub(super) camera_entity: Entity,
    pub(super) fov: f32,
    pub(super) primary_light: Entity,
    pub(super) fill_light: Entity,
}

pub(super) struct SceneSetupTimings {
    pub(super) background_elapsed: std::time::Duration,
    pub(super) camera_elapsed: std::time::Duration,
    pub(super) sky_light_elapsed: std::time::Duration,
    pub(super) model_elapsed: std::time::Duration,
}

#[derive(SystemParam)]
pub(super) struct CharSelectSceneSetupParams<'w, 's> {
    pub(super) commands: Commands<'w, 's>,
    pub(super) assets: CharSelectRenderAssets<'w>,
    pub(super) heightmap: ResMut<'w, TerrainHeightmap>,
    pub(super) creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    pub(super) customization_db: Res<'w, CustomizationDb>,
    pub(super) char_list: Res<'w, CharacterList>,
    pub(super) selected: Res<'w, SelectedCharIndex>,
    pub(super) displayed: ResMut<'w, DisplayedCharacterId>,
    pub(super) active_scene: ResMut<'w, ActiveWarbandSceneId>,
    pub(super) pending_supplemental: ResMut<'w, PendingSupplementalWarbandScene>,
    pub(super) cloud_maps: Res<'w, ProceduralCloudMaps>,
    pub(super) warband: Option<Res<'w, WarbandScenes>>,
    pub(super) selected_scene: Option<Res<'w, SelectedWarbandScene>>,
}

#[derive(SystemParam)]
pub(super) struct CharSelectModelSyncParams<'w, 's> {
    pub(super) commands: Commands<'w, 's>,
    pub(super) assets: CharSelectRenderAssets<'w>,
    pub(super) creature_display_map: Res<'w, creature_display::CreatureDisplayMap>,
    pub(super) customization_db: Res<'w, CustomizationDb>,
    pub(super) heightmap: Res<'w, TerrainHeightmap>,
    pub(super) char_list: Res<'w, CharacterList>,
    pub(super) selected: Res<'w, SelectedCharIndex>,
    pub(super) current_model: Query<'w, 's, Entity, With<CharSelectModelWrapper>>,
    pub(super) displayed: ResMut<'w, DisplayedCharacterId>,
    pub(super) scene_tree: Option<ResMut<'w, SceneTree>>,
    pub(super) warband: Option<Res<'w, WarbandScenes>>,
    pub(super) selected_scene: Option<Res<'w, SelectedWarbandScene>>,
    pub(super) camera_query: Query<
        'w,
        's,
        (
            &'static mut Transform,
            &'static mut camera::CharSelectOrbit,
            &'static mut Projection,
        ),
        (With<CharSelectScene>, Without<CharSelectModelRoot>),
    >,
}

#[derive(SystemParam)]
pub(super) struct CharSelectAppearanceSyncParams<'w, 's> {
    pub(super) customization_db: Res<'w, CustomizationDb>,
    pub(super) char_tex: Res<'w, CharTextureData>,
    pub(super) outfit_data: Res<'w, OutfitData>,
    pub(super) char_list: Res<'w, CharacterList>,
    pub(super) selected: Res<'w, SelectedCharIndex>,
    pub(super) displayed_appearance: ResMut<'w, DisplayedCharacterAppearance>,
    pub(super) root_query:
        Query<'w, 's, (Entity, &'static CharSelectModelCharacter), With<CharSelectModelRoot>>,
    pub(super) parent_query: Query<'w, 's, &'static ChildOf>,
    pub(super) geoset_query: Query<
        'w,
        's,
        (
            Entity,
            &'static crate::m2_spawn::GeosetMesh,
            &'static ChildOf,
        ),
    >,
    pub(super) visibility_query: Query<'w, 's, &'static mut Visibility>,
    pub(super) equipment_item_query: Query<'w, 's, (), With<EquipmentItem>>,
    pub(super) material_query: Query<
        'w,
        's,
        (
            Entity,
            &'static MeshMaterial3d<StandardMaterial>,
            Option<&'static crate::m2_spawn::GeosetMesh>,
            Option<&'static crate::m2_spawn::BatchTextureType>,
            &'static ChildOf,
        ),
    >,
    pub(super) equipment_query: Query<'w, 's, &'static mut crate::equipment::Equipment>,
    pub(super) images: ResMut<'w, Assets<Image>>,
    pub(super) materials: ResMut<'w, Assets<StandardMaterial>>,
}
