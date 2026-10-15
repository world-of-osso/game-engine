use super::*;
use std::path::Path;

fn catalog() -> MountSpecialCatalog {
    MountSpecialCatalog::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")).unwrap()
}

#[test]
fn mount_special_grrloc_preserves_authored_delay_offset_and_upper_body() {
    let catalog = catalog();
    let mount = catalog.mount(1312).unwrap();
    assert_eq!((mount.rider_kit, mount.visual_kit), (19345, 124416));
    let kit = catalog.kit(mount.rider_kit).unwrap();
    let timeline = KitTimeline::resolve(kit, |_| Some(3000.0)).unwrap();
    assert!(timeline.sample(599.0).is_empty());
    let layers = timeline.sample(600.0);
    assert_eq!(layers.len(), 1);
    assert_eq!((layers[0].animation, layers[0].time_ms), (133, 2000.0));
    assert_eq!(layers[0].bones[0].root_key_bone, Some(4));
    assert_eq!(layers[0].bones[0].priority, 185);
    assert!(timeline.sample(1601.0).is_empty());
}

#[test]
fn mount_special_spelltome_runs_every_segment_with_overlap_and_reverse() {
    let catalog = catalog();
    let timeline = KitTimeline::resolve(catalog.kit(23323).unwrap(), |_| Some(1600.0)).unwrap();
    let first = timeline.sample(500.0);
    assert_eq!(first[0].animation, 876);
    assert!((first[0].time_ms - 800.0).abs() < 0.001);
    let channel = timeline.sample(1100.0);
    assert!(channel.iter().any(|layer| layer.animation == 878));
    let reverse = timeline.sample(1500.0);
    let reverse = reverse.iter().find(|layer| layer.segment_id == 46013).unwrap();
    assert!((reverse.time_ms - 1475.0).abs() < 0.01);
    let overlap = timeline.sample(3300.0);
    assert!(overlap.iter().any(|layer| layer.segment_id == 46013));
    assert!(overlap.iter().any(|layer| layer.segment_id == 46014));
    assert_eq!(timeline.sample(5000.0).iter().map(|layer| layer.animation).collect::<Vec<_>>(), vec![1088]);
}

#[test]
fn mount_special_simultaneous_masks_and_visual_loop_are_not_first_segment_only() {
    let catalog = catalog();
    let timeline = KitTimeline::resolve(catalog.kit(3012).unwrap(), |_| Some(1000.0)).unwrap();
    let layers = timeline.sample(2500.0);
    assert_eq!(layers.iter().map(|layer| layer.animation).collect::<Vec<_>>(), vec![500, 484]);
    assert_eq!(layers[0].bones[0].root_key_bone, None);
    assert_eq!(layers[1].bones[0].root_key_bone, Some(4));
    let timeline = KitTimeline::resolve(catalog.kit(19705).unwrap(), |_| Some(3000.0)).unwrap();
    let looped = timeline.sample(7600.0);
    assert_eq!((looped[0].animation, looped[0].time_ms), (134, 0.0));
}

#[test]
fn mount_special_missing_rider_clip_is_an_explicit_error_not_a_stand_pose() {
    let catalog = catalog();
    let error = KitTimeline::resolve(catalog.kit(23323).unwrap(), |id| (id != 878).then_some(1000.0)).unwrap_err();
    assert!(error.contains("878"), "{error}");
}
