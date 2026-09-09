use super::*;
use crate::camera::WowCamera;

fn owner(app: &mut App, distance: f32, nested: bool) -> (Entity, Entity) {
    let root = app.world_mut().spawn(NpcVisualRoot).id();
    let model = app
        .world_mut()
        .spawn((
            M2AnimPlayer {
                current_seq_idx: 0,
                time_ms: 0.0,
                looping: true,
                transition: None,
            },
            GlobalTransform::from_translation(Vec3::X * distance),
            ChildOf(root),
        ))
        .id();
    let mesh_parent = if nested {
        app.world_mut().spawn(ChildOf(model)).id()
    } else {
        model
    };
    let mesh = app
        .world_mut()
        .spawn((ViewVisibility::HIDDEN, ChildOf(mesh_parent)))
        .id();
    (model, mesh)
}

#[test]
fn npc_lod_sees_visible_mesh_below_grounded_root_and_preserves_distance_policy() {
    let mut app = App::new();
    app.add_systems(Update, assign_npc_animation_lod);
    app.world_mut()
        .spawn((WowCamera::default(), GlobalTransform::IDENTITY));
    let (near, mesh) = owner(&mut app, 10.0, true);
    let (far, far_mesh) = owner(&mut app, 70.0, true);
    let (mid, mid_mesh) = owner(&mut app, 45.0, false);
    for visible in [mesh, far_mesh, mid_mesh] {
        app.world_mut()
            .entity_mut(visible)
            .insert(ViewVisibility::VISIBLE);
    }
    app.update();
    assert_eq!(
        app.world().get::<AnimationLod>(near),
        Some(&AnimationLod::Full)
    );
    assert_eq!(
        app.world().get::<AnimationLod>(mid),
        Some(&AnimationLod::Half)
    );
    assert_eq!(
        app.world().get::<AnimationLod>(far),
        Some(&AnimationLod::Frozen)
    );
    app.world_mut()
        .entity_mut(mesh)
        .insert(ViewVisibility::HIDDEN);
    app.update();
    assert_eq!(
        app.world().get::<AnimationLod>(near),
        Some(&AnimationLod::Frozen)
    );
}
