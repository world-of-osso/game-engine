//! M2 point lights of the one-handed torch (club_1h_torch_a_01, FDID 145304) as retail
//! evaluates them (WebWowViewerCpp `calcLights` / `collectLights`).
use game_engine_core::{
    m2,
    m2_lights::{LightTime, PointLight, light_animates, point_light},
};

fn torch() -> m2::Model {
    let read = |name: &str| {
        let path = format!("{}/../../data/models/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    m2::parse_model(&read("145304.m2"), &read("14530400.skin")).unwrap()
}

fn at_start(model: &m2::Model) -> LightTime<'_> {
    LightTime {
        sequence: 0,
        time_ms: 0,
        global_ms: 0,
        global_sequences: &model.global_sequences,
    }
}

#[test]
fn torch_flame_light_is_its_authored_diffuse_with_retail_attenuation() {
    let model = torch();
    assert_eq!(model.lights.len(), 1);
    let light = &model.lights[0];
    // On the flame's bone 9; authored attenuation 1.389-2.222 yd is replaced because
    // the torch lacks global flag 0x8000 (its flags are 0x80).
    assert_eq!((light.light_type, light.bone_index), (1, 9));
    assert_eq!(model.flags, 0x80);
    let PointLight {
        color,
        attenuation_start,
        attenuation_end,
        visible,
    } = point_light(light, model.flags, &at_start(&model)).unwrap();
    // Diffuse (119, 74, 34) / 255 x intensity 1.1.
    let expected = [0.466_666_7 * 1.1, 0.290_196_1 * 1.1, 0.133_333_3 * 1.1];
    for (actual, expected) in color.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-5, "{color:?}");
    }
    assert_eq!((attenuation_start, attenuation_end), (1.6666, 5.266_660_2));
    assert!(visible);
    assert!(!light_animates(light));
}

#[test]
fn authored_attenuation_flag_keeps_the_tracks() {
    let model = torch();
    let point = point_light(&model.lights[0], model.flags | 0x8000, &at_start(&model)).unwrap();
    assert!((point.attenuation_start - 1.388_889).abs() < 1e-5);
    assert!((point.attenuation_end - 2.222_222).abs() < 1e-5);
}

#[test]
fn torch_halo_and_flame_bones_are_billboards() {
    let model = torch();
    // Bone 1 carries the halo quad, bone 2 parents the flame emitter's bone 10.
    assert_eq!(model.bones[1].flags, 0x208);
    assert_eq!(model.bones[2].flags, 0x8);
    assert_eq!(model.bones[10].parent_bone_id, 2);
    assert_eq!(model.particle_emitters[0].bone_index, 10);
    for (index, bone) in model.bones.iter().enumerate() {
        let expected = matches!(index, 1 | 2);
        assert_eq!(
            game_engine_core::m2_billboard::is_billboard(bone.flags),
            expected
        );
    }
}
