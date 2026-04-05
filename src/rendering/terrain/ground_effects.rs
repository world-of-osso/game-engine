use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::little_endian::read_le_u32;
use crate::sound_footsteps::FootstepSurface;

const GROUND_EFFECT_TEXTURE_DB2_FDID: u32 = 1_308_499;
const TERRAIN_TYPE_SOUNDS_DB2_FDID: u32 = 1_284_822;
const GROUND_EFFECT_LAYOUT_HASHES: &[u32] = &[0xD93D_5678, 0x3DEC_72D8];
const TERRAIN_TYPE_SOUNDS_LAYOUT_HASHES: &[u32] = &[0xB99F_5777, 0x5462_668A, 0x3AF6_B1EA];

static GROUND_EFFECTS: OnceLock<HashMap<u32, GroundEffectEntry>> = OnceLock::new();
static TERRAIN_SOUND_SURFACES: OnceLock<HashMap<u8, FootstepSurface>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundEffectEntry {
    pub effect_id: u32,
    pub density: u32,
    pub terrain_sound_id: u8,
}

pub fn resolve_ground_effect(effect_id: u32) -> Option<GroundEffectEntry> {
    cached_ground_effects().get(&effect_id).copied()
}

pub fn resolve_ground_effect_surface(effect_id: u32) -> Option<FootstepSurface> {
    let effect = resolve_ground_effect(effect_id)?;
    cached_terrain_sound_surfaces()
        .get(&effect.terrain_sound_id)
        .copied()
}

fn cached_ground_effects() -> &'static HashMap<u32, GroundEffectEntry> {
    GROUND_EFFECTS.get_or_init(load_ground_effects)
}

fn cached_terrain_sound_surfaces() -> &'static HashMap<u8, FootstepSurface> {
    TERRAIN_SOUND_SURFACES.get_or_init(load_terrain_sound_surfaces)
}

fn load_ground_effects() -> HashMap<u32, GroundEffectEntry> {
    let Some(path) = ensure_db2_path(
        GROUND_EFFECT_TEXTURE_DB2_FDID,
        Path::new("data/dbfilesclient/1308499.db2"),
    ) else {
        return HashMap::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return HashMap::new();
    };
    parse_ground_effect_entries(&bytes).unwrap_or_default()
}

fn load_terrain_sound_surfaces() -> HashMap<u8, FootstepSurface> {
    let Some(path) = ensure_db2_path(
        TERRAIN_TYPE_SOUNDS_DB2_FDID,
        Path::new("data/dbfilesclient/1284822.db2"),
    ) else {
        return HashMap::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return HashMap::new();
    };
    parse_terrain_type_sounds(&bytes)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(terrain_sound_id, name)| {
            classify_surface_from_terrain_sound_name(&name)
                .map(|surface| (terrain_sound_id, surface))
        })
        .collect()
}

fn ensure_db2_path(fdid: u32, path: &Path) -> Option<std::path::PathBuf> {
    if path.exists() {
        return Some(path.to_path_buf());
    }
    crate::asset::asset_cache::file_at_path(fdid, path)
}

fn parse_ground_effect_entries(bytes: &[u8]) -> Result<HashMap<u32, GroundEffectEntry>, String> {
    let table = ParsedFixedLayoutDb2::parse_any_layout(bytes, GROUND_EFFECT_LAYOUT_HASHES)?;
    if table.record_size != 5 {
        return Err(format!(
            "unexpected GroundEffectTexture record size {}, expected 5",
            table.record_size
        ));
    }

    let mut entries = HashMap::with_capacity(table.record_count);
    for row_index in 0..table.record_count {
        let record_offset = table.record_offset(row_index)?;
        let effect_id = table.row_id(row_index)?;
        entries.insert(
            effect_id,
            GroundEffectEntry {
                effect_id,
                density: read_le_u32(bytes, record_offset),
                terrain_sound_id: bytes
                    .get(record_offset + 4)
                    .copied()
                    .ok_or_else(|| "GroundEffectTexture sound byte out of bounds".to_string())?,
            },
        );
    }
    Ok(entries)
}

fn parse_terrain_type_sounds(bytes: &[u8]) -> Result<HashMap<u8, String>, String> {
    let table = ParsedFixedLayoutDb2::parse_any_layout(bytes, TERRAIN_TYPE_SOUNDS_LAYOUT_HASHES)?;
    if table.record_size != 4 {
        return Err(format!(
            "unexpected TerrainTypeSounds record size {}, expected 4",
            table.record_size
        ));
    }

    let strings = read_c_string_block(bytes, table.string_block_offset(), table.string_table_size)?;
    if strings.len() != table.record_count {
        return Err(format!(
            "TerrainTypeSounds string count {} does not match record count {}",
            strings.len(),
            table.record_count
        ));
    }

    let mut map = HashMap::with_capacity(table.record_count);
    for (row_index, name) in strings.into_iter().enumerate() {
        let terrain_sound_id = table.row_id(row_index)?;
        let terrain_sound_id = u8::try_from(terrain_sound_id).map_err(|_| {
            format!("TerrainTypeSounds row id {terrain_sound_id} does not fit in u8")
        })?;
        map.insert(terrain_sound_id, name);
    }
    Ok(map)
}

fn read_c_string_block(
    bytes: &[u8],
    offset: usize,
    string_table_size: usize,
) -> Result<Vec<String>, String> {
    let end = offset
        .checked_add(string_table_size)
        .ok_or_else(|| "string block end overflow".to_string())?;
    let block = bytes
        .get(offset..end)
        .ok_or_else(|| "string block out of bounds".to_string())?;
    let mut strings = Vec::new();
    for value in block.split(|byte| *byte == 0) {
        if value.is_empty() {
            continue;
        }
        strings.push(
            std::str::from_utf8(value)
                .map_err(|err| format!("TerrainTypeSounds invalid UTF-8: {err}"))?
                .to_string(),
        );
    }
    Ok(strings)
}

fn classify_surface_from_terrain_sound_name(name: &str) -> Option<FootstepSurface> {
    let lower = name.to_ascii_lowercase();
    if lower.contains("metal") || lower.contains("coin") {
        return Some(FootstepSurface::Metal);
    }
    if lower.contains("snow") {
        return Some(FootstepSurface::Snow);
    }
    if lower.contains("wood") {
        return Some(FootstepSurface::Wood);
    }
    if lower.contains("grass") || lower.contains("leaf") || lower.contains("twig") {
        return Some(FootstepSurface::Grass);
    }
    if lower.contains("water") {
        return Some(FootstepSurface::Water);
    }
    if lower.contains("swamp")
        || lower.contains("soggy")
        || lower.contains("mud")
        || lower.contains("lava")
    {
        return Some(FootstepSurface::Mud);
    }
    if lower.contains("carpet") {
        return Some(FootstepSurface::Carpet);
    }
    if lower.contains("ice") || lower.contains("glass") {
        return Some(FootstepSurface::Ice);
    }
    if lower.contains("stone") || lower.contains("gravel") || lower.contains("crystalline") {
        return Some(FootstepSurface::Stone);
    }
    if lower.contains("dirt") || lower.contains("sand") {
        return Some(FootstepSurface::Dirt);
    }
    None
}

struct ParsedFixedLayoutDb2 {
    bytes: Vec<u8>,
    record_count: usize,
    record_size: usize,
    file_offset: usize,
    id_list_offset: usize,
    string_table_size: usize,
}

impl ParsedFixedLayoutDb2 {
    fn parse_any_layout(bytes: &[u8], layout_hashes: &[u32]) -> Result<Self, String> {
        let header = parse_wdc5_header(bytes)?;
        if !layout_hashes.contains(&header.layout_hash) {
            return Err(format!(
                "unexpected WDC5 layout hash 0x{:08X}",
                header.layout_hash
            ));
        }
        let section = parse_wdc5_section(bytes, header.section_offset)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            record_count: header.record_count,
            record_size: header.record_size,
            file_offset: section.file_offset,
            id_list_offset: section.file_offset
                + header.record_count * header.record_size
                + section.string_table_size,
            string_table_size: section.string_table_size,
        })
    }

    fn record_offset(&self, row_index: usize) -> Result<usize, String> {
        if row_index >= self.record_count {
            return Err(format!("row index {row_index} out of bounds"));
        }
        Ok(self.file_offset + row_index * self.record_size)
    }

    fn row_id(&self, row_index: usize) -> Result<u32, String> {
        if row_index >= self.record_count {
            return Err(format!("row index {row_index} out of bounds"));
        }
        Ok(read_le_u32(
            &self.bytes,
            self.id_list_offset + row_index * 4,
        ))
    }

    fn string_block_offset(&self) -> usize {
        self.file_offset + self.record_count * self.record_size
    }
}

fn parse_wdc5_header(bytes: &[u8]) -> Result<Wdc5Header, String> {
    if bytes.get(0..4) != Some(b"WDC5") {
        return Err("expected WDC5 DB2".to_string());
    }
    let offset = 136usize;
    Ok(Wdc5Header {
        record_count: read_le_u32(bytes, offset) as usize,
        record_size: read_le_u32(bytes, offset + 8) as usize,
        string_table_size: read_le_u32(bytes, offset + 12) as usize,
        layout_hash: read_le_u32(bytes, offset + 20),
        section_offset: offset + 68,
    })
}

fn parse_wdc5_section(bytes: &[u8], offset: usize) -> Result<Wdc5Section, String> {
    if bytes.get(offset..offset + 40).is_none() {
        return Err("truncated WDC5 section header".to_string());
    }
    Ok(Wdc5Section {
        file_offset: read_le_u32(bytes, offset + 8) as usize,
        string_table_size: read_le_u32(bytes, offset + 16) as usize,
    })
}

struct Wdc5Header {
    record_count: usize,
    record_size: usize,
    string_table_size: usize,
    layout_hash: u32,
    section_offset: usize,
}

struct Wdc5Section {
    file_offset: usize,
    string_table_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_effect_entries_read_density_and_sound() {
        let bytes = test_wdc5_bytes(
            GROUND_EFFECT_LAYOUT_HASHES[0],
            5,
            &[7, 0, 0, 0, 3],
            &[],
            &[42],
        );

        let entries =
            parse_ground_effect_entries(&bytes).expect("expected GroundEffectTexture DB2 to parse");

        assert_eq!(
            entries.get(&42),
            Some(&GroundEffectEntry {
                effect_id: 42,
                density: 7,
                terrain_sound_id: 3,
            })
        );
    }

    #[test]
    fn terrain_sound_name_maps_to_expected_surface() {
        assert_eq!(
            classify_surface_from_terrain_sound_name("Stone"),
            Some(FootstepSurface::Stone)
        );
        assert_eq!(
            classify_surface_from_terrain_sound_name("Mud / Lava"),
            Some(FootstepSurface::Mud)
        );
        assert_eq!(
            classify_surface_from_terrain_sound_name("Metal Grate"),
            Some(FootstepSurface::Metal)
        );
        assert_eq!(
            classify_surface_from_terrain_sound_name("Twiggy"),
            Some(FootstepSurface::Grass)
        );
    }

    fn test_wdc5_bytes(
        layout_hash: u32,
        record_size: u32,
        record_bytes: &[u8],
        string_bytes: &[u8],
        row_ids: &[u32],
    ) -> Vec<u8> {
        let header_offset = 136usize;
        let section_offset = header_offset + 68;
        let file_offset = section_offset + 40;
        let record_count = row_ids.len() as u32;
        let string_table_size = string_bytes.len() as u32;
        let total_size = file_offset + record_bytes.len() + string_bytes.len() + row_ids.len() * 4;
        let mut bytes = vec![0u8; total_size];

        bytes[0..4].copy_from_slice(b"WDC5");
        bytes[header_offset..header_offset + 4].copy_from_slice(&record_count.to_le_bytes());
        bytes[header_offset + 8..header_offset + 12].copy_from_slice(&record_size.to_le_bytes());
        bytes[header_offset + 12..header_offset + 16]
            .copy_from_slice(&string_table_size.to_le_bytes());
        bytes[header_offset + 20..header_offset + 24].copy_from_slice(&layout_hash.to_le_bytes());

        bytes[section_offset + 8..section_offset + 12]
            .copy_from_slice(&(file_offset as u32).to_le_bytes());
        bytes[section_offset + 16..section_offset + 20]
            .copy_from_slice(&string_table_size.to_le_bytes());

        bytes[file_offset..file_offset + record_bytes.len()].copy_from_slice(record_bytes);
        let string_offset = file_offset + record_bytes.len();
        bytes[string_offset..string_offset + string_bytes.len()].copy_from_slice(string_bytes);

        let ids_offset = string_offset + string_bytes.len();
        for (index, row_id) in row_ids.iter().enumerate() {
            let offset = ids_offset + index * 4;
            bytes[offset..offset + 4].copy_from_slice(&row_id.to_le_bytes());
        }

        bytes
    }
}
