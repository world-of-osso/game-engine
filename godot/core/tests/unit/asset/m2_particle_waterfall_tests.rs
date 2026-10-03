use super::*;

const MIST_FDID: u32 = 1028937;
const PARTICLE_ARRAY_OFFSET: usize = 0x128;
const GRAVITY_TRACK_OFFSET: usize = 0x84;
const SCALE_VARIATION_Y_OFFSET: usize = 0x138;

fn read_particle_md20(fdid: u32) -> Vec<u8> {
    let path = format!("data/models/{fdid}.m2");
    let bytes = std::fs::read(path).expect("authored waterfall mist fixture");
    assert_eq!(&bytes[..4], b"MD21");
    let length = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    bytes[8..8 + length].to_vec()
}

fn load_waterfall_visual_fields(fdid: u32) -> [f32; 7] {
    let emitters = parse_particle_emitters(&read_particle_md20(fdid));
    assert_eq!(emitters.len(), 1);
    let emitter = &emitters[0];
    [
        emitter.tail_length,
        emitter.twinkle_speed,
        emitter.twinkle_percent,
        emitter.twinkle_scale_min,
        emitter.twinkle_scale_max,
        emitter.burst_multiplier,
        emitter.drag,
    ]
}

#[test]
fn waterfall_mist_reads_authored_tail_twinkle_burst_and_drag() {
    assert_eq!(
        load_waterfall_visual_fields(1028937),
        [1.0, 18.0, 1.0, 8.0, 8.0, 1.0, 5.0],
        "tail length, twinkle speed/percent/min/max, burst multiplier, drag",
    );
}

#[test]
fn waterfall_ripple_reads_authored_tail_twinkle_burst_and_drag() {
    assert_eq!(
        load_waterfall_visual_fields(2904370),
        [0.1, 10.0, 1.0, 180.0, 180.0, 1.0, 0.0],
        "tail length, twinkle speed/percent/min/max, burst multiplier, drag",
    );
}

fn mist_emitter_offset(md20: &[u8]) -> usize {
    let count = read_u32(md20, PARTICLE_ARRAY_OFFSET).unwrap();
    assert_eq!(count, 1);
    read_u32(md20, PARTICLE_ARRAY_OFFSET + 4).unwrap() as usize
}

#[test]
fn waterfall_mist_retains_three_authored_textures_and_uv_motion() {
    let mut emitters = parse_particle_emitters(&read_particle_md20(MIST_FDID));
    resolve_texture_fdids(&mut emitters, &[1029067, 1029068]);
    let multi = emitters[0]
        .multi_texture
        .as_ref()
        .expect("waterfall mist declares three texture layers");
    assert_eq!(multi.texture_indices, [0, 1, 1]);
    assert_eq!(
        multi.texture_fdids,
        [Some(1029067), Some(1029068), Some(1029068)]
    );
    assert_eq!(multi.uv_scale_bytes, [16, 44]);
    assert_eq!(multi.velocity_midpoints, [[0.0, 0.25], [0.0, 0.3984375]]);
    assert_eq!(
        multi.velocity_ranges,
        [[0.048828125, 0.099609375], [0.048828125, 0.048828125]],
    );
    assert_eq!(emitters[0].texture_fdid, Some(1029067));
}

#[test]
fn waterfall_mist_missing_layer_ids_are_not_replaced_by_the_first_texture() {
    let mut emitters = parse_particle_emitters(&read_particle_md20(MIST_FDID));
    resolve_texture_fdids(&mut emitters, &[1029067]);
    let multi = emitters[0].multi_texture.as_ref().unwrap();
    assert_eq!(multi.texture_fdids, [Some(1029067), None, None]);
}

#[test]
fn waterfall_mist_preserves_signed_uv_velocity() {
    const VELOCITY_MIDPOINT_OFFSET: usize = 0x1DC;
    let mut md20 = read_particle_md20(MIST_FDID);
    let key = mist_emitter_offset(&md20) + VELOCITY_MIDPOINT_OFFSET;
    md20[key..key + 2].copy_from_slice(&(-128_i16).to_le_bytes());
    md20[key + 2..key + 4].copy_from_slice(&(-512_i16).to_le_bytes());
    let emitters = parse_particle_emitters(&md20);
    let multi = emitters[0].multi_texture.as_ref().unwrap();
    assert_eq!(multi.velocity_midpoints[0], [-0.25, -1.0]);
    assert_eq!(multi.velocity_midpoints[1], [0.0, 0.3984375]);
}

#[test]
fn waterfall_single_texture_mist_retains_its_texture_index() {
    let mut emitters = parse_particle_emitters(&read_particle_md20(2904370));
    assert_eq!(emitters.len(), 1);
    assert_eq!(emitters[0].texture_index, 4);
    assert_eq!(emitters[0].multi_texture, None);
    resolve_texture_fdids(&mut emitters, &[0, 0, 0, 0, 2904679]);
    assert_eq!(emitters[0].texture_fdid, Some(2904679));
    assert_eq!(emitters[0].scale_variation, 0.0);
    assert_eq!(emitters[0].scale_variation_y, 0.0);
}

#[test]
fn waterfall_mist_reads_authored_scale_variation() {
    let emitters = parse_particle_emitters(&read_particle_md20(MIST_FDID));
    assert_eq!(emitters.len(), 1);
    assert_eq!(emitters[0].scale_variation, 0.5);
    assert_eq!(emitters[0].scale_variation_y, 0.0);
}

#[test]
fn waterfall_compressed_gravity_is_independent_of_scale_variation() {
    let mut md20 = read_particle_md20(MIST_FDID);
    let emitter = mist_emitter_offset(&md20);
    let outer = read_u32(&md20, emitter + GRAVITY_TRACK_OFFSET + 16).unwrap() as usize;
    let gravity_key = read_u32(&md20, outer + 4).unwrap() as usize;
    // A signed compressed Z value of -1 is a NaN if misread as an IEEE float.
    md20[gravity_key..gravity_key + 4].copy_from_slice(&[0, 0, 255, 255]);
    let scale_y = emitter + SCALE_VARIATION_Y_OFFSET;
    md20[scale_y..scale_y + 4].copy_from_slice(&0.75_f32.to_le_bytes());

    let emitters = parse_particle_emitters(&md20);
    assert_eq!(emitters.len(), 1);
    assert_eq!(emitters[0].gravity, 0.042_386_48);
    assert_eq!(emitters[0].gravity_vector, [0.0, 0.0, -0.042_386_48]);
    assert_eq!(emitters[0].scale_variation_y, 0.75);
}
