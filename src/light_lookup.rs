use std::path::Path;
use std::sync::OnceLock;

const LIGHT_PARAMS_DB2_FDID: u32 = 1_334_669;
const LIGHT_SKYBOX_DB2_FDID: u32 = 1_308_501;
const LIGHT_PARAMS_LAYOUT_HASH: u32 = 0xCAE3_94E7;
const LIGHT_SKYBOX_LAYOUT_HASHES: &[u32] = &[0x9D49_56FF, 0x407F_EBCF, 0xD466_A5C2];
const LIGHT_PARAMS_SKYBOX_FIELD_INDEX: usize = 3;
const LIGHT_SKYBOX_FDID_FIELD_INDEX: usize = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct LightEntry {
    pub id: u32,
    pub map_id: u32,
    pub position: [f32; 3],
    pub falloff_end: f32,
    pub light_params_ids: [u32; 8],
}

static LIGHTS: OnceLock<Vec<LightEntry>> = OnceLock::new();
static LIGHT_PARAMS_SKYBOX_IDS: OnceLock<Vec<(u32, u32)>> = OnceLock::new();
static LIGHT_SKYBOX_FDIDS: OnceLock<Vec<(u32, u32)>> = OnceLock::new();

pub fn resolve_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    resolve_light_params_ids(map_id, wow_position)?
        .into_iter()
        .find(|id| *id != 0)
}

pub fn resolve_skybox_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    resolve_light_params_ids(map_id, wow_position)?
        .into_iter()
        .find(|id| *id != 0 && resolve_light_skybox_id(*id).is_some())
}

pub fn resolve_light_skybox_id(light_params_id: u32) -> Option<u32> {
    cached_light_params_skybox_ids()
        .iter()
        .find(|(id, _)| *id == light_params_id)
        .map(|(_, skybox_id)| *skybox_id)
        .filter(|skybox_id| *skybox_id != 0)
}

pub fn resolve_light_skybox_fdid(light_skybox_id: u32) -> Option<u32> {
    cached_light_skybox_fdids()
        .iter()
        .find(|(id, _)| *id == light_skybox_id)
        .map(|(_, fdid)| *fdid)
        .filter(|fdid| *fdid != 0)
}

pub fn resolve_light_skybox_wow_path(light_skybox_id: u32) -> Option<&'static str> {
    let fdid = resolve_light_skybox_fdid(light_skybox_id)?;
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    wow_path.ends_with(".m2").then_some(wow_path)
}

fn cached_light_params_skybox_ids() -> &'static [(u32, u32)] {
    LIGHT_PARAMS_SKYBOX_IDS.get_or_init(load_light_params_skybox_ids)
}

fn cached_light_skybox_fdids() -> &'static [(u32, u32)] {
    LIGHT_SKYBOX_FDIDS.get_or_init(load_light_skybox_fdids)
}

fn cached_lights() -> &'static [LightEntry] {
    LIGHTS.get_or_init(|| load_lights(Path::new("data/Light.csv")))
}

fn resolve_light_params_ids(map_id: u32, wow_position: [f32; 3]) -> Option<[u32; 8]> {
    select_light_row(map_id, wow_position)
        .or_else(|| select_light_row(0, wow_position))
        .map(|row| row.light_params_ids)
}

fn select_light_row(map_id: u32, wow_position: [f32; 3]) -> Option<&'static LightEntry> {
    cached_lights()
        .iter()
        .filter(|row| row.map_id == map_id)
        .filter_map(|row| score_light_row(row, wow_position).map(|score| (score, row)))
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .map(|(_, row)| row)
}

fn load_light_params_skybox_ids() -> Vec<(u32, u32)> {
    let Some(path) = ensure_db2_path(
        LIGHT_PARAMS_DB2_FDID,
        Path::new("data/dbfilesclient/1334669.db2"),
    ) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    let Ok(db2) = ParsedWdc5Db2::parse(&bytes, LIGHT_PARAMS_LAYOUT_HASH) else {
        return Vec::new();
    };
    db2.rows()
        .into_iter()
        .map(|row_index| {
            (
                db2.row_id(row_index),
                db2.decode_field(row_index, LIGHT_PARAMS_SKYBOX_FIELD_INDEX),
            )
        })
        .collect()
}

fn load_light_skybox_fdids() -> Vec<(u32, u32)> {
    let Some(path) = ensure_db2_path(
        LIGHT_SKYBOX_DB2_FDID,
        Path::new("data/dbfilesclient/1308501.db2"),
    ) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    let Ok(db2) = ParsedWdc5Db2::parse_any_layout(&bytes, LIGHT_SKYBOX_LAYOUT_HASHES) else {
        return Vec::new();
    };
    db2.rows()
        .into_iter()
        .map(|row_index| {
            (
                db2.row_id(row_index),
                db2.decode_field(row_index, LIGHT_SKYBOX_FDID_FIELD_INDEX),
            )
        })
        .collect()
}

fn ensure_db2_path(fdid: u32, path: &Path) -> Option<std::path::PathBuf> {
    if path.exists() {
        return Some(path.to_path_buf());
    }
    crate::asset::asset_cache::file_at_path(fdid, path)
}

fn score_light_row(row: &LightEntry, wow_position: [f32; 3]) -> Option<f32> {
    if row.position == [0.0, 0.0, 0.0] {
        return Some(f32::MAX / 4.0);
    }
    let dx = row.position[0] - wow_position[0];
    let dy = row.position[1] - wow_position[1];
    let dz = row.position[2] - wow_position[2];
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    if row.falloff_end > 0.0 && distance > row.falloff_end {
        return None;
    }
    Some(distance)
}

fn load_lights(path: &Path) -> Vec<LightEntry> {
    let Ok(data) = std::fs::read_to_string(path) else {
        eprintln!("Light.csv not found at {}", path.display());
        return Vec::new();
    };
    let mut rows = Vec::new();
    for line in data.lines().skip(1) {
        if let Some(row) = parse_light_line(line) {
            rows.push(row);
        }
    }
    rows
}

fn parse_light_line(line: &str) -> Option<LightEntry> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 15 {
        return None;
    }
    Some(LightEntry {
        id: fields[0].parse().ok()?,
        position: [
            fields[1].parse().ok()?,
            fields[2].parse().ok()?,
            fields[3].parse().ok()?,
        ],
        falloff_end: fields[5].parse().ok()?,
        map_id: fields[6].parse().ok()?,
        light_params_ids: [
            fields[7].parse().ok()?,
            fields[8].parse().ok()?,
            fields[9].parse().ok()?,
            fields[10].parse().ok()?,
            fields[11].parse().ok()?,
            fields[12].parse().ok()?,
            fields[13].parse().ok()?,
            fields[14].parse().ok()?,
        ],
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct Wdc5FieldStorage {
    offset_bits: u16,
    size_bits: u16,
    additional_data_size: u32,
    storage_type: u32,
}

struct ParsedWdc5Db2<'a> {
    bytes: &'a [u8],
    record_size: usize,
    sections: Vec<Wdc5Section>,
    pallet_offset: usize,
    pallet_offsets: Vec<usize>,
    common_offset: usize,
    common_offsets: Vec<usize>,
    fields: Vec<Wdc5FieldStorage>,
}

impl<'a> ParsedWdc5Db2<'a> {
    fn parse(bytes: &'a [u8], expected_layout_hash: u32) -> Result<Self, String> {
        let header = parse_wdc5_header(bytes)?;
        if header.layout_hash != expected_layout_hash {
            return Err(format!(
                "unexpected WDC5 layout hash 0x{:08X}, expected 0x{:08X}",
                header.layout_hash, expected_layout_hash
            ));
        }
        Self::from_header(bytes, &header)
    }

    fn parse_any_layout(bytes: &'a [u8], expected_layout_hashes: &[u32]) -> Result<Self, String> {
        let header = parse_wdc5_header(bytes)?;
        if !expected_layout_hashes.contains(&header.layout_hash) {
            return Err(format!(
                "unexpected WDC5 layout hash 0x{:08X}",
                header.layout_hash
            ));
        }
        Self::from_header(bytes, &header)
    }

    fn from_header(bytes: &'a [u8], header: &Wdc5Header) -> Result<Self, String> {
        let sections = parse_wdc5_sections(
            bytes,
            header.section_offset,
            header.section_count,
            header.record_size,
        )?;
        let fields_offset =
            header.section_offset + header.section_count * 40 + header.total_field_count * 4;
        let fields = parse_wdc5_field_storage(bytes, fields_offset, header.total_field_count)?;
        let pallet_offset = fields_offset + header.total_field_count * 24;
        Ok(Self {
            bytes,
            record_size: header.record_size,
            sections,
            pallet_offset,
            pallet_offsets: wdc5_pallet_offsets(&fields),
            common_offset: pallet_offset + header.pallet_data_size,
            common_offsets: wdc5_common_offsets(&fields),
            fields,
        })
    }

    fn rows(&self) -> Vec<Wdc5RowRef> {
        let mut rows = Vec::new();
        for (section_index, section) in self.sections.iter().enumerate() {
            for row_index in 0..section.record_count {
                rows.push(Wdc5RowRef {
                    section_index,
                    row_index,
                });
            }
        }
        rows
    }

    fn row_id(&self, row: Wdc5RowRef) -> u32 {
        let section = &self.sections[row.section_index];
        read_le_u32(self.bytes, section.id_list_offset + row.row_index * 4)
    }

    fn decode_field(&self, row: Wdc5RowRef, field_index: usize) -> u32 {
        let field = self.fields[field_index];
        if field.size_bits == 0 {
            return decode_wdc5_storage_value(self, field_index, row, 0);
        }
        let section = &self.sections[row.section_index];
        let record_offset = section.file_offset + row.row_index * self.record_size;
        let lo = field.offset_bits as usize / 8;
        let hi = (field.offset_bits as usize + field.size_bits as usize - 1) / 8;
        let raw = read_le(self.bytes, record_offset + lo, hi - lo + 1);
        let mask = if field.size_bits == 32 {
            u64::MAX
        } else {
            (1u64 << field.size_bits as usize) - 1
        };
        let value = (raw >> (field.offset_bits as usize % 8)) & mask;
        decode_wdc5_storage_value(self, field_index, row, value as usize)
    }
}

fn decode_wdc5_storage_value(
    parsed: &ParsedWdc5Db2<'_>,
    field_index: usize,
    row: Wdc5RowRef,
    value_index: usize,
) -> u32 {
    let field = parsed.fields[field_index];
    match field.storage_type {
        0 | 1 | 5 => value_index as u32,
        2 => parsed_common_value(parsed, field_index, row),
        3 | 4 => read_le_u32(
            parsed.bytes,
            parsed.pallet_offset + parsed.pallet_offsets[field_index] + value_index * 4,
        ),
        other => panic!("unsupported WDC5 storage type {other}"),
    }
}

fn parsed_common_value(parsed: &ParsedWdc5Db2<'_>, field_index: usize, row: Wdc5RowRef) -> u32 {
    let field = parsed.fields[field_index];
    let row_id = parsed.row_id(row);
    let start = parsed.common_offset + parsed.common_offsets[field_index];
    let end = start + field.additional_data_size as usize;
    let mut cursor = start;
    while cursor < end {
        if read_le_u32(parsed.bytes, cursor) == row_id {
            return read_le_u32(parsed.bytes, cursor + 4);
        }
        cursor += 8;
    }
    0
}

fn wdc5_pallet_offsets(fields: &[Wdc5FieldStorage]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(fields.len());
    let mut next = 0usize;
    for field in fields {
        offsets.push(next);
        if matches!(field.storage_type, 3 | 4) {
            next += field.additional_data_size as usize;
        }
    }
    offsets
}

fn wdc5_common_offsets(fields: &[Wdc5FieldStorage]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(fields.len());
    let mut next = 0usize;
    for field in fields {
        offsets.push(next);
        if field.storage_type == 2 {
            next += field.additional_data_size as usize;
        }
    }
    offsets
}

struct Wdc5Header {
    record_size: usize,
    total_field_count: usize,
    section_offset: usize,
    layout_hash: u32,
    pallet_data_size: usize,
    section_count: usize,
}

fn parse_wdc5_header(bytes: &[u8]) -> Result<Wdc5Header, String> {
    if bytes.get(0..4) != Some(b"WDC5") {
        return Err("expected WDC5 DB2".to_string());
    }
    let offset = 136usize;
    Ok(Wdc5Header {
        record_size: read_le_u32(bytes, offset + 8) as usize,
        layout_hash: read_le_u32(bytes, offset + 20),
        total_field_count: read_le_u32(bytes, offset + 40) as usize,
        pallet_data_size: read_le_u32(bytes, offset + 56) as usize,
        section_count: read_le_u32(bytes, offset + 64) as usize,
        section_offset: offset + 68,
    })
}

#[derive(Clone, Copy)]
struct Wdc5RowRef {
    section_index: usize,
    row_index: usize,
}

struct Wdc5Section {
    file_offset: usize,
    record_count: usize,
    string_table_size: usize,
    id_list_offset: usize,
}

fn parse_wdc5_sections(
    bytes: &[u8],
    offset: usize,
    section_count: usize,
    record_size: usize,
) -> Result<Vec<Wdc5Section>, String> {
    let mut sections = Vec::with_capacity(section_count);
    for index in 0..section_count {
        let base = offset + index * 40;
        if bytes.len() < base + 40 {
            return Err("truncated WDC5 section header".to_string());
        }
        let file_offset = read_le_u32(bytes, base + 8) as usize;
        let record_count = read_le_u32(bytes, base + 12) as usize;
        let string_table_size = read_le_u32(bytes, base + 16) as usize;
        let _id_list_size = read_le_u32(bytes, base + 24) as usize;
        sections.push(Wdc5Section {
            file_offset,
            record_count,
            string_table_size,
            id_list_offset: file_offset + record_count * record_size + string_table_size,
        });
    }
    Ok(sections)
}

fn parse_wdc5_field_storage(
    bytes: &[u8],
    offset: usize,
    field_count: usize,
) -> Result<Vec<Wdc5FieldStorage>, String> {
    let mut fields = Vec::with_capacity(field_count);
    for index in 0..field_count {
        let base = offset + index * 24;
        if bytes.len() < base + 24 {
            return Err("truncated WDC5 field storage info".to_string());
        }
        fields.push(Wdc5FieldStorage {
            offset_bits: read_le_u16(bytes, base),
            size_bits: read_le_u16(bytes, base + 2),
            additional_data_size: read_le_u32(bytes, base + 4),
            storage_type: read_le_u32(bytes, base + 8),
        });
    }
    Ok(fields)
}

fn read_le(bytes: &[u8], offset: usize, len: usize) -> u64 {
    let mut value = 0u64;
    for (index, byte) in bytes[offset..offset + len].iter().enumerate() {
        value |= (*byte as u64) << (index * 8);
    }
    value
}

fn read_le_u16(bytes: &[u8], offset: usize) -> u16 {
    read_le(bytes, offset, 2) as u16
}

fn read_le_u32(bytes: &[u8], offset: usize) -> u32 {
    read_le(bytes, offset, 4) as u32
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_light_params_id, resolve_light_skybox_fdid, resolve_light_skybox_id,
        resolve_light_skybox_wow_path, resolve_skybox_light_params_id,
    };

    #[test]
    fn authored_light_lookup_matches_ohnahran_scene() {
        let scene = crate::warband_scene::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 4)
            .expect("known scene");

        let params = resolve_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(6577));
    }

    #[test]
    fn authored_light_lookup_matches_freywold_scene() {
        let scene = crate::warband_scene::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 7)
            .expect("known scene");

        let params = resolve_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(5615));
    }

    #[test]
    fn authored_skybox_params_lookup_uses_alternate_param_slots() {
        let scene = crate::warband_scene::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 4)
            .expect("known scene");

        let params = resolve_skybox_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(5119));
    }

    #[test]
    fn authored_skybox_params_lookup_can_fall_back_to_global_light_rows() {
        let scene = crate::warband_scene::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 1)
            .expect("known scene");

        let params = resolve_skybox_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(3));
    }

    #[test]
    fn primary_light_params_id_can_be_missing_while_alternate_slot_resolves_skybox() {
        let scene = crate::warband_scene::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 4)
            .expect("known scene");

        assert_eq!(
            resolve_light_params_id(scene.map_id, scene.position),
            Some(6577)
        );
        assert_eq!(resolve_light_skybox_id(6577), None);
        assert_eq!(
            resolve_skybox_light_params_id(scene.map_id, scene.position),
            Some(5119)
        );
    }

    #[test]
    fn primary_light_params_id_can_exist_but_later_slot_supplies_the_skybox() {
        let scene = crate::warband_scene::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 25)
            .expect("known scene");

        assert_eq!(
            resolve_light_params_id(scene.map_id, scene.position),
            Some(6412)
        );
        assert_eq!(resolve_light_skybox_id(6412), None);
        assert_eq!(
            resolve_skybox_light_params_id(scene.map_id, scene.position),
            Some(3)
        );
    }

    #[test]
    fn authored_light_params_rows_resolve_expected_skybox_ids() {
        assert_eq!(resolve_light_skybox_id(5615), Some(653));
        assert_eq!(resolve_light_skybox_id(6577), None);
    }

    #[test]
    fn authored_light_skybox_rows_resolve_expected_fdids() {
        assert_eq!(resolve_light_skybox_fdid(653), Some(5_412_968));
    }

    #[test]
    fn authored_light_skybox_rows_resolve_expected_wow_paths() {
        assert_eq!(
            resolve_light_skybox_wow_path(653),
            Some("environments/stars/11xp_cloudsky01.m2")
        );
    }
}
