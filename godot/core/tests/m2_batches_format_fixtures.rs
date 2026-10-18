use game_engine_core::asset::m2_format::parser::{parse_chunks, parse_texture_unit_lookup};

#[test]
fn real_skybox_has_empty_texture_unit_lookup() {
    let data = std::fs::read("data/models/skyboxes/11xp_cloudsky01.m2").unwrap();
    let chunks = parse_chunks(&data).unwrap();
    assert!(parse_texture_unit_lookup(chunks.md20).unwrap().is_empty());
}
