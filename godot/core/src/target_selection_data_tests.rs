use crate::target_selection_data::{next_target, opaque_to_alpha_mask, previous_target};

#[test]
fn tab_cycles_nearest_first_and_wraps() {
    let sorted = [7_u64, 3, 9];
    assert_eq!(next_target(&sorted, None), Some(7));
    assert_eq!(next_target(&sorted, Some(7)), Some(3));
    assert_eq!(next_target(&sorted, Some(3)), Some(9));
    assert_eq!(next_target(&sorted, Some(9)), Some(7));
    // A current target outside the candidate list restarts at the nearest.
    assert_eq!(next_target(&sorted, Some(42)), Some(7));
    assert_eq!(next_target::<u64>(&[], Some(7)), None);
}

#[test]
fn opaque_ring_texture_becomes_an_intensity_alpha_mask() {
    let mut pixels = vec![200, 40, 10, 255, 0, 0, 0, 255];
    opaque_to_alpha_mask(&mut pixels);
    assert_eq!(pixels, vec![200, 200, 200, 200, 0, 0, 0, 0]);
    // Textures with authored alpha are left unchanged.
    let mut authored = vec![200, 40, 10, 128];
    opaque_to_alpha_mask(&mut authored);
    assert_eq!(authored, vec![200, 40, 10, 128]);
}

#[test]
fn shift_tab_walks_the_tab_cycle_backwards_from_the_farthest() {
    let sorted = [7u64, 3, 9];
    assert_eq!(previous_target(&sorted, None), Some(9));
    assert_eq!(previous_target(&sorted, Some(9)), Some(3));
    assert_eq!(previous_target(&sorted, Some(3)), Some(7));
    assert_eq!(previous_target(&sorted, Some(7)), Some(9));
    assert_eq!(previous_target(&sorted, Some(42)), Some(9));
    assert_eq!(previous_target::<u64>(&[], Some(7)), None);
}
