use super::*;

const TEST_ROTATION_BITS: u32 = 3;
const TEST_SPEED_BITS: u32 = 5 << 3;
const TEST_SECOND_SPEED_BITS: u32 = 2 << 3;
const TEST_SECOND_ROTATION_BITS: u32 = 1;
const TEST_TEXTURE_FLAG_0: u32 = 0x10;
const TEST_TEXTURE_FLAG_1: u32 = 0x20;
const TEST_TEXTURE_PARAM_FLAG_0: u32 = 0x100;
const TEST_TEXTURE_PARAM_FLAG_1: u32 = 0x200;
const TEST_ANIMATED_REFLECTIVE_FLAGS: u32 = MCLY_FLAG_USE_CUBE_MAP_REFLECTION
    | MCLY_FLAG_ALPHA_COMPRESSED
    | MCLY_FLAG_USE_ALPHA_MAP
    | MCLY_FLAG_OVERBRIGHT
    | MCLY_FLAG_ANIMATION_ENABLED
    | TEST_SPEED_BITS
    | TEST_ROTATION_BITS;
const TEST_OVERBRIGHT_REFLECTIVE_FLAGS: u32 =
    MCLY_FLAG_USE_CUBE_MAP_REFLECTION | MCLY_FLAG_OVERBRIGHT | MCLY_FLAG_ANIMATION_ENABLED;

#[test]
fn mcly_flags_decode_animation_and_reflection_bits() {
    let flags = MclyFlags {
        raw: TEST_ANIMATED_REFLECTIVE_FLAGS,
    };

    assert_eq!(flags.animation_rotation(), 3);
    assert_eq!(flags.animation_speed(), 5);
    assert!(flags.animation_enabled());
    assert!(flags.overbright());
    assert!(flags.use_alpha_map());
    assert!(flags.alpha_compressed());
    assert!(flags.use_cube_map_reflection());
}

#[test]
fn build_texture_layers_exposes_mcly_flags_and_effect_id() {
    let mcly = mcly_entry_payload(
        7,
        TEST_OVERBRIGHT_REFLECTIVE_FLAGS | TEST_SECOND_SPEED_BITS | TEST_SECOND_ROTATION_BITS,
        0,
        99,
    );

    let layers = build_texture_layers(&mcly, &[], Some([12, 0, 0, 0]), false)
        .expect("expected MCLY layer to parse");
    let layer = &layers[0];

    assert_eq!(layer.texture_index, 7);
    assert_eq!(layer.effect_id, 99);
    assert_eq!(layer.material_id, 12);
    assert_eq!(layer.flags.animation_rotation(), 1);
    assert_eq!(layer.flags.animation_speed(), 2);
    assert!(layer.flags.animation_enabled());
    assert!(layer.flags.overbright());
    assert!(layer.flags.use_cube_map_reflection());
    assert_eq!(layer.alpha_map, None);
}

#[test]
fn load_adt_tex0_preserves_parsed_mcly_flags_per_chunk() {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"PMAM", 0u32.to_le_bytes().to_vec());
    append_subchunk(&mut payload, b"DIDM", 3u32.to_le_bytes().to_vec());
    append_subchunk(
        &mut payload,
        b"KNCM",
        tex0_mcnk_payload(
            mcly_entry_payload(0, MCLY_FLAG_USE_ALPHA_MAP, 0, 11),
            Some([4, 0, 0, 0]),
            vec![0x7F; 4096],
        ),
    );

    let parsed = load_adt_tex0(&payload).expect("expected _tex0 payload to parse");
    let layer = &parsed.chunk_layers[0].layers[0];

    assert_eq!(parsed.texture_amplifier, Some(0));
    assert_eq!(parsed.texture_fdids, vec![3]);
    assert!(parsed.height_texture_fdids.is_empty());
    assert!(layer.flags.use_alpha_map());
    assert_eq!(layer.effect_id, 11);
    assert_eq!(layer.material_id, 4);
    assert_eq!(layer.alpha_map.as_ref().map(Vec::len), Some(4096));
}

#[test]
fn load_adt_tex0_reads_mcmt_material_ids_per_layer() {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"PMAM", 0u32.to_le_bytes().to_vec());
    append_subchunk(&mut payload, b"DIDM", u32_array_payload(&[3, 4]));
    let mcly = [
        mcly_entry_payload(0, 0, 0, 0),
        mcly_entry_payload(1, MCLY_FLAG_USE_ALPHA_MAP, 0, 0),
    ]
    .concat();
    append_subchunk(
        &mut payload,
        b"KNCM",
        tex0_mcnk_payload(mcly, Some([7, 9, 0, 0]), vec![0x7F; 4096]),
    );

    let parsed = load_adt_tex0(&payload).expect("expected _tex0 payload to parse");
    let layers = &parsed.chunk_layers[0].layers;

    assert_eq!(layers[0].material_id, 7);
    assert_eq!(layers[1].material_id, 9);
}

#[test]
fn load_adt_tex0_reads_height_texture_fdids_from_mhid() {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"PMAM", 0u32.to_le_bytes().to_vec());
    append_subchunk(&mut payload, b"DIDM", u32_array_payload(&[3, 4]));
    append_subchunk(&mut payload, b"DIHM", u32_array_payload(&[30, 40]));
    append_subchunk(
        &mut payload,
        b"KNCM",
        tex0_mcnk_payload(mcly_entry_payload(0, 0, 0, 0), None, Vec::new()),
    );

    let parsed = load_adt_tex0(&payload).expect("expected _tex0 payload to parse");

    assert_eq!(parsed.texture_fdids, vec![3, 4]);
    assert_eq!(parsed.height_texture_fdids, vec![30, 40]);
}

#[test]
fn load_adt_tex0_reads_texture_flags_and_params() {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"PMAM", 0u32.to_le_bytes().to_vec());
    append_subchunk(&mut payload, b"DIDM", u32_array_payload(&[3, 4]));
    append_subchunk(
        &mut payload,
        b"FXTM",
        u32_array_payload(&[TEST_TEXTURE_FLAG_0, TEST_TEXTURE_FLAG_1]),
    );
    append_subchunk(
        &mut payload,
        b"PXTM",
        texture_params_payload(&[
            TextureParams {
                flags: TEST_TEXTURE_PARAM_FLAG_0,
                height_scale: 1.5,
                height_offset: -0.25,
            },
            TextureParams {
                flags: TEST_TEXTURE_PARAM_FLAG_1,
                height_scale: 0.75,
                height_offset: 0.125,
            },
        ]),
    );
    append_subchunk(
        &mut payload,
        b"KNCM",
        tex0_mcnk_payload(mcly_entry_payload(0, 0, 0, 0), None, Vec::new()),
    );

    let parsed = load_adt_tex0(&payload).expect("expected _tex0 payload to parse");

    assert_eq!(
        parsed.texture_flags,
        vec![TEST_TEXTURE_FLAG_0, TEST_TEXTURE_FLAG_1]
    );
    assert_eq!(
        parsed.texture_params,
        vec![
            TextureParams {
                flags: TEST_TEXTURE_PARAM_FLAG_0,
                height_scale: 1.5,
                height_offset: -0.25,
            },
            TextureParams {
                flags: TEST_TEXTURE_PARAM_FLAG_1,
                height_scale: 0.75,
                height_offset: 0.125,
            },
        ]
    );
}

#[test]
fn load_adt_tex0_fixes_uncompressed_alpha_map_edges_by_default() {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"PMAM", 0u32.to_le_bytes().to_vec());
    append_subchunk(&mut payload, b"DIDM", u32_array_payload(&[3]));
    let mcal = uncompressed_mcal_payload(|x, y| {
        if x == 63 || y == 63 {
            15
        } else if x == 62 || y == 62 {
            3
        } else {
            0
        }
    });
    append_subchunk(
        &mut payload,
        b"KNCM",
        tex0_mcnk_payload(
            mcly_entry_payload(0, MCLY_FLAG_USE_ALPHA_MAP, 0, 0),
            None,
            mcal,
        ),
    );

    let parsed = load_adt_tex0(&payload).expect("expected _tex0 payload to parse");
    let alpha = parsed.chunk_layers[0].layers[0]
        .alpha_map
        .as_ref()
        .expect("expected alpha map");

    assert_eq!(alpha_at(alpha, 62, 10), 51);
    assert_eq!(alpha_at(alpha, 63, 10), 51);
    assert_eq!(alpha_at(alpha, 10, 62), 51);
    assert_eq!(alpha_at(alpha, 10, 63), 51);
    assert_eq!(alpha_at(alpha, 63, 63), 51);
}

#[test]
fn load_adt_tex0_preserves_uncompressed_alpha_map_edges_when_flagged() {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"PMAM", 0u32.to_le_bytes().to_vec());
    append_subchunk(&mut payload, b"DIDM", u32_array_payload(&[3]));
    let mcal = uncompressed_mcal_payload(|x, y| {
        if x == 63 || y == 63 {
            15
        } else if x == 62 || y == 62 {
            3
        } else {
            0
        }
    });
    append_subchunk(
        &mut payload,
        b"KNCM",
        tex0_mcnk_payload(
            mcly_entry_payload(0, MCLY_FLAG_USE_ALPHA_MAP, 0, 0),
            None,
            mcal,
        ),
    );

    let parsed = load_adt_tex0_with_chunk_alpha_flags(&payload, &[true])
        .expect("expected _tex0 payload to parse");
    let alpha = parsed.chunk_layers[0].layers[0]
        .alpha_map
        .as_ref()
        .expect("expected alpha map");

    assert_eq!(alpha_at(alpha, 62, 10), 51);
    assert_eq!(alpha_at(alpha, 63, 10), 255);
    assert_eq!(alpha_at(alpha, 10, 62), 51);
    assert_eq!(alpha_at(alpha, 10, 63), 255);
    assert_eq!(alpha_at(alpha, 63, 63), 255);
}

#[test]
fn load_adt_tex0_reads_mamp_texture_amplifier() {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"PMAM", 2u32.to_le_bytes().to_vec());
    append_subchunk(&mut payload, b"DIDM", u32_array_payload(&[3]));
    append_subchunk(
        &mut payload,
        b"KNCM",
        tex0_mcnk_payload(mcly_entry_payload(0, 0, 0, 0), None, Vec::new()),
    );

    let parsed = load_adt_tex0(&payload).expect("expected _tex0 payload to parse");

    assert_eq!(parsed.texture_amplifier, Some(2));
}

#[test]
fn parse_mh2o_reads_chunk_fishable_and_deep_masks() {
    let fishable = (1u64 << 0) | (1u64 << 7) | (1u64 << 15);
    let deep = (1u64 << 3) | (1u64 << 63);
    let payload = mh2o_payload(0, 1, Some((fishable, deep)));

    let parsed = parse_mh2o(&payload).expect("expected MH2O payload to parse");
    let chunk = &parsed.chunks[0];
    let attributes = chunk.attributes.expect("expected MH2O attributes");

    assert_eq!(chunk.layers.len(), 1);
    assert_eq!(attributes.fishable, fishable);
    assert_eq!(attributes.deep, deep);
    assert!(attributes.is_fishable(0, 0));
    assert!(attributes.is_fishable(7, 0));
    assert!(attributes.is_fishable(7, 1));
    assert!(!attributes.is_fishable(0, 1));
    assert!(attributes.is_deep(3, 0));
    assert!(!attributes.is_deep(0, 0));
    assert!(attributes.is_deep(7, 7));
}

#[test]
fn parse_mh2o_reads_lvf1_height_and_uv_vertices() {
    let vertex_data = [
        mh2o_height_uv_vertex(1.5, 64, 128),
        mh2o_height_uv_vertex(2.5, 255, 0),
        mh2o_height_uv_vertex(3.5, 32, 96),
        mh2o_height_uv_vertex(4.5, 16, 240),
    ]
    .concat();
    let payload = mh2o_payload_with_vertex_data(0, 1, None, 1, &vertex_data);

    let parsed = parse_mh2o(&payload).expect("expected MH2O payload to parse");
    let layer = &parsed.chunks[0].layers[0];

    assert_eq!(layer.vertex_heights, vec![1.5, 2.5, 3.5, 4.5]);
    assert_eq!(layer.vertex_uvs.len(), 4);
    assert_eq!(layer.vertex_uvs[0], [64.0 / 255.0, 128.0 / 255.0]);
    assert_eq!(layer.vertex_uvs[1], [1.0, 0.0]);
    assert_eq!(layer.vertex_uvs[2], [32.0 / 255.0, 96.0 / 255.0]);
    assert_eq!(layer.vertex_uvs[3], [16.0 / 255.0, 240.0 / 255.0]);
}

#[test]
fn parse_mh2o_reads_lvf2_depth_only_vertices() {
    let vertex_data = [12u8, 34, 56, 78];
    let payload = mh2o_payload_with_vertex_data(0, 1, None, 2, &vertex_data);

    let parsed = parse_mh2o(&payload).expect("expected MH2O payload to parse");
    let layer = &parsed.chunks[0].layers[0];

    assert!(layer.vertex_heights.is_empty());
    assert!(layer.vertex_uvs.is_empty());
    assert_eq!(layer.vertex_depths, vertex_data);
}

#[test]
fn parse_mh2o_reads_lvf3_height_uv_and_depth_vertices() {
    let vertex_data = [
        mh2o_height_uv_depth_vertex(1.5, 64, 128, 12),
        mh2o_height_uv_depth_vertex(2.5, 255, 0, 34),
        mh2o_height_uv_depth_vertex(3.5, 32, 96, 56),
        mh2o_height_uv_depth_vertex(4.5, 16, 240, 78),
    ]
    .concat();
    let payload = mh2o_payload_with_vertex_data(0, 1, None, 3, &vertex_data);

    let parsed = parse_mh2o(&payload).expect("expected MH2O payload to parse");
    let layer = &parsed.chunks[0].layers[0];

    assert_eq!(layer.vertex_heights, vec![1.5, 2.5, 3.5, 4.5]);
    assert_eq!(layer.vertex_uvs.len(), 4);
    assert_eq!(layer.vertex_uvs[0], [64.0 / 255.0, 128.0 / 255.0]);
    assert_eq!(layer.vertex_uvs[1], [1.0, 0.0]);
    assert_eq!(layer.vertex_uvs[2], [32.0 / 255.0, 96.0 / 255.0]);
    assert_eq!(layer.vertex_uvs[3], [16.0 / 255.0, 240.0 / 255.0]);
    assert_eq!(layer.vertex_depths, vec![12, 34, 56, 78]);
}

fn mcly_entry_payload(
    texture_index: u32,
    flags: u32,
    offset_in_mcal: u32,
    effect_id: u32,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&texture_index.to_le_bytes());
    payload.extend_from_slice(&flags.to_le_bytes());
    payload.extend_from_slice(&offset_in_mcal.to_le_bytes());
    payload.extend_from_slice(&effect_id.to_le_bytes());
    payload
}

fn tex0_mcnk_payload(mcly: Vec<u8>, mcmt: Option<[u8; 4]>, mcal: Vec<u8>) -> Vec<u8> {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"YLCM", mcly);
    if let Some(material_ids) = mcmt {
        append_subchunk(&mut payload, b"TMCM", material_ids.to_vec());
    }
    append_subchunk(&mut payload, b"LACM", mcal);
    payload
}

fn append_subchunk(payload: &mut Vec<u8>, tag: &[u8; 4], chunk_payload: Vec<u8>) {
    payload.extend_from_slice(tag);
    payload.extend_from_slice(&(chunk_payload.len() as u32).to_le_bytes());
    payload.extend_from_slice(&chunk_payload);
}

fn u32_array_payload(values: &[u32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(std::mem::size_of_val(values));
    for value in values {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload
}

fn texture_params_payload(values: &[TextureParams]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(values.len() * size_of::<RawTextureParams>());
    for value in values {
        payload.extend_from_slice(&value.flags.to_le_bytes());
        payload.extend_from_slice(&value.height_scale.to_le_bytes());
        payload.extend_from_slice(&value.height_offset.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
    }
    payload
}

fn mh2o_payload(chunk_index: usize, layer_count: u32, attributes: Option<(u64, u64)>) -> Vec<u8> {
    const CHUNK_COUNT: usize = 256;
    const HEADER_SIZE: usize = CHUNK_COUNT * size_of::<Mh2oChunkHeader>();

    let instance_offset = HEADER_SIZE as u32;
    let attributes_offset =
        attributes.map(|_| instance_offset + size_of::<LiquidInstanceHeader>() as u32);
    let mut payload = vec![0u8; HEADER_SIZE];

    let header_base = chunk_index * size_of::<Mh2oChunkHeader>();
    payload[header_base..header_base + 4].copy_from_slice(&instance_offset.to_le_bytes());
    payload[header_base + 4..header_base + 8].copy_from_slice(&layer_count.to_le_bytes());
    payload[header_base + 8..header_base + 12]
        .copy_from_slice(&attributes_offset.unwrap_or(0).to_le_bytes());

    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.extend_from_slice(&1.0f32.to_le_bytes());
    payload.extend_from_slice(&2.0f32.to_le_bytes());
    payload.extend_from_slice(&0u8.to_le_bytes());
    payload.extend_from_slice(&0u8.to_le_bytes());
    payload.extend_from_slice(&8u8.to_le_bytes());
    payload.extend_from_slice(&8u8.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());

    if let Some((fishable, deep)) = attributes {
        payload.extend_from_slice(&fishable.to_le_bytes());
        payload.extend_from_slice(&deep.to_le_bytes());
    }

    payload
}

fn mh2o_payload_with_vertex_data(
    chunk_index: usize,
    layer_count: u32,
    attributes: Option<(u64, u64)>,
    liquid_object: u16,
    vertex_data: &[u8],
) -> Vec<u8> {
    const CHUNK_COUNT: usize = 256;
    const HEADER_SIZE: usize = CHUNK_COUNT * size_of::<Mh2oChunkHeader>();
    let instance_offset = HEADER_SIZE as u32;
    let attributes_size = attributes.map_or(0, |_| size_of::<Mh2oAttributes>() as u32);
    let vertex_offset =
        instance_offset + size_of::<LiquidInstanceHeader>() as u32 + attributes_size;
    let attributes_offset =
        attributes.map(|_| instance_offset + size_of::<LiquidInstanceHeader>() as u32);

    let mut payload = vec![0u8; HEADER_SIZE];
    let header_base = chunk_index * size_of::<Mh2oChunkHeader>();
    payload[header_base..header_base + 4].copy_from_slice(&instance_offset.to_le_bytes());
    payload[header_base + 4..header_base + 8].copy_from_slice(&layer_count.to_le_bytes());
    payload[header_base + 8..header_base + 12]
        .copy_from_slice(&attributes_offset.unwrap_or(0).to_le_bytes());

    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.extend_from_slice(&liquid_object.to_le_bytes());
    payload.extend_from_slice(&1.0f32.to_le_bytes());
    payload.extend_from_slice(&2.0f32.to_le_bytes());
    payload.extend_from_slice(&0u8.to_le_bytes());
    payload.extend_from_slice(&0u8.to_le_bytes());
    payload.extend_from_slice(&1u8.to_le_bytes());
    payload.extend_from_slice(&1u8.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&vertex_offset.to_le_bytes());

    if let Some((fishable, deep)) = attributes {
        payload.extend_from_slice(&fishable.to_le_bytes());
        payload.extend_from_slice(&deep.to_le_bytes());
    }
    payload.extend_from_slice(vertex_data);
    payload
}

fn mh2o_height_uv_vertex(height: f32, u: u16, v: u16) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8);
    payload.extend_from_slice(&height.to_le_bytes());
    payload.extend_from_slice(&u.to_le_bytes());
    payload.extend_from_slice(&v.to_le_bytes());
    payload
}

fn mh2o_height_uv_depth_vertex(height: f32, u: u16, v: u16, depth: u8) -> Vec<u8> {
    let mut payload = Vec::with_capacity(9);
    payload.extend_from_slice(&height.to_le_bytes());
    payload.extend_from_slice(&u.to_le_bytes());
    payload.extend_from_slice(&v.to_le_bytes());
    payload.push(depth);
    payload
}

fn uncompressed_mcal_payload(alpha: impl Fn(usize, usize) -> u8) -> Vec<u8> {
    let mut payload = Vec::with_capacity(2048);
    for x in 0..64usize {
        for y in (0..64usize).step_by(2) {
            let lower = alpha(x, y) & 0x0F;
            let upper = alpha(x, y + 1) & 0x0F;
            payload.push(lower | (upper << 4));
        }
    }
    payload
}

fn alpha_at(alpha: &[u8], x: usize, y: usize) -> u8 {
    alpha[x * 64 + y]
}
