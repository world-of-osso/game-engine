use crate::target_selection_data::{PickHit, first_picked_unit, next_target, opaque_to_alpha_mask};

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
fn nearest_visible_unit_is_picked() {
    let hits = [
        PickHit::Unit {
            id: 11_u64,
            visible: true,
        },
        PickHit::World { visible: true },
    ];
    assert_eq!(first_picked_unit(hits), Some(11));
}

#[test]
fn visible_world_geometry_in_front_occludes_the_unit_behind_it() {
    let hits = [
        PickHit::World { visible: true },
        PickHit::Unit {
            id: 11_u64,
            visible: true,
        },
    ];
    assert_eq!(first_picked_unit(hits), None);
}

#[test]
fn hidden_units_and_hidden_geometry_do_not_stop_the_ray() {
    let hits = [
        PickHit::Unit {
            id: 5_u64,
            visible: false,
        },
        PickHit::World { visible: false },
        PickHit::Unit {
            id: 6,
            visible: true,
        },
    ];
    assert_eq!(first_picked_unit(hits), Some(6));
}

#[test]
fn a_ray_that_hits_nothing_or_only_ground_picks_nothing() {
    assert_eq!(first_picked_unit(Vec::<PickHit<u64>>::new()), None);
    assert_eq!(
        first_picked_unit([PickHit::<u64>::World { visible: true }]),
        None
    );
}

#[test]
fn picking_stops_consuming_hits_at_the_first_visible_one() {
    let mut consumed = 0;
    let hits = [
        PickHit::World { visible: false },
        PickHit::World { visible: true },
        PickHit::Unit {
            id: 1_u64,
            visible: true,
        },
    ]
    .into_iter()
    .inspect(|_| consumed += 1);
    assert_eq!(first_picked_unit(hits), None);
    assert_eq!(consumed, 2);
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
