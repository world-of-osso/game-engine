use super::*;

const MIST_FDID: u32 = 1028937;
const PARTICLE_ARRAY_OFFSET: usize = 0x128;
const GRAVITY_TRACK_OFFSET: usize = 0x84;
const SCALE_VARIATION_Y_OFFSET: usize = 0x138;

fn read_mist_md20() -> Vec<u8> {
    let path = format!("data/models/{MIST_FDID}.m2");
    let bytes = std::fs::read(path).expect("authored waterfall mist fixture");
    assert_eq!(&bytes[..4], b"MD21");
    let length = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    bytes[8..8 + length].to_vec()
}

fn mist_emitter_offset(md20: &[u8]) -> usize {
    let count = read_u32(md20, PARTICLE_ARRAY_OFFSET).unwrap();
    assert_eq!(count, 1);
    read_u32(md20, PARTICLE_ARRAY_OFFSET + 4).unwrap() as usize
}

#[test]
fn waterfall_mist_reads_authored_scale_variation() {
    let emitters = parse_particle_emitters(&read_mist_md20());
    assert_eq!(emitters.len(), 1);
    assert_eq!(emitters[0].scale_variation, 0.5);
    assert_eq!(emitters[0].scale_variation_y, 0.0);
}

#[test]
fn waterfall_compressed_gravity_is_independent_of_scale_variation() {
    let mut md20 = read_mist_md20();
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
