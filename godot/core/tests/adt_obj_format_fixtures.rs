use game_engine_core::asset::adt_format::adt_obj::*;

#[test]
fn parse_elwynn_obj0() {
    let data = std::fs::read("data/terrain/azeroth_32_48_obj0.adt").expect("missing test asset");
    let obj = load_adt_obj0(&data).expect("parse failed");
    assert!(!obj.doodads.is_empty(), "expected doodads");
    let d = &obj.doodads[0];
    assert!(d.scale > 0.0, "scale should be positive");
    assert!(
        d.position[0] != 0.0 || d.position[1] != 0.0,
        "position shouldn't be zero"
    );
}

#[test]
fn parse_elwynn_obj0_reads_chunk_object_refs() {
    let data = std::fs::read("data/terrain/azeroth_32_48_obj0.adt").expect("missing test asset");
    let obj = load_adt_obj0(&data).expect("parse failed");

    assert_eq!(
        obj.chunk_refs.len(),
        256,
        "expected one KNCM ref set per ADT chunk"
    );
    assert!(
        obj.chunk_refs
            .iter()
            .any(|chunk| !chunk.doodad_refs.is_empty() || !chunk.wmo_refs.is_empty()),
        "expected at least one chunk to reference doodads or WMOs"
    );
}
