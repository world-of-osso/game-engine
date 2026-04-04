use bevy::prelude::*;
use game_engine::scene_tree::SceneTree;

pub fn teardown_tagged_scene<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    commands.remove_resource::<SceneTree>();
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
