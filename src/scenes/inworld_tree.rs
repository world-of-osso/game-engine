//! Scene tree construction for the in-world networked scene.

use bevy::prelude::*;
use game_engine::scene_tree::{NodeProps, SceneNode, SceneTree};
use lightyear::prelude::Replicated;
use shared::components::{ModelDisplay, Npc, Player as NetPlayer};

use crate::networking::{LocalPlayer, ReplicatedVisualEntity, ResolvedModelAssetInfo};

pub struct InWorldSceneTreePlugin;

impl Plugin for InWorldSceneTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            build_inworld_scene_tree.run_if(in_state(crate::game_state::GameState::InWorld)),
        );
    }
}

fn build_inworld_scene_tree(
    mut commands: Commands,
    players: Query<
        (Entity, &NetPlayer, Option<&ResolvedModelAssetInfo>),
        With<ReplicatedVisualEntity>,
    >,
    npcs: Query<
        (
            Entity,
            &Npc,
            Option<&ModelDisplay>,
            Option<&ResolvedModelAssetInfo>,
        ),
        With<Replicated>,
    >,
    camera: Query<Entity, With<Camera3d>>,
    local_player: Query<Entity, With<LocalPlayer>>,
) {
    if players.is_empty() && npcs.is_empty() {
        return;
    }
    let mut children = Vec::new();
    collect_player_nodes(&players, &local_player, &mut children);
    collect_npc_nodes(&npcs, &mut children);
    collect_camera_nodes(&camera, &mut children);

    commands.insert_resource(SceneTree {
        root: SceneNode {
            label: "InWorldScene".into(),
            entity: None,
            props: NodeProps::Scene,
            children,
        },
    });
}

fn collect_player_nodes(
    players: &Query<
        (Entity, &NetPlayer, Option<&ResolvedModelAssetInfo>),
        With<ReplicatedVisualEntity>,
    >,
    local_player: &Query<Entity, With<LocalPlayer>>,
    children: &mut Vec<SceneNode>,
) {
    for (entity, player, model_info) in players.iter() {
        let is_local = local_player.get(entity).is_ok();
        children.push(SceneNode {
            label: "Player".into(),
            entity: Some(entity),
            props: NodeProps::Player {
                name: player.name.clone(),
                is_local,
                model_path: model_info.map(|info| info.model_path.clone()),
                skin_path: model_info.and_then(|info| info.skin_path.clone()),
                display_scale: model_info.and_then(|info| info.display_scale),
            },
            children: vec![],
        });
    }
}

fn collect_npc_nodes(
    npcs: &Query<
        (
            Entity,
            &Npc,
            Option<&ModelDisplay>,
            Option<&ResolvedModelAssetInfo>,
        ),
        With<Replicated>,
    >,
    children: &mut Vec<SceneNode>,
) {
    for (entity, npc, display, model_info) in npcs.iter() {
        children.push(SceneNode {
            label: "Npc".into(),
            entity: Some(entity),
            props: NodeProps::Npc {
                name: format!("template_{}", npc.template_id),
                display_id: display.map(|d| d.display_id),
                model_path: model_info.map(|info| info.model_path.clone()),
                skin_path: model_info.and_then(|info| info.skin_path.clone()),
                display_scale: model_info.and_then(|info| info.display_scale),
            },
            children: vec![],
        });
    }
}

fn collect_camera_nodes(camera: &Query<Entity, With<Camera3d>>, children: &mut Vec<SceneNode>) {
    for entity in camera.iter() {
        children.push(SceneNode {
            label: "Camera".into(),
            entity: Some(entity),
            props: NodeProps::Camera { fov: 60.0 },
            children: vec![],
        });
    }
}
