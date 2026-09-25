use super::unified_gpu::{WallShading, render_batch_center};
use super::*;

/// Northshire Abbey (107074) group 0 is an interior group with MOCV: 13 interior
/// batches, then 1 exterior batch.
const NORTHSHIRE_ABBEY_ROOT_FDID: u32 = 107074;
const ABBEY_INTERIOR_BATCH: usize = 0;
const ABBEY_EXTERIOR_BATCH: usize = 13;

/// Retail lights interior-group vertices (fixed MOCV alpha 0) with the interior
/// ambient plus the doubled MOCV, and no sun: `texture * (ambient + 2 * MOCV)`
/// (WebWowViewerCpp `calcLight`, `precomputedLight = vColor.rgb * 2`).
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn abbey_interior_batch_is_lit_by_interior_ambient_plus_doubled_mocv() {
    assert_matches_retail_interior(ABBEY_INTERIOR_BATCH, wmo::WmoBatchType::Interior);
}

/// The exterior batch of an interior group is still interior-lit: only its MOCV
/// alpha, 0 after the fixup, selects the interior/exterior blend.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn abbey_exterior_batch_of_interior_group_is_interior_lit() {
    assert_matches_retail_interior(ABBEY_EXTERIOR_BATCH, wmo::WmoBatchType::Exterior);
}

fn assert_matches_retail_interior(batch_index: usize, batch_type: wmo::WmoBatchType) {
    let (root, group) = load_abbey_group_0();
    assert_eq!(group.batches[batch_index].batch_type, batch_type);
    let ambient = wmo_interior_ambient(&root, 0);
    let wall = render_batch_center(&root, &group, batch_index, WallShading::Wmo);
    let retail = render_batch_center(
        &root,
        &group,
        batch_index,
        WallShading::RetailInterior { ambient },
    );
    println!("batch {batch_index}: wmo {wall:?}, retail {retail:?}");
    let close = wall
        .iter()
        .zip(retail)
        .all(|(wall, retail)| wall.abs_diff(retail) <= 4);
    assert!(
        close,
        "batch {batch_index}: wmo {wall:?} != retail {retail:?}"
    );
}

fn load_abbey_group_0() -> (wmo::WmoRootData, wmo::WmoGroupData) {
    let root_data = std::fs::read(format!("data/models/{NORTHSHIRE_ABBEY_ROOT_FDID}.wmo"))
        .expect("Northshire Abbey root WMO in data/models");
    let root = wmo::load_wmo_root(&root_data).expect("parse abbey root");
    let group_fdid = root.group_file_data_ids[0];
    let group_data = std::fs::read(format!("data/models/{group_fdid}.wmo"))
        .expect("Northshire Abbey group 0 WMO in data/models");
    let group = wmo::load_wmo_group_with_root(&group_data, Some(&root)).expect("parse group 0");
    assert!(group.header.group_flags.interior);
    (root, group)
}
