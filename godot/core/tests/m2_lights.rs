//! M2 point lights of the one-handed torch (club_1h_torch_a_01, FDID 145304) as retail
//! evaluates them (WebWowViewerCpp `calcLights` / `collectLights`).
use game_engine_core::{
    asset::m2_format::{m2_anim::AnimTrack, m2_light::M2Light},
    m2,
    m2_lights::{LightTime, PointLight, light_animates, point_light},
};

use glam::{Affine3A, Mat3, Quat, Vec3};

fn track<T>(global_sequence: i16, sequences: Vec<(Vec<u32>, Vec<T>)>) -> AnimTrack<T> {
    AnimTrack {
        interpolation_type: 1,
        global_sequence,
        sequences,
    }
}

fn unauthored_light() -> M2Light {
    M2Light {
        light_type: 1,
        bone_index: -1,
        position: [0.0; 3],
        ambient_color: track(-1, vec![]),
        ambient_intensity: track(-1, vec![]),
        diffuse_color: track(-1, vec![]),
        diffuse_intensity: track(-1, vec![]),
        attenuation_start: track(-1, vec![]),
        attenuation_end: track(-1, vec![]),
        visibility: track(-1, vec![]),
    }
}

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
fn light_tracks_use_the_playing_sequence_and_interpolate_diffuse_product() {
    let mut light = unauthored_light();
    light.diffuse_color = track(
        -1,
        vec![
            (vec![0], vec![[9.0; 3]]),
            (vec![0, 100], vec![[0.2, 0.4, 0.6], [0.6, 0.8, 1.0]]),
        ],
    );
    light.diffuse_intensity = track(
        -1,
        vec![(vec![0], vec![9.0]), (vec![0, 100], vec![1.0, 3.0])],
    );
    // Ambient is not part of a point light's diffuse output.
    light.ambient_color = track(-1, vec![(vec![0], vec![[100.0; 3]]); 2]);
    light.ambient_intensity = track(-1, vec![(vec![0], vec![100.0]); 2]);
    let time = LightTime {
        sequence: 1,
        time_ms: 50,
        global_ms: 700,
        global_sequences: &[],
    };
    let point = point_light(&light, 0, &time).unwrap();
    for (actual, expected) in point.color.into_iter().zip([0.8, 1.2, 1.6]) {
        assert!((actual - expected).abs() < 1e-6, "{point:?}");
    }
    assert!(light_animates(&light));
}

#[test]
fn global_light_tracks_wrap_independently_of_the_playing_sequence() {
    let mut light = unauthored_light();
    light.diffuse_color = track(0, vec![(vec![0, 100], vec![[0.0; 3], [1.0; 3]])]);
    light.diffuse_intensity = track(1, vec![(vec![0, 200], vec![1.0, 3.0])]);
    light.attenuation_start = track(0, vec![(vec![0, 100], vec![1.0, 3.0])]);
    light.attenuation_end = track(1, vec![(vec![0, 200], vec![4.0, 8.0])]);
    let mut time = LightTime {
        sequence: 7,
        time_ms: 99,
        global_ms: 250,
        global_sequences: &[100, 200],
    };
    assert_eq!(
        point_light(&light, 0x8000, &time).unwrap(),
        PointLight {
            color: [0.75; 3],
            attenuation_start: 2.0,
            attenuation_end: 5.0,
            visible: true,
        }
    );
    time.global_ms = 400;
    assert_eq!(
        point_light(&light, 0x8000, &time).unwrap(),
        PointLight {
            color: [0.0; 3],
            attenuation_start: 1.0,
            attenuation_end: 4.0,
            visible: true,
        }
    );
}

#[test]
fn zero_duration_global_light_track_samples_its_first_key() {
    let mut light = unauthored_light();
    light.diffuse_intensity = track(0, vec![(vec![0, 100], vec![2.0, 4.0])]);
    let time = LightTime {
        sequence: 9,
        time_ms: 100,
        global_ms: u64::MAX,
        global_sequences: &[0],
    };
    assert_eq!(point_light(&light, 0, &time).unwrap().color, [2.0; 3]);
}

#[test]
fn visibility_steps_at_keys_and_repeats_on_the_global_clock() {
    let mut light = unauthored_light();
    light.visibility = track(0, vec![(vec![0, 40, 80], vec![1, 0, 2])]);
    for (global_ms, expected) in [
        (0, true),
        (39, true),
        (40, false),
        (79, false),
        (80, true),
        (100, true),
        (140, false),
    ] {
        let time = LightTime {
            sequence: 3,
            time_ms: 0,
            global_ms,
            global_sequences: &[100],
        };
        assert_eq!(
            point_light(&light, 0, &time).unwrap().visible,
            expected,
            "global clock {global_ms} ms"
        );
    }
    assert!(light_animates(&light));
}

#[test]
fn unauthored_light_tracks_default_to_white_visible_and_unit_attenuation() {
    let light = unauthored_light();
    let time = LightTime {
        sequence: 5,
        time_ms: 700,
        global_ms: 900,
        global_sequences: &[],
    };
    assert_eq!(
        point_light(&light, 0x8000, &time).unwrap(),
        PointLight {
            color: [1.0; 3],
            attenuation_start: 1.0,
            attenuation_end: 1.0,
            visible: true,
        }
    );
    assert!(!light_animates(&light));
}

#[test]
fn reversed_authored_attenuation_ends_one_yard_after_start() {
    let mut light = unauthored_light();
    light.attenuation_start = track(-1, vec![(vec![0], vec![4.0])]);
    light.attenuation_end = track(-1, vec![(vec![0], vec![2.0])]);
    let time = LightTime {
        sequence: 0,
        time_ms: 0,
        global_ms: 0,
        global_sequences: &[],
    };
    let point = point_light(&light, 0x8000, &time).unwrap();
    assert_eq!((point.attenuation_start, point.attenuation_end), (4.0, 5.0));
    let retail = point_light(&light, 0, &time).unwrap();
    assert_eq!(
        (retail.attenuation_start, retail.attenuation_end),
        (1.6666, 5.266_660_2)
    );
}

#[test]
fn directional_light_is_not_emitted_as_a_point_light() {
    let mut light = unauthored_light();
    light.light_type = 0;
    let time = LightTime {
        sequence: 0,
        time_ms: 0,
        global_ms: 0,
        global_sequences: &[],
    };
    assert_eq!(point_light(&light, 0, &time), None);
}

#[test]
fn torch_billboard_flags_turn_the_halo_and_flame_but_not_the_handle() {
    let model = torch();
    let local = Mat3::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let pivot = Vec3::new(0.5, 1.0, -3.0);
    for yaw in [0.0, 0.7, 2.4] {
        let view = Affine3A::from_scale_rotation_translation(
            Vec3::new(2.0, 3.0, 4.0),
            Quat::from_rotation_y(yaw),
            pivot,
        );
        let halo =
            game_engine_core::m2_billboard::billboard_bone(model.bones[1].flags, view, local);
        let flame =
            game_engine_core::m2_billboard::billboard_bone(model.bones[2].flags, view, local);
        // Both face the camera. Only halo flag 0x200 preserves its local roll.
        for bone in [halo, flame] {
            assert!(Vec3::from(bone.matrix3.x_axis).distance(Vec3::Z * 2.0) < 1e-5);
            assert_eq!(Vec3::from(bone.translation), pivot);
        }
        assert!(Vec3::from(halo.matrix3.y_axis).distance(Vec3::NEG_X * 3.0) < 1e-5);
        assert!(Vec3::from(halo.matrix3.z_axis).distance(Vec3::NEG_Y * 4.0) < 1e-5);
        assert!(Vec3::from(flame.matrix3.y_axis).distance(Vec3::Y * 3.0) < 1e-5);
        assert!(Vec3::from(flame.matrix3.z_axis).distance(Vec3::NEG_X * 4.0) < 1e-5);
        assert_eq!(
            game_engine_core::m2_billboard::billboard_bone(model.bones[0].flags, view, local),
            view
        );
    }
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

#[test]
fn start_fraction_places_the_linear_ramp_within_the_reach() {
    let point = |attenuation_start, attenuation_end| PointLight {
        color: [1.0; 3],
        attenuation_start,
        attenuation_end,
        visible: true,
    };
    // Retail's forced torch reach: flat to 1.6666 of 5.2667 yd.
    assert!((point(1.6666, 5.266_660_2).start_fraction() - 0.316_445).abs() < 1e-5);
    // Unauthored tracks (1, 1) light fully up to the end.
    assert_eq!(point(1.0, 1.0).start_fraction(), 1.0);
    // A zero-reach light and a negative start carry no NaN or negative ramp.
    assert_eq!(point(0.0, 0.0).start_fraction(), 0.0);
    assert_eq!(point(-1.0, 4.0).start_fraction(), 0.0);
}
