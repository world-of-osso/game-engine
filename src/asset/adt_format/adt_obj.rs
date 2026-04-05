use std::io::Cursor;
use std::mem::size_of;

use binrw::BinRead;

use super::adt::ChunkIter;

pub struct DoodadPlacement {
    pub name_id: u32,
    pub unique_id: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: f32,
    pub flags: u16,
    pub fdid: Option<u32>,
    pub path: Option<String>,
}

pub struct WmoPlacement {
    pub name_id: u32,
    pub unique_id: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub extents_min: [f32; 3],
    pub extents_max: [f32; 3],
    pub flags: u16,
    pub doodad_set: u16,
    pub name_set: u16,
    pub scale: f32,
    pub fdid: Option<u32>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChunkObjectRefs {
    pub doodad_refs: Vec<u32>,
    pub wmo_refs: Vec<u32>,
}

pub struct AdtObjData {
    pub doodads: Vec<DoodadPlacement>,
    pub wmos: Vec<WmoPlacement>,
    pub chunk_refs: Vec<ChunkObjectRefs>,
}

pub fn load_adt_obj0(data: &[u8]) -> Result<AdtObjData, String> {
    let chunks = collect_obj0_chunks(data)?;
    let paths = parse_string_table(&chunks.mmdx);
    let wmo_paths = parse_string_table(&chunks.mwmo);

    let doodads = match chunks.mddf {
        Some(p) => parse_mddf(&p, &chunks.mmdx, &chunks.mmid)?,
        None => Vec::new(),
    };
    let wmos = match chunks.modf {
        Some(p) => parse_modf(&p, &chunks.mwmo, &chunks.mwid)?,
        None => Vec::new(),
    };
    let chunk_refs = parse_chunk_object_refs(&chunks.mcnk_chunks)?;

    eprintln!(
        "Loaded _obj0: {} doodads ({} model paths), {} WMOs ({} WMO paths), {} chunk ref sets",
        doodads.len(),
        paths.len(),
        wmos.len(),
        wmo_paths.len(),
        chunk_refs.len(),
    );

    Ok(AdtObjData {
        doodads,
        wmos,
        chunk_refs,
    })
}

struct Obj0Chunks {
    mmdx: Vec<u8>,
    mmid: Vec<u32>,
    mwmo: Vec<u8>,
    mwid: Vec<u32>,
    mddf: Option<Vec<u8>>,
    modf: Option<Vec<u8>>,
    mcnk_chunks: Vec<Vec<u8>>,
}

#[derive(BinRead)]
#[br(little)]
struct MddfEntry {
    name_id: u32,
    unique_id: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    scale_raw: u16,
    flags: u16,
}

#[derive(BinRead)]
#[br(little)]
struct ModfEntry {
    name_id: u32,
    unique_id: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    _lower_bounds: [f32; 3],
    _upper_bounds: [f32; 3],
    flags: u16,
    doodad_set: u16,
    name_set: u16,
    scale_raw: u16,
}

fn collect_obj0_chunks(data: &[u8]) -> Result<Obj0Chunks, String> {
    let mut c = Obj0Chunks {
        mmdx: Vec::new(),
        mmid: Vec::new(),
        mwmo: Vec::new(),
        mwid: Vec::new(),
        mddf: None,
        modf: None,
        mcnk_chunks: Vec::new(),
    };
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"XDMM" => c.mmdx = payload.to_vec(),
            b"DIMM" => c.mmid = parse_u32_array(payload),
            b"OMWM" => c.mwmo = payload.to_vec(),
            b"DIWM" => c.mwid = parse_u32_array(payload),
            b"FDDM" => c.mddf = Some(payload.to_vec()),
            b"FDOM" => c.modf = Some(payload.to_vec()),
            b"KNCM" => c.mcnk_chunks.push(payload.to_vec()),
            _ => {}
        }
    }
    Ok(c)
}

fn parse_string_table(data: &[u8]) -> Vec<String> {
    if data.is_empty() {
        return Vec::new();
    }
    data.split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

fn parse_u32_array(data: &[u8]) -> Vec<u32> {
    data.chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn parse_chunk_object_refs(mcnk_chunks: &[Vec<u8>]) -> Result<Vec<ChunkObjectRefs>, String> {
    mcnk_chunks
        .iter()
        .map(|payload| parse_mcnk_object_refs(payload))
        .collect()
}

fn parse_mcnk_object_refs(payload: &[u8]) -> Result<ChunkObjectRefs, String> {
    let mut refs = ChunkObjectRefs::default();
    for chunk in ChunkIter::new(payload) {
        let (tag, chunk_payload) = chunk?;
        match tag {
            b"DRCM" => refs.doodad_refs = parse_u32_array(chunk_payload),
            b"WRCM" => refs.wmo_refs = parse_u32_array(chunk_payload),
            _ => {}
        }
    }
    Ok(refs)
}

fn string_at_offset(table: &[u8], offset: u32) -> Option<String> {
    let start = offset as usize;
    if start >= table.len() {
        return None;
    }
    let end = table[start..].iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&table[start..start + end]).into_owned())
}

fn parse_binrw_value<T>(data: &[u8], offset: usize, label: &str) -> Result<T, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let end = offset
        .checked_add(size_of::<T>())
        .ok_or_else(|| format!("{label} end offset overflow"))?;
    let slice = data
        .get(offset..end)
        .ok_or_else(|| format!("{label} out of bounds at {offset:#x}"))?;
    T::read_le(&mut Cursor::new(slice))
        .map_err(|err| format!("{label} parse failed at {offset:#x}: {err}"))
}

const MDDF_FLAG_FILEDATAID: u16 = 0x40;

fn resolve_doodad(
    name_id: u32,
    flags: u16,
    string_table: &[u8],
    offset_table: &[u32],
) -> (Option<u32>, Option<String>) {
    if (flags & MDDF_FLAG_FILEDATAID) != 0 {
        return (Some(name_id), None);
    }
    let offset = offset_table.get(name_id as usize).copied();
    let path = offset.and_then(|o| string_at_offset(string_table, o));
    (None, path)
}

fn parse_mddf_entry(
    data: &[u8],
    base: usize,
    string_table: &[u8],
    offset_table: &[u32],
) -> Result<DoodadPlacement, String> {
    let entry: MddfEntry = parse_binrw_value(data, base, "MDDF entry")?;
    let (fdid, path) = resolve_doodad(entry.name_id, entry.flags, string_table, offset_table);

    Ok(DoodadPlacement {
        name_id: entry.name_id,
        unique_id: entry.unique_id,
        position: entry.position,
        rotation: entry.rotation,
        scale: entry.scale_raw as f32 / 1024.0,
        flags: entry.flags,
        fdid,
        path,
    })
}

fn parse_mddf(
    data: &[u8],
    string_table: &[u8],
    offset_table: &[u32],
) -> Result<Vec<DoodadPlacement>, String> {
    let count = data.len() / size_of::<MddfEntry>();
    (0..count)
        .map(|i| parse_mddf_entry(data, i * size_of::<MddfEntry>(), string_table, offset_table))
        .collect()
}

const MODF_FLAG_HAS_SCALE: u16 = 0x4;
const MODF_FLAG_FILEDATAID: u16 = 0x8;

fn resolve_wmo(
    name_id: u32,
    flags: u16,
    string_table: &[u8],
    offset_table: &[u32],
) -> (Option<u32>, Option<String>) {
    if (flags & MODF_FLAG_FILEDATAID) != 0 {
        return (Some(name_id), None);
    }
    let offset = offset_table.get(name_id as usize).copied();
    let path = offset.and_then(|o| string_at_offset(string_table, o));
    (None, path)
}

fn parse_modf_entry(
    data: &[u8],
    base: usize,
    string_table: &[u8],
    offset_table: &[u32],
) -> Result<WmoPlacement, String> {
    let entry: ModfEntry = parse_binrw_value(data, base, "MODF entry")?;
    let (fdid, path) = resolve_wmo(entry.name_id, entry.flags, string_table, offset_table);
    let scale = if (entry.flags & MODF_FLAG_HAS_SCALE) != 0 {
        entry.scale_raw as f32 / 1024.0
    } else {
        1.0
    };

    Ok(WmoPlacement {
        name_id: entry.name_id,
        unique_id: entry.unique_id,
        position: entry.position,
        rotation: entry.rotation,
        extents_min: entry._lower_bounds,
        extents_max: entry._upper_bounds,
        flags: entry.flags,
        doodad_set: entry.doodad_set,
        name_set: entry.name_set,
        scale,
        fdid,
        path,
    })
}

fn parse_modf(
    data: &[u8],
    string_table: &[u8],
    offset_table: &[u32],
) -> Result<Vec<WmoPlacement>, String> {
    let count = data.len() / size_of::<ModfEntry>();
    (0..count)
        .map(|i| parse_modf_entry(data, i * size_of::<ModfEntry>(), string_table, offset_table))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_elwynn_obj0() {
        let data =
            std::fs::read("data/terrain/azeroth_32_48_obj0.adt").expect("missing test asset");
        let obj = load_adt_obj0(&data).expect("parse failed");
        assert!(!obj.doodads.is_empty(), "expected doodads");
        let d = &obj.doodads[0];
        assert!(d.scale > 0.0, "scale should be positive");
        assert!(
            d.position[0] != 0.0 || d.position[1] != 0.0,
            "position shouldn't be zero"
        );
    }

    #[test]
    fn parse_modf_entry_preserves_doodad_and_name_sets() {
        let mut entry = Vec::new();
        entry.extend_from_slice(&7u32.to_le_bytes());
        entry.extend_from_slice(&99u32.to_le_bytes());
        entry.extend_from_slice(&1.0f32.to_le_bytes());
        entry.extend_from_slice(&2.0f32.to_le_bytes());
        entry.extend_from_slice(&3.0f32.to_le_bytes());
        entry.extend_from_slice(&10.0f32.to_le_bytes());
        entry.extend_from_slice(&20.0f32.to_le_bytes());
        entry.extend_from_slice(&30.0f32.to_le_bytes());
        entry.extend_from_slice(&[0u8; 24]);
        entry.extend_from_slice(&MODF_FLAG_HAS_SCALE.to_le_bytes());
        entry.extend_from_slice(&5u16.to_le_bytes());
        entry.extend_from_slice(&8u16.to_le_bytes());
        entry.extend_from_slice(&2048u16.to_le_bytes());

        let parsed = parse_modf_entry(&entry, 0, b"", &[]).expect("modf entry should parse");

        assert_eq!(parsed.unique_id, 99);
        assert_eq!(parsed.extents_min, [0.0, 0.0, 0.0]);
        assert_eq!(parsed.extents_max, [0.0, 0.0, 0.0]);
        assert_eq!(parsed.doodad_set, 5);
        assert_eq!(parsed.name_set, 8);
        assert_eq!(parsed.scale, 2.0);
    }

    #[test]
    fn parse_modf_entry_preserves_extents() {
        let mut entry = Vec::new();
        entry.extend_from_slice(&7u32.to_le_bytes());
        entry.extend_from_slice(&99u32.to_le_bytes());
        entry.extend_from_slice(&1.0f32.to_le_bytes());
        entry.extend_from_slice(&2.0f32.to_le_bytes());
        entry.extend_from_slice(&3.0f32.to_le_bytes());
        entry.extend_from_slice(&10.0f32.to_le_bytes());
        entry.extend_from_slice(&20.0f32.to_le_bytes());
        entry.extend_from_slice(&30.0f32.to_le_bytes());
        entry.extend_from_slice(&40.0f32.to_le_bytes());
        entry.extend_from_slice(&50.0f32.to_le_bytes());
        entry.extend_from_slice(&60.0f32.to_le_bytes());
        entry.extend_from_slice(&70.0f32.to_le_bytes());
        entry.extend_from_slice(&80.0f32.to_le_bytes());
        entry.extend_from_slice(&90.0f32.to_le_bytes());
        entry.extend_from_slice(&0u16.to_le_bytes());
        entry.extend_from_slice(&0u16.to_le_bytes());
        entry.extend_from_slice(&0u16.to_le_bytes());
        entry.extend_from_slice(&0u16.to_le_bytes());

        let parsed = parse_modf_entry(&entry, 0, b"", &[]).expect("modf entry should parse");

        assert_eq!(parsed.extents_min, [40.0, 50.0, 60.0]);
        assert_eq!(parsed.extents_max, [70.0, 80.0, 90.0]);
    }

    #[test]
    fn load_adt_obj0_reads_per_chunk_object_refs() {
        let mut payload = Vec::new();
        append_subchunk(&mut payload, b"XDMM", b"foo.m2\0".to_vec());
        append_subchunk(&mut payload, b"DIMM", 0u32.to_le_bytes().to_vec());
        append_subchunk(&mut payload, b"OMWM", b"bar.wmo\0".to_vec());
        append_subchunk(&mut payload, b"DIWM", 0u32.to_le_bytes().to_vec());
        append_subchunk(&mut payload, b"FDDM", Vec::new());
        append_subchunk(&mut payload, b"FDOM", Vec::new());
        append_subchunk(
            &mut payload,
            b"KNCM",
            mcnk_object_refs_payload(&[1, 4, 7], &[2]),
        );
        append_subchunk(
            &mut payload,
            b"KNCM",
            mcnk_object_refs_payload(&[], &[5, 6]),
        );

        let parsed = load_adt_obj0(&payload).expect("expected obj0 payload to parse");

        assert_eq!(parsed.chunk_refs.len(), 2);
        assert_eq!(parsed.chunk_refs[0].doodad_refs, vec![1, 4, 7]);
        assert_eq!(parsed.chunk_refs[0].wmo_refs, vec![2]);
        assert!(parsed.chunk_refs[1].doodad_refs.is_empty());
        assert_eq!(parsed.chunk_refs[1].wmo_refs, vec![5, 6]);
    }

    #[test]
    fn parse_elwynn_obj0_reads_chunk_object_refs() {
        let data =
            std::fs::read("data/terrain/azeroth_32_48_obj0.adt").expect("missing test asset");
        let obj = load_adt_obj0(&data).expect("parse failed");

        assert_eq!(
            obj.chunk_refs.len(),
            256,
            "expected one KNCM ref set per ADT chunk"
        );
        assert!(
            obj.chunk_refs
                .iter()
                .any(|chunk| !chunk.doodad_refs.is_empty() || !chunk.wmo_refs.is_empty()),
            "expected at least one chunk to reference doodads or WMOs"
        );
    }

    fn mcnk_object_refs_payload(doodad_refs: &[u32], wmo_refs: &[u32]) -> Vec<u8> {
        let mut payload = Vec::new();
        if !doodad_refs.is_empty() {
            append_subchunk(&mut payload, b"DRCM", u32_array_payload(doodad_refs));
        }
        if !wmo_refs.is_empty() {
            append_subchunk(&mut payload, b"WRCM", u32_array_payload(wmo_refs));
        }
        payload
    }

    fn u32_array_payload(values: &[u32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn append_subchunk(payload: &mut Vec<u8>, tag: &[u8; 4], chunk_payload: Vec<u8>) {
        payload.extend_from_slice(tag);
        payload.extend_from_slice(&(chunk_payload.len() as u32).to_le_bytes());
        payload.extend_from_slice(&chunk_payload);
    }
}
