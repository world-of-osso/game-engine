use game_engine_core::mount_preview::MountPreview;
use std::path::Path;

fn data() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}
#[test]
fn celestial_steed_uses_installed_mount_metadata_and_scene() {
    let mount = MountPreview::read(&data(), 376, 418).unwrap();
    assert!(mount.shows_rider(false));
    assert!(!mount.shows_rider(true));
    assert_eq!(mount.rider_animation_kit, 0);
    assert_eq!(mount.rider_spell_visual_kit, 0);
    assert_eq!(mount.actors["mount"].id, 1212);
    assert_eq!(mount.actors["player-rider"].id, 1214);
}
#[test]
fn self_and_disabled_mounts_do_not_acquire_player_rider() {
    let mut mount = MountPreview::read(&data(), 376, 418).unwrap();
    for flags in [2, 4096, 4098] {
        mount.flags = flags;
        assert!(!mount.shows_rider(false), "flags {flags}");
    }
    mount.flags = 4;
    assert!(mount.shows_rider(false));
}
