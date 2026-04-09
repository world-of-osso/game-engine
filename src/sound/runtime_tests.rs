use super::*;

#[test]
fn footstep_samples_length() {
    let samples = generate_footstep_samples(0.5, 100);
    assert_eq!(samples.len(), 4410);
}

#[test]
fn footstep_samples_decay() {
    let samples = generate_footstep_samples(0.5, 100);
    let first_avg: f32 = samples[..100]
        .iter()
        .map(|s| s.unsigned_abs() as f32)
        .sum::<f32>()
        / 100.0;
    let last_avg: f32 = samples[4300..]
        .iter()
        .map(|s| s.unsigned_abs() as f32)
        .sum::<f32>()
        / 110.0;
    assert!(
        first_avg > last_avg,
        "first_avg={first_avg} should be > last_avg={last_avg}"
    );
}

#[test]
fn ambient_samples_length() {
    let samples = generate_ambient_samples(2000);
    assert_eq!(samples.len(), 88200);
}

#[test]
fn music_samples_length() {
    let samples = generate_music_samples(4000);
    assert_eq!(samples.len(), 176_400);
}

#[test]
fn wav_header_valid() {
    let samples = vec![0i16; 100];
    let wav = generate_wav(&samples);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[12..16], b"fmt ");
    assert_eq!(&wav[36..40], b"data");
    let file_size = u32::from_le_bytes(wav[4..8].try_into().unwrap());
    assert_eq!(file_size, 236);
}

#[test]
fn sound_settings_default() {
    let s = SoundSettings::default();
    assert!(!s.muted);
    assert_eq!(s.master_volume, 1.0);
    assert_eq!(s.effects_volume, 0.8);
}

#[test]
fn mute_toggle() {
    let mut s = SoundSettings::default();
    assert!(!s.muted);
    s.muted = !s.muted;
    assert!(s.muted);
    assert_eq!(compute_ambient_volume(&s), 0.0);
}

#[test]
fn ambient_volume_unmuted() {
    let s = SoundSettings::default();
    let vol = compute_ambient_volume(&s);
    assert!(vol > 0.0);
    assert!((vol - 0.3).abs() < f32::EPSILON);
}

#[test]
fn music_volume_unmuted() {
    let s = SoundSettings::default();
    let vol = compute_music_volume(&s);
    assert!(vol > 0.0);
    assert!((vol - 0.45).abs() < f32::EPSILON);
}

#[test]
fn music_volume_muted() {
    let s = SoundSettings {
        muted: true,
        ..Default::default()
    };
    assert_eq!(compute_music_volume(&s), 0.0);
}

#[test]
fn effects_volume_uses_master_and_effects_slider() {
    let s = SoundSettings {
        master_volume: 0.5,
        effects_volume: 0.4,
        ..Default::default()
    };
    assert!((compute_effects_volume(&s) - 0.2).abs() < f32::EPSILON);
}

#[test]
fn effects_volume_muted() {
    let s = SoundSettings {
        muted: true,
        ..Default::default()
    };
    assert_eq!(compute_effects_volume(&s), 0.0);
}

#[test]
fn movement_anim_detection() {
    assert!(is_movement_anim(4));
    assert!(is_movement_anim(5));
    assert!(is_movement_anim(11));
    assert!(is_movement_anim(12));
    assert!(is_movement_anim(13));
    assert!(!is_movement_anim(0));
    assert!(!is_movement_anim(37));
}

#[test]
fn footstep_tracker_defaults() {
    let tracker = FootstepTracker::default();
    assert_eq!(tracker.last_half, 0);
    assert_eq!(tracker.last_seq_idx, 0);
}

#[test]
fn select_footstep_surface_prefers_smallest_containing_wmo() {
    let position = Vec3::new(1.0, 1.0, 1.0);
    let terrain_surface = Some(FootstepSurface::Grass);
    let wmo_surfaces = vec![
        (
            game_engine::culling::WmoRootBounds {
                world_min: Vec3::ZERO,
                world_max: Vec3::splat(10.0),
            },
            FootstepSurface::Stone,
        ),
        (
            game_engine::culling::WmoRootBounds {
                world_min: Vec3::splat(0.5),
                world_max: Vec3::splat(2.0),
            },
            FootstepSurface::Wood,
        ),
    ];

    let surface = select_footstep_surface(position, terrain_surface, wmo_surfaces.into_iter());

    assert_eq!(surface, FootstepSurface::Wood);
}

#[test]
fn select_footstep_surface_falls_back_to_terrain_when_outside_wmo() {
    let surface = select_footstep_surface(
        Vec3::splat(20.0),
        Some(FootstepSurface::Grass),
        [(
            game_engine::culling::WmoRootBounds {
                world_min: Vec3::ZERO,
                world_max: Vec3::splat(10.0),
            },
            FootstepSurface::Stone,
        )]
        .into_iter(),
    );

    assert_eq!(surface, FootstepSurface::Grass);
}

#[test]
fn observe_active_spell_queues_cast_sound_once_per_new_spell() {
    let mut queue = SpellSoundQueue::default();
    let mut last_spell_id = None;
    let mut casting = game_engine::casting_data::CastingState::default();

    casting.start(game_engine::casting_data::ActiveCast {
        spell_name: "Fireball".into(),
        spell_id: 133,
        icon_fdid: 135810,
        cast_type: game_engine::casting_data::CastType::Cast,
        interruptible: true,
        duration: 2.5,
        elapsed: 0.0,
    });

    observe_active_spell(&casting, &mut last_spell_id, &mut queue);
    observe_active_spell(&casting, &mut last_spell_id, &mut queue);

    assert_eq!(
        queue.requests,
        vec![SpellSoundRequest {
            spell_id: 133,
            kind: SpellSoundKind::CastStart,
        }]
    );
}

#[test]
fn observe_active_spell_ignores_zero_spell_id_and_clears_tracker() {
    let mut queue = SpellSoundQueue::default();
    let mut last_spell_id = Some(133);
    let mut casting = game_engine::casting_data::CastingState::default();

    casting.start(game_engine::casting_data::ActiveCast {
        spell_name: "Mining Copper Vein".into(),
        spell_id: 0,
        icon_fdid: 0,
        cast_type: game_engine::casting_data::CastType::Cast,
        interruptible: true,
        duration: 2.0,
        elapsed: 0.0,
    });

    observe_active_spell(&casting, &mut last_spell_id, &mut queue);
    assert!(queue.requests.is_empty());
    assert_eq!(last_spell_id, None);

    casting.cancel();
    observe_active_spell(&casting, &mut last_spell_id, &mut queue);
    assert_eq!(last_spell_id, None);
}
