//! Replicated server game objects (`GameObjectInfo`, e.g. the Guild Vault): the
//! model from Retail `GameObjectDisplayInfo.FileDataID` at the replicated position
//! and facing, pickable with right-click (`WorldObjectInteractionKind::ServerObject`,
//! which sends `UseGameObject`).

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use shared::components::{Position as NetPosition, Rotation as NetRotation};
use shared::protocol::GameObjectInfo;

use crate::networking_npc::NpcSpawnAssets;
use crate::target::{WorldObjectInteraction, WorldObjectInteractionKind};

const GAME_OBJECT_DISPLAY_CSV: &str = "data/db2/12.1.0.69933/GameObjectDisplayInfo.csv";

/// Model FileDataID of every Retail `GameObjectDisplayInfo` row.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub(crate) struct GameObjectDisplays(HashMap<u32, u32>);

impl GameObjectDisplays {
    pub(crate) fn load() -> Self {
        let displays = std::fs::read_to_string(GAME_OBJECT_DISPLAY_CSV)
            .map_err(|err| format!("read {GAME_OBJECT_DISPLAY_CSV}: {err}"))
            .and_then(|text| parse_game_object_displays(&text));
        match displays {
            Ok(displays) => Self(displays),
            Err(err) => {
                error!("Game objects spawn without models: {err}");
                Self::default()
            }
        }
    }

    pub(crate) fn model_fdid(&self, display_id: u32) -> Option<u32> {
        self.0.get(&display_id).copied().filter(|fdid| *fdid != 0)
    }
}

fn parse_game_object_displays(text: &str) -> Result<HashMap<u32, u32>, String> {
    use crate::csv_util::{header_index, parse_csv_line};
    let path = Path::new(GAME_OBJECT_DISPLAY_CSV);
    let mut lines = text.lines();
    let headers = parse_csv_line(lines.next().ok_or("empty GameObjectDisplayInfo.csv")?);
    let id = header_index(&headers, "ID", path)?;
    let fdid = header_index(&headers, "FileDataID", path)?;
    lines
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields = parse_csv_line(line);
            let number = |index: usize| {
                fields
                    .get(index)
                    .and_then(|value| value.parse::<u32>().ok())
                    .ok_or_else(|| format!("bad GameObjectDisplayInfo row: {line}"))
            };
            Ok((number(id)?, number(fdid)?))
        })
        .collect()
}

/// Place a new replicated game object and attach its M2 under a child turned like
/// NPC models (`networking_npc::spawn_npc_visual_root`).
pub(crate) fn spawn_replicated_game_object(
    trigger: On<Add, GameObjectInfo>,
    mut commands: Commands,
    mut assets: NpcSpawnAssets,
    objects: Query<(&NetPosition, &GameObjectInfo, Option<&NetRotation>)>,
    displays: Option<Res<GameObjectDisplays>>,
) {
    let entity = trigger.entity;
    let Ok((position, info, rotation)) = objects.get(entity) else {
        return;
    };
    let yaw = rotation.map_or(0.0, |rotation| rotation.y);
    commands.entity(entity).insert((
        Name::new(info.name.clone()),
        Transform::from_translation(crate::networking::net_position_to_bevy(position))
            .with_rotation(Quat::from_rotation_y(yaw)),
        Visibility::default(),
        WorldObjectInteraction {
            kind: WorldObjectInteractionKind::ServerObject,
        },
    ));
    let Some(fdid) = displays.and_then(|displays| displays.model_fdid(info.display_id)) else {
        warn!(
            "game object {} has no GameObjectDisplayInfo {}",
            info.entry, info.display_id
        );
        return;
    };
    let Some(path) = crate::asset::asset_cache::model(fdid) else {
        warn!(
            "game object {} model {fdid} could not be extracted",
            info.entry
        );
        return;
    };
    let visual = commands
        .spawn((
            Name::new("GameObjectModel"),
            Transform::from_scale(Vec3::splat(info.scale.max(0.01)))
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
            ChildOf(entity),
        ))
        .id();
    let mut spawn_assets = crate::m2_spawn::SpawnAssets {
        meshes: &mut assets.meshes,
        materials: &mut assets.materials,
        effect_materials: &mut assets.effect_materials,
        skybox_materials: None,
        images: &mut assets.images,
        inverse_bindposes: &mut assets.inv_bp,
    };
    if !crate::m2_spawn::spawn_m2_on_entity(
        &mut commands,
        &mut spawn_assets,
        &path,
        visual,
        &[0, 0, 0],
    ) {
        warn!(
            "game object {} model {} did not load",
            info.entry,
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_rows_map_to_model_file_ids() {
        let text = "ID,GeoBox_0,GeoBox_1,GeoBox_2,GeoBox_3,GeoBox_4,GeoBox_5,FileDataID,ObjectEffectPackageID\n\
                    7606,-2.9,-2.9,0,-0.9,-0.9,3.2,199779,0\n\
                    7607,-0.70,-1.95,0.03,1.10,1.93,5.82,199769,0\n\
                    9000,0,0,0,0,0,0,0,0\n";
        let displays = GameObjectDisplays(parse_game_object_displays(text).unwrap());
        // Stormwind Guild Vault: guildvault_human_01.m2.
        assert_eq!(displays.model_fdid(7607), Some(199_769));
        assert_eq!(displays.model_fdid(9000), None);
        assert_eq!(displays.model_fdid(1), None);
    }

    #[test]
    fn a_replicated_vault_is_placed_facing_its_rotation_and_pickable() {
        use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<crate::retail_m2_material::M2Material>>()
            .init_resource::<Assets<crate::m2_effect_material::M2EffectMaterial>>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
            .insert_resource(GameObjectDisplays::default())
            .add_observer(spawn_replicated_game_object);
        let vault = app
            .world_mut()
            .spawn((
                NetPosition {
                    x: -8934.91,
                    y: 100.589,
                    z: -618.273,
                },
                NetRotation {
                    x: 0.0,
                    y: 1.25,
                    z: 0.0,
                },
            ))
            .id();
        app.world_mut().entity_mut(vault).insert(GameObjectInfo {
            entry: 187_329,
            go_type: shared::protocol::GAMEOBJECT_TYPE_GUILD_BANK,
            display_id: 7607,
            name: "Guild Vault".into(),
            scale: 1.0,
        });
        app.update();
        let entity = app.world().entity(vault);
        let transform = entity.get::<Transform>().unwrap();
        assert_eq!(
            transform.translation,
            Vec3::new(-8934.91, 100.589, -618.273)
        );
        assert!(
            transform
                .rotation
                .abs_diff_eq(Quat::from_rotation_y(1.25), 1e-6)
        );
        assert_eq!(
            entity.get::<WorldObjectInteraction>().map(|i| i.kind),
            Some(WorldObjectInteractionKind::ServerObject)
        );
    }

    #[test]
    fn a_row_with_a_non_numeric_id_is_an_error() {
        let text = "ID,FileDataID\nabc,199769\n";
        assert!(parse_game_object_displays(text).is_err());
    }
}
