use super::*;

const SINGLE_LIGHT_FIXTURE_SIZE: usize = 0x53C;

#[test]
fn parse_lights_reads_single_point_light() {
    let mut md20 = vec![0u8; SINGLE_LIGHT_FIXTURE_SIZE];
    let light_base = 0x180usize;
    md20[MD20_LIGHTS_OFFSET..MD20_LIGHTS_OFFSET + 4].copy_from_slice(&(1u32).to_le_bytes());
    md20[MD20_LIGHTS_OFFSET + 4..MD20_LIGHTS_OFFSET + 8]
        .copy_from_slice(&(light_base as u32).to_le_bytes());
    md20[light_base..light_base + 2].copy_from_slice(&(M2_LIGHT_TYPE_POINT).to_le_bytes());
    md20[light_base + 2..light_base + 4].copy_from_slice(&(10i16).to_le_bytes());
    md20[light_base + 4..light_base + 8].copy_from_slice(&(1.25f32).to_le_bytes());
    md20[light_base + 8..light_base + 12].copy_from_slice(&(-2.0f32).to_le_bytes());
    md20[light_base + 12..light_base + 16].copy_from_slice(&(3.75f32).to_le_bytes());
    // ambient_color track with one key [0.2, 0.3, 0.4]
    md20[light_base + 0x10..light_base + 0x12].copy_from_slice(&0u16.to_le_bytes());
    md20[light_base + 0x12..light_base + 0x14].copy_from_slice(&(-1i16).to_le_bytes());
    md20[light_base + 0x14..light_base + 0x18].copy_from_slice(&(1u32).to_le_bytes());
    md20[light_base + 0x18..light_base + 0x1C].copy_from_slice(&(0x400u32).to_le_bytes());
    md20[light_base + 0x1C..light_base + 0x20].copy_from_slice(&(1u32).to_le_bytes());
    md20[light_base + 0x20..light_base + 0x24].copy_from_slice(&(0x420u32).to_le_bytes());
    md20[0x400..0x404].copy_from_slice(&(1u32).to_le_bytes());
    md20[0x404..0x408].copy_from_slice(&(0x500u32).to_le_bytes());
    md20[0x420..0x424].copy_from_slice(&(1u32).to_le_bytes());
    md20[0x424..0x428].copy_from_slice(&(0x520u32).to_le_bytes());
    md20[0x500..0x504].copy_from_slice(&0u32.to_le_bytes());
    md20[0x520..0x524].copy_from_slice(&(0.2f32).to_le_bytes());
    md20[0x524..0x528].copy_from_slice(&(0.3f32).to_le_bytes());
    md20[0x528..0x52C].copy_from_slice(&(0.4f32).to_le_bytes());
    // Fill remaining required tracks with static one-key defaults.
    for off in [0x24, 0x38, 0x4C, 0x60, 0x74] {
        md20[light_base + off..light_base + off + 2].copy_from_slice(&0u16.to_le_bytes());
        md20[light_base + off + 2..light_base + off + 4]
            .copy_from_slice(&(-1i16).to_le_bytes());
        md20[light_base + off + 4..light_base + off + 8].copy_from_slice(&(1u32).to_le_bytes());
        md20[light_base + off + 8..light_base + off + 12]
            .copy_from_slice(&(0x430u32).to_le_bytes());
        md20[light_base + off + 12..light_base + off + 16]
            .copy_from_slice(&(1u32).to_le_bytes());
        md20[light_base + off + 16..light_base + off + 20]
            .copy_from_slice(&(0x440u32).to_le_bytes());
    }
    md20[0x430..0x434].copy_from_slice(&(1u32).to_le_bytes());
    md20[0x434..0x438].copy_from_slice(&(0x510u32).to_le_bytes());
    md20[0x440..0x444].copy_from_slice(&(1u32).to_le_bytes());
    md20[0x444..0x448].copy_from_slice(&(0x530u32).to_le_bytes());
    md20[0x510..0x514].copy_from_slice(&0u32.to_le_bytes());
    md20[0x530..0x534].copy_from_slice(&(1.0f32).to_le_bytes());
    // visibility track
    let off = 0x88;
    md20[light_base + off..light_base + off + 2].copy_from_slice(&0u16.to_le_bytes());
    md20[light_base + off + 2..light_base + off + 4].copy_from_slice(&(-1i16).to_le_bytes());
    md20[light_base + off + 4..light_base + off + 8].copy_from_slice(&(1u32).to_le_bytes());
    md20[light_base + off + 8..light_base + off + 12]
        .copy_from_slice(&(0x450u32).to_le_bytes());
    md20[light_base + off + 12..light_base + off + 16].copy_from_slice(&(1u32).to_le_bytes());
    md20[light_base + off + 16..light_base + off + 20]
        .copy_from_slice(&(0x464u32).to_le_bytes());
    md20[0x450..0x454].copy_from_slice(&(1u32).to_le_bytes());
    md20[0x454..0x458].copy_from_slice(&(0x514u32).to_le_bytes());
    md20[0x464..0x468].copy_from_slice(&(1u32).to_le_bytes());
    md20[0x468..0x46C].copy_from_slice(&(0x534u32).to_le_bytes());
    md20[0x514..0x518].copy_from_slice(&0u32.to_le_bytes());
    md20[0x534] = 1u8;

    let lights = parse_lights(&md20);
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].light_type, M2_LIGHT_TYPE_POINT);
    assert_eq!(lights[0].bone_index, 10);
    assert_eq!(lights[0].position, [1.25, -2.0, 3.75]);
    let evaluated = evaluate_light(&lights[0], 0, 0);
    assert!(evaluated.visible);
    assert!(evaluated.intensity > 1.0);
}
