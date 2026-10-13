//! Authenticated Retail1417024 (Prismatic Bauble), not a layout-shaped mock.
//! M2Ribbon field semantics: WebWowViewerCpp M2FileHeader.h and CRibbonEmitter.
use game_engine_core::asset::m2_format::m2_ribbon::parse_ribbon_emitters;
use std::path::Path;

#[test]
fn toyfx26_rainbow_ribbon_has_source_fifty_edges_and_half_second_lifetime() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/1417024.m2");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let ribbons = parse_ribbon_emitters(&bytes[8..8 + size]).unwrap();
    assert_eq!(ribbons.len(), 1);
    let ribbon = &ribbons[0];
    assert_eq!(ribbon.bone_index, 1);
    assert_eq!(ribbon.position, [0.0, 0.0, 0.0]);
    assert_eq!(ribbon.edges_per_second, 50.0);
    assert_eq!(ribbon.edge_lifetime, 0.5);
    assert_eq!(ribbon.gravity, 0.0);
    assert_eq!((ribbon.texture_rows, ribbon.texture_cols), (1, 1));
    assert_eq!(ribbon.texture_indices, [0]);
    assert_eq!(ribbon.material_indices, [0]);
    assert_eq!(ribbon.color.sequences[0].1, [[1.0; 3]]);
    assert_eq!(ribbon.height_above.sequences[0].1, [0.5555556]);
    assert_eq!(ribbon.height_below.sequences[0].1, [0.5555556]);
    assert_eq!(ribbon.texture_slot.sequences[0].1, [0]);
    assert!(ribbon.visibility.sequences.is_empty());
}
