use super::*;

#[test]
fn test_resolve_targetable_ancestor_finds_remote_root_from_child_mesh() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetResolutionResult>();

    let root = app
        .world_mut()
        .spawn((Transform::default(), RemoteEntity, Npc { template_id: 99 }))
        .id();
    let child = app.world_mut().spawn(Transform::default()).id();
    app.world_mut().entity_mut(child).insert(ChildOf(root));
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              remote_query: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
              visibility_query: Query<&Visibility>,
              mut result: ResMut<TargetResolutionResult>| {
            result.0 =
                resolve_targetable_ancestor(child, &parent_query, &remote_query, &visibility_query);
        },
    );
    app.update();

    assert_eq!(
        app.world().resource::<TargetResolutionResult>().0,
        Some(root)
    );
}

#[test]
fn test_resolve_targetable_ancestor_ignores_remote_players() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetResolutionResult>();

    let root = app
        .world_mut()
        .spawn((
            Transform::default(),
            RemoteEntity,
            shared::components::Player {
                name: "Friendly".into(),
                race: 1,
                class: 2,
                appearance: Default::default(),
            },
        ))
        .id();
    let child = app.world_mut().spawn(Transform::default()).id();
    app.world_mut().entity_mut(child).insert(ChildOf(root));
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              remote_query: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
              visibility_query: Query<&Visibility>,
              mut result: ResMut<TargetResolutionResult>| {
            result.0 =
                resolve_targetable_ancestor(child, &parent_query, &remote_query, &visibility_query);
        },
    );
    app.update();

    assert_eq!(app.world().resource::<TargetResolutionResult>().0, None);
}

#[test]
fn test_resolve_targetable_ancestor_ignores_hidden_npcs() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetResolutionResult>();

    let root = app
        .world_mut()
        .spawn((
            Transform::default(),
            RemoteEntity,
            Npc { template_id: 7 },
            Visibility::Hidden,
        ))
        .id();
    let child = app.world_mut().spawn(Transform::default()).id();
    app.world_mut().entity_mut(child).insert(ChildOf(root));
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              remote_query: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
              visibility_query: Query<&Visibility>,
              mut result: ResMut<TargetResolutionResult>| {
            result.0 =
                resolve_targetable_ancestor(child, &parent_query, &remote_query, &visibility_query);
        },
    );
    app.update();

    assert_eq!(app.world().resource::<TargetResolutionResult>().0, None);
}

#[test]
fn resolve_interaction_ancestor_finds_world_object_root() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<TargetResolutionResult>();

    let root = app
        .world_mut()
        .spawn((
            Transform::default(),
            WorldObjectInteraction {
                kind: WorldObjectInteractionKind::Mailbox,
            },
        ))
        .id();
    let child = app.world_mut().spawn(Transform::default()).id();
    app.world_mut().entity_mut(child).insert(ChildOf(root));
    app.add_systems(
        Update,
        move |parent_query: Query<&ChildOf>,
              npc_query: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
              object_query: Query<&WorldObjectInteraction>,
              quest_query: Query<(), With<game_engine::quest_tracking::QuestTrackedItem>>,
              visibility_query: Query<&Visibility>,
              mut result: ResMut<TargetResolutionResult>| {
            result.0 = resolve_interaction_ancestor(
                child,
                &parent_query,
                &npc_query,
                &object_query,
                &quest_query,
                &visibility_query,
            )
            .map(|target| match target {
                InteractionTarget::Npc(entity) | InteractionTarget::Object(entity, _) => entity,
            });
        },
    );
    app.update();

    assert_eq!(
        app.world().resource::<TargetResolutionResult>().0,
        Some(root)
    );
}
