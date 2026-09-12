use super::*;

const FLOAT_TOLERANCE: f32 = 0.000_01;
const RETAIL_LIGHT_RECORD_BYTES: usize = 156;
const ARRAY_START: usize = 384;

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
    assert!(evaluate_light(light, 0, 0).visible);
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= FLOAT_TOLERANCE,
        "expected {expected}, got {actual}",
    );
}

fn empty_track_light_array(count: u32) -> Vec<u8> {
    let mut bytes = vec![0; ARRAY_START + count as usize * RETAIL_LIGHT_RECORD_BYTES];
    bytes[..4].copy_from_slice(b"MD20");
    bytes[4..8].copy_from_slice(&274_u32.to_le_bytes());
    bytes[0x108..0x10C].copy_from_slice(&count.to_le_bytes());
    bytes[0x10C..0x110].copy_from_slice(&(ARRAY_START as u32).to_le_bytes());
    for index in 0..count as usize {
        let record = ARRAY_START + index * RETAIL_LIGHT_RECORD_BYTES;
        bytes[record..record + 2].copy_from_slice(&1_u16.to_le_bytes());
        bytes[record + 2..record + 4].copy_from_slice(&(index as i16 + 7).to_le_bytes());
    }
    bytes
}

#[test]
fn declared_light_array_rejects_truncated_final_record() {
    let mut bytes = empty_track_light_array(2);
    bytes.pop();
    assert!(parse_lights(&bytes).is_empty());
}

#[test]
fn declared_light_array_rejects_out_of_bounds_offset_and_count() {
    for (count, offset) in [(1_u32, u32::MAX), (u32::MAX, ARRAY_START as u32)] {
        let mut bytes = empty_track_light_array(1);
        bytes[0x108..0x10C].copy_from_slice(&count.to_le_bytes());
        bytes[0x10C..0x110].copy_from_slice(&offset.to_le_bytes());
        assert!(parse_lights(&bytes).is_empty());
    }
}

#[test]
fn malformed_light_track_preserves_only_valid_prefix() {
    let mut bytes = empty_track_light_array(3);
    let second_ambient = ARRAY_START + RETAIL_LIGHT_RECORD_BYTES + 0x10;
    let invalid_offset = bytes.len() as u32 + 64;
    bytes[second_ambient + 4..second_ambient + 8].copy_from_slice(&1_u32.to_le_bytes());
    bytes[second_ambient + 8..second_ambient + 12].copy_from_slice(&invalid_offset.to_le_bytes());
    bytes[second_ambient + 12..second_ambient + 16].copy_from_slice(&1_u32.to_le_bytes());
    bytes[second_ambient + 16..second_ambient + 20].copy_from_slice(&invalid_offset.to_le_bytes());
    let lights = parse_lights(&bytes);
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].bone_index, 7);
}
