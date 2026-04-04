use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::little_endian::{read_le, read_le_u16, read_le_u32};

const HELMET_GEOSET_DATA_FDID: u32 = 2_821_752;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HelmetGeosetRule {
    pub(crate) race_id: u8,
    pub(crate) race_bit_selection: u32,
    pub(crate) hide_geoset_group: u16,
}

pub(crate) fn load_helmet_geoset_rules(
    data_dir: &Path,
) -> Result<HashMap<u32, Vec<HelmetGeosetRule>>, String> {
    let path = ensure_helmet_geoset_data_path(data_dir)?;
    let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(ParsedHelmetGeosetDb2::parse(&bytes)?.rules_by_vis_id())
}

fn ensure_helmet_geoset_data_path(data_dir: &Path) -> Result<PathBuf, String> {
    let path = data_dir.join("db2/HelmetGeosetData.db2");
    if path.exists() {
        return Ok(path);
    }
    crate::asset::asset_cache::file_at_path(HELMET_GEOSET_DATA_FDID, &path)
        .ok_or_else(|| format!("extract HelmetGeosetData.db2 FDID {HELMET_GEOSET_DATA_FDID}"))
}

#[derive(Clone, Copy, Debug, Default)]
struct HelmetFieldStorage {
    offset_bits: u16,
    size_bits: u16,
    additional_data_size: u32,
    storage_type: u32,
}

#[derive(Debug)]
struct ParsedHelmetGeosetDb2<'a> {
    bytes: &'a [u8],
    record_count: usize,
    record_size: usize,
    file_offset: usize,
    id_list_offset: usize,
    relation_offset: usize,
    relation_size: usize,
    pallet_offset: usize,
    pallet_offsets: [usize; 4],
    fields: [HelmetFieldStorage; 4],
}

impl<'a> ParsedHelmetGeosetDb2<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, String> {
        let header = parse_helmet_wdc5_header(bytes)?;
        let section = parse_helmet_wdc5_section(bytes, header.section_offset);
        let fields_offset = header.section_offset + 40 + header.total_field_count * 4;
        let fields = parse_helmet_field_storage(bytes, fields_offset);
        let pallet_offset = fields_offset + header.total_field_count * 24;
        Ok(Self {
            bytes,
            record_count: header.record_count,
            record_size: header.record_size,
            file_offset: section.file_offset,
            id_list_offset: section.file_offset
                + header.record_count * header.record_size
                + section.string_table_size,
            relation_offset: section.file_offset
                + header.record_count * header.record_size
                + section.string_table_size
                + section.id_list_size,
            relation_size: section.relationship_data_size,
            pallet_offset,
            pallet_offsets: helmet_pallet_offsets(&fields),
            fields,
        })
    }

    fn rules_by_vis_id(&self) -> HashMap<u32, Vec<HelmetGeosetRule>> {
        let mut rules = HashMap::new();
        for row_index in 0..self.record_count {
            let Some(vis_id) = helmet_relation_value(
                self.bytes,
                self.relation_offset,
                self.relation_size,
                row_index,
            ) else {
                continue;
            };
            rules
                .entry(vis_id)
                .or_insert_with(Vec::new)
                .push(self.decode_row(row_index));
        }
        rules
    }

    fn decode_row(&self, row_index: usize) -> HelmetGeosetRule {
        let record_offset = self.file_offset + row_index * self.record_size;
        HelmetGeosetRule {
            race_id: self.decode_field(record_offset, row_index, 0) as u8,
            hide_geoset_group: self.decode_field(record_offset, row_index, 1) as u16,
            race_bit_selection: self.decode_field(record_offset, row_index, 2),
        }
    }

    fn decode_field(&self, record_offset: usize, row_index: usize, field_index: usize) -> u32 {
        let field = self.fields[field_index];
        if field.size_bits == 0 {
            return decode_helmet_storage_value(self, field_index, row_index, 0);
        }
        let lo = field.offset_bits as usize / 8;
        let hi = (field.offset_bits as usize + field.size_bits as usize - 1) / 8;
        let raw = read_le(self.bytes, record_offset + lo, hi - lo + 1);
        let mask = (1u64 << field.size_bits as usize) - 1;
        let value = (raw >> (field.offset_bits as usize % 8)) & mask;
        decode_helmet_storage_value(self, field_index, row_index, value as usize)
    }
}

fn decode_helmet_storage_value(
    parsed: &ParsedHelmetGeosetDb2<'_>,
    field_index: usize,
    row_index: usize,
    pallet_index: usize,
) -> u32 {
    let field = parsed.fields[field_index];
    match field.storage_type {
        1 | 5 => pallet_index as u32,
        2 => parsed_common_value(parsed, field_index, row_index),
        3 => read_le_u32(
            parsed.bytes,
            parsed.pallet_offset + parsed.pallet_offsets[field_index] + pallet_index * 4,
        ),
        other => panic!("unsupported HelmetGeosetData storage type {other}"),
    }
}

fn parsed_common_value(
    parsed: &ParsedHelmetGeosetDb2<'_>,
    field_index: usize,
    row_index: usize,
) -> u32 {
    let field = parsed.fields[field_index];
    let row_id = read_le_u32(parsed.bytes, parsed.id_list_offset + row_index * 4);
    let start = parsed.pallet_offset + parsed.pallet_offsets[field_index];
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

fn helmet_pallet_offsets(fields: &[HelmetFieldStorage; 4]) -> [usize; 4] {
    let mut offsets = [0; 4];
    let mut next = 0usize;
    for (index, field) in fields.iter().enumerate() {
        offsets[index] = next;
        if matches!(field.storage_type, 3 | 4) {
            next += field.additional_data_size as usize;
        }
    }
    offsets
}

struct HelmetWdc5Header {
    record_count: usize,
    record_size: usize,
    total_field_count: usize,
    section_offset: usize,
}

fn parse_helmet_wdc5_header(bytes: &[u8]) -> Result<HelmetWdc5Header, String> {
    if bytes.get(0..4) != Some(b"WDC5") {
        return Err("expected WDC5 HelmetGeosetData".to_string());
    }
    let offset = 136usize;
    Ok(HelmetWdc5Header {
        record_count: read_le_u32(bytes, offset) as usize,
        record_size: read_le_u32(bytes, offset + 8) as usize,
        total_field_count: read_le_u32(bytes, offset + 40) as usize,
        section_offset: offset + 68,
    })
}

struct HelmetWdc5Section {
    file_offset: usize,
    string_table_size: usize,
    id_list_size: usize,
    relationship_data_size: usize,
}

fn parse_helmet_wdc5_section(bytes: &[u8], offset: usize) -> HelmetWdc5Section {
    HelmetWdc5Section {
        file_offset: read_le_u32(bytes, offset + 8) as usize,
        string_table_size: read_le_u32(bytes, offset + 16) as usize,
        id_list_size: read_le_u32(bytes, offset + 24) as usize,
        relationship_data_size: read_le_u32(bytes, offset + 28) as usize,
    }
}

fn parse_helmet_field_storage(bytes: &[u8], offset: usize) -> [HelmetFieldStorage; 4] {
    let mut fields = [HelmetFieldStorage::default(); 4];
    for (index, field) in fields.iter_mut().enumerate() {
        let base = offset + index * 24;
        *field = HelmetFieldStorage {
            offset_bits: read_le_u16(bytes, base),
            size_bits: read_le_u16(bytes, base + 2),
            additional_data_size: read_le_u32(bytes, base + 4),
            storage_type: read_le_u32(bytes, base + 8),
        };
    }
    fields
}

fn helmet_relation_value(
    bytes: &[u8],
    offset: usize,
    size: usize,
    row_index: usize,
) -> Option<u32> {
    let mut cursor = offset + 12;
    let end = offset + size;
    while cursor < end {
        if read_le_u32(bytes, cursor + 4) as usize == row_index {
            return Some(read_le_u32(bytes, cursor));
        }
        cursor += 8;
    }
    None
}
