use super::unified_gpu::{WallShading, render_batch_center};
use super::*;

/// Northshire Abbey (107074) takes the ordinary MapObj path (MOHD 0x5). Group 0 is an
/// interior group with MOCV: 13 interior batches, then 1 exterior batch.
const NORTHSHIRE_ABBEY_ROOT_FDID: u32 = 107074;
const ABBEY_INTERIOR_BATCH: usize = 0;
const ABBEY_EXTERIOR_BATCH: usize = 13;

/// Stock lights an ordinary interior batch with its MOCV alone, doubled
/// (`texture * 2 * fixed MOCV`); the MOHD ambient does not multiply in.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn abbey_interior_batch_is_lit_by_doubled_mocv() {
    let (root, group) = load_abbey_group_0();
    assert_eq!(
        group.batches[ABBEY_INTERIOR_BATCH].batch_type,
        wmo::WmoBatchType::Interior
    );
    assert_matches_stock(&root, &group, ABBEY_INTERIOR_BATCH, false);
}

/// Stock lights an ordinary exterior batch with daylight scaled by its doubled MOCV
/// (`texture * 2 * fixed MOCV * daylight`).
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn abbey_exterior_batch_is_daylight_scaled_by_doubled_mocv() {
    let (root, group) = load_abbey_group_0();
    assert_eq!(
        group.batches[ABBEY_EXTERIOR_BATCH].batch_type,
        wmo::WmoBatchType::Exterior
    );
    assert_matches_stock(&root, &group, ABBEY_EXTERIOR_BATCH, true);
}

fn assert_matches_stock(
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    batch_index: usize,
    lit: bool,
) {
    let wall = render_batch_center(root, group, batch_index, WallShading::Wmo);
    let stock = render_batch_center(root, group, batch_index, WallShading::StockMocv { lit });
    println!("batch {batch_index}: wmo {wall:?}, stock {stock:?}");
    let close = wall
        .iter()
        .zip(stock)
        .all(|(wall, stock)| wall.abs_diff(stock) <= 4);
    assert!(
        close,
        "batch {batch_index}: wmo {wall:?} != stock {stock:?}"
    );
}

fn load_abbey_group_0() -> (wmo::WmoRootData, wmo::WmoGroupData) {
    let root_data = std::fs::read(format!("data/models/{NORTHSHIRE_ABBEY_ROOT_FDID}.wmo"))
        .expect("Northshire Abbey root WMO in data/models");
    let root = wmo::load_wmo_root(&root_data).expect("parse abbey root");
    assert!(!root.flags.use_unified_render_path);
    let group_fdid = root.group_file_data_ids[0];
    let group_data = std::fs::read(format!("data/models/{group_fdid}.wmo"))
        .expect("Northshire Abbey group 0 WMO in data/models");
    let group = wmo::load_wmo_group_with_root(&group_data, Some(&root)).expect("parse group 0");
    assert!(group.header.group_flags.interior);
    (root, group)
}
