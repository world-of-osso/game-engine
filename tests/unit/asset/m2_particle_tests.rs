use super::*;

#[test]
fn parse_torch_particle_emitter() {
    let path = std::path::Path::new("data/models/club_1h_torch_a_01.m2");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(path).unwrap();
    let md20_size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let md20 = &data[8..8 + md20_size];

    let emitters = parse_particle_emitters(md20);
    assert_eq!(emitters.len(), 1, "torch should have 1 particle emitter");

    let em = &emitters[0];
    assert_eq!(em.bone_index, 10);
    assert_eq!(em.blend_type, 4, "additive blending");
    assert_eq!(em.emitter_type, 1, "sphere emitter");
    assert_eq!(em.tile_rows, 4);
    assert_eq!(em.tile_cols, 4);
    assert!(em.emission_speed > 0.5, "speed={}", em.emission_speed);
    assert!(em.lifespan > 0.7, "lifespan={}", em.lifespan);
    assert!(em.emission_rate > 19.0, "rate={}", em.emission_rate);
    assert_eq!(
        em.head_cell_track,
        [0, 7, 8],
        "torch uses an authored head cell track"
    );
    assert_eq!(
        em.tail_cell_track,
        [0, 0, 0],
        "torch has no authored tail cell track"
    );
    assert_eq!(
        em.flags & 0x0010_0000,
        0,
        "torch does not use RANDOM_TEXTURE, so sprite selection comes from the authored head track"
    );
    assert!(em.colors[0][0] > 200.0, "start red={}", em.colors[0][0]);
    assert!(em.opacity[1] > 0.9, "mid opacity={}", em.opacity[1]);
    assert!(
        em.opacity[2] <= 0.001,
        "torch authored end opacity already fades to zero: {:?}",
        em.opacity
    );
    assert!(em.burst_multiplier > 0.9, "burst={}", em.burst_multiplier);
}

#[test]
fn parse_272_particle_emitters_use_full_stride() {
    let path = std::path::Path::new("data/models/390126.m2");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(path).unwrap();
    let md20_size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let md20 = &data[8..8 + md20_size];

    let emitters = parse_particle_emitters(md20);

    assert_eq!(emitters.len(), 3);
    assert_eq!(emitters[1].blend_type, 4);
    assert_eq!(emitters[1].emitter_type, 1);
    assert_eq!(emitters[1].tile_rows, 4);
    assert_eq!(emitters[1].tile_cols, 4);
}

#[test]
fn parse_274_particle_emitters_use_full_stride() {
    let path = std::path::Path::new("data/models/5152423.m2");
    if !path.exists() {
        return;
    }
    let data = std::fs::read(path).unwrap();
    let md20_size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let md20 = &data[8..8 + md20_size];

    let emitters = parse_particle_emitters(md20);

    assert_eq!(emitters.len(), 5);
    assert_eq!(emitters[0].blend_type, 2);
    assert_eq!(emitters[0].tile_rows, 2);
    assert_eq!(emitters[0].tile_cols, 4);
    assert!((emitters[0].base_spin_variation - std::f32::consts::TAU).abs() < 0.0001);
    assert!((emitters[0].spin - 0.87266463).abs() < 0.0001);
    assert!((emitters[0].spin_variation - 0.08726646).abs() < 0.0001);
    assert_eq!(emitters[3].emitter_type, 2);
    assert_eq!(emitters[3].tile_rows, 2);
    assert_eq!(emitters[3].tile_cols, 2);
}

#[test]
fn opacity_values_use_signed_fixed16() {
    let mut md20 = vec![0u8; 8];
    let emitter = [
        0, 0, 0, 0, 0, 0, 0, 0, // timestamps
        2, 0, 0, 0, // count
        0, 0, 0, 0, // offset placeholder
    ];
    let values_offset = md20.len();
    md20.extend_from_slice(&(-1_i16).to_le_bytes());
    md20.extend_from_slice(&(16384_i16).to_le_bytes());
    let mut emitter = emitter;
    emitter[12..16].copy_from_slice(&(values_offset as u32).to_le_bytes());

    let opacities = read_opacity_values(&md20, &emitter, 0);

    assert_eq!(opacities[0], 0.0);
    assert!((opacities[1] - (16384.0 / 32767.0)).abs() < 0.0001);
}

#[test]
fn color_keys_preserve_full_fake_animblock_timeline() {
    let mut md20 = vec![0u8; 96];
    let mut emitter = vec![0u8; 16];

    emitter[0..4].copy_from_slice(&(4u32).to_le_bytes());
    emitter[4..8].copy_from_slice(&(32u32).to_le_bytes());
    emitter[8..12].copy_from_slice(&(4u32).to_le_bytes());
    emitter[12..16].copy_from_slice(&(40u32).to_le_bytes());

    for (idx, time) in [0u16, 8192, 16384, 32767].into_iter().enumerate() {
        md20[32 + idx * 2..34 + idx * 2].copy_from_slice(&time.to_le_bytes());
    }
    let colors = [
        [1.0f32, 2.0, 3.0],
        [4.0f32, 5.0, 6.0],
        [7.0f32, 8.0, 9.0],
        [10.0f32, 11.0, 12.0],
    ];
    for (idx, color) in colors.into_iter().enumerate() {
        let base = 40 + idx * 12;
        md20[base..base + 4].copy_from_slice(&color[0].to_le_bytes());
        md20[base + 4..base + 8].copy_from_slice(&color[1].to_le_bytes());
        md20[base + 8..base + 12].copy_from_slice(&color[2].to_le_bytes());
    }

    let keys = read_color_keys(&md20, &emitter, 0);

    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0], (0.0, [1.0, 2.0, 3.0]));
    assert!((keys[1].0 - (8192.0 / 32767.0)).abs() < 0.0001);
    assert_eq!(keys[1].1, [4.0, 5.0, 6.0]);
    assert!((keys[2].0 - (16384.0 / 32767.0)).abs() < 0.0001);
    assert_eq!(keys[2].1, [7.0, 8.0, 9.0]);
    assert_eq!(keys[3], (1.0, [10.0, 11.0, 12.0]));
}

#[test]
fn opacity_keys_preserve_full_fake_animblock_timeline() {
    let mut md20 = vec![0u8; 64];
    let mut emitter = vec![0u8; 16];

    emitter[0..4].copy_from_slice(&(4u32).to_le_bytes());
    emitter[4..8].copy_from_slice(&(32u32).to_le_bytes());
    emitter[8..12].copy_from_slice(&(4u32).to_le_bytes());
    emitter[12..16].copy_from_slice(&(40u32).to_le_bytes());

    for (idx, time) in [0u16, 8192, 16384, 32767].into_iter().enumerate() {
        md20[32 + idx * 2..34 + idx * 2].copy_from_slice(&time.to_le_bytes());
    }
    for (idx, value) in [0i16, 8192, 16384, 32767].into_iter().enumerate() {
        md20[40 + idx * 2..42 + idx * 2].copy_from_slice(&value.to_le_bytes());
    }

    let keys = read_opacity_keys(&md20, &emitter, 0);

    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0], (0.0, 0.0));
    assert!((keys[1].0 - (8192.0 / 32767.0)).abs() < 0.0001);
    assert!((keys[1].1 - (8192.0 / 32767.0)).abs() < 0.0001);
    assert!((keys[2].0 - (16384.0 / 32767.0)).abs() < 0.0001);
    assert!((keys[2].1 - (16384.0 / 32767.0)).abs() < 0.0001);
    assert_eq!(keys[3], (1.0, 1.0));
}

#[test]
fn parses_head_tail_tracks_and_burst_multiplier() {
    let mut md20 = vec![0u8; 0x180];
    let mut emitter = vec![0u8; 0x178];

    let head_offset = 0x40usize;
    md20[head_offset..head_offset + 6].copy_from_slice(&[1, 0, 2, 0, 3, 0]);
    emitter[0x13C + 8..0x13C + 12].copy_from_slice(&(3u32).to_le_bytes());
    emitter[0x13C + 12..0x13C + 16].copy_from_slice(&(head_offset as u32).to_le_bytes());

    let tail_offset = 0x50usize;
    md20[tail_offset..tail_offset + 6].copy_from_slice(&[4, 0, 5, 0, 6, 0]);
    emitter[0x14C + 8..0x14C + 12].copy_from_slice(&(3u32).to_le_bytes());
    emitter[0x14C + 12..0x14C + 16].copy_from_slice(&(tail_offset as u32).to_le_bytes());

    emitter[0x174..0x178].copy_from_slice(&(1.75_f32).to_le_bytes());

    let mut parsed = parse_emitter_header(&emitter).unwrap();
    fill_visual_values(&mut parsed, &md20, &emitter);

    assert_eq!(parsed.head_cell_track, [1, 2, 3]);
    assert_eq!(parsed.tail_cell_track, [4, 5, 6]);
    assert!((parsed.burst_multiplier - 1.75).abs() < 0.0001);
}

#[test]
fn parses_particle_type_and_head_or_tail() {
    let mut emitter = vec![0u8; 0x178];
    emitter[0x2A] = 1;
    emitter[0x2B] = 2;

    let parsed = parse_emitter_header(&emitter).unwrap();

    assert_eq!(parsed.particle_type, 1);
    assert_eq!(parsed.head_or_tail, 2);
}

#[test]
fn parses_spin_fields_from_272_suffix() {
    let mut md20 = vec![0u8; 0x1ec];
    md20[EMITTER_BASE_SPIN_OFFSET..EMITTER_BASE_SPIN_OFFSET + 4]
        .copy_from_slice(&(0.25_f32).to_le_bytes());
    md20[EMITTER_BASE_SPIN_VARIATION_OFFSET..EMITTER_BASE_SPIN_VARIATION_OFFSET + 4]
        .copy_from_slice(&(1.5_f32).to_le_bytes());
    md20[EMITTER_SPIN_OFFSET..EMITTER_SPIN_OFFSET + 4].copy_from_slice(&(0.75_f32).to_le_bytes());
    md20[EMITTER_SPIN_VARIATION_OFFSET..EMITTER_SPIN_VARIATION_OFFSET + 4]
        .copy_from_slice(&(0.5_f32).to_le_bytes());

    let mut parsed = parse_emitter_header(&md20).unwrap();
    fill_track_values(&mut parsed, &md20, &md20);

    assert!((parsed.base_spin - 0.25).abs() < 0.0001);
    assert!((parsed.base_spin_variation - 1.5).abs() < 0.0001);
    assert!((parsed.spin - 0.75).abs() < 0.0001);
    assert!((parsed.spin_variation - 0.5).abs() < 0.0001);
}

#[test]
fn parses_tail_length_from_272_suffix() {
    let mut md20 = vec![0u8; 0x1ec];
    md20[EMITTER_TAIL_LENGTH_OFFSET..EMITTER_TAIL_LENGTH_OFFSET + 4]
        .copy_from_slice(&(2.5_f32).to_le_bytes());

    let mut parsed = parse_emitter_header(&md20).unwrap();
    fill_track_values(&mut parsed, &md20, &md20);

    assert!((parsed.tail_length - 2.5).abs() < 0.0001);
}

#[test]
fn parses_size_variation_fields_from_272_suffix() {
    let mut md20 = vec![0u8; 0x1ec];
    md20[EMITTER_SCALE_VARIATION_OFFSET..EMITTER_SCALE_VARIATION_OFFSET + 4]
        .copy_from_slice(&(0.4_f32).to_le_bytes());
    md20[EMITTER_SCALE_VARIATION_Y_OFFSET..EMITTER_SCALE_VARIATION_Y_OFFSET + 4]
        .copy_from_slice(&(0.2_f32).to_le_bytes());

    let mut parsed = parse_emitter_header(&md20).unwrap();
    fill_track_values(&mut parsed, &md20, &md20);

    assert!((parsed.scale_variation - 0.4).abs() < 0.0001);
    assert!((parsed.scale_variation_y - 0.2).abs() < 0.0001);
}

#[test]
fn parses_lifespan_variation_from_272_suffix() {
    let mut md20 = vec![0u8; 0x1ec];
    md20[EMITTER_LIFESPAN_VARIATION_OFFSET..EMITTER_LIFESPAN_VARIATION_OFFSET + 4]
        .copy_from_slice(&(0.4_f32).to_le_bytes());

    let mut parsed = parse_emitter_header(&md20).unwrap();
    fill_track_values(&mut parsed, &md20, &md20);

    assert!((parsed.lifespan_variation - 0.4).abs() < 0.0001);
}

#[test]
fn parses_uncompressed_gravity_as_negative_wow_z() {
    let mut md20 = vec![0u8; 0x1ec];
    let mut emitter = vec![0u8; 0x1ec];
    let gravity_offset = 0x40usize;
    md20[gravity_offset..gravity_offset + 4].copy_from_slice(&(2.0_f32).to_le_bytes());
    emitter[EMITTER_GRAVITY_OFFSET + 12..EMITTER_GRAVITY_OFFSET + 16]
        .copy_from_slice(&(1u32).to_le_bytes());
    emitter[EMITTER_GRAVITY_OFFSET + 16..EMITTER_GRAVITY_OFFSET + 20]
        .copy_from_slice(&(gravity_offset as u32).to_le_bytes());

    let mut parsed = parse_emitter_header(&emitter).unwrap();
    fill_track_values(&mut parsed, &md20, &emitter);

    assert!((parsed.gravity - 2.0).abs() < 0.0001);
    assert_eq!(parsed.gravity_vector, [0.0, 0.0, -2.0]);
}

#[test]
fn parses_compressed_gravity_vector() {
    let mut md20 = vec![0u8; 0x1ec];
    let mut emitter = vec![0u8; 0x1ec];
    emitter[EMITTER_FLAGS_OFFSET..EMITTER_FLAGS_OFFSET + 4]
        .copy_from_slice(&(0x0080_0000u32).to_le_bytes());
    let gravity_offset = 0x40usize;
    md20[gravity_offset..gravity_offset + 4].copy_from_slice(&[64, 0, 100, 0]);
    emitter[EMITTER_GRAVITY_OFFSET + 12..EMITTER_GRAVITY_OFFSET + 16]
        .copy_from_slice(&(1u32).to_le_bytes());
    emitter[EMITTER_GRAVITY_OFFSET + 16..EMITTER_GRAVITY_OFFSET + 20]
        .copy_from_slice(&(gravity_offset as u32).to_le_bytes());

    let mut parsed = parse_emitter_header(&emitter).unwrap();
    fill_track_values(&mut parsed, &md20, &emitter);

    assert!((parsed.gravity_vector[0] - 2.119324).abs() < 0.0001);
    assert!((parsed.gravity_vector[1] - 0.0).abs() < 0.0001);
    assert!((parsed.gravity_vector[2] - 3.670643).abs() < 0.0002);
}

#[test]
fn parses_twinkle_fields_from_272_suffix() {
    let mut md20 = vec![0u8; 0x1ec];
    md20[EMITTER_TWINKLE_SPEED_OFFSET..EMITTER_TWINKLE_SPEED_OFFSET + 4]
        .copy_from_slice(&(2.0_f32).to_le_bytes());
    md20[EMITTER_TWINKLE_PERCENT_OFFSET..EMITTER_TWINKLE_PERCENT_OFFSET + 4]
        .copy_from_slice(&(0.75_f32).to_le_bytes());
    md20[EMITTER_TWINKLE_SCALE_MIN_OFFSET..EMITTER_TWINKLE_SCALE_MIN_OFFSET + 4]
        .copy_from_slice(&(0.5_f32).to_le_bytes());
    md20[EMITTER_TWINKLE_SCALE_MAX_OFFSET..EMITTER_TWINKLE_SCALE_MAX_OFFSET + 4]
        .copy_from_slice(&(1.5_f32).to_le_bytes());

    let mut parsed = parse_emitter_header(&md20).unwrap();
    fill_visual_values(&mut parsed, &md20, &md20);

    assert!((parsed.twinkle_speed - 2.0).abs() < 0.0001);
    assert!((parsed.twinkle_percent - 0.75).abs() < 0.0001);
    assert!((parsed.twinkle_scale_min - 0.5).abs() < 0.0001);
    assert!((parsed.twinkle_scale_max - 1.5).abs() < 0.0001);
}

#[test]
fn parses_wind_fields_from_272_suffix() {
    let mut md20 = vec![0u8; 0x1ec];
    md20[EMITTER_WIND_VECTOR_OFFSET..EMITTER_WIND_VECTOR_OFFSET + 4]
        .copy_from_slice(&(1.0_f32).to_le_bytes());
    md20[EMITTER_WIND_VECTOR_OFFSET + 4..EMITTER_WIND_VECTOR_OFFSET + 8]
        .copy_from_slice(&(2.0_f32).to_le_bytes());
    md20[EMITTER_WIND_VECTOR_OFFSET + 8..EMITTER_WIND_VECTOR_OFFSET + 12]
        .copy_from_slice(&(3.0_f32).to_le_bytes());
    md20[EMITTER_WIND_TIME_OFFSET..EMITTER_WIND_TIME_OFFSET + 4]
        .copy_from_slice(&(4.5_f32).to_le_bytes());

    let mut parsed = parse_emitter_header(&md20).unwrap();
    fill_track_values(&mut parsed, &md20, &md20);

    assert_eq!(parsed.wind_vector, [1.0, 2.0, 3.0]);
    assert!((parsed.wind_time - 4.5).abs() < 0.0001);
}

#[test]
fn scale_keys_preserve_full_fake_animblock_timeline() {
    let mut md20 = vec![0u8; 96];
    let mut emitter = vec![0u8; 16];

    emitter[0..4].copy_from_slice(&(4u32).to_le_bytes());
    emitter[4..8].copy_from_slice(&(32u32).to_le_bytes());
    emitter[8..12].copy_from_slice(&(4u32).to_le_bytes());
    emitter[12..16].copy_from_slice(&(40u32).to_le_bytes());

    for (idx, time) in [0u16, 8192, 16384, 32767].into_iter().enumerate() {
        md20[32 + idx * 2..34 + idx * 2].copy_from_slice(&time.to_le_bytes());
    }
    let scales = [[1.0f32, 2.0], [3.0f32, 4.0], [5.0f32, 6.0], [7.0f32, 8.0]];
    for (idx, scale) in scales.into_iter().enumerate() {
        let base = 40 + idx * 8;
        md20[base..base + 4].copy_from_slice(&scale[0].to_le_bytes());
        md20[base + 4..base + 8].copy_from_slice(&scale[1].to_le_bytes());
    }

    let keys = read_scale_keys(&md20, &emitter, 0);

    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0], (0.0, [1.0, 2.0]));
    assert!((keys[1].0 - (8192.0 / 32767.0)).abs() < 0.0001);
    assert_eq!(keys[1].1, [3.0, 4.0]);
    assert!((keys[2].0 - (16384.0 / 32767.0)).abs() < 0.0001);
    assert_eq!(keys[2].1, [5.0, 6.0]);
    assert_eq!(keys[3], (1.0, [7.0, 8.0]));
}
