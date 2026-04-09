use super::{MCVT_COUNT, TEST_AREA_ID};

pub(super) fn mcnk_subchunks_payload(
    include_mcsh: bool,
    include_mclv: bool,
    include_mcse: bool,
    include_mcbb: bool,
    include_mcdd: bool,
) -> Vec<u8> {
    let mut payload = Vec::new();
    append_base_subchunks(&mut payload);
    append_optional_mcsh_subchunks(&mut payload, include_mcsh);
    append_optional_mclv_subchunk(&mut payload, include_mclv);
    append_optional_mcse_subchunk(&mut payload, include_mcse);
    append_optional_mcbb_subchunk(&mut payload, include_mcbb);
    append_optional_mcdd_subchunk(&mut payload, include_mcdd);
    payload
}

pub(super) fn adt_file_payload(include_blend_mesh: bool) -> Vec<u8> {
    let mut payload = Vec::new();
    append_subchunk(
        &mut payload,
        b"KNCM",
        mcnk_payload(false, false, false, true, false),
    );
    append_subchunk(&mut payload, b"OFBM", mfbo_payload());
    if include_blend_mesh {
        append_subchunk(
            &mut payload,
            b"HMBM",
            blend_mesh_header_payload(77, 5, 0, 3, 3, 0, 0),
        );
        append_subchunk(&mut payload, b"BBBM", blend_mesh_bounds_payload(77));
        append_subchunk(&mut payload, b"VNBM", blend_mesh_vertex_payload());
        append_subchunk(&mut payload, b"IMBM", blend_mesh_index_payload());
    }
    payload
}

pub(super) fn append_subchunk(payload: &mut Vec<u8>, tag: &[u8; 4], chunk_payload: Vec<u8>) {
    payload.extend_from_slice(tag);
    payload.extend_from_slice(&(chunk_payload.len() as u32).to_le_bytes());
    payload.extend_from_slice(&chunk_payload);
}

pub(super) fn shadow_map_bit(shadow_map: &[u8; 512], row: usize, col: usize) -> bool {
    let byte = shadow_map[row * 8 + col / 8];
    ((byte >> (7 - (col % 8))) & 1) != 0
}

pub(super) fn set_shadow_map_payload_bit(
    shadow_map: &mut [u8],
    row: usize,
    col: usize,
    value: bool,
) {
    let byte_index = row * 8 + col / 8;
    let mask = 1 << (7 - (col % 8));
    if value {
        shadow_map[byte_index] |= mask;
    } else {
        shadow_map[byte_index] &= !mask;
    }
}

fn mfbo_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    for value in [-10i16, -9, -8, -7, -6, -5, -4, -3, -2] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in [20i16, 21, 22, 23, 24, 25, 26, 27, 28] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload
}

fn mcnk_payload(
    include_mcsh: bool,
    include_mclv: bool,
    include_mcse: bool,
    include_mcbb: bool,
    include_mcdd: bool,
) -> Vec<u8> {
    let mut payload = vec![0; 128];
    payload[4..8].copy_from_slice(&3u32.to_le_bytes());
    payload[8..12].copy_from_slice(&7u32.to_le_bytes());
    payload[60..64].copy_from_slice(&TEST_AREA_ID.to_le_bytes());
    payload.extend_from_slice(&mcnk_subchunks_payload(
        include_mcsh,
        include_mclv,
        include_mcse,
        include_mcbb,
        include_mcdd,
    ));
    payload
}

fn append_base_subchunks(payload: &mut Vec<u8>) {
    append_subchunk(payload, b"TVCM", vec![0; MCVT_COUNT * 4]);
    append_subchunk(payload, b"RNCM", vec![0; MCVT_COUNT * 3]);
}

fn append_optional_mcsh_subchunks(payload: &mut Vec<u8>, include_mcsh: bool) {
    if !include_mcsh {
        return;
    }

    let mut shadow_map = vec![0; 512];
    set_shadow_map_payload_bit(&mut shadow_map, 0, 62, false);
    set_shadow_map_payload_bit(&mut shadow_map, 0, 63, true);
    set_shadow_map_payload_bit(&mut shadow_map, 62, 0, false);
    set_shadow_map_payload_bit(&mut shadow_map, 62, 1, true);
    set_shadow_map_payload_bit(&mut shadow_map, 63, 0, true);
    set_shadow_map_payload_bit(&mut shadow_map, 63, 1, false);
    set_shadow_map_payload_bit(&mut shadow_map, 63, 63, false);
    append_subchunk(payload, b"HSCM", shadow_map);
    append_subchunk(payload, b"VCCM", vec![0x7F; MCVT_COUNT * 4]);
}

fn append_optional_mclv_subchunk(payload: &mut Vec<u8>, include_mclv: bool) {
    if !include_mclv {
        return;
    }

    let mut vertex_lighting = vec![0; MCVT_COUNT * 4];
    vertex_lighting[0..4].copy_from_slice(&[0x00, 0x40, 0x80, 0xFF]);
    append_subchunk(payload, b"VLCM", vertex_lighting);
}

fn append_optional_mcse_subchunk(payload: &mut Vec<u8>, include_mcse: bool) {
    if !include_mcse {
        return;
    }

    let mut sound_emitters = Vec::new();
    sound_emitters.extend_from_slice(&42u32.to_le_bytes());
    sound_emitters.extend_from_slice(&100.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&200.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&300.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&10.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&20.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&30.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&7u32.to_le_bytes());
    sound_emitters.extend_from_slice(&1.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&2.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&3.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&4.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&5.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&6.0f32.to_le_bytes());
    append_subchunk(payload, b"MCSE", sound_emitters);
}

fn append_optional_mcbb_subchunk(payload: &mut Vec<u8>, include_mcbb: bool) {
    if !include_mcbb {
        return;
    }

    let mut blend_batches = Vec::new();
    append_blend_batch(&mut blend_batches, 1, 3, 5, 4, 6);
    append_blend_batch(&mut blend_batches, 7, 8, 9, 10, 11);
    append_subchunk(payload, b"BBCM", blend_batches);
}

fn append_optional_mcdd_subchunk(payload: &mut Vec<u8>, include_mcdd: bool) {
    if !include_mcdd {
        return;
    }

    let mut disable = [0u8; 64];
    disable[0] = 0b0000_0001;
    disable[1] = 0b1000_0000;
    disable[7] = 0b0101_0101;
    append_subchunk(payload, b"DDCM", disable.to_vec());
}

fn append_blend_batch(
    payload: &mut Vec<u8>,
    mbmh_index: u32,
    index_count: u32,
    index_first: u32,
    vertex_count: u32,
    vertex_first: u32,
) {
    payload.extend_from_slice(&mbmh_index.to_le_bytes());
    payload.extend_from_slice(&index_count.to_le_bytes());
    payload.extend_from_slice(&index_first.to_le_bytes());
    payload.extend_from_slice(&vertex_count.to_le_bytes());
    payload.extend_from_slice(&vertex_first.to_le_bytes());
}

fn blend_mesh_header_payload(
    map_object_id: u32,
    texture_id: u32,
    unknown: u32,
    index_count: u32,
    vertex_count: u32,
    index_start: u32,
    vertex_start: u32,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&map_object_id.to_le_bytes());
    payload.extend_from_slice(&texture_id.to_le_bytes());
    payload.extend_from_slice(&unknown.to_le_bytes());
    payload.extend_from_slice(&index_count.to_le_bytes());
    payload.extend_from_slice(&vertex_count.to_le_bytes());
    payload.extend_from_slice(&index_start.to_le_bytes());
    payload.extend_from_slice(&vertex_start.to_le_bytes());
    payload
}

fn blend_mesh_bounds_payload(map_object_id: u32) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&map_object_id.to_le_bytes());
    for value in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload
}

fn blend_mesh_vertex_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    append_blend_mesh_vertex(
        &mut payload,
        [10.0, 20.0, 30.0],
        [0.0, 1.0, 0.0],
        [0.25, 0.75],
        [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]],
    );
    append_blend_mesh_vertex(
        &mut payload,
        [11.0, 21.0, 31.0],
        [1.0, 0.0, 0.0],
        [0.5, 0.25],
        [[13, 14, 15, 16], [17, 18, 19, 20], [21, 22, 23, 24]],
    );
    append_blend_mesh_vertex(
        &mut payload,
        [12.0, 22.0, 32.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0],
        [[25, 26, 27, 28], [29, 30, 31, 32], [33, 34, 35, 36]],
    );
    payload
}

fn append_blend_mesh_vertex(
    payload: &mut Vec<u8>,
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [[u8; 4]; 3],
) {
    for value in position {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in normal {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in uv {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for rgba in color {
        payload.extend_from_slice(&rgba);
    }
}

fn blend_mesh_index_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    for index in [0u16, 1, 2] {
        payload.extend_from_slice(&index.to_le_bytes());
    }
    payload
}
