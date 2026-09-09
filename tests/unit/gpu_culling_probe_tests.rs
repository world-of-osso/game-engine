use super::register_gpu_culling_probe;
use bevy::app::TaskPoolPlugin;
use bevy::camera::primitives::{Aabb, Frustum};
use bevy::camera::visibility::{
    VisibleEntities, check_visibility_cpu_culling, check_visibility_gpu_culling,
    visibility_propagate_system,
};
use bevy::math::primitives::ViewFrustum;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use std::env::VarError;
use std::time::Duration;

fn visibility_app() -> App {
    let mut app = App::new();
    app.add_plugins(TaskPoolPlugin::default());
    app.init_resource::<Time<Real>>();
    app.add_systems(
        PostUpdate,
        (
            visibility_propagate_system,
            check_visibility_cpu_culling,
            check_visibility_gpu_culling,
        )
            .chain(),
    );
    app.world_mut().spawn((
        Camera::default(),
        VisibleEntities::default(),
        Frustum(ViewFrustum::from_clip_from_world(&Mat4::perspective_rh(
            1.0, 1.0, 0.1, 100.0,
        ))),
    ));
    app
}

fn spawn_offscreen(app: &mut App, visual: impl Bundle) -> Entity {
    app.world_mut()
        .spawn((
            visual,
            Visibility::Inherited,
            GlobalTransform::from_translation(Vec3::new(1000.0, 0.0, -5.0)),
            Aabb::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5)),
        ))
        .id()
}

fn mesh() -> Mesh3d {
    Mesh3d(Handle::default())
}

fn advance(app: &mut App, seconds: f64) {
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs_f64(seconds));
    app.update();
}

fn visible(app: &App, entity: Entity) -> bool {
    app.world().get::<ViewVisibility>(entity).unwrap().get()
}

#[test]
fn gpu_culling_probe_missing_env_preserves_cpu_path_without_a_clock() {
    let mut app = visibility_app();
    let entity = spawn_offscreen(&mut app, mesh());
    register_gpu_culling_probe(&mut app, Err(VarError::NotPresent)).unwrap();
    app.world_mut().remove_resource::<Time<Real>>();
    app.update();
    assert!(!visible(&app, entity));
}

#[test]
fn gpu_culling_probe_invalid_values_fail_with_variable_context() {
    for value in ["", "oops", "-1", "NaN", "inf", "1e300"] {
        let mut app = visibility_app();
        let error = register_gpu_culling_probe(&mut app, Ok(value.to_owned()))
            .expect_err("invalid deadline must fail");
        assert!(error.contains("WOO_PERF_GPU_CULLING_AFTER_SECS"), "{error}");
    }
    let mut app = visibility_app();
    let error = register_gpu_culling_probe(
        &mut app,
        Err(VarError::NotUnicode("invalid unicode".into())),
    )
    .expect_err("non-Unicode environment value must fail");
    assert!(error.contains("WOO_PERF_GPU_CULLING_AFTER_SECS"), "{error}");
}

#[test]
fn gpu_culling_probe_keeps_cpu_culling_until_the_real_time_deadline() {
    let mut app = visibility_app();
    let offscreen = spawn_offscreen(&mut app, mesh());
    let onscreen = spawn_offscreen(&mut app, mesh());
    app.world_mut()
        .entity_mut(onscreen)
        .insert(GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -5.0)));
    register_gpu_culling_probe(&mut app, Ok("2.5".to_owned())).unwrap();

    advance(&mut app, 2.0);
    assert!(!visible(&app, offscreen));
    assert!(visible(&app, onscreen));

    advance(&mut app, 0.5);
    assert!(visible(&app, offscreen));
    assert!(visible(&app, onscreen));
}

#[test]
fn gpu_culling_probe_respects_hidden_hierarchy_and_later_parent_changes() {
    let mut app = visibility_app();
    let parent = app.world_mut().spawn(Visibility::Hidden).id();
    let child = spawn_offscreen(&mut app, mesh());
    app.world_mut().entity_mut(child).insert(ChildOf(parent));
    register_gpu_culling_probe(&mut app, Ok("0".to_owned())).unwrap();

    app.update();
    assert!(!visible(&app, child));
    app.world_mut()
        .entity_mut(parent)
        .insert(Visibility::Inherited);
    app.update();
    assert!(visible(&app, child));
    app.world_mut()
        .entity_mut(parent)
        .insert(Visibility::Hidden);
    app.update();
    assert!(!visible(&app, child));
}

#[test]
fn gpu_culling_probe_keeps_skinned_meshes_2d_text_and_ui_on_cpu_path() {
    let mut app = visibility_app();
    let skinned = spawn_offscreen(
        &mut app,
        (
            mesh(),
            SkinnedMesh {
                inverse_bindposes: Handle::default(),
                joints: Vec::new(),
            },
        ),
    );
    let flat = spawn_offscreen(&mut app, Mesh2d(Handle::default()));
    let text = spawn_offscreen(&mut app, Text2d::new("diagnostic fixture"));
    let ui = spawn_offscreen(&mut app, Node::default());
    register_gpu_culling_probe(&mut app, Ok("0".to_owned())).unwrap();
    app.update();
    for entity in [skinned, flat, text, ui] {
        assert!(!visible(&app, entity));
    }

    app.world_mut()
        .entity_mut(skinned)
        .insert(GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -5.0)));
    app.update();
    assert!(visible(&app, skinned));
}

#[test]
fn gpu_culling_probe_does_not_enroll_entities_created_after_its_one_shot() {
    let mut app = visibility_app();
    let original = spawn_offscreen(&mut app, mesh());
    register_gpu_culling_probe(&mut app, Ok("1".to_owned())).unwrap();
    advance(&mut app, 1.0);
    assert!(visible(&app, original));

    let late = spawn_offscreen(&mut app, mesh());
    advance(&mut app, 1.0);
    assert!(visible(&app, original));
    assert!(!visible(&app, late));
}
