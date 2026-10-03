//! Bevy-free authored Warband scene records and CSV parsing.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::asset::adt::CHUNK_SIZE;
use crate::csv_util::{header_index, parse_csv_line as parse_csv_fields};

/// Build whose UI atlas and texture kit tables live under `data/db2/`.
const UI_DB2_DIR: &str = "db2/12.1.0.69933";

/// A single warband scene entry (parsed from WarbandScene.csv).
#[derive(Debug, Clone)]
pub struct WarbandSceneEntry {
    pub id: u32,
    pub name: String,
    pub description: String,
    /// WoW world position [X, Y, Z] for the camera.
    pub position: [f32; 3],
    /// WoW world look-at point [X, Y, Z].
    pub look_at: [f32; 3],
    pub map_id: u32,
    pub fov: f32,
    pub texture_kit: u32,
}

/// Character placement slot within a warband scene.
#[derive(Debug, Clone)]
pub struct WarbandScenePlacement {
    pub id: u32,
    pub scene_id: u32,
    pub slot_type: u32,
    /// WoW world position [X, Y, Z].
    pub position: [f32; 3],
    /// Rotation in degrees.
    pub rotation: f32,
    pub slot_id: u32,
}

/// Optional authored overrides for a placement in alternate warband layouts.
#[derive(Debug, Clone)]
pub struct WarbandScenePlacementOption {
    pub placement_id: u32,
    pub layout_key: u32,
    pub position: [f32; 3],
    pub orientation: f32,
    pub scale: f32,
}

#[derive(Debug)]
pub struct WarbandSceneCatalog {
    pub scenes: Vec<WarbandSceneEntry>,
    pub placements: Vec<WarbandScenePlacement>,
    pub placement_options: Vec<WarbandScenePlacementOption>,
}

/// Read original authored rows directly; never substitute an empty catalog on file errors.
pub fn read_authored_catalog(data_root: &Path) -> Result<WarbandSceneCatalog, String> {
    Ok(WarbandSceneCatalog {
        scenes: read_rows(&data_root.join("WarbandScene.csv"), parse_scene_line)?,
        placements: read_rows(
            &data_root.join("WarbandScenePlacement.csv"),
            parse_placement_line,
        )?,
        placement_options: read_rows(
            &data_root.join("WarbandScenePlacementOption.csv"),
            parse_placement_option_line,
        )?,
    })
}

/// A texture FileDataID and the normalized `[left, right, top, bottom]` crop of one atlas member.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtlasArt {
    pub fdid: u32,
    pub tex_coords: [f32; 4],
}

/// Campsite card art per `UiTextureKit` ID in `kits`, as retail draws it: the kit's `KitPrefix`
/// (`WarbandSceneInfo.textureKit`) is passed to `SetAtlas`, naming a `UiTextureAtlasElement`
/// (case-insensitive) whose base member (same committed name, not the `-2x` variant) crops its
/// `UiTextureAtlas`. Kits without a prefix or atlas element are absent from the result.
pub fn read_texture_kit_art(
    data_root: &Path,
    kits: &[u32],
) -> Result<HashMap<u32, AtlasArt>, String> {
    let dir = data_root.join(UI_DB2_DIR);
    let prefixes: HashMap<u32, String> =
        read_columns(&dir.join("UiTextureKit.csv"), &["ID", "KitPrefix"])?
            .into_iter()
            .filter_map(|row| Some((row[0].parse().ok()?, row[1].to_ascii_lowercase())))
            .filter(|(id, _)| kits.contains(id))
            .collect();
    let wanted: HashSet<&str> = prefixes.values().map(String::as_str).collect();
    let elements: HashMap<String, u32> =
        read_columns(&dir.join("UiTextureAtlasElement.csv"), &["Name", "ID"])?
            .into_iter()
            .map(|row| (row[0].to_ascii_lowercase(), row))
            .filter(|(name, _)| wanted.contains(name.as_str()))
            .filter_map(|(name, row)| Some((name, row[1].parse().ok()?)))
            .collect();
    let members = read_base_members(&dir, &elements)?;
    Ok(prefixes
        .into_iter()
        .filter_map(|(kit, prefix)| Some((kit, *members.get(elements.get(&prefix)?)?)))
        .collect())
}

/// The art each named `UiTextureAtlasElement` (case-insensitive) draws at UI scale 1, as
/// `SetAtlas(name)` crops it: the member of the same name, else the element's smallest
/// member (`-1x` rather than `-2x`). An unknown name is an error.
pub fn read_atlas_art(
    data_root: &Path,
    names: &[&str],
) -> Result<HashMap<String, AtlasArt>, String> {
    let dir = data_root.join(UI_DB2_DIR);
    let wanted: HashSet<String> = names.iter().map(|name| name.to_ascii_lowercase()).collect();
    let elements: HashMap<u32, String> =
        read_columns(&dir.join("UiTextureAtlasElement.csv"), &["Name", "ID"])?
            .into_iter()
            .filter(|row| wanted.contains(&row[0].to_ascii_lowercase()))
            .filter_map(|row| Some((row[1].parse().ok()?, row[0].to_ascii_lowercase())))
            .collect();
    let atlases = read_atlas_sizes(&dir)?;
    let columns = [
        "CommittedName",
        "UiTextureAtlasElementID",
        "UiTextureAtlasID",
        "Width",
        "CommittedLeft",
        "CommittedRight",
        "CommittedTop",
        "CommittedBottom",
    ];
    // Per element: (exact name, member width, art) of the best member so far.
    let mut best: HashMap<u32, (bool, u32, AtlasArt)> = HashMap::new();
    for row in read_columns(&dir.join("UiTextureAtlasMember.csv"), &columns)? {
        let parsed: Option<Vec<u32>> = row[1..].iter().map(|field| field.parse().ok()).collect();
        let Some([element, atlas, width, left, right, top, bottom]) =
            parsed.and_then(|values| <[u32; 7]>::try_from(values).ok())
        else {
            continue;
        };
        let Some(name) = elements.get(&element) else {
            continue;
        };
        let exact = row[0].eq_ignore_ascii_case(name);
        let art = member_art(&atlases, atlas, [left, right, top, bottom], &row[0])?;
        let better = best
            .get(&element)
            .is_none_or(|&(best_exact, best_width, _)| {
                (exact, std::cmp::Reverse(width)) > (best_exact, std::cmp::Reverse(best_width))
            });
        if better {
            best.insert(element, (exact, width, art));
        }
    }
    let by_name: HashMap<&String, AtlasArt> = elements
        .iter()
        .filter_map(|(element, name)| Some((name, best.get(element)?.2)))
        .collect();
    names
        .iter()
        .map(|name| {
            let key = name.to_ascii_lowercase();
            let art = by_name
                .get(&key)
                .copied()
                .ok_or_else(|| format!("no UiTextureAtlas member for atlas {name}"))?;
            Ok((key, art))
        })
        .collect()
}

/// `UiTextureAtlas` ID to its texture FileDataID, width and height.
fn read_atlas_sizes(dir: &Path) -> Result<HashMap<u32, [u32; 3]>, String> {
    Ok(read_columns(
        &dir.join("UiTextureAtlas.csv"),
        &["ID", "FileDataID", "AtlasWidth", "AtlasHeight"],
    )?
    .into_iter()
    .filter_map(|row| {
        Some((
            row[0].parse().ok()?,
            [
                row[1].parse().ok()?,
                row[2].parse().ok()?,
                row[3].parse().ok()?,
            ],
        ))
    })
    .collect())
}

/// A member's `[left, right, top, bottom]` pixels normalized by its atlas size.
fn member_art(
    atlases: &HashMap<u32, [u32; 3]>,
    atlas: u32,
    [left, right, top, bottom]: [u32; 4],
    member: &str,
) -> Result<AtlasArt, String> {
    let [fdid, width, height] = *atlases
        .get(&atlas)
        .ok_or_else(|| format!("UiTextureAtlasMember {member} names missing atlas {atlas}"))?;
    let (width, height) = (width as f32, height as f32);
    Ok(AtlasArt {
        fdid,
        tex_coords: [
            left as f32 / width,
            right as f32 / width,
            top as f32 / height,
            bottom as f32 / height,
        ],
    })
}

/// Base-member art keyed by atlas element ID for the named `elements`.
fn read_base_members(
    dir: &Path,
    elements: &HashMap<String, u32>,
) -> Result<HashMap<u32, AtlasArt>, String> {
    let atlases = read_atlas_sizes(dir)?;
    let columns = [
        "CommittedName",
        "UiTextureAtlasElementID",
        "UiTextureAtlasID",
        "CommittedLeft",
        "CommittedRight",
        "CommittedTop",
        "CommittedBottom",
    ];
    let mut members = HashMap::new();
    for row in read_columns(&dir.join("UiTextureAtlasMember.csv"), &columns)? {
        let Some(&element) = elements.get(&row[0].to_ascii_lowercase()) else {
            continue;
        };
        let parsed: Option<Vec<u32>> = row[1..].iter().map(|field| field.parse().ok()).collect();
        let Some([member_element, atlas, left, right, top, bottom]) =
            parsed.and_then(|values| <[u32; 6]>::try_from(values).ok())
        else {
            return Err(format!("malformed UiTextureAtlasMember row {row:?}"));
        };
        if member_element != element {
            continue;
        }
        let art = member_art(&atlases, atlas, [left, right, top, bottom], &row[0])?;
        members.insert(element, art);
    }
    Ok(members)
}

/// The named `columns` of every data row of CSV `path`, in the given order.
fn read_columns(path: &Path, columns: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let mut lines = contents.lines();
    let headers = parse_csv_fields(lines.next().unwrap_or_default());
    let indices = columns
        .iter()
        .map(|column| header_index(&headers, column, path))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(lines
        .map(parse_csv_fields)
        .map(|fields| {
            indices
                .iter()
                .map(|&index| fields.get(index).cloned().unwrap_or_default())
                .collect()
        })
        .collect())
}

fn read_rows<T>(path: &Path, parse: fn(&str) -> Option<T>) -> Result<Vec<T>, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    Ok(contents.lines().skip(1).filter_map(parse).collect())
}

impl WarbandSceneEntry {
    /// Compute the ADT tile coordinates for this scene's camera position.
    pub fn tile_coords(&self) -> (u32, u32) {
        // WarbandScene camera positions use standard WoW world axes for transforms,
        // but ADT filenames still map tile row from world Y and tile column from world X.
        let tile_size = CHUNK_SIZE * 16.0;
        let center = 32.0 * tile_size;
        let row = ((center - self.position[1]) / tile_size).floor() as i32;
        let col = ((center - self.position[0]) / tile_size).floor() as i32;
        (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
    }

    /// Map name for listfile lookup (warband maps use numeric names).
    pub fn map_name(&self) -> String {
        self.map_id.to_string()
    }
}

/// Extra tiles needed to complete authored campsite backdrops across tile borders.
pub fn supplemental_terrain_tile_coords(scene: &WarbandSceneEntry) -> Vec<(u32, u32)> {
    match scene.id {
        // Adventurer's Rest waterfall occupies the western neighboring tile.
        1 => vec![(31, 36)],
        _ => Vec::new(),
    }
}

impl WarbandScenePlacement {
    pub fn is_character_slot(&self) -> bool {
        self.slot_type == 0
    }
}

pub(crate) fn parse_scene_line(line: &str) -> Option<WarbandSceneEntry> {
    // CSV with quoted strings: Name_lang,Description_lang,Position_0..2,LookAt_0..2,ID,MapID,Fov,...,Flags,...
    let fields = parse_csv_fields(line);
    if fields.len() < 13 {
        return None;
    }
    let flags: u32 = fields[12].parse().ok()?;
    // Flags & 7 means test/internal entries (values 1, 3, 7)
    if flags & 7 != 0 {
        return None;
    }
    Some(WarbandSceneEntry {
        id: fields[8].parse().ok()?,
        name: fields[0].trim_matches('"').to_string(),
        description: fields[1].trim_matches('"').to_string(),
        position: [
            fields[2].parse().ok()?,
            fields[3].parse().ok()?,
            fields[4].parse().ok()?,
        ],
        look_at: [
            fields[5].parse().ok()?,
            fields[6].parse().ok()?,
            fields[7].parse().ok()?,
        ],
        map_id: fields[9].parse().ok()?,
        fov: fields[10].parse().ok()?,
        texture_kit: fields[15].parse().ok()?,
    })
}

pub(crate) fn parse_placement_line(line: &str) -> Option<WarbandScenePlacement> {
    // Position_0,Position_1,Position_2,ID,WarbandSceneID,SlotType,Rotation,Scale,...,SlotID,...
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 12 {
        return None;
    }
    Some(WarbandScenePlacement {
        id: fields[3].parse().ok()?,
        scene_id: fields[4].parse().ok()?,
        slot_type: fields[5].parse().ok()?,
        position: [
            fields[0].parse().ok()?,
            fields[1].parse().ok()?,
            fields[2].parse().ok()?,
        ],
        rotation: fields[6].parse().ok()?,
        slot_id: fields[11].parse().ok()?,
    })
}

pub(crate) fn parse_placement_option_line(line: &str) -> Option<WarbandScenePlacementOption> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 8 {
        return None;
    }
    Some(WarbandScenePlacementOption {
        placement_id: fields[1].parse().ok()?,
        layout_key: fields[2].parse().ok()?,
        position: [
            fields[3].parse().ok()?,
            fields[4].parse().ok()?,
            fields[5].parse().ok()?,
        ],
        orientation: fields[6].parse().ok()?,
        scale: fields[7].parse().ok()?,
    })
}
