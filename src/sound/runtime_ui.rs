use bevy::audio::{AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;

use crate::ui_input::walk_up_for_onclick;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::UiState;

use super::{SoundAssets, SoundSettings, compute_effects_volume, load_generated_audio};

pub struct LoadedUiAudioAssets {
    pub ui_button_click: Handle<AudioSource>,
    pub ui_bag_open: Handle<AudioSource>,
    pub ui_bag_close: Handle<AudioSource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSoundKind {
    ButtonClick,
    BagOpen,
    BagClose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UiSoundRequest {
    kind: UiSoundKind,
}

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct UiSoundQueue {
    requests: Vec<UiSoundRequest>,
}

impl UiSoundQueue {
    pub fn queued_kinds(&self) -> Vec<UiSoundKind> {
        self.requests.iter().map(|request| request.kind).collect()
    }
}

pub fn queue_ui_sound(queue: &mut UiSoundQueue, kind: UiSoundKind) {
    queue.requests.push(UiSoundRequest { kind });
}

pub(super) fn load_ui_audio_assets(audio_assets: &mut Assets<AudioSource>) -> LoadedUiAudioAssets {
    LoadedUiAudioAssets {
        ui_button_click: load_generated_audio(audio_assets, &generate_button_click_samples()),
        ui_bag_open: load_generated_audio(audio_assets, &generate_bag_open_samples()),
        ui_bag_close: load_generated_audio(audio_assets, &generate_bag_close_samples()),
    }
}

pub(super) fn queue_button_click_sound(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Option<Res<UiState>>,
    mut queue: ResMut<UiSoundQueue>,
) {
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(ui) = ui else { return };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    if walk_up_for_onclick(&ui.registry, frame_id).is_some() {
        queue_ui_sound(&mut queue, UiSoundKind::ButtonClick);
    }
}

pub(super) fn play_queued_ui_sounds(
    mut commands: Commands,
    sound_assets: Option<Res<SoundAssets>>,
    settings: Res<SoundSettings>,
    mut queue: ResMut<UiSoundQueue>,
) {
    let Some(sound_assets) = sound_assets else {
        queue.requests.clear();
        return;
    };
    if queue.requests.is_empty() {
        return;
    }

    let base_volume = compute_effects_volume(&settings);
    let requests = std::mem::take(&mut queue.requests);
    for request in requests {
        play_ui_sound(&mut commands, &sound_assets, base_volume, request.kind);
    }
}

fn play_ui_sound(
    commands: &mut Commands,
    sound_assets: &SoundAssets,
    base_volume: f32,
    kind: UiSoundKind,
) {
    let handle = match kind {
        UiSoundKind::ButtonClick => sound_assets.ui_button_click.clone(),
        UiSoundKind::BagOpen => sound_assets.ui_bag_open.clone(),
        UiSoundKind::BagClose => sound_assets.ui_bag_close.clone(),
    };
    let volume = base_volume * ui_sound_volume_scale(kind);
    commands.spawn((
        AudioPlayer::<AudioSource>::new(handle),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
    ));
}

fn ui_sound_volume_scale(kind: UiSoundKind) -> f32 {
    match kind {
        UiSoundKind::ButtonClick => 0.55,
        UiSoundKind::BagOpen => 0.75,
        UiSoundKind::BagClose => 0.65,
    }
}

fn generate_button_click_samples() -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let duration_ms = 40;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(4.0);
        let tone = (t * 2.0 * std::f32::consts::PI * 1_300.0).sin() * 0.6;
        let tick = (t * 2.0 * std::f32::consts::PI * 2_600.0).sin() * 0.2;
        samples.push(((tone + tick) * envelope * 10_500.0) as i16);
    }
    samples
}

fn generate_bag_open_samples() -> Vec<i16> {
    generate_bag_rustle_samples(180.0, 95, 0.75)
}

fn generate_bag_close_samples() -> Vec<i16> {
    generate_bag_rustle_samples(140.0, 75, 0.55)
}

fn generate_bag_rustle_samples(base_hz: f32, duration_ms: u32, noise_mix: f32) -> Vec<i16> {
    let sample_rate = 44_100.0_f32;
    let sample_count = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    let mut samples = Vec::with_capacity(sample_count);
    let mut rng_state: u32 = 99;

    for i in 0..sample_count {
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = ((rng_state >> 16) as i32 - 32768) as f32 / 32768.0;
        let t = i as f32 / sample_count as f32;
        let envelope = (1.0 - t).powf(2.7);
        let tone = (t * 2.0 * std::f32::consts::PI * base_hz).sin() * (1.0 - noise_mix);
        let sample = (tone + noise * noise_mix) * envelope * 12_500.0;
        samples.push(sample.clamp(-32_767.0, 32_767.0) as i16);
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_ui_sound_preserves_request_order() {
        let mut queue = UiSoundQueue::default();

        queue_ui_sound(&mut queue, UiSoundKind::ButtonClick);
        queue_ui_sound(&mut queue, UiSoundKind::BagOpen);
        queue_ui_sound(&mut queue, UiSoundKind::BagClose);

        assert_eq!(
            queue.requests,
            vec![
                UiSoundRequest {
                    kind: UiSoundKind::ButtonClick
                },
                UiSoundRequest {
                    kind: UiSoundKind::BagOpen
                },
                UiSoundRequest {
                    kind: UiSoundKind::BagClose
                },
            ]
        );
    }

    #[test]
    fn generated_ui_samples_have_expected_lengths() {
        assert_eq!(generate_button_click_samples().len(), 1_764);
        assert_eq!(generate_bag_open_samples().len(), 4_189);
        assert_eq!(generate_bag_close_samples().len(), 3_307);
    }
}
