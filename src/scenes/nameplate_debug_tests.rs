use super::*;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[test]
fn preview_casts_pause_resume_and_loop_without_losing_entities() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<PreviewPlayback>();
    app.add_systems(
        Update,
        (toggle_preview_pause, advance_preview_casts).chain(),
    );
    let unit = app.world_mut().spawn((PreviewUnit, demo_cast(false))).id();
    app.update();
    app.update();
    let elapsed = app.world().get::<CastState>(unit).unwrap().elapsed;
    assert!(elapsed > 0.0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();
    assert!(app.world().resource::<PreviewPlayback>().paused);
    assert_eq!(app.world().get::<CastState>(unit).unwrap().elapsed, elapsed);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Space);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    assert_eq!(app.world().get::<CastState>(unit).unwrap().elapsed, elapsed);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();
    assert!(!app.world().resource::<PreviewPlayback>().paused);
    assert!(app.world().get::<CastState>(unit).unwrap().elapsed > elapsed);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    for _ in 0..100 {
        app.update();
    }
    let cast = app.world().get::<CastState>(unit).unwrap();
    assert!(cast.elapsed < cast.duration);
    assert_eq!(
        app.world_mut()
            .query::<&PreviewUnit>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn preview_examples_include_normal_and_channel_casts_with_real_progress() {
    let normal = demo_cast(false);
    let channel = demo_cast(true);
    assert_eq!(normal.cast_type, shared::casting::CastType::Normal);
    assert_eq!(channel.cast_type, shared::casting::CastType::Channel);
    assert!(!normal.spell_name.is_empty());
    assert!(!channel.spell_name.is_empty());
    assert!(normal.elapsed > 0.0 && normal.elapsed < normal.duration);
    assert!(channel.elapsed > 0.0 && channel.elapsed < channel.duration);
}

#[test]
fn leaving_preview_removes_owners_camera_and_labels() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.init_state::<GameState>();
    let mut images = Assets::<Image>::default();
    let mut fonts = Assets::<Font>::default();
    let art = NameplateArtCache::fixture(&mut images, &mut fonts);
    app.insert_resource(images)
        .insert_resource(fonts)
        .insert_resource(art);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.add_plugins(NameplateDebugPlugin);
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::NameplateDebug);
    app.update();
    let owners: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<PreviewUnit>>()
        .iter(app.world())
        .collect();
    assert_eq!(owners.len(), 3);
    assert_eq!(
        app.world_mut()
            .query::<&Camera3d>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(
        app.world_mut().query::<&Text2d>().iter(app.world()).count(),
        4
    );
    for entity in &owners {
        assert!(
            app.world()
                .get::<shared::components::Npc>(*entity)
                .is_none()
        );
        assert!(
            app.world()
                .get::<shared::components::Player>(*entity)
                .is_none()
        );
    }
    app.world_mut().resource_mut::<CurrentTarget>().0 = Some(owners[0]);
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Login);
    app.update();
    assert!(
        owners
            .iter()
            .all(|entity| app.world().get_entity(*entity).is_err())
    );
    assert_eq!(
        app.world_mut().query::<&Text2d>().iter(app.world()).count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&Camera3d>()
            .iter(app.world())
            .count(),
        0
    );
    assert_eq!(app.world().resource::<CurrentTarget>().0, None);
}

#[test]
fn nameplate_debug_route_does_not_schedule_authentication() {
    let mut actions = Vec::new();
    let mut server = None;
    let mut state = Some("nameplatedebug".parse::<GameState>().unwrap());
    let mut enter = false;
    let mut login = None;
    crate::screen_auto_login::apply(
        &mut actions,
        &mut server,
        &mut state,
        &mut enter,
        &mut login,
        false,
        None,
    );
    assert_eq!(state, Some(GameState::NameplateDebug));
    assert!(!GameState::NameplateDebug.is_logged_in());
    assert!(server.is_none() && login.is_none() && actions.is_empty() && !enter);
    assert!(
        crate::app_setup::default_connecting_server_arg(
            state,
            crate::cli_args::RealmPreset::default()
        )
        .is_none()
    );
}
