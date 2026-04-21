use crate::little_endian::{read_le, read_le_u16, read_le_u32};

#[derive(Clone, Copy, Debug, Default)]
struct Wdc5FieldStorage {
    offset_bits: u16,
    size_bits: u16,
    additional_data_size: u32,
    storage_type: u32,
}

pub(super) struct ParsedWdc5Db2<'a> {
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
    pub(super) fn parse(bytes: &'a [u8], expected_layout_hash: u32) -> Result<Self, String> {
        let header = parse_wdc5_header(bytes)?;
        if header.layout_hash != expected_layout_hash {
            return Err(format!(
                "unexpected WDC5 layout hash 0x{:08X}, expected 0x{:08X}",
                header.layout_hash, expected_layout_hash
            ));
        }
        Self::from_header(bytes, &header)
    }

    pub(super) fn parse_any_layout(
        bytes: &'a [u8],
        expected_layout_hashes: &[u32],
    ) -> Result<Self, String> {
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

    pub(super) fn rows(&self) -> Vec<Wdc5RowRef> {
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

    pub(super) fn row_id(&self, row: Wdc5RowRef) -> u32 {
        let section = &self.sections[row.section_index];
        read_le_u32(self.bytes, section.id_list_offset + row.row_index * 4)
    }

    pub(super) fn decode_field(&self, row: Wdc5RowRef, field_index: usize) -> u32 {
        let field = self.fields[field_index];
        let value_index = self.decode_field_value_index(row, field);
        decode_wdc5_storage_value(self, field_index, row, value_index)
    }

    fn decode_field_value_index(&self, row: Wdc5RowRef, field: Wdc5FieldStorage) -> usize {
        if field.size_bits == 0 {
            return 0;
        }

        let record_offset = self.row_record_offset(row);
        let raw = self.read_field_raw(record_offset, field);
        let shifted = raw >> Self::field_bit_shift(field);
        let value = shifted & Self::field_value_mask(field.size_bits);
        value as usize
    }

    fn row_record_offset(&self, row: Wdc5RowRef) -> usize {
        let section = &self.sections[row.section_index];
        section.file_offset + row.row_index * self.record_size
    }

    fn read_field_raw(&self, record_offset: usize, field: Wdc5FieldStorage) -> u64 {
        let (lo, len) = Self::field_byte_range(field);
        read_le(self.bytes, record_offset + lo, len)
    }

    fn field_byte_range(field: Wdc5FieldStorage) -> (usize, usize) {
        let lo = field.offset_bits as usize / 8;
        let hi = (field.offset_bits as usize + field.size_bits as usize - 1) / 8;
        (lo, hi - lo + 1)
    }

    fn field_bit_shift(field: Wdc5FieldStorage) -> usize {
        field.offset_bits as usize % 8
    }

    fn field_value_mask(size_bits: u16) -> u64 {
        if size_bits == 32 {
            return u64::MAX;
        }

        (1u64 << size_bits as usize) - 1
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
pub(super) struct Wdc5RowRef {
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
        let section = parse_wdc5_section(bytes, offset, index, record_size)?;
        sections.push(section);
    }
    Ok(sections)
}

fn parse_wdc5_section(
    bytes: &[u8],
    section_offset: usize,
    section_index: usize,
    record_size: usize,
) -> Result<Wdc5Section, String> {
    let base = wdc5_section_base_offset(section_offset, section_index);
    ensure_wdc5_section_header(bytes, base)?;

    let file_offset = read_le_u32(bytes, base + 8) as usize;
    let record_count = read_le_u32(bytes, base + 12) as usize;
    let string_table_size = read_le_u32(bytes, base + 16) as usize;
    let _id_list_size = read_le_u32(bytes, base + 24) as usize;

    Ok(Wdc5Section {
        file_offset,
        record_count,
        string_table_size,
        id_list_offset: wdc5_id_list_offset(
            file_offset,
            record_count,
            record_size,
            string_table_size,
        ),
    })
}

fn wdc5_section_base_offset(section_offset: usize, section_index: usize) -> usize {
    section_offset + section_index * 40
}

fn ensure_wdc5_section_header(bytes: &[u8], base: usize) -> Result<(), String> {
    if bytes.len() < base + 40 {
        return Err("truncated WDC5 section header".to_string());
    }

    Ok(())
}

fn wdc5_id_list_offset(
    file_offset: usize,
    record_count: usize,
    record_size: usize,
    string_table_size: usize,
) -> usize {
    file_offset + record_count * record_size + string_table_size
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
