use bevy::audio::Volume;
use bevy::prelude::*;

use super::{MusicPlaybackState, MusicSound, SoundAssets, SoundSettings};

pub(super) fn spawn_music_sound(
    mut commands: Commands,
    sound_assets: Res<SoundAssets>,
    settings: Res<SoundSettings>,
    current_zone: Option<Res<crate::networking::CurrentZone>>,
    mut state: ResMut<MusicPlaybackState>,
) {
    if !settings.music_enabled {
        return;
    }
    spawn_next_music_track(
        &mut commands,
        &sound_assets,
        &settings,
        current_zone.as_deref(),
        &mut state,
    );
}

pub(super) fn maintain_music_playback(
    mut commands: Commands,
    sound_assets: Res<SoundAssets>,
    settings: Res<SoundSettings>,
    current_zone: Option<Res<crate::networking::CurrentZone>>,
    mut state: ResMut<MusicPlaybackState>,
    music_query: Query<Entity, With<MusicSound>>,
) {
    let desired_zone_id = desired_zone_id(current_zone.as_deref());
    if !sync_music_zone_state(
        &mut commands,
        &settings,
        desired_zone_id,
        &mut state,
        &music_query,
    ) {
        return;
    }
    if !music_query.is_empty() {
        return;
    }
    spawn_next_music_track(
        &mut commands,
        &sound_assets,
        &settings,
        current_zone.as_deref(),
        &mut state,
    );
}

fn sync_music_zone_state(
    commands: &mut Commands,
    settings: &SoundSettings,
    desired_zone_id: Option<u32>,
    state: &mut MusicPlaybackState,
    music_query: &Query<Entity, With<MusicSound>>,
) -> bool {
    if !settings.music_enabled {
        clear_music_entities(commands, music_query);
        clear_active_music_state(state);
        return false;
    }
    if state.active_zone_id != desired_zone_id && !music_query.is_empty() {
        clear_music_entities(commands, music_query);
        clear_active_music_state(state);
        return false;
    }
    true
}

fn spawn_next_music_track(
    commands: &mut Commands,
    sound_assets: &SoundAssets,
    settings: &SoundSettings,
    current_zone: Option<&crate::networking::CurrentZone>,
    state: &mut MusicPlaybackState,
) {
    if let Some((handle, name, looped, zone_id)) =
        next_music_track(sound_assets, current_zone, state)
    {
        commands.spawn((
            MusicSound,
            AudioPlayer::<AudioSource>::new(handle),
            playback_settings_for_track(settings, looped),
        ));
        state.active_track_name = Some(name);
        state.active_zone_id = zone_id;
    }
}

fn playback_settings_for_track(settings: &SoundSettings, looped: bool) -> PlaybackSettings {
    let base = if looped {
        PlaybackSettings::LOOP
    } else {
        PlaybackSettings::DESPAWN
    };
    base.with_volume(Volume::Linear(compute_music_volume(settings)))
}

fn compute_music_volume(settings: &SoundSettings) -> f32 {
    if settings.muted || !settings.music_enabled {
        0.0
    } else {
        settings.music_volume * settings.master_volume
    }
}

fn clear_music_entities(commands: &mut Commands, music_query: &Query<Entity, With<MusicSound>>) {
    for entity in music_query {
        commands.entity(entity).despawn();
    }
}

fn clear_active_music_state(state: &mut MusicPlaybackState) {
    state.active_track_name = None;
    state.active_zone_id = None;
}

fn desired_zone_id(current_zone: Option<&crate::networking::CurrentZone>) -> Option<u32> {
    current_zone
        .map(|zone| zone.zone_id)
        .filter(|zone_id| *zone_id != 0)
}

fn next_music_track(
    sound_assets: &SoundAssets,
    current_zone: Option<&crate::networking::CurrentZone>,
    state: &mut MusicPlaybackState,
) -> Option<(Handle<AudioSource>, String, bool, Option<u32>)> {
    if let Some(zone_track) =
        next_zone_music_track(sound_assets, desired_zone_id(current_zone), state)
    {
        return Some(zone_track);
    }
    if !sound_assets.music_tracks.is_empty() {
        let idx = state.next_track_idx % sound_assets.music_tracks.len();
        state.next_track_idx = state.next_track_idx.wrapping_add(1);
        let track = &sound_assets.music_tracks[idx];
        return Some((track.handle.clone(), track.name.clone(), false, None));
    }
    Some((
        sound_assets.music_loop_fallback.clone(),
        "procedural-fallback".to_string(),
        true,
        None,
    ))
}

fn next_zone_music_track(
    sound_assets: &SoundAssets,
    zone_id: Option<u32>,
    state: &mut MusicPlaybackState,
) -> Option<(Handle<AudioSource>, String, bool, Option<u32>)> {
    let zone_id = zone_id?;
    let track_indices = sound_assets.music_tracks_by_zone.get(&zone_id)?;
    if track_indices.is_empty() {
        return None;
    }
    let next_idx = state.next_zone_track_idx.entry(zone_id).or_default();
    let track_idx = track_indices[*next_idx % track_indices.len()];
    *next_idx = next_idx.wrapping_add(1);
    let track = &sound_assets.music_tracks[track_idx];
    Some((
        track.handle.clone(),
        format!("zone:{zone_id}:{}", track.name),
        false,
        Some(zone_id),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::LoadedMusicTrack;

    #[test]
    fn playback_settings_for_looped_tracks_loops() {
        let settings = SoundSettings::default();
        assert!(matches!(
            playback_settings_for_track(&settings, true).mode,
            bevy::audio::PlaybackMode::Loop
        ));
    }

    #[test]
    fn playback_settings_for_one_shot_tracks_despawn() {
        let settings = SoundSettings::default();
        assert!(matches!(
            playback_settings_for_track(&settings, false).mode,
            bevy::audio::PlaybackMode::Despawn
        ));
    }

    #[test]
    fn desired_zone_id_ignores_zero() {
        let zone = crate::networking::CurrentZone { zone_id: 0 };
        assert_eq!(desired_zone_id(Some(&zone)), None);
    }

    #[test]
    fn clear_active_music_state_resets_active_track() {
        let mut state = MusicPlaybackState {
            active_track_name: Some("track".to_string()),
            active_zone_id: Some(12),
            ..Default::default()
        };
        clear_active_music_state(&mut state);
        assert_eq!(state.active_track_name, None);
        assert_eq!(state.active_zone_id, None);
    }

    #[test]
    fn next_music_track_falls_back_when_no_tracks_exist() {
        let sound_assets = SoundAssets {
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
            music_tracks: Vec::new(),
            music_tracks_by_zone: Default::default(),
        };
        let mut state = MusicPlaybackState::default();
        let (_, name, looped, zone_id) = next_music_track(&sound_assets, None, &mut state).unwrap();
        assert_eq!(name, "procedural-fallback");
        assert!(looped);
        assert_eq!(zone_id, None);
    }

    #[test]
    fn next_zone_music_track_advances_per_zone_cursor() {
        let sound_assets = SoundAssets {
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
                    name: "a".to_string(),
                },
                LoadedMusicTrack {
                    handle: Handle::default(),
                    name: "b".to_string(),
                },
            ],
            music_tracks_by_zone: [(5, vec![0, 1])].into_iter().collect(),
        };
        let mut state = MusicPlaybackState::default();
        let first = next_zone_music_track(&sound_assets, Some(5), &mut state).unwrap();
        let second = next_zone_music_track(&sound_assets, Some(5), &mut state).unwrap();
        assert_eq!(first.1, "zone:5:a");
        assert_eq!(second.1, "zone:5:b");
    }
}
