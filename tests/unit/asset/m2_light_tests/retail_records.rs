use super::*;

const RETAIL_LIGHT_RECORD_BYTES: usize = 156;
const ARRAY_START: usize = 384;

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
