#[test]
fn trade_district_momt_reads_texture_2_before_diff_color() {
    let data = std::fs::read("data/models/322057.wmo").expect("sw_tradedistrict root WMO");
    let root = game_engine_core::asset::wmo_format::parser::load_wmo_root(&data)
        .expect("parse sw_tradedistrict root");

    let floor = &root.materials[8];
    assert_eq!(floor.shader, 13);
    assert_eq!(floor.texture_fdid, 127980); // mm_strmwnd_int_floor_02.blp
    assert_eq!(floor.texture_2_fdid, 465176); // mm_strmwnd_int_floor_02_burn.blp
    assert_eq!(
        floor.diff_color,
        [
            0x7F as f32 / 255.0,
            0x7F as f32 / 255.0,
            0x7F as f32 / 255.0,
            1.0
        ]
    );
    assert_eq!(floor.ground_type, 2);
    assert_eq!(floor.texture_3_fdid, 0);
    let crenellation = &root.materials[46];
    assert_eq!(crenellation.texture_fdid, 358469); // strmwnd_crenlatn.blp
    assert_eq!(crenellation.texture_2_fdid, 464811); // strmwnd_crenlatn_burned.blp
    assert_eq!(
        crenellation.diff_color,
        [
            0x95 as f32 / 255.0,
            0x95 as f32 / 255.0,
            0x95 as f32 / 255.0,
            1.0
        ]
    );
    assert_eq!(crenellation.ground_type, 2);
    assert_eq!(crenellation.texture_3_fdid, 0);
    assert_eq!(root.materials[4].texture_2_fdid, 0);
}
