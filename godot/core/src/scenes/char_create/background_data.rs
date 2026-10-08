//! Renderer-free character-creation backdrop catalog and authored camera framing.

use crate::asset::m2_format::m2_camera::M2CameraSnapshot;
use crate::csv_util::{header_index, parse_csv_line_trimmed};
use glam::{Quat, Vec3};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
pub struct CreationSceneCatalog {
    scenes: HashMap<u8, u32>,
}

impl CreationSceneCatalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
        let mut catalog = Self::parse(BufReader::new(file), path)?;
        let data_root = path.parent().ok_or("ChrRaces path has no data root")?;
        let forever_path = data_root
            .join(crate::player_model_data::FOREVER_DB2_DIR)
            .join("ChrRaces.csv");
        let forever_file = File::open(&forever_path)
            .map_err(|err| format!("open {}: {err}", forever_path.display()))?;
        let forever = Self::parse(BufReader::new(forever_file), &forever_path)?;
        for race in crate::player_model_data::FOREVER_RACES {
            catalog.scenes.insert(race, forever.lookup(race)?);
        }
        Ok(catalog)
    }

    pub fn lookup(&self, race: u8) -> Result<u32, String> {
        self.scenes
            .get(&race)
            .copied()
            .ok_or_else(|| format!("no creation scene for race {race}"))
    }

    pub fn parse(reader: impl BufRead, path: &Path) -> Result<Self, String> {
        let mut lines = reader.lines();
        let header = lines
            .next()
            .transpose()
            .map_err(|err| format!("read {} header: {err}", path.display()))?
            .ok_or_else(|| format!("{} missing header", path.display()))?;
        let headers = parse_csv_line_trimmed(&header);
        let race_index = header_index(&headers, "ID", path)?;
        let scene_index = header_index(&headers, "CreateScreenFileDataID", path)?;
        let mut scenes = HashMap::new();

        for (line_index, line) in lines.enumerate() {
            let line_number = line_index + 2;
            let line =
                line.map_err(|err| format!("read {}:{line_number}: {err}", path.display()))?;
            let fields = parse_csv_line_trimmed(&line);
            if fields.len() != headers.len() {
                return Err(format!(
                    "{}:{line_number}: expected {} columns, found {}",
                    path.display(),
                    headers.len(),
                    fields.len()
                ));
            }
            let location = format!("{}:{line_number}", path.display());
            let (race, scene) = parse_scene_row(&fields, race_index, scene_index, &location)?;
            if scene != 0 && scenes.insert(race, scene).is_some() {
                return Err(format!(
                    "{}:{line_number}: duplicate ID {race}",
                    path.display()
                ));
            }
        }

        Ok(Self { scenes })
    }
}

fn parse_scene_row(
    fields: &[String],
    race_index: usize,
    scene_index: usize,
    location: &str,
) -> Result<(u8, u32), String> {
    let race = fields[race_index]
        .parse::<u8>()
        .map_err(|err| format!("{location}: invalid ID {:?}: {err}", fields[race_index]))?;
    let scene = fields[scene_index].parse::<u32>().map_err(|err| {
        format!(
            "{location}: invalid CreateScreenFileDataID {:?}: {err}",
            fields[scene_index]
        )
    })?;
    Ok((race, scene))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Framing {
    pub eye: Vec3,
    pub focus: Vec3,
    /// Authored M2 UI camera FOV (diagonal, radians).
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

/// Backdrop placement that moves the authored actor anchor (attachment 0) to the
/// preview origin and aligns the camera axis with +Z; the shot is unchanged.
#[derive(Clone, Copy, Debug)]
pub struct SceneNormalization {
    pub rotation: Quat,
    pub translation: Vec3,
    pub framing: Framing,
}

pub fn normalize_scene(camera: &M2CameraSnapshot, attachment: [f32; 3]) -> SceneNormalization {
    let convert = |p: [f32; 3]| Vec3::new(p[0], p[2], -p[1]);
    let anchor = convert(attachment);
    let eye = convert(camera.position);
    let focus = convert(camera.target);
    let offset = eye - focus;
    let rotation = Quat::from_rotation_y((-offset.x).atan2(offset.z));
    let translation = -(rotation * anchor);
    let place = |point: Vec3| rotation * point + translation;
    SceneNormalization {
        rotation,
        translation,
        framing: Framing {
            eye: place(eye),
            focus: place(focus),
            fov: camera.fov,
            near: camera.near_clip,
            far: camera.far_clip,
        },
    }
}

/// M2 UI cameras use diagonal FOV; renderers expect vertical FOV.
/// Reference: wow_client/src/ui/model.c camera projection.
pub fn vertical_fov(diagonal_fov: f32, aspect: f32) -> f32 {
    diagonal_fov / (1.0 + aspect * aspect).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skyborne_creation_scenes_overlay_only_forever_races() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("skyborne-scenes-{}-{nonce}", std::process::id()));
        let forever = root.join(crate::player_model_data::FOREVER_DB2_DIR);
        std::fs::create_dir_all(&forever).unwrap();
        let path = root.join("ChrRaces.csv");
        std::fs::write(&path, "ID,CreateScreenFileDataID\n1,623712\n95,123\n").unwrap();
        std::fs::write(
            forever.join("ChrRaces.csv"),
            "ID,CreateScreenFileDataID\n1,999\n95,8035354\n96,8035354\n",
        )
        .unwrap();
        let loaded = CreationSceneCatalog::load(&path).unwrap();
        std::fs::write(
            forever.join("ChrRaces.csv"),
            "ID,CreateScreenFileDataID\n95,8035354\n",
        )
        .unwrap();
        let incomplete = CreationSceneCatalog::load(&path).unwrap_err();
        std::fs::remove_dir_all(root).unwrap();
        assert_eq!(loaded.lookup(1).unwrap(), 623712);
        assert_eq!(loaded.lookup(95).unwrap(), 8035354);
        assert_eq!(loaded.lookup(96).unwrap(), 8035354);
        assert!(incomplete.contains("race 96"), "{incomplete}");
    }
}
