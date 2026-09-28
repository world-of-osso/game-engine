use std::collections::HashMap;
use std::io::BufReader;

use bevy::audio::Volume;
use bevy::prelude::*;
use game_engine::paths;

#[cfg(test)]
use super::LoadedMusicTrack;
use super::{AmbientPlaybackState, AmbientSound, SoundAssets, SoundSettings};

pub(super) fn spawn_ambient_sound(
    mut commands: Commands,
    sound_assets: Res<SoundAssets>,
    settings: Res<SoundSettings>,
    current_zone: Option<Res<crate::networking::CurrentZone>>,
    mut state: ResMut<AmbientPlaybackState>,
) {
    spawn_next_ambient_track(
        &mut commands,
        &sound_assets,
        &settings,
        current_zone.as_deref(),
        &mut state,
    );
}

pub(super) fn maintain_ambient_playback(
    mut commands: Commands,
    sound_assets: Res<SoundAssets>,
    settings: Res<SoundSettings>,
    current_zone: Option<Res<crate::networking::CurrentZone>>,
    mut state: ResMut<AmbientPlaybackState>,
    ambient_query: Query<Entity, With<AmbientSound>>,
) {
    let desired_zone_id = desired_zone_id(current_zone.as_deref());
    if state.active_zone_id != desired_zone_id && !ambient_query.is_empty() {
        clear_ambient_entities(&mut commands, &ambient_query);
        clear_active_ambient_state(&mut state);
        return;
    }
    if !ambient_query.is_empty() {
        return;
    }
    spawn_next_ambient_track(
        &mut commands,
        &sound_assets,
        &settings,
        current_zone.as_deref(),
        &mut state,
    );
}

fn spawn_next_ambient_track(
    commands: &mut Commands,
    sound_assets: &SoundAssets,
    settings: &SoundSettings,
    current_zone: Option<&crate::networking::CurrentZone>,
    state: &mut AmbientPlaybackState,
) {
    if let Some((handle, name, looped, _)) = next_ambient_track(sound_assets, current_zone, state) {
        commands.spawn((
            AmbientSound,
            AudioPlayer::<AudioSource>::new(handle),
            playback_settings_for_track(settings, looped),
        ));
        state.active_track_name = Some(name);
        state.active_zone_id = desired_zone_id(current_zone);
    }
}

fn playback_settings_for_track(settings: &SoundSettings, looped: bool) -> PlaybackSettings {
    let base = if looped {
        PlaybackSettings::LOOP
    } else {
        PlaybackSettings::DESPAWN
    };
    base.with_volume(Volume::Linear(super::compute_ambient_volume(settings)))
}

fn desired_zone_id(current_zone: Option<&crate::networking::CurrentZone>) -> Option<u32> {
    current_zone
        .map(|zone| zone.zone_id)
        .filter(|zone_id| *zone_id != 0)
}

fn next_ambient_track(
    sound_assets: &SoundAssets,
    current_zone: Option<&crate::networking::CurrentZone>,
    state: &mut AmbientPlaybackState,
) -> Option<(Handle<AudioSource>, String, bool, Option<u32>)> {
    if let Some(zone_track) =
        next_zone_ambient_track(sound_assets, desired_zone_id(current_zone), state)
    {
        return Some(zone_track);
    }
    Some((
        sound_assets.ambient_loop.clone(),
        "procedural-ambient-fallback".to_string(),
        true,
        None,
    ))
}

fn next_zone_ambient_track(
    sound_assets: &SoundAssets,
    zone_id: Option<u32>,
    state: &mut AmbientPlaybackState,
) -> Option<(Handle<AudioSource>, String, bool, Option<u32>)> {
    let zone_id = zone_id?;
    let track_indices = sound_assets.ambient_tracks_by_zone.get(&zone_id)?;
    if track_indices.is_empty() {
        return None;
    }
    let next_idx = state.next_zone_track_idx.entry(zone_id).or_default();
    let track_idx = track_indices[*next_idx % track_indices.len()];
    *next_idx = next_idx.wrapping_add(1);
    let track = &sound_assets.music_tracks[track_idx];
    Some((
        track.handle.clone(),
        format!("ambient:{zone_id}:{}", track.name),
        false,
        Some(zone_id),
    ))
}

fn clear_ambient_entities(
    commands: &mut Commands,
    ambient_query: &Query<Entity, With<AmbientSound>>,
) {
    for entity in ambient_query {
        commands.entity(entity).despawn();
    }
}

fn clear_active_ambient_state(state: &mut AmbientPlaybackState) {
    state.active_track_name = None;
    state.active_zone_id = None;
}

pub(super) fn load_zone_ambient_catalog(
    track_index_by_fdid: &HashMap<u32, usize>,
) -> HashMap<u32, Vec<usize>> {
    match load_zone_ambient_catalog_inner(track_index_by_fdid) {
        Ok(by_zone) => by_zone,
        Err(err) => {
            eprintln!("Failed to load ambient zone catalog: {err}");
            HashMap::new()
        }
    }
}

fn load_zone_ambient_catalog_inner(
    track_index_by_fdid: &HashMap<u32, usize>,
) -> Result<HashMap<u32, Vec<usize>>, String> {
    let path = paths::shared_data_path("music_manifest.csv");
    let reader = open_music_manifest_reader(&path)?;
    game_engine::catalog_data::read_ambient_zone_catalog(reader, &path, track_index_by_fdid)
}

fn open_music_manifest_reader(path: &std::path::Path) -> Result<BufReader<std::fs::File>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    Ok(BufReader::new(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_ambient_track_falls_back_when_no_zone_tracks_exist() {
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
            ambient_tracks_by_zone: Default::default(),
            music_tracks_by_zone: Default::default(),
        };
        let mut state = AmbientPlaybackState::default();
        let (_, name, looped, zone_id) =
            next_ambient_track(&sound_assets, None, &mut state).expect("ambient fallback");
        assert_eq!(name, "procedural-ambient-fallback");
        assert!(looped);
        assert_eq!(zone_id, None);
    }

    #[test]
    fn next_zone_ambient_track_advances_per_zone_cursor() {
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
                    name: "a".into(),
                },
                LoadedMusicTrack {
                    handle: Handle::default(),
                    name: "b".into(),
                },
            ],
            ambient_tracks_by_zone: [(5, vec![0, 1])].into_iter().collect(),
            music_tracks_by_zone: Default::default(),
        };
        let mut state = AmbientPlaybackState::default();
        let first = next_zone_ambient_track(&sound_assets, Some(5), &mut state).unwrap();
        let second = next_zone_ambient_track(&sound_assets, Some(5), &mut state).unwrap();
        assert_eq!(first.1, "ambient:5:a");
        assert_eq!(second.1, "ambient:5:b");
    }
}
