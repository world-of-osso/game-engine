use super::*;

fn collision_model(fdid: u32) -> crate::asset::m2::M2Model {
    crate::asset::m2::load_m2_uncached(
        &std::path::PathBuf::from(format!("data/models/{fdid}.m2")),
        &[0, 0, 0],
    )
    .expect("required local doodad model")
}

fn move_against_model(
    model: &crate::asset::m2::M2Model,
    transform: Transform,
    from: Vec3,
    to: Vec3,
) -> Vec3 {
    let collider = build_doodad_collider(model, &transform).expect("authored solid geometry");
    crate::collision::clamp_movement_against_doodad_colliders(
        from,
        to,
        &[(collider.world_min, collider.world_max)],
    )
}

#[test]
fn doodad_collision_real_canopy_open_space_is_walkable() {
    let model = collision_model(189929);
    // Both endpoints lie inside the canopy's collision bounds. The first actual
    // collision triangle along this ray is beyond x=11, not in this open segment.
    let from = Vec3::new(0.0, 0.4, 0.0);
    let to = Vec3::new(5.0, 0.4, 0.0);
    assert_eq!(
        move_against_model(&model, Transform::default(), from, to),
        to
    );
}

#[test]
fn doodad_collision_real_canopy_trunk_stops_at_surface_not_bounds() {
    let model = collision_model(189929);
    let from = Vec3::new(10.0, 0.4, 0.0);
    let to = Vec3::new(13.0, 0.4, 0.0);
    let moved = move_against_model(&model, Transform::default(), from, to);
    assert!(
        moved.x > 10.5,
        "open space before the trunk must remain walkable: {moved:?}"
    );
    assert!(
        moved.x < 11.075,
        "must stop before the authored trunk surface: {moved:?}"
    );
}

#[test]
fn doodad_collision_real_canopy_open_space_survives_rotated_scaled_placement() {
    let model = collision_model(189929);
    let transform = Transform::from_translation(Vec3::new(17.0, 3.0, -29.0))
        .with_rotation(Quat::from_rotation_y(0.8))
        .with_scale(Vec3::splat(1.75));
    let from = transform.transform_point(Vec3::ZERO);
    let to = transform.transform_point(Vec3::new(5.0, 0.0, 0.0));
    let moved = move_against_model(&model, transform, from, to);
    assert!(
        moved.abs_diff_eq(to, 0.001),
        "placement must preserve open space: {moved:?}"
    );
}

#[test]
fn doodad_collision_decorative_bush_has_no_solid_collider() {
    let model = collision_model(189700);
    assert!(build_doodad_collider(&model, &Transform::default()).is_none());
}
