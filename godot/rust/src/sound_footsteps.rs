//! Local authored footsteps on owned Godot 3D emitters.
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use game_engine_core::{
    client_options_data::SoundOptionsFile,
    footstep_data::{
        FootstepCatalog, FootstepCatalogEntry, FootstepCreature, FootstepMovement,
        FootstepPhaseTracker, FootstepRequest, FootstepSurface, classify_player_creature,
    },
};
use godot::{
    classes::{AudioStream, AudioStreamOggVorbis, AudioStreamPlayer3D, Node3D},
    prelude::*,
};

const MAX_FILES_PER_BUCKET: usize = 6;

pub(super) struct Footsteps {
    pub root: Gd<Node3D>,
    catalog: FootstepCatalog,
    streams: HashMap<u32, Gd<AudioStream>>,
    active: Vec<Gd<AudioStreamPlayer3D>>,
    tracker: FootstepPhaseTracker,
    player_id: Option<u64>,
}

impl Footsteps {
    pub fn new() -> Self {
        let mut root = Node3D::new_alloc();
        root.set_name("Footsteps");
        Self {
            root,
            catalog: FootstepCatalog::default(),
            streams: HashMap::new(),
            active: Vec::new(),
            tracker: FootstepPhaseTracker::default(),
            player_id: None,
        }
    }

    pub fn load(&mut self, data_root: &Path) -> Result<(), String> {
        self.apply(read_footstep_files(data_root)?)
    }

    /// Replace the catalog with `files`' footsteps.
    pub fn apply(&mut self, files: FootstepFiles) -> Result<(), String> {
        self.stop();
        self.catalog.entries.clear();
        self.streams.clear();
        for error in files.errors {
            godot_error!("{error}");
        }
        for (entry, bytes) in files.files {
            match ogg_stream(&bytes, entry.fdid) {
                Some(stream) => {
                    self.streams.insert(entry.fdid, stream);
                    self.catalog.entries.push(entry);
                }
                None => godot_error!("Footstep {}: Godot rejected Ogg stream", entry.fdid),
            }
        }
        if self.catalog.entries.is_empty() {
            return Err(format!(
                "{}: no supported local Ogg footstep files loaded",
                files.listfile.display()
            ));
        }
        Ok(())
    }

    pub fn observe(
        &mut self,
        player_id: u64,
        race: u8,
        phase: (usize, u16, f32, f32),
        position: Vector3,
        surface: FootstepSurface,
        settings: &SoundOptionsFile,
    ) {
        if self.player_id != Some(player_id) {
            self.stop();
            self.player_id = Some(player_id);
        }
        self.remove_finished();
        for player in &mut self.active {
            player.set_global_position(position);
        }
        // Legacy muted tracking does not advance: unmuting observes the current half once.
        if settings.muted {
            return;
        }
        let Some((fdid, movement)) = self.select_step(race, phase, surface) else {
            return;
        };
        self.play_step(fdid, movement, position, settings);
    }

    fn select_step(
        &mut self,
        race: u8,
        phase: (usize, u16, f32, f32),
        surface: FootstepSurface,
    ) -> Option<(u32, FootstepMovement)> {
        let (seq_idx, anim_id, duration, time_ms) = phase;
        let (movement, seed) = self.tracker.observe(seq_idx, anim_id, duration, time_ms)?;
        let request = FootstepRequest {
            creature: classify_player_creature(race),
            surface,
            movement,
            seed,
        };
        self.catalog
            .select(request)
            .map(|entry| (entry.fdid, movement))
    }

    fn play_step(
        &mut self,
        fdid: u32,
        movement: FootstepMovement,
        position: Vector3,
        settings: &SoundOptionsFile,
    ) {
        let Some(stream) = self.streams.get(&fdid) else {
            return;
        };
        let gain = settings.master_volume * settings.effects_volume * movement_gain(movement);
        let mut player = AudioStreamPlayer3D::new_alloc();
        player.set_stream(stream);
        player.set_volume_linear(gain);
        self.root.add_child(&player);
        player.set_global_position(position);
        player.play();
        self.active.push(player);
    }

    pub fn stop(&mut self) {
        for player in self.active.drain(..) {
            player.free();
        }
        self.tracker = FootstepPhaseTracker::default();
        self.player_id = None;
    }

    fn remove_finished(&mut self) {
        self.active.retain(|player| {
            if player.is_playing() {
                true
            } else {
                player.clone().free();
                false
            }
        });
    }
}

/// Local footstep Ogg files the community listfile names, at most
/// `MAX_FILES_PER_BUCKET` per creature and surface, with the files that could not be
/// read. Reading touches no Godot object, so it runs on any thread.
pub(crate) struct FootstepFiles {
    listfile: PathBuf,
    files: Vec<(FootstepCatalogEntry, Vec<u8>)>,
    errors: Vec<String>,
}

pub(crate) fn read_footstep_files(data_root: &Path) -> Result<FootstepFiles, String> {
    let listfile = data_root.join("community-listfile.csv");
    let file =
        File::open(&listfile).map_err(|error| format!("open {}: {error}", listfile.display()))?;
    let mut files = FootstepFiles {
        listfile,
        files: Vec::new(),
        errors: Vec::new(),
    };
    let mut counts = HashMap::new();
    for line in BufReader::new(file).lines() {
        let line =
            line.map_err(|error| format!("read {}: {error}", files.listfile.display()))?;
        files.read_row(&line, data_root, &mut counts);
    }
    Ok(files)
}

impl FootstepFiles {
    fn read_row(
        &mut self,
        line: &str,
        data_root: &Path,
        counts: &mut HashMap<(FootstepCreature, FootstepSurface), usize>,
    ) {
        let Some((fdid, source)) = parse_supported_row(line) else {
            return;
        };
        let local = data_root
            .join("sounds/footsteps")
            .join(format!("{fdid}.ogg"));
        if !local.exists() {
            return;
        }
        let Some(entry) = FootstepCatalogEntry::from_path(fdid, source) else {
            return;
        };
        let count = counts.entry((entry.creature, entry.surface)).or_insert(0);
        if *count >= MAX_FILES_PER_BUCKET {
            return;
        }
        match read_ogg(&local) {
            Ok(bytes) => {
                self.files.push((entry, bytes));
                *count += 1;
            }
            Err(error) => self.errors.push(format!("Footstep {fdid}: {error}")),
        }
    }
}

fn parse_supported_row(line: &str) -> Option<(u32, &str)> {
    let (fdid, path) = line.split_once(';')?;
    let lower = path.to_ascii_lowercase();
    let supported = lower.ends_with(".ogg")
        && (lower.starts_with("sound/character/footsteps/")
            || (lower.starts_with("sound/creature/") && lower.contains("footstep")));
    if !supported {
        return None;
    }
    Some((fdid.parse().ok()?, path))
}

pub(super) fn load_ogg(path: &PathBuf, fdid: u32) -> Result<Gd<AudioStream>, String> {
    let bytes = read_ogg(path)?;
    ogg_stream(&bytes, fdid).ok_or_else(|| format!("{}: Godot rejected Ogg stream", path.display()))
}

fn read_ogg(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    if !bytes.starts_with(b"OggS") {
        return Err(format!("{}: not Ogg data", path.display()));
    }
    Ok(bytes)
}

fn ogg_stream(bytes: &[u8], fdid: u32) -> Option<Gd<AudioStream>> {
    let mut stream: Gd<AudioStream> =
        AudioStreamOggVorbis::load_from_buffer(&PackedByteArray::from(bytes))?.upcast();
    stream.set_name(&fdid.to_string());
    Some(stream)
}

fn movement_gain(movement: FootstepMovement) -> f32 {
    match movement {
        FootstepMovement::Walk => 0.85,
        FootstepMovement::Run => 1.0,
        FootstepMovement::Strafe | FootstepMovement::Backpedal => 0.8,
    }
}
