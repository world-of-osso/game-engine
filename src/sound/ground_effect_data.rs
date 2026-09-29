use std::collections::HashMap;

use crate::footstep_data::FootstepSurface;
use crate::little_endian::read_le_u32;

const GROUND_EFFECT_LAYOUT_HASHES: &[u32] = &[0xD93D_5678, 0x3DEC_72D8];
const TERRAIN_TYPE_SOUNDS_LAYOUT_HASHES: &[u32] = &[0xB99F_5777, 0x5462_668A, 0x3AF6_B1EA];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundEffectEntry {
    pub effect_id: u32,
    pub density: u32,
    pub terrain_sound_id: u8,
}

pub fn parse_ground_effect_entries(
    bytes: &[u8],
) -> Result<HashMap<u32, GroundEffectEntry>, String> {
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

pub fn parse_terrain_type_sounds(bytes: &[u8]) -> Result<HashMap<u8, String>, String> {
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

pub fn classify_surface_from_terrain_sound_name(name: &str) -> Option<FootstepSurface> {
    let lower = name.to_ascii_lowercase();
    SURFACE_KEYWORD_RULES
        .iter()
        .find_map(|rule| matches_surface_rule(&lower, rule))
}

struct SurfaceKeywordRule {
    surface: FootstepSurface,
    keywords: &'static [&'static str],
}

const SURFACE_KEYWORD_RULES: &[SurfaceKeywordRule] = &[
    SurfaceKeywordRule {
        surface: FootstepSurface::Metal,
        keywords: &["metal", "coin"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Snow,
        keywords: &["snow"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Wood,
        keywords: &["wood"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Grass,
        keywords: &["grass", "leaf", "twig"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Water,
        keywords: &["water"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Mud,
        keywords: &["swamp", "soggy", "mud", "lava"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Carpet,
        keywords: &["carpet"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Ice,
        keywords: &["ice", "glass"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Stone,
        keywords: &["stone", "gravel", "crystalline"],
    },
    SurfaceKeywordRule {
        surface: FootstepSurface::Dirt,
        keywords: &["dirt", "sand"],
    },
];

fn matches_surface_rule(lower_name: &str, rule: &SurfaceKeywordRule) -> Option<FootstepSurface> {
    contains_any_keyword(lower_name, rule.keywords).then_some(rule.surface)
}

fn contains_any_keyword(lower_name: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| lower_name.contains(keyword))
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
    layout_hash: u32,
    section_offset: usize,
}

struct Wdc5Section {
    file_offset: usize,
    string_table_size: usize,
}
