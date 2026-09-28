//! Retail WMO data that WebWowViewerCpp feeds its MOMT shaders: nine material
//! textures (`wmoObject.cpp` `getMaterialInstance`), four MOTV sets and MOC2
//! (`wmoGroupGeom.cpp` MOTV/MOC2 readers).
use game_engine_core::wmo::{parse_group, parse_root};

fn read(fdid: u32) -> Vec<u8> {
    std::fs::read(format!("data/models/{fdid}.wmo")).unwrap()
}

/// Character-select campsite WMO 4907674: MOMT 0 is shader 23 (MapObjDFShader).
#[test]
fn campsite_df_shader_material_exposes_nine_retail_textures() {
    let root = parse_root(&read(4_907_674)).unwrap();
    let material = &root.materials[0];
    assert_eq!(material.shader, 23);
    assert_eq!(
        material.extra_texture_fdids,
        [
            4_897_302, 4_881_307, 4_287_164, 4_287_164, 4_287_164, 4_287_164
        ]
    );
    assert_eq!(
        material.retail_texture_fdids(20),
        [
            0, 4_897_302, 4_881_307, 4_897_302, 4_881_307, 4_287_164, 4_287_164, 4_287_164,
            4_287_164
        ]
    );
    // Only MapObjParallax (19) and MapObjDFShader (20) bind textures 4..9.
    assert_eq!(
        material.retail_texture_fdids(0),
        [0, 4_897_302, 4_881_307, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        material.retail_texture_fdids(19),
        [
            0, 4_897_302, 4_881_307, 4_897_302, 4_881_307, 4_287_164, 0, 0, 0
        ]
    );
}

/// Its only group 4908148 carries four distinct MOTV sets and one MOC2.
#[test]
fn campsite_df_shader_group_keeps_four_uv_sets_and_moc2() {
    let group = parse_group(&read(4_908_148)).unwrap();
    let raw = &group.geometry;
    let vertices = raw.vertices.len();
    assert_eq!(vertices, 3822);
    for set in [&raw.uvs, &raw.second_uvs, &raw.third_uvs, &raw.fourth_uvs] {
        assert_eq!(set.len(), vertices);
    }
    assert_eq!(raw.uvs[0], [5.397_291, -2.657_399_2]);
    assert_eq!(raw.second_uvs[0], [3.277_757_6, -1.536_762_2]);
    assert_eq!(raw.third_uvs[0], [0.867_070_6, -1.024_171_6]);
    assert_eq!(raw.fourth_uvs[0], [5.097_880_4, 17.103_159]);
    assert_eq!(raw.moc2_colors.len(), vertices);
    // MOC2 is BGRA like MOCV: bytes [0,255,0,0] are green with alpha 0.
    assert_eq!(raw.moc2_colors[0], [0.0, 1.0, 0.0, 0.0]);
    assert!(raw.colors.is_empty());
    assert!(raw.second_color_blend_alphas.is_empty());
}
