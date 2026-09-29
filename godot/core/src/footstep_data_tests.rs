use crate::footstep_data::*;

fn sample_entry(path: &str) -> FootstepCatalogEntry {
    FootstepCatalogEntry::from_path(1, path).expect("sample footstep entry should parse")
}

#[test]
fn footstep_surface_detects_texture_path_keywords() {
    assert_eq!(
        classify_surface_from_texture_path("tileset/elwynn/elwynngrassbase.blp"),
        FootstepSurface::Grass
    );
    assert_eq!(
        classify_surface_from_texture_path("tileset/ironforge/ironforge_metaltrim.blp"),
        FootstepSurface::Metal
    );
    assert_eq!(
        classify_surface_from_texture_path("tileset/winter/snowpacked.blp"),
        FootstepSurface::Snow
    );
    assert_eq!(
        classify_surface_from_texture_path("tileset/deadmines/planks_woodfloor.blp"),
        FootstepSurface::Wood
    );
}

#[test]
fn footstep_creature_class_uses_race_and_model_keywords() {
    assert_eq!(
        classify_player_creature(1),
        FootstepCreature::HumanoidMedium
    );
    assert_eq!(classify_player_creature(7), FootstepCreature::HumanoidSmall);
    assert_eq!(classify_player_creature(6), FootstepCreature::Hoof);
    assert_eq!(
        classify_model_creature("character/vulpera/male/vulperamale.m2"),
        FootstepCreature::Paw
    );
    assert_eq!(
        classify_model_creature("creature/horse/horse.m2"),
        FootstepCreature::Horse
    );
}

#[test]
fn footstep_movement_classifies_anim_ids() {
    assert_eq!(movement_from_anim(4), Some(FootstepMovement::Walk));
    assert_eq!(movement_from_anim(5), Some(FootstepMovement::Run));
    assert_eq!(movement_from_anim(11), Some(FootstepMovement::Strafe));
    assert_eq!(movement_from_anim(13), Some(FootstepMovement::Backpedal));
    assert_eq!(movement_from_anim(0), None);
}

#[test]
fn footstep_catalog_prefers_exact_surface_match_before_fallback() {
    let catalog = FootstepCatalog {
        entries: vec![
            sample_entry("sound/character/footsteps/mfootsmallgrassa.ogg"),
            sample_entry("sound/character/footsteps/mfootsmalldirta.ogg"),
            sample_entry("sound/character/footsteps/mfootsmallstonea.ogg"),
        ],
    };

    let grass = catalog
        .select(FootstepRequest {
            creature: FootstepCreature::HumanoidSmall,
            surface: FootstepSurface::Grass,
            movement: FootstepMovement::Walk,
            seed: 7,
        })
        .expect("grass match");
    let mud = catalog
        .select(FootstepRequest {
            creature: FootstepCreature::HumanoidSmall,
            surface: FootstepSurface::Mud,
            movement: FootstepMovement::Walk,
            seed: 7,
        })
        .expect("fallback match");

    assert!(grass.path.contains("grass"));
    assert!(mud.path.contains("dirt"));
}

#[test]
fn footstep_catalog_picks_heavier_variant_for_run() {
    let catalog = FootstepCatalog {
        entries: vec![
            sample_entry("sound/character/footsteps/mfootsmallgrassa.ogg"),
            sample_entry("sound/character/footsteps/mfootmediumlargegrassa.ogg"),
        ],
    };

    let walk = catalog
        .select(FootstepRequest {
            creature: FootstepCreature::HumanoidSmall,
            surface: FootstepSurface::Grass,
            movement: FootstepMovement::Walk,
            seed: 1,
        })
        .expect("walk match");
    let run = catalog
        .select(FootstepRequest {
            creature: FootstepCreature::HumanoidSmall,
            surface: FootstepSurface::Grass,
            movement: FootstepMovement::Run,
            seed: 1,
        })
        .expect("run match");

    assert!(walk.path.contains("mfootsmall"));
    assert!(run.path.contains("mfootmediumlarge"));
}

#[test]
fn catalog_seed_selects_stable_tied_entry_and_index() {
    let catalog = FootstepCatalog {
        entries: vec![
            FootstepCatalogEntry::from_path(101, "sound/character/footsteps/mfootsmallgrassa.ogg")
                .unwrap(),
            FootstepCatalogEntry::from_path(102, "sound/character/footsteps/mfootsmallgrassb.ogg")
                .unwrap(),
            FootstepCatalogEntry::from_path(103, "sound/character/footsteps/mfootsmallgrassc.ogg")
                .unwrap(),
        ],
    };
    let request = FootstepRequest {
        creature: FootstepCreature::HumanoidSmall,
        surface: FootstepSurface::Grass,
        movement: FootstepMovement::Walk,
        seed: 4,
    };
    assert_eq!(catalog.select(request).map(|entry| entry.fdid), Some(102));
    assert_eq!(catalog.select_index(request), Some(1));
}

#[test]
fn catalog_with_no_eligible_creature_returns_none() {
    let catalog = FootstepCatalog {
        entries: vec![
            FootstepCatalogEntry::from_path(201, "sound/creature/horse/horse_footstep_stone.ogg")
                .unwrap(),
        ],
    };
    let request = FootstepRequest {
        creature: FootstepCreature::HumanoidSmall,
        surface: FootstepSurface::Stone,
        movement: FootstepMovement::Run,
        seed: 0,
    };
    assert_eq!(catalog.select(request), None);
    assert_eq!(catalog.select_index(request), None);
}

#[test]
fn half_cycle_emits_once_per_observed_half_and_wrap() {
    let mut tracker = FootstepPhaseTracker::default();
    assert_eq!(tracker.observe(3, 5, 600.0, 0.0), None);
    assert_eq!(
        tracker.observe(3, 5, 600.0, 300.0),
        Some((FootstepMovement::Run, 769))
    );
    assert_eq!(tracker.observe(3, 5, 600.0, 450.0), None);
    assert_eq!(
        tracker.observe(3, 5, 600.0, 600.0),
        Some((FootstepMovement::Run, 768))
    );
    assert_eq!(
        tracker.observe(3, 5, 600.0, 900.0),
        Some((FootstepMovement::Run, 769))
    );
}

#[test]
fn half_cycle_does_not_invent_missed_steps() {
    let mut tracker = FootstepPhaseTracker::default();
    assert_eq!(tracker.observe(2, 4, 400.0, 1200.0), None);
    assert_eq!(tracker.observe(2, 4, 400.0, 2050.0), None);
    assert_eq!(
        tracker.observe(2, 4, 400.0, 2300.0),
        Some((FootstepMovement::Walk, 513))
    );
}

#[test]
fn nonmovement_updates_sequence_index_without_resetting_half() {
    let mut tracker = FootstepPhaseTracker::default();
    assert_eq!(
        tracker.observe(3, 5, 600.0, 300.0),
        Some((FootstepMovement::Run, 769))
    );
    assert_eq!(tracker.observe(7, 0, 600.0, 20.0), None);
    assert_eq!(tracker.observe(7, 5, 600.0, 300.0), None);
    assert_eq!(
        tracker.observe(7, 5, 600.0, 0.0),
        Some((FootstepMovement::Run, 1792))
    );
}

#[test]
fn new_movement_sequence_resets_half_before_sampling() {
    let mut tracker = FootstepPhaseTracker::default();
    assert_eq!(
        tracker.observe(2, 5, 600.0, 300.0),
        Some((FootstepMovement::Run, 513))
    );
    assert_eq!(tracker.observe(4, 11, 600.0, 0.0), None);
    assert_eq!(
        tracker.observe(4, 11, 600.0, 300.0),
        Some((FootstepMovement::Strafe, 1025))
    );
    assert_eq!(
        tracker.observe(5, 13, 600.0, 300.0),
        Some((FootstepMovement::Backpedal, 1281))
    );
}

#[test]
fn zero_duration_skips_but_records_movement_sequence_change() {
    let mut tracker = FootstepPhaseTracker::default();
    assert_eq!(
        tracker.observe(2, 5, 600.0, 300.0),
        Some((FootstepMovement::Run, 513))
    );
    assert_eq!(tracker.observe(4, 4, 0.0, 300.0), None);
    assert_eq!(
        tracker.observe(4, 4, 600.0, 300.0),
        Some((FootstepMovement::Walk, 1025))
    );
}
