use crate::asset::m2_anim::{evaluate_f32_track, evaluate_vec3_track};
use crate::asset::m2_light::{M2_LIGHT_TYPE_POINT, M2Light, evaluate_light};

const FLOAT_TOLERANCE: f32 = 0.000_01;

struct ExpectedLight {
    bone: i16,
    diffuse: [f32; 3],
    intensity: f32,
    attenuation: [f32; 2],
}

#[test]
fn retail_cauldron_preserves_both_authored_lights() {
    let lights = load_retail_lights(4238519);
    assert_eq!(
        lights.len(),
        2,
        "cauldron must retain both authored records"
    );
    let expected = [
        ExpectedLight {
            bone: 1,
            diffuse: [1.0, 0.352_941_2, 0.0],
            intensity: 1.499_85,
            attenuation: [1.816_882_3, 3.617_28],
        },
        ExpectedLight {
            bone: 2,
            diffuse: [1.0, 0.564_705_9, 0.0],
            intensity: 1.0,
            attenuation: [0.722_222_2, 2.033_333_3],
        },
    ];
    for (light, expected) in lights.iter().zip(expected) {
        assert_authored_light(light, expected);
    }
}

#[test]
fn retail_wall_lantern_preserves_four_authored_lights() {
    assert_retail_lantern_lights(5149702);
}

#[test]
fn retail_lit_lantern_preserves_four_authored_lights() {
    assert_retail_lantern_lights(5140152);
}

fn load_retail_lights(fdid: u32) -> Vec<M2Light> {
    let path = format!("data/models/{fdid}.m2");
    crate::asset::m2::load_m2_uncached(std::path::Path::new(&path), &[0; 3])
        .unwrap_or_else(|error| panic!("retail fixture {fdid}: {error}"))
        .lights
}

fn assert_retail_lantern_lights(fdid: u32) {
    let lights = load_retail_lights(fdid);
    assert_eq!(
        lights.len(),
        4,
        "lantern {fdid} must retain all four records"
    );
    for (light, bone) in lights.iter().zip([2, 3, 4, 5]) {
        assert_authored_light(
            light,
            ExpectedLight {
                bone,
                diffuse: [0.976_470_6, 0.752_941_2, 0.490_196_08],
                intensity: 1.1,
                attenuation: [1.944_444_4, 6.25],
            },
        );
    }
}

fn assert_authored_light(light: &M2Light, expected: ExpectedLight) {
    assert_eq!(light.light_type, M2_LIGHT_TYPE_POINT);
    assert_eq!(light.bone_index, expected.bone);
    let ambient = evaluate_vec3_track(&light.ambient_color, 0, 0).unwrap();
    let diffuse = evaluate_vec3_track(&light.diffuse_color, 0, 0).unwrap();
    for channel in 0..3 {
        assert_close(ambient[channel], 1.0);
        assert_close(diffuse[channel], expected.diffuse[channel]);
    }
    assert_close(
        evaluate_f32_track(&light.ambient_intensity, 0, 0).unwrap(),
        0.0,
    );
    assert_close(
        evaluate_f32_track(&light.diffuse_intensity, 0, 0).unwrap(),
        expected.intensity,
    );
    assert_close(
        evaluate_f32_track(&light.attenuation_start, 0, 0).unwrap(),
        expected.attenuation[0],
    );
    assert_close(
        evaluate_f32_track(&light.attenuation_end, 0, 0).unwrap(),
        expected.attenuation[1],
    );
    assert!(evaluate_light(light, 0, 0, 0, &[]).visible);
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= FLOAT_TOLERANCE,
        "expected {expected}, got {actual}",
    );
}
