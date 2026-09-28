//! Authored zone music and ambience on native Godot audio players.
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use game_engine_core::catalog_data::{
    read_ambient_zone_catalog, read_music_zone_catalog, strip_ambient_tracks_from_music_catalog,
};
use game_engine_core::client_options_data::SoundOptionsFile;
use godot::classes::{
    AudioStream, AudioStreamMp3, AudioStreamOggVorbis, AudioStreamPlayer, AudioStreamWav, INode,
    Node,
};
use godot::prelude::*;

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
            if !path.is_file() {
                continue;
            }
            let Some(fdid) = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.parse::<u32>().ok())
            else {
                continue;
            };
            if !matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("mp3" | "ogg" | "wav" | "flac")
            ) {
                continue;
            }
            if indices.contains_key(&fdid) {
                continue;
            }
            indices.insert(fdid, tracks.len());
            tracks.push(path);
        }
    }
    Ok((tracks, indices))
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
        }
    }

    fn ready(&mut self) {
        let music = self.music.player.clone();
        let ambient = self.ambient.player.clone();
        self.base_mut().add_child(&music);
        self.base_mut().add_child(&ambient);
    }

    fn exit_tree(&mut self) {
        self.stop();
        self.music.player.set_stream(None::<Gd<AudioStream>>);
        self.ambient.player.set_stream(None::<Gd<AudioStream>>);
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
    }
}
