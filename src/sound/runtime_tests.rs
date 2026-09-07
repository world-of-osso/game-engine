use super::runtime_assets::{
    generate_ambient_samples, generate_footstep_samples, generate_music_samples,
    strip_ambient_tracks_from_music_catalog,
};
use super::*;

fn maintenance_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(SoundPlugin)
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<InputBindings>()
        .insert_resource(SoundAssets {
            footstep_light: Handle::default(),
            footstep_heavy: Handle::default(),
            footstep_catalog: Default::default(),
            spell_cast: Handle::default(),
            spell_impact: Handle::default(),
            spell_heal: Handle::default(),
            spell_miss: Handle::default(),
            spell_interrupt: Handle::default(),
            ui_button_click: Handle::default(),
            ui_bag_open: Handle::default(),
            ui_bag_close: Handle::default(),
            ambient_loop: Handle::default(),
            music_loop_fallback: Handle::default(),
            music_tracks: vec![
                LoadedMusicTrack {
                    handle: Handle::default(),
                    name: "first".into(),
                },
                LoadedMusicTrack {
                    handle: Handle::default(),
                    name: "second".into(),
                },
            ],
            ambient_tracks_by_zone: HashMap::from([(5, vec![0, 1])]),
            music_tracks_by_zone: HashMap::from([(5, vec![0, 1])]),
        });
    app
}

fn sound_update(app: &mut App) {
    // Exercise production Update registration without opening an audio device or loading files.
    app.world_mut().run_schedule(Update);
    app.world_mut().clear_trackers();
}

fn sound_entity<T: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<T>>()
        .single(app.world())
        .unwrap()
}

fn test_anim_player() -> crate::animation::M2AnimPlayer {
    crate::animation::M2AnimPlayer {
        current_seq_idx: 0,
        time_ms: 0.0,
        looping: true,
        transition: None,
    }
}

#[test]
fn maintenance_attaches_tracker_at_either_relevance_boundary_without_an_update() {
    let mut app = maintenance_test_app();
    let player_first = app.world_mut().spawn(crate::camera::Player).id();
    let model_first = app.world_mut().spawn(test_anim_player()).id();
    assert!(app.world().get::<FootstepTracker>(player_first).is_none());
    assert!(app.world().get::<FootstepTracker>(model_first).is_none());
    app.world_mut()
        .entity_mut(player_first)
        .insert(test_anim_player());
    app.world_mut()
        .entity_mut(model_first)
        .insert(crate::camera::Player);
    app.world_mut().flush();
    assert!(app.world().get::<FootstepTracker>(player_first).is_some());
    assert!(app.world().get::<FootstepTracker>(model_first).is_some());
    app.world_mut()
        .get_mut::<FootstepTracker>(player_first)
        .unwrap()
        .last_half = 1;
    app.world_mut()
        .entity_mut(player_first)
        .insert(test_anim_player());
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<FootstepTracker>(player_first)
            .unwrap()
            .last_half,
        1
    );
}

#[test]
fn maintenance_idle_does_not_acquire_sound_assets() {
    let mut app = maintenance_test_app();
    sound_update(&mut app);
    sound_update(&mut app);
    let ambient = sound_entity::<AmbientSound>(&mut app);
    let music = sound_entity::<MusicSound>(&mut app);
    app.world_mut().remove_resource::<SoundAssets>();
    for _ in 0..3 {
        sound_update(&mut app);
    }
    assert_eq!(sound_entity::<AmbientSound>(&mut app), ambient);
    assert_eq!(sound_entity::<MusicSound>(&mut app), music);
}

#[test]
fn maintenance_unlisted_zone_keeps_playback_through_idle_and_volume_changes() {
    let mut app = maintenance_test_app();
    app.insert_resource(crate::networking::CurrentZone { zone_id: 99 });
    sound_update(&mut app);
    let ambient = sound_entity::<AmbientSound>(&mut app);
    let music = sound_entity::<MusicSound>(&mut app);
    for _ in 0..3 {
        sound_update(&mut app);
    }
    app.world_mut()
        .resource_mut::<SoundSettings>()
        .master_volume = 0.4;
    sound_update(&mut app);
    assert_eq!(sound_entity::<AmbientSound>(&mut app), ambient);
    assert_eq!(sound_entity::<MusicSound>(&mut app), music);
}

#[test]
fn maintenance_completion_removal_advances_zone_tracks_and_recovers() {
    let mut app = maintenance_test_app();
    app.insert_resource(crate::networking::CurrentZone { zone_id: 5 });
    sound_update(&mut app);
    let ambient = sound_entity::<AmbientSound>(&mut app);
    let music = sound_entity::<MusicSound>(&mut app);
    assert_eq!(
        app.world()
            .resource::<MusicPlaybackState>()
            .active_track_name
            .as_deref(),
        Some("zone:5:first")
    );
    // Bevy cleanup_finished_audio despawns DESPAWN players after sink completion.
    app.world_mut().despawn(ambient);
    app.world_mut().despawn(music);
    sound_update(&mut app);
    assert_ne!(sound_entity::<AmbientSound>(&mut app), ambient);
    assert_ne!(sound_entity::<MusicSound>(&mut app), music);
    assert_eq!(
        app.world()
            .resource::<AmbientPlaybackState>()
            .active_track_name
            .as_deref(),
        Some("ambient:5:second")
    );
    assert_eq!(
        app.world()
            .resource::<MusicPlaybackState>()
            .active_track_name
            .as_deref(),
        Some("zone:5:second")
    );
    app.world_mut()
        .resource_mut::<SoundSettings>()
        .music_enabled = false;
    sound_update(&mut app);
    sound_update(&mut app);
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<MusicSound>>()
            .iter(app.world())
            .count(),
        0
    );
    app.world_mut()
        .resource_mut::<SoundSettings>()
        .music_enabled = true;
    sound_update(&mut app);
    assert_eq!(
        app.world()
            .resource::<MusicPlaybackState>()
            .active_track_name
            .as_deref(),
        Some("zone:5:first")
    );
}

#[test]
fn maintenance_zone_removal_replaces_tracks_then_stays_idle() {
    let mut app = maintenance_test_app();
    app.insert_resource(crate::networking::CurrentZone { zone_id: 5 });
    sound_update(&mut app);
    let previous = sound_entity::<AmbientSound>(&mut app);
    app.world_mut()
        .remove_resource::<crate::networking::CurrentZone>();
    sound_update(&mut app);
    sound_update(&mut app);
    let current = sound_entity::<AmbientSound>(&mut app);
    assert_ne!(previous, current);
    assert_eq!(
        app.world()
            .resource::<AmbientPlaybackState>()
            .active_track_name
            .as_deref(),
        Some("procedural-ambient-fallback")
    );
    for _ in 0..3 {
        sound_update(&mut app);
    }
    assert_eq!(sound_entity::<AmbientSound>(&mut app), current);
}

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
            emitter_entity: None,
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

#[test]
fn resolve_spell_sound_emitter_uses_local_player_for_cast_start() {
    let local_player = Entity::from_bits(42);
    let request = SpellSoundRequest {
        spell_id: 133,
        kind: SpellSoundKind::CastStart,
        emitter_entity: None,
    };

    let emitter = resolve_spell_sound_emitter(&request, Some(local_player));

    assert_eq!(emitter, Some(local_player));
}

#[test]
fn resolve_spell_sound_emitter_prefers_explicit_entity() {
    let explicit = Entity::from_bits(7);
    let request = SpellSoundRequest {
        spell_id: 2061,
        kind: SpellSoundKind::Heal,
        emitter_entity: Some(explicit),
    };

    let emitter = resolve_spell_sound_emitter(&request, Some(Entity::from_bits(42)));

    assert_eq!(emitter, Some(explicit));
}

#[test]
fn spawn_spatial_audio_child_marks_playback_spatial() {
    let mut world = World::new();
    let emitter = world.spawn(Transform::default()).id();
    let handle = Handle::<AudioSource>::default();

    spawn_spatial_audio_child(&mut world.commands(), emitter, handle, 0.5);
    world.flush();

    let children = world.get::<Children>(emitter).expect("child sound entity");
    let child = children.first().expect("spawned sound child");
    let settings = world
        .get::<PlaybackSettings>(*child)
        .expect("playback settings");
    assert!(settings.spatial);
}

#[test]
fn strip_ambient_tracks_from_music_catalog_removes_overlap() {
    let mut music_tracks_by_zone = HashMap::from([(5, vec![0usize, 1usize, 2usize])]);
    let ambient_tracks_by_zone = HashMap::from([(5, vec![1usize])]);

    strip_ambient_tracks_from_music_catalog(&mut music_tracks_by_zone, &ambient_tracks_by_zone);

    assert_eq!(music_tracks_by_zone.get(&5), Some(&vec![0, 2]));
}
