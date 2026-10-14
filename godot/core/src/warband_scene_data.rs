//! Bevy-free authored Warband scene records and CSV parsing.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::asset::adt::CHUNK_SIZE;
use crate::csv_util::{header_index, parse_csv_line as parse_csv_fields};

/// Build whose UI atlas and texture kit tables live under `data/db2/`.
const UI_DB2_DIR: &str = "db2/active";

/// WoW Forever build whose UI atlas tables live under `data/db2/` (Wago DB2CSV exports; see
/// `scripts/import_forever_atlas.py`).
const FOREVER_UI_DB2_DIR: &str = "db2/1.60.1.69913";

/// Forever's `UiTextureAtlas.UiTextureAtlasSetID` of its `c60` re-skins of Retail atlas names.
const FOREVER_ATLAS_SET: &str = "1";

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
            .map(|row| {
                Ok((
                    crate::csv_util::parse_field(&row, 0)?,
                    row[1].to_ascii_lowercase(),
                ))
            })
            .collect::<Result<HashMap<u32, String>, String>>()?
            .into_iter()
            .filter(|(id, _)| kits.contains(id))
            .collect();
    let wanted: HashSet<&str> = prefixes.values().map(String::as_str).collect();
    let elements: HashMap<String, u32> =
        read_columns(&dir.join("UiTextureAtlasElement.csv"), &["Name", "ID"])?
            .into_iter()
            .map(|row| (row[0].to_ascii_lowercase(), row))
            .filter(|(name, _)| wanted.contains(name.as_str()))
            .map(|(name, row)| Ok((name, crate::csv_util::parse_field(&row, 1)?)))
            .collect::<Result<_, String>>()?;
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
    read_atlas_art_in(&data_root.join(UI_DB2_DIR), names, None)
}

/// The art each named Retail `UiTextureAtlasElement` draws under WoW Forever's skin: as
/// [`read_atlas_art`], over only the members on `UiTextureAtlasSetID` 1 atlases, whose
/// committed names add `-c60` (so the smallest, `-c60` rather than `-c60-2x`, wins).
/// A name without a set-1 member is an error.
pub fn read_forever_atlas_art(
    data_root: &Path,
    names: &[&str],
) -> Result<HashMap<String, AtlasArt>, String> {
    let dir = data_root.join(FOREVER_UI_DB2_DIR);
    let set_atlases: HashSet<u32> = read_columns(
        &dir.join("UiTextureAtlas.csv"),
        &["ID", "UiTextureAtlasSetID"],
    )?
    .into_iter()
    .filter(|row| row[1] == FOREVER_ATLAS_SET)
    .map(|row| crate::csv_util::parse_field(&row, 0))
    .collect::<Result<_, _>>()?;
    read_atlas_art_in(&dir, names, Some(&set_atlases))
}

/// [`read_atlas_art`] over the tables in `dir`, considering only members of `atlas_set`
/// atlases when given.
fn read_atlas_art_in(
    dir: &Path,
    names: &[&str],
    atlas_set: Option<&HashSet<u32>>,
) -> Result<HashMap<String, AtlasArt>, String> {
    let wanted: HashSet<String> = names.iter().map(|name| name.to_ascii_lowercase()).collect();
    let elements: HashMap<u32, String> =
        read_columns(&dir.join("UiTextureAtlasElement.csv"), &["Name", "ID"])?
            .into_iter()
            .filter(|row| wanted.contains(&row[0].to_ascii_lowercase()))
            .map(|row| {
                Ok((
                    crate::csv_util::parse_field(&row, 1)?,
                    row[0].to_ascii_lowercase(),
                ))
            })
            .collect::<Result<_, String>>()?;
    let atlases = read_atlas_sizes(dir)?;
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
        let parsed = row[1..]
            .iter()
            .map(|field| field.parse::<u32>().map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let [element, atlas, width, left, right, top, bottom] = <[u32; 7]>::try_from(parsed)
            .map_err(|_| "malformed atlas member columns".to_string())?;
        let Some(name) = elements.get(&element) else {
            continue;
        };
        if atlas_set.is_some_and(|set| !set.contains(&atlas)) {
            continue;
        }
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
    .map(|row| {
        Ok((
            crate::csv_util::parse_field(&row, 0)?,
            [
                crate::csv_util::parse_field(&row, 1)?,
                crate::csv_util::parse_field(&row, 2)?,
                crate::csv_util::parse_field(&row, 3)?,
            ],
        ))
    })
    .collect::<Result<_, String>>()?)
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
        let parsed = row[1..]
            .iter()
            .map(|field| field.parse::<u32>().map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let [member_element, atlas, left, right, top, bottom] = <[u32; 6]>::try_from(parsed)
            .map_err(|_| "malformed atlas member columns".to_string())?;
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
    lines
        .enumerate()
        .map(|(line, text)| {
            let fields = parse_csv_fields(text);
            columns
                .iter()
                .zip(&indices)
                .map(|(column, &index)| {
                    let value = fields.get(index).ok_or_else(|| {
                        format!("{}:{}: missing {column}", path.display(), line + 2)
                    })?;
                    if !matches!(*column, "Name" | "KitPrefix" | "CommittedName") {
                        value.parse::<u32>().map_err(|error| {
                            format!(
                                "{}:{}: {column}={value:?}: {error}",
                                path.display(),
                                line + 2
                            )
                        })?;
                    }
                    Ok(value.clone())
                })
                .collect()
        })
        .collect()
}

fn read_rows<T>(
    path: &Path,
    parse: fn(&crate::csv_util::Db2Row<'_>) -> Result<Option<T>, String>,
) -> Result<Vec<T>, String> {
    let mut rows = Vec::new();
    crate::csv_util::read_db2_rows(path, |row| {
        if let Some(value) = parse(row)? {
            rows.push(value);
        }
        Ok(())
    })?;
    Ok(rows)
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

impl WarbandScenePlacement {
    pub fn is_character_slot(&self) -> bool {
        self.slot_type == 0
    }
}

fn parse_scene_line(
    row: &crate::csv_util::Db2Row<'_>,
) -> Result<Option<WarbandSceneEntry>, String> {
    let flags: u32 = row.parse("Flags")?;
    let scene = WarbandSceneEntry {
        id: row.parse("ID")?,
        name: row.field("Name_lang")?.to_string(),
        description: row.field("Description_lang")?.to_string(),
        position: [
            row.parse("Position_0")?,
            row.parse("Position_1")?,
            row.parse("Position_2")?,
        ],
        look_at: [
            row.parse("LookAt_0")?,
            row.parse("LookAt_1")?,
            row.parse("LookAt_2")?,
        ],
        map_id: row.parse("MapID")?,
        fov: row.parse("Fov")?,
        texture_kit: row.parse("TextureKit")?,
    };
    // Authored flags mark test/internal scenes; parse them before intentional filtering.
    Ok((flags & 7 == 0).then_some(scene))
}

fn parse_placement_line(
    row: &crate::csv_util::Db2Row<'_>,
) -> Result<Option<WarbandScenePlacement>, String> {
    Ok(Some(WarbandScenePlacement {
        id: row.parse("ID")?,
        scene_id: row.parse("WarbandSceneID")?,
        slot_type: row.parse("SlotType")?,
        position: [
            row.parse("Position_0")?,
            row.parse("Position_1")?,
            row.parse("Position_2")?,
        ],
        rotation: row.parse("Rotation")?,
        slot_id: row.parse("SlotID")?,
    }))
}

fn parse_placement_option_line(
    row: &crate::csv_util::Db2Row<'_>,
) -> Result<Option<WarbandScenePlacementOption>, String> {
    Ok(Some(WarbandScenePlacementOption {
        placement_id: row.parse("WarbandScenePlacementID")?,
        layout_key: row.parse("LayoutKey")?,
        position: [
            row.parse("Position_0")?,
            row.parse("Position_1")?,
            row.parse("Position_2")?,
        ],
        orientation: row.parse("Orientation")?,
        scale: row.parse("Scale")?,
    }))
}

#[cfg(test)]
mod db2_header_tests {
    use super::*;
    #[test]
    fn db2_header_atlas_rejects_invalid_canonical_row() {
        let fixture = crate::db2_test_support::ExportFixture::copy("UiTextureAtlas", Some("ID"));
        let error = read_atlas_sizes(fixture.0.parent().unwrap()).unwrap_err();
        assert!(error.contains("UiTextureAtlas.csv:2:"), "{error}");
    }
}
