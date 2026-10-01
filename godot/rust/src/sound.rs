//! Authored zone music and ambience on native Godot audio players.
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use game_engine_core::catalog_data::{
    read_ambient_zone_catalog, read_music_zone_catalog, strip_ambient_tracks_from_music_catalog,
};
use game_engine_core::client_options_data::SoundOptionsFile;
use game_engine_core::ui_click_data::{
    CLICK_SAMPLE_RATE, CLICK_VOLUME_SCALE, generate_button_click_samples,
};
use godot::classes::{
    AudioStream, AudioStreamMp3, AudioStreamOggVorbis, AudioStreamPlayer, AudioStreamWav, INode,
    Node,
};
use godot::prelude::*;

use crate::sound_footsteps::Footsteps;
use crate::sound_ui_kits::UiKits;

struct Channel {
    player: Gd<AudioStreamPlayer>,
    by_zone: HashMap<u32, Vec<usize>>,
    cursors: HashMap<u32, usize>,
    active_zone: Option<u32>,
    failed: Option<(u32, usize)>,
}

impl Channel {
    fn new(name: &str) -> Self {
        let mut player = AudioStreamPlayer::new_alloc();
        player.set_name(name);
        Self {
            player,
            by_zone: HashMap::new(),
            cursors: HashMap::new(),
            active_zone: None,
            failed: None,
        }
    }

    fn stop(&mut self) {
        self.player.stop();
        self.active_zone = None;
    }

    fn sync(
        &mut self,
        zone: Option<u32>,
        enabled: bool,
        volume: f32,
        tracks: &[PathBuf],
        cache: &mut HashMap<usize, Gd<AudioStream>>,
    ) -> Result<(), String> {
        self.player.set_volume_linear(volume);
        if !enabled || zone.is_none() {
            self.stop();
            return Ok(());
        }
        let zone = zone.expect("checked above");
        if self.active_zone != Some(zone) {
            self.stop();
            self.failed = None;
        } else if self.player.is_playing() || self.failed.is_some() {
            return Ok(());
        }
        self.play_next(zone, tracks, cache)
    }

    fn play_next(
        &mut self,
        zone: u32,
        tracks: &[PathBuf],
        cache: &mut HashMap<usize, Gd<AudioStream>>,
    ) -> Result<(), String> {
        let Some(indices) = self.by_zone.get(&zone) else {
            return Ok(());
        };
        if indices.is_empty() {
            return Ok(());
        }
        let cursor = self.cursors.entry(zone).or_default();
        let index = indices[*cursor % indices.len()];
        self.active_zone = Some(zone);
        let stream = load_stream(&tracks[index], index, cache).map_err(|error| {
            self.failed = Some((zone, index));
            error
        })?;
        self.player.set_stream(&stream);
        self.player.play();
        *cursor = cursor.wrapping_add(1);
        Ok(())
    }
}

fn load_stream(
    path: &Path,
    index: usize,
    cache: &mut HashMap<usize, Gd<AudioStream>>,
) -> Result<Gd<AudioStream>, String> {
    if let Some(stream) = cache.get(&index) {
        return Ok(stream.clone());
    }
    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let buffer = PackedByteArray::from(bytes.as_slice());
    let mut stream: Gd<AudioStream> = if bytes.starts_with(b"OggS") {
        AudioStreamOggVorbis::load_from_buffer(&buffer).map(|value| value.upcast())
    } else if bytes.starts_with(b"ID3") || is_mp3_frame(&bytes) {
        AudioStreamMp3::load_from_buffer(&buffer).map(|value| value.upcast())
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE") {
        AudioStreamWav::load_from_buffer(&buffer).map(|value| value.upcast())
    } else if bytes.starts_with(b"fLaC") {
        return Err(format!(
            "{}: FLAC playback unsupported by native Godot decoder",
            path.display()
        ));
    } else {
        return Err(format!("{}: unknown audio format", path.display()));
    }
    .ok_or_else(|| format!("{}: Godot rejected audio stream", path.display()))?;
    stream.set_name(
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown"),
    );
    cache.insert(index, stream.clone());
    Ok(stream)
}

fn click_stream() -> Gd<AudioStreamWav> {
    pcm_stream(&generate_button_click_samples(), CLICK_SAMPLE_RATE as u32)
}

pub(super) fn pcm_stream(samples: &[i16], sample_rate: u32) -> Gd<AudioStreamWav> {
    let data_size = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_size as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    AudioStreamWav::load_from_buffer(&PackedByteArray::from(wav.as_slice()))
        .expect("generated samples are valid mono PCM WAV")
}

fn is_mp3_frame(bytes: &[u8]) -> bool {
    let Some(&[0xff, header, ..]) = bytes.get(..4) else {
        return false;
    };
    header & 0xe0 == 0xe0 && (header >> 3) & 3 != 1 && (header >> 1) & 3 != 0
}

fn discover_tracks(root: &Path) -> Result<(Vec<PathBuf>, HashMap<u32, usize>), String> {
    let mut tracks = Vec::new();
    let mut indices = HashMap::new();
    for dir in [root.join("sound/music"), root.join("music")] {
        if !dir.exists() {
            continue;
        }
        let mut entries = std::fs::read_dir(&dir)
            .map_err(|error| format!("read {}: {error}", dir.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("read {} entry: {error}", dir.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        for path in entries {
            let Some(fdid) = music_file_id(&path) else {
                continue;
            };
            if indices.contains_key(&fdid) {
                continue;
            }
            indices.insert(fdid, tracks.len());
            tracks.push(path);
        }
    }
    Ok((tracks, indices))
}

fn music_file_id(path: &Path) -> Option<u32> {
    if !path.is_file()
        || !matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("mp3" | "ogg" | "wav" | "flac")
        )
    {
        return None;
    }
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.parse().ok())
}

fn open_catalog(root: &Path, name: &str) -> Result<BufReader<File>, String> {
    let path = root.join(name);
    File::open(&path)
        .map(BufReader::new)
        .map_err(|error| format!("open {}: {error}", path.display()))
}

#[derive(GodotClass)]
#[class(base = Node)]
pub struct NativeSound {
    base: Base<Node>,
    tracks: Vec<PathBuf>,
    cache: HashMap<usize, Gd<AudioStream>>,
    music: Channel,
    ambient: Channel,
    effects: Gd<AudioStreamPlayer>,
    footsteps: Footsteps,
    pub(crate) ui_kits: UiKits,
}

#[godot_api]
impl INode for NativeSound {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            tracks: Vec::new(),
            cache: HashMap::new(),
            music: Channel::new("Music"),
            ambient: Channel::new("Ambient"),
            effects: AudioStreamPlayer::new_alloc(),
            footsteps: Footsteps::new(),
            ui_kits: UiKits::new(),
        }
    }

    fn ready(&mut self) {
        let music = self.music.player.clone();
        let ambient = self.ambient.player.clone();
        self.base_mut().add_child(&music);
        self.base_mut().add_child(&ambient);
        let mut effects = self.effects.clone();
        effects.set_name("Effects");
        self.base_mut().add_child(&effects);
        self.effects.set_stream(&click_stream());
        let footsteps = self.footsteps.root.clone();
        self.base_mut().add_child(&footsteps);
        let ui_kits = self.ui_kits.root.clone();
        self.base_mut().add_child(&ui_kits);
    }

    fn exit_tree(&mut self) {
        self.stop();
        self.ui_kits.stop();
        self.effects.stop();
        self.effects.set_stream(Gd::<AudioStream>::null_arg());
        self.music.player.set_stream(Gd::<AudioStream>::null_arg());
        self.ambient
            .player
            .set_stream(Gd::<AudioStream>::null_arg());
        self.cache.clear();
    }
}

#[godot_api]
impl NativeSound {
    #[func]
    fn configure(&mut self, data_root: GString) -> bool {
        match self.load_catalog(Path::new(&data_root.to_string())) {
            Ok(()) => true,
            Err(error) => {
                godot_error!("{error}");
                false
            }
        }
    }

    #[func]
    fn configure_footsteps(&mut self, data_root: GString) -> bool {
        match self.load_footsteps(Path::new(&data_root.to_string())) {
            Ok(()) => true,
            Err(error) => {
                godot_error!("{error}");
                false
            }
        }
    }

    #[func]
    pub fn play_ui_click(&mut self, master: f32, effects: f32, muted: bool) -> bool {
        self.effects.set_volume_linear(if muted {
            0.0
        } else {
            master * effects * CLICK_VOLUME_SCALE
        });
        self.effects.play();
        true
    }

    #[func]
    fn sync_options(
        &mut self,
        zone: i64,
        master: f32,
        music: f32,
        ambient: f32,
        music_enabled: bool,
        muted: bool,
    ) -> bool {
        let settings = SoundOptionsFile {
            master_volume: master,
            music_volume: music,
            ambient_volume: ambient,
            effects_volume: 0.0,
            music_enabled,
            muted,
        };
        match self.sync(
            u32::try_from(zone).ok().filter(|value| *value != 0),
            &settings,
        ) {
            Ok(()) => true,
            Err(error) => {
                godot_error!("{error}");
                false
            }
        }
    }
}

impl NativeSound {
    pub fn load_footsteps(&mut self, data_root: &Path) -> Result<(), String> {
        self.footsteps.load(data_root)
    }

    pub fn observe_footstep(
        &mut self,
        player_id: u64,
        race: u8,
        phase: (usize, u16, f32, f32),
        position: Vector3,
        surface: game_engine_core::footstep_data::FootstepSurface,
        settings: &SoundOptionsFile,
    ) {
        self.footsteps
            .observe(player_id, race, phase, position, surface, settings);
    }

    pub fn stop_footsteps(&mut self) {
        self.footsteps.stop();
    }

    pub fn load_catalog(&mut self, data_root: &Path) -> Result<(), String> {
        let (tracks, indices) = discover_tracks(data_root)?;
        let mut music =
            read_music_zone_catalog(open_catalog(data_root, "music_zone_links.csv")?, &indices)
                .map_err(|error| {
                    format!(
                        "{}: {error}",
                        data_root.join("music_zone_links.csv").display()
                    )
                })?;
        let ambient = read_ambient_zone_catalog(
            open_catalog(data_root, "music_manifest.csv")?,
            &data_root.join("music_manifest.csv"),
            &indices,
        )?;
        strip_ambient_tracks_from_music_catalog(&mut music, &ambient);
        self.stop();
        self.tracks = tracks;
        self.cache.clear();
        self.music.by_zone = music;
        self.ambient.by_zone = ambient;
        self.music.cursors.clear();
        self.ambient.cursors.clear();
        Ok(())
    }

    pub fn sync(&mut self, zone: Option<u32>, settings: &SoundOptionsFile) -> Result<(), String> {
        let volume = if settings.muted {
            0.0
        } else {
            settings.master_volume
        };
        self.music.sync(
            zone,
            settings.music_enabled,
            volume * settings.music_volume,
            &self.tracks,
            &mut self.cache,
        )?;
        self.ambient.sync(
            zone,
            true,
            volume * settings.ambient_volume,
            &self.tracks,
            &mut self.cache,
        )
    }

    pub fn stop(&mut self) {
        self.music.stop();
        self.ambient.stop();
        self.stop_footsteps();
    }
}
