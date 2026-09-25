use super::*;

#[test]
fn m2_render_flags_and_blend_modes_map_to_retail_params() {
    let expected_gx = [0, 1, 2, 10, 3, 4, 5, 13];
    for (m2_blend, gx) in expected_gx.into_iter().enumerate() {
        assert_eq!(m2_blend_to_gx_blend(m2_blend as u16), gx);
    }
    let material = retail_m2_material(StandardMaterial::default(), 0x01 | 0x02 | 0x04, 4);
    assert_eq!(
        material.extension.params.flags,
        RETAIL_UNLIT | RETAIL_UNFOGGED
    );
    assert_eq!(material.extension.params.gx_blend, 3);
    assert!(!material.base.fog_enabled);
    assert_eq!(material.extension.scene_light, RETAIL_SCENE_LIGHT_BUFFER);
}
