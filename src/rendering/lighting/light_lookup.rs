use std::path::Path;
use std::sync::OnceLock;

use crate::little_endian::{read_le, read_le_u16, read_le_u32};

#[path = "light_lookup_cache.rs"]
mod cache;

const LIGHT_PARAMS_DB2_FDID: u32 = 1_334_669;
const LIGHT_SKYBOX_DB2_FDID: u32 = 1_308_501;
const LIGHT_PARAMS_LAYOUT_HASH: u32 = 0xCAE3_94E7;
const LIGHT_SKYBOX_LAYOUT_HASHES: &[u32] = &[0x9D49_56FF, 0x407F_EBCF, 0xD466_A5C2];
const LIGHT_PARAMS_SKYBOX_FIELD_INDEX: usize = 3;
const LIGHT_SKYBOX_FLAGS_FIELD_INDEX: usize = 1;
const LIGHT_SKYBOX_FDID_FIELD_INDEX: usize = 2;
const OUTDOOR_SKYBOX_LIGHT_PARAM_SLOTS: std::ops::Range<usize> = 0..4;

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
static LIGHT_SKYBOX_METADATA: OnceLock<Vec<(u32, LightSkyboxMetadata)>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LightSkyboxFlags(u32);

impl LightSkyboxFlags {
    pub const FULL_DAY_SKYBOX: Self = Self(1 << 0);
    pub const COMBINE_PROCEDURAL_AND_SKYBOX: Self = Self(1 << 1);
    pub const PROCEDURAL_FOG_COLOR_BLEND: Self = Self(1 << 2);
    pub const FORCE_SUNSHAFTS: Self = Self(1 << 3);
    pub const DISABLE_USE_SUN_FOG_COLOR: Self = Self(1 << 4);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for LightSkyboxFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for LightSkyboxFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LightSkyboxMetadata {
    fdid: u32,
    flags: LightSkyboxFlags,
}

pub fn map_name_to_id(map_name: &str) -> Option<u32> {
    let normalized = normalize_map_name(map_name);
    if let Ok(id) = normalized.parse() {
        return Some(id);
    }
    match normalized.as_str() {
        "azeroth" => Some(0),
        "kalimdor" => Some(1),
        "expansion01" | "outland" => Some(530),
        "northrend" => Some(571),
        "deepholm" => Some(646),
        "pandaria" => Some(870),
        "draenor" => Some(1116),
        "brokenisles" | "brokenshorecontinent" => Some(1220),
        "argus" => Some(1669),
        "kultiras" | "kultirascontinent" => Some(1643),
        "zandalar" => Some(1642),
        "zandalarcontinentfinale" => Some(1642),
        "nazjatar" => Some(1355),
        "shadowlands" => Some(2222),
        "dragonisles" => Some(2444),
        "khazalgar" => Some(2552),
        _ => None,
    }
}

fn normalize_map_name(map_name: &str) -> String {
    let normalized = map_name.trim().replace('\\', "/").to_ascii_lowercase();
    let map_segment = normalized
        .split("world/maps/")
        .nth(1)
        .and_then(|tail| tail.split('/').next())
        .filter(|segment| !segment.is_empty())
        .unwrap_or(normalized.as_str());

    map_segment
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

pub fn resolve_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    resolve_light_params_ids(map_id, wow_position)?
        .into_iter()
        .find(|id| *id != 0)
}

pub fn resolve_skybox_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    resolve_light_params_ids(map_id, wow_position).and_then(first_resolvable_skybox_light_params_id)
}

pub fn resolve_local_skybox_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    select_light_row(map_id, wow_position)
        .and_then(|row| first_resolvable_skybox_light_params_id(row.light_params_ids))
}

pub fn resolve_light_skybox_id(light_params_id: u32) -> Option<u32> {
    cached_light_params_skybox_ids()
        .iter()
        .find(|(id, _)| *id == light_params_id)
        .map(|(_, skybox_id)| *skybox_id)
        .filter(|skybox_id| *skybox_id != 0)
}

pub fn resolve_light_skybox_fdid(light_skybox_id: u32) -> Option<u32> {
    cached_light_skybox_metadata()
        .iter()
        .find(|(id, _)| *id == light_skybox_id)
        .map(|(_, metadata)| metadata.fdid)
        .filter(|fdid| *fdid != 0)
}

pub fn resolve_light_skybox_flags(light_skybox_id: u32) -> Option<LightSkyboxFlags> {
    cached_light_skybox_metadata()
        .iter()
        .find(|(id, _)| *id == light_skybox_id)
        .map(|(_, metadata)| metadata.flags)
}

pub fn resolve_light_skybox_wow_path(light_skybox_id: u32) -> Option<&'static str> {
    let fdid = resolve_light_skybox_fdid(light_skybox_id)?;
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    wow_path.ends_with(".m2").then_some(wow_path)
}

fn cached_light_params_skybox_ids() -> &'static [(u32, u32)] {
    LIGHT_PARAMS_SKYBOX_IDS.get_or_init(load_light_params_skybox_ids)
}

fn cached_light_skybox_metadata() -> &'static [(u32, LightSkyboxMetadata)] {
    LIGHT_SKYBOX_METADATA.get_or_init(load_light_skybox_metadata)
}

fn cached_lights() -> &'static [LightEntry] {
    LIGHTS.get_or_init(
        || match cache::load_light_entries(Path::new("data/Light.csv")) {
            Ok(rows) => rows,
            Err(err) => {
                eprintln!("Failed to load Light.csv cache: {err}");
                cache::load_light_entries_uncached(Path::new("data/Light.csv")).unwrap_or_default()
            }
        },
    )
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

fn load_light_skybox_metadata() -> Vec<(u32, LightSkyboxMetadata)> {
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
                LightSkyboxMetadata {
                    fdid: db2.decode_field(row_index, LIGHT_SKYBOX_FDID_FIELD_INDEX),
                    flags: LightSkyboxFlags::from_bits(
                        db2.decode_field(row_index, LIGHT_SKYBOX_FLAGS_FIELD_INDEX),
                    ),
                },
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

fn first_resolvable_skybox_light_params_id(light_params_ids: [u32; 8]) -> Option<u32> {
    OUTDOOR_SKYBOX_LIGHT_PARAM_SLOTS
        .map(|slot| light_params_ids[slot])
        .find(|id| *id != 0 && resolve_light_skybox_id(*id).is_some())
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

#[cfg(test)]
mod tests {
    use super::{
        LightSkyboxFlags, map_name_to_id, resolve_light_params_id, resolve_light_skybox_fdid,
        resolve_light_skybox_flags, resolve_light_skybox_id, resolve_light_skybox_wow_path,
        resolve_local_skybox_light_params_id, resolve_skybox_light_params_id,
    };

    #[test]
    fn authored_light_lookup_matches_ohnahran_scene() {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 4)
            .expect("known scene");

        let params = resolve_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(6577));
    }

    #[test]
    fn authored_light_lookup_matches_freywold_scene() {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 7)
            .expect("known scene");

        let params = resolve_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(5615));
    }

    #[test]
    fn authored_skybox_params_lookup_uses_alternate_param_slots() {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 4)
            .expect("known scene");

        let params = resolve_skybox_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(5119));
    }

    #[test]
    fn authored_skybox_params_lookup_does_not_scavenge_global_death_slots() {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 1)
            .expect("known scene");

        let params = resolve_skybox_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, None);
    }

    #[test]
    fn local_authored_skybox_params_do_not_use_global_light_rows() {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 1)
            .expect("known scene");

        let params = resolve_local_skybox_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, None);
    }

    #[test]
    fn local_authored_skybox_params_still_resolve_scene_specific_rows() {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 4)
            .expect("known scene");

        let params = resolve_local_skybox_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(5119));
    }

    #[test]
    fn primary_light_params_id_can_be_missing_while_alternate_slot_resolves_skybox() {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
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
    fn primary_light_params_id_does_not_fall_through_to_death_skybox_slots() {
        let scene = crate::scenes::char_select::warband::WarbandScenes::load()
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
            None
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
    fn authored_light_skybox_rows_resolve_expected_flags() {
        assert_eq!(
            resolve_light_skybox_flags(653),
            Some(
                LightSkyboxFlags::FULL_DAY_SKYBOX
                    | LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX
                    | LightSkyboxFlags::PROCEDURAL_FOG_COLOR_BLEND
                    | LightSkyboxFlags::FORCE_SUNSHAFTS
            )
        );
        assert_eq!(
            resolve_light_skybox_flags(81),
            Some(LightSkyboxFlags::empty())
        );
    }

    #[test]
    fn authored_light_skybox_rows_resolve_expected_wow_paths() {
        assert_eq!(
            resolve_light_skybox_wow_path(653),
            Some("environments/stars/11xp_cloudsky01.m2")
        );
    }

    #[test]
    fn common_world_map_names_map_to_expected_ids() {
        assert_eq!(map_name_to_id("azeroth"), Some(0));
        assert_eq!(map_name_to_id("kalimdor"), Some(1));
        assert_eq!(map_name_to_id("expansion01"), Some(530));
        assert_eq!(map_name_to_id("outland"), Some(530));
        assert_eq!(map_name_to_id("northrend"), Some(571));
        assert_eq!(map_name_to_id("Kul_Tiras"), Some(1643));
        assert_eq!(
            map_name_to_id("world/maps/kultiras/kultiras_32_32.adt"),
            Some(1643)
        );
        assert_eq!(map_name_to_id("ZandalarContinentFinale"), Some(1642));
        assert_eq!(map_name_to_id("Khaz Algar"), Some(2552));
        assert_eq!(map_name_to_id("2703"), Some(2703));
        assert_eq!(map_name_to_id("unknown_map_name"), None);
    }
}
