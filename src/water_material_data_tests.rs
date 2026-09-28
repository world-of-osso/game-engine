use super::{generate_water_normal_rgba, WATER_NORMAL_SIZE};

#[test]
fn normal_map_matches_original_encoded_pixels() {
    let bytes = generate_water_normal_rgba();
    assert_eq!(WATER_NORMAL_SIZE, 256);
    assert_eq!(bytes.len(), 256 * 256 * 4);
    for ((x, y), expected) in [
        ((0, 0), [128, 128, 254, 255]),
        ((1, 0), [132, 128, 254, 255]),
        ((0, 1), [127, 134, 254, 255]),
        ((127, 127), [128, 129, 254, 255]),
        ((255, 255), [124, 125, 254, 255]),
        ((255, 0), [124, 128, 254, 255]),
        ((0, 255), [128, 125, 254, 255]),
        ((42, 199), [125, 126, 254, 255]),
    ] {
        let index = ((y * WATER_NORMAL_SIZE + x) * 4) as usize;
        assert_eq!(&bytes[index..index + 4], &expected, "pixel ({x}, {y})");
    }
}

#[test]
fn normal_map_matches_original_entire_byte_stream() {
    let bytes = generate_water_normal_rgba();
    let fnv64 = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    assert_eq!(fnv64, 0x19ce3bebcdd57d27);
}
