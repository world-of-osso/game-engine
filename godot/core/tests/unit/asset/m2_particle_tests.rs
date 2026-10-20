use game_engine_core::asset::m2_format::m2_particle::*;

#[path = "../required_asset.rs"]
mod required_asset;
use required_asset::require_asset;

const RANDOM_TEXTURE_FLAG: u32 = 0x0010_0000;

#[test]
fn parse_torch_particle_emitter() {
    let path = std::path::Path::new("data/models/club_1h_torch_a_01.m2");
    let data = std::fs::read(require_asset(path)).unwrap();
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
        em.flags & RANDOM_TEXTURE_FLAG,
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
    let data = std::fs::read(require_asset(path)).unwrap();
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
    let data = std::fs::read(require_asset(path)).unwrap();
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
fn parse_274_particle_emitters_keep_legacy_header_offsets() {
    let path = std::path::Path::new("data/models/5152423.m2");
    let data = std::fs::read(require_asset(path)).unwrap();
    let md20_size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let md20 = &data[8..8 + md20_size];
    let emitters_offset = u32::from_le_bytes(md20[0x12C..0x130].try_into().unwrap()) as usize;
    let emitter = &md20[emitters_offset..emitters_offset + 0x1EC];

    assert_eq!(
        u32::from_le_bytes(emitter[0x18..0x1C].try_into().unwrap()),
        0,
        "274 emitter keeps legacy geometry filename count at 0x18"
    );
    assert_eq!(
        u32::from_le_bytes(emitter[0x20..0x24].try_into().unwrap()),
        0,
        "274 emitter keeps legacy recursion filename count at 0x20"
    );
    assert_eq!(emitter[0x28], 2, "blend type still lives at 0x28");
    assert_eq!(emitter[0x29], 1, "emitter type still lives at 0x29");
    assert_eq!(emitter[0x2A], 0, "particle type still lives at 0x2A");
    assert_eq!(emitter[0x2B], 0, "head/tail still lives at 0x2B");
    assert_eq!(
        u16::from_le_bytes(emitter[0x30..0x32].try_into().unwrap()),
        2,
        "tile rows still live at 0x30"
    );
    assert_eq!(
        u16::from_le_bytes(emitter[0x32..0x34].try_into().unwrap()),
        4,
        "tile cols still live at 0x32"
    );
}
