use game_engine_core::adt::{self, Chunk, chunk_geometry, parse_root};

fn flat_chunk() -> Chunk {
    Chunk {
        index_x: 0,
        index_y: 0,
        position: [12.0, -20.0, 7.0],
        area_id: 0,
        do_not_fix_alpha_map: false,
        heights: [0.0; 145],
        normals: [[0.0, 1.0, 0.0]; 145],
        vertex_colors: [[1.0; 4]; 145],
        vertex_lighting: None,
        sound_emitters: Vec::new(),
        blend_batches: Vec::new(),
        detail_doodad_disable: None,
        texture_selection: [0; 8],
        detail_exclusion: [0; 8],
        has_vertex_colors: false,
        holes_low_res: 0,
        holes_high_res: None,
        shadow_map: None,
    }
}

#[test]
fn flat_grid_has_original_uvs_winding_and_normals() {
    let mesh = chunk_geometry(&flat_chunk(), None);
    assert_eq!(mesh.positions.len(), 145);
    assert_eq!(mesh.positions[0], [-20.0, 7.0, -12.0]);
    assert_eq!(mesh.positions[1], [-20.0, 7.0, -12.0 + adt::UNIT_SIZE]);
    assert_eq!(
        mesh.positions[9],
        [
            -20.0 - adt::UNIT_SIZE / 2.0,
            7.0,
            -12.0 + adt::UNIT_SIZE / 2.0
        ]
    );
    assert_eq!(mesh.normals[9], [0.0, 1.0, 0.0]);
    assert_eq!(mesh.uvs[0], [0.0, 0.0]);
    assert_eq!(mesh.uvs[9], [0.0625, 0.0625]);
    assert_eq!(mesh.uvs[144], [1.0, 1.0]);
    assert_eq!(
        &mesh.indices[0..12],
        &[0, 9, 1, 1, 9, 18, 18, 9, 17, 17, 9, 0]
    );
    assert_eq!(mesh.indices.len(), 768);
}

#[test]
fn low_and_high_resolution_holes_preserve_original_priority() {
    let mut chunk = flat_chunk();
    chunk.holes_low_res = 1;
    let low = chunk_geometry(&chunk, None);
    assert_eq!(low.indices.len(), 768 - 4 * 12);
    assert!(!low.indices.contains(&9));
    chunk.holes_high_res = Some(1);
    let high = chunk_geometry(&chunk, None);
    assert_eq!(high.indices.len(), 768 - 12);
    assert!(!high.indices.contains(&9));
    assert!(high.indices.contains(&10));
}

#[test]
fn cached_azeroth_chunk_matches_authored_header_and_mcvt_samples() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/terrain/azeroth_32_48.adt"
    ))
    .expect("cached ADT");
    let root = parse_root(&bytes).expect("parse cached ADT");
    let mesh = chunk_geometry(&root.chunks[0], None);
    // First KNCM raw header pos=(-8533.333984375, 0, 233.5927734375),
    // parsed as [Y, X, Z]=[0, -8533.333984375, 233.5927734375].
    // TVCM heights[0]=0, heights[1]=-4.90399169921875, heights[9]=-6.7233123779296875.
    assert_eq!(mesh.positions[0], [-8533.333984375, 233.5927734375, 0.0]);
    assert_eq!(
        mesh.positions[1],
        [-8533.333984375, 228.68878, adt::UNIT_SIZE]
    );
    assert_eq!(
        mesh.positions[9],
        [
            -8533.333984375 - adt::UNIT_SIZE / 2.0,
            226.86946,
            adt::UNIT_SIZE / 2.0
        ]
    );
    assert_eq!(mesh.uvs[9], [0.0625, 0.0625]);
    assert_eq!(mesh.normals[9], root.chunks[0].normals[9]);
}
