use game_engine_core::elastic_tree::{
    BranchState, Capsule, deflect_motion, parse_annotation, sweep_capsule,
};
use glam::Vec3;

fn annotation() -> game_engine_core::elastic_tree::TreeAnnotation {
    parse_annotation(r#"{"model_fdid":123,"trunk":{"start":[0,0,0],"end":[0,4,0],"radius":0.4},"branches":[{"name":"thin","pivot":[0,2,0],"tip":[4,2,0],"radius":0.1,"stiffness":4,"damping":4,"max_angle":0.7,"region_min":[0,1,-1],"region_max":[4,3,1]},{"name":"thick","pivot":[0,3,0],"tip":[-4,3,0],"radius":0.3,"stiffness":16,"damping":8,"max_angle":0.4,"region_min":[-4,2,-1],"region_max":[0,4,1]}]}"#).unwrap()
}

#[test]
fn annotations_are_strict_and_finite() {
    let tree = annotation();
    let json = serde_json::to_value(&tree).unwrap();
    assert_eq!(parse_annotation(&json.to_string()).unwrap(), tree);
    let mut bad = Vec::new();
    for (path, value) in [
        ("radius", serde_json::json!(-1)),
        ("radius", serde_json::json!(0)),
    ] {
        let mut item = json.clone();
        item["trunk"][path] = value;
        bad.push(item);
    }
    let mut item = json.clone();
    item["trunk"]["end"] = item["trunk"]["start"].clone();
    bad.push(item);
    for key in ["radius", "stiffness", "damping", "max_angle"] {
        let mut item = json.clone();
        item["branches"][0][key] = serde_json::json!(-1);
        bad.push(item);
    }
    let mut item = json.clone();
    item["branches"][0]["tip"] = item["branches"][0]["pivot"].clone();
    bad.push(item);
    let mut item = json.clone();
    item["branches"][0]["region_min"] = serde_json::json!([5, 1, -1]);
    bad.push(item);
    let mut item = json.clone();
    let extra = item["branches"][0].clone();
    item["branches"].as_array_mut().unwrap().push(extra);
    bad.push(item);
    for key in ["unknown", "foliage"] {
        let mut item = json.clone();
        item[key] = serde_json::json!(true);
        bad.push(item);
    }
    let mut item = json.clone();
    item["trunk"]["unknown"] = serde_json::json!(1);
    bad.push(item);
    let mut item = json.clone();
    item["branches"][0]["unknown"] = serde_json::json!(1);
    bad.push(item);
    for item in bad {
        assert!(parse_annotation(&item.to_string()).is_err(), "{item}");
    }
    assert!(parse_annotation(&json.to_string().replacen("0.4", "1e100", 1)).is_err());
}

#[test]
fn thin_limb_bends_more_and_tip_has_more_leverage() {
    let tree = annotation();
    let mut thin = BranchState::default();
    let mut thick = BranchState::default();
    let mut base = BranchState::default();
    thin.apply_contact(&tree.branches[0], Vec3::new(4., 2., 0.), Vec3::Z);
    thick.apply_contact(&tree.branches[1], Vec3::new(-4., 3., 0.), Vec3::Z);
    base.apply_contact(&tree.branches[0], Vec3::new(0.1, 2., 0.), Vec3::Z);
    thin.advance(&tree.branches[0], 0.1);
    thick.advance(&tree.branches[1], 0.1);
    base.advance(&tree.branches[0], 0.1);
    assert!(thin.rotation.length() > thick.rotation.length());
    assert!(thin.rotation.length() > base.rotation.length());
    let collider = thin.collider(&tree.branches[0]);
    assert_eq!(collider.start, tree.branches[0].pivot);
    assert!(Vec3::from_array(collider.end).z > 0.);
    assert!(
        (Vec3::from_array(collider.end).distance(Vec3::from_array(collider.start)) - 4.).abs()
            < 1e-5
    );
    assert!((thin.rotation_quat().length() - 1.).abs() < 1e-5);
}

#[test]
fn repeated_impacts_are_continuous_and_long_frames_recover() {
    let branch = &annotation().branches[0];
    let mut state = BranchState::default();
    state.apply_contact(branch, Vec3::new(4., 2., 0.), Vec3::Z * 100.);
    state.advance(branch, 0.2);
    let rotation = state.rotation;
    state.apply_contact(branch, Vec3::new(4., 2., 0.), Vec3::Z);
    assert_eq!(state.rotation, rotation);
    for _ in 0..20 {
        state.advance(branch, 1.);
        assert!(state.rotation.is_finite());
        assert!(state.rotation.length() <= branch.max_angle + 1e-6);
    }
    assert!(state.rotation.length() < 1e-4);
    assert!(state.angular_velocity.length() < 1e-4);
}

#[test]
fn weights_fix_trunk_and_smoothly_increase_toward_tip() {
    let tree = annotation();
    assert_eq!(tree.vertex_weights(Vec3::new(0., 2., 0.)), [0., 0.]);
    assert_eq!(tree.vertex_weights(Vec3::new(4., 8., 0.)), [0., 0.]);
    let base = tree.vertex_weights(Vec3::new(0.5, 2., 0.))[0];
    let middle = tree.vertex_weights(Vec3::new(2., 2., 0.))[0];
    assert!(base > 0. && base < middle && middle < 1.);
    assert_eq!(tree.vertex_weights(Vec3::new(4., 2., 0.)), [1., 0.]);
    assert_eq!(tree.vertex_weights(Vec3::new(-4., 3., 0.)), [0., 1.]);
}

fn capsule() -> Capsule {
    Capsule {
        start: [0., -2., 0.],
        end: [0., 2., 0.],
        radius: 0.5,
    }
}

#[test]
fn sweep_catches_high_speed_caps_and_oblique_hits_but_not_misses() {
    let body = capsule();
    let hit = sweep_capsule(
        Vec3::new(-100., 0., 0.),
        Vec3::new(100., 0., 0.),
        0.5,
        &body,
    )
    .unwrap();
    assert!((hit.fraction - 0.495).abs() < 1e-5);
    assert!(hit.normal.distance(-Vec3::X) < 1e-5);
    assert!(hit.point.distance(Vec3::new(-0.5, 0., 0.)) < 1e-5);
    let cap = sweep_capsule(Vec3::new(0., 10., 0.), Vec3::ZERO, 0.5, &body).unwrap();
    assert!((cap.fraction - 0.7).abs() < 1e-5);
    assert!(cap.normal.distance(Vec3::Y) < 1e-5);
    let glance = sweep_capsule(
        Vec3::new(-10., 0., 0.9),
        Vec3::new(10., 1., 0.9),
        0.5,
        &body,
    )
    .unwrap();
    assert!(glance.normal.z > 0.8);
    assert!(
        sweep_capsule(
            Vec3::new(-10., 0., 1.1),
            Vec3::new(10., 0., 1.1),
            0.5,
            &body
        )
        .is_none()
    );
}

#[test]
fn overlap_escaping_does_not_block_but_inward_motion_does() {
    let body = capsule();
    assert!(sweep_capsule(Vec3::new(0.8, 0., 0.), Vec3::new(3., 0., 0.), 0.5, &body).is_none());
    assert_eq!(
        sweep_capsule(Vec3::new(0.8, 0., 0.), Vec3::ZERO, 0.5, &body)
            .unwrap()
            .fraction,
        0.
    );
}

#[test]
fn resistance_preserves_tangent_and_never_adds_energy() {
    let motion = Vec3::new(-4., 0., 3.);
    let thin = deflect_motion(motion, Vec3::X, 0.25);
    let thick = deflect_motion(motion, Vec3::X, 0.75);
    assert_eq!(thin, Vec3::new(-3., 0., 3.));
    assert_eq!(thick, Vec3::new(-1., 0., 3.));
    assert_eq!(deflect_motion(motion, Vec3::X, 1.), Vec3::new(0., 0., 3.));
    assert_eq!(deflect_motion(-motion, Vec3::X, 1.), -motion);
    for resistance in [-1., 0., 0.5, 1., 2.] {
        assert!(deflect_motion(motion, Vec3::X * 2., resistance).length() <= motion.length());
    }
}
