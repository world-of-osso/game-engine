use super::adt::ChunkIter;

#[allow(dead_code)]
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

#[allow(dead_code)]
pub struct WmoPlacement {
    pub name_id: u32,
    pub unique_id: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub flags: u16,
    #[allow(dead_code)]
    pub doodad_set: u16,
    #[allow(dead_code)]
    pub name_set: u16,
    pub scale: f32,
    pub fdid: Option<u32>,
    pub path: Option<String>,
}

pub struct AdtObjData {
    pub doodads: Vec<DoodadPlacement>,
    pub wmos: Vec<WmoPlacement>,
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

    eprintln!(
        "Loaded _obj0: {} doodads ({} model paths), {} WMOs ({} WMO paths)",
        doodads.len(),
        paths.len(),
        wmos.len(),
        wmo_paths.len(),
    );

    Ok(AdtObjData { doodads, wmos })
}

struct Obj0Chunks {
    mmdx: Vec<u8>,
    mmid: Vec<u32>,
    mwmo: Vec<u8>,
    mwid: Vec<u32>,
    mddf: Option<Vec<u8>>,
    modf: Option<Vec<u8>>,
}

fn collect_obj0_chunks(data: &[u8]) -> Result<Obj0Chunks, String> {
    let mut c = Obj0Chunks {
        mmdx: Vec::new(),
        mmid: Vec::new(),
        mwmo: Vec::new(),
        mwid: Vec::new(),
        mddf: None,
        modf: None,
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

fn string_at_offset(table: &[u8], offset: u32) -> Option<String> {
    let start = offset as usize;
    if start >= table.len() {
        return None;
    }
    let end = table[start..].iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&table[start..start + end]).into_owned())
}

fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_u16 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

fn read_placement_transform(data: &[u8], base: usize) -> Result<([f32; 3], [f32; 3]), String> {
    let position = [
        read_f32(data, base + 8)?,
        read_f32(data, base + 12)?,
        read_f32(data, base + 16)?,
    ];
    let rotation = [
        read_f32(data, base + 20)?,
        read_f32(data, base + 24)?,
        read_f32(data, base + 28)?,
    ];
    Ok((position, rotation))
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
    let name_id = read_u32(data, base)?;
    let unique_id = read_u32(data, base + 4)?;
    let (position, rotation) = read_placement_transform(data, base)?;
    let scale_raw = read_u16(data, base + 32)?;
    let flags = read_u16(data, base + 34)?;
    let (fdid, path) = resolve_doodad(name_id, flags, string_table, offset_table);

    Ok(DoodadPlacement {
        name_id,
        unique_id,
        position,
        rotation,
        scale: scale_raw as f32 / 1024.0,
        flags,
        fdid,
        path,
    })
}

fn parse_mddf(
    data: &[u8],
    string_table: &[u8],
    offset_table: &[u32],
) -> Result<Vec<DoodadPlacement>, String> {
    let count = data.len() / 36;
    (0..count)
        .map(|i| parse_mddf_entry(data, i * 36, string_table, offset_table))
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
    let name_id = read_u32(data, base)?;
    let unique_id = read_u32(data, base + 4)?;
    let (position, rotation) = read_placement_transform(data, base)?;
    let flags = read_u16(data, base + 56)?;
    let doodad_set = read_u16(data, base + 58)?;
    let name_set = read_u16(data, base + 60)?;
    let scale_raw = read_u16(data, base + 62)?;
    let (fdid, path) = resolve_wmo(name_id, flags, string_table, offset_table);
    let scale = if (flags & MODF_FLAG_HAS_SCALE) != 0 {
        scale_raw as f32 / 1024.0
    } else {
        1.0
    };

    Ok(WmoPlacement {
        name_id,
        unique_id,
        position,
        rotation,
        flags,
        doodad_set,
        name_set,
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
    let count = data.len() / 64;
    (0..count)
        .map(|i| parse_modf_entry(data, i * 64, string_table, offset_table))
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
}
