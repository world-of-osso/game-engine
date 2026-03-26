#[derive(Debug, Clone, PartialEq)]
pub struct FogVolume {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub extents: [f32; 3],
    pub color: [f32; 3],
    pub density: f32,
    pub model_fdid: u32,
    pub fog_level: u32,
    pub fog_id: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FogsWdt {
    pub version: u32,
    pub volumes: Vec<FogVolume>,
}

const VFOG_ENTRY_SIZE: usize = 0x68;
const VFEX_ENTRY_SIZE: usize = 0x60;
const VFE2_ENTRY_SIZE: usize = 0xb0;

struct ChunkIter<'a> {
    data: &'a [u8],
    off: usize,
}

impl<'a> ChunkIter<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, off: 0 }
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = Result<(&'a str, &'a [u8]), String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.off + 8 > self.data.len() {
            return None;
        }
        let tag_bytes = &self.data[self.off..self.off + 4];
        let tag = match std::str::from_utf8(tag_bytes) {
            Ok(tag) => tag,
            Err(err) => return Some(Err(format!("invalid fourcc at {:#x}: {err}", self.off))),
        };
        let size = match read_u32(self.data, self.off + 4) {
            Ok(size) => size as usize,
            Err(err) => return Some(Err(err)),
        };
        let payload_start = self.off + 8;
        let payload_end = payload_start + size;
        if payload_end > self.data.len() {
            return Some(Err(format!(
                "chunk {tag} truncated at {:#x}: need {} bytes",
                self.off, size
            )));
        }
        self.off = payload_end;
        Some(Ok((tag, &self.data[payload_start..payload_end])))
    }
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

fn parse_vfog(payload: &[u8]) -> Result<Vec<FogVolume>, String> {
    if !payload.len().is_multiple_of(VFOG_ENTRY_SIZE) {
        return Err(format!(
            "VFOG payload size {} is not a multiple of {}",
            payload.len(),
            VFOG_ENTRY_SIZE
        ));
    }
    let mut volumes = Vec::new();
    for base in (0..payload.len()).step_by(VFOG_ENTRY_SIZE) {
        let color = [
            read_f32(payload, base)?,
            read_f32(payload, base + 4)?,
            read_f32(payload, base + 8)?,
        ];
        let density = read_f32(payload, base + 12)?;
        let position = [
            read_f32(payload, base + 28)?,
            read_f32(payload, base + 32)?,
            read_f32(payload, base + 36)?,
        ];
        let rotation = [
            read_f32(payload, base + 44)?,
            read_f32(payload, base + 48)?,
            read_f32(payload, base + 52)?,
            read_f32(payload, base + 56)?,
        ];
        let flags = read_u32(payload, base + 88)?;
        let model_fdid = read_u32(payload, base + 92)?;
        let fog_level = read_u32(payload, base + 96)?;
        let fog_id = read_u32(payload, base + 100)?;
        volumes.push(FogVolume {
            position,
            rotation,
            extents: [1.0, 1.0, 1.0],
            color,
            density,
            model_fdid,
            fog_level,
            fog_id,
            flags,
        });
    }
    Ok(volumes)
}

fn parse_vfex(payload: &[u8]) -> Result<Vec<([f32; 3], u32)>, String> {
    if !payload.len().is_multiple_of(VFEX_ENTRY_SIZE) {
        return Err(format!(
            "VFEX payload size {} is not a multiple of {}",
            payload.len(),
            VFEX_ENTRY_SIZE
        ));
    }
    let mut extras = Vec::new();
    for base in (0..payload.len()).step_by(VFEX_ENTRY_SIZE) {
        let extents = [
            read_f32(payload, base + 4)?,
            read_f32(payload, base + 8)?,
            read_f32(payload, base + 12)?,
        ];
        let fog_id = read_u32(payload, base + 68)?;
        extras.push((extents, fog_id));
    }
    Ok(extras)
}

fn parse_vfe2(payload: &[u8]) -> Result<Vec<f32>, String> {
    if !payload.len().is_multiple_of(VFE2_ENTRY_SIZE) {
        return Err(format!(
            "VFE2 payload size {} is not a multiple of {}",
            payload.len(),
            VFE2_ENTRY_SIZE
        ));
    }
    let mut densities = Vec::new();
    for base in (0..payload.len()).step_by(VFE2_ENTRY_SIZE) {
        densities.push(read_f32(payload, base + 12)?);
    }
    Ok(densities)
}

pub fn load_fogs_wdt(data: &[u8]) -> Result<FogsWdt, String> {
    let mut version = None;
    let mut volumes = Vec::new();
    let mut vfex = Vec::new();
    let mut vfe2 = Vec::new();

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            "REVM" => version = Some(read_u32(payload, 0)?),
            "GOFV" => volumes = parse_vfog(payload)?,
            "XEFV" => vfex = parse_vfex(payload)?,
            "2EFV" => vfe2 = parse_vfe2(payload)?,
            _ => {}
        }
    }

    let version = version.ok_or_else(|| "fogs.wdt missing REVM chunk".to_string())?;
    for (index, volume) in volumes.iter_mut().enumerate() {
        if let Some((extents, fog_id)) = vfex
            .iter()
            .find(|(_, fog_id)| *fog_id == volume.fog_id)
            .or_else(|| vfex.get(index))
        {
            volume.extents = *extents;
            if *fog_id != volume.fog_id {
                return Err(format!(
                    "fog volume metadata mismatch at index {index}: vfog={} vfex={fog_id}",
                    volume.fog_id
                ));
            }
        }
        if let Some(density) = vfe2.get(index) {
            volume.density = *density;
        }
    }

    Ok(FogsWdt { version, volumes })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_adventurers_rest_fogs_wdt_fixture() {
        let data = std::fs::read("data/fogs/5493445.wdt")
            .expect("expected extracted fog fixture data/fogs/5493445.wdt");

        let parsed = load_fogs_wdt(&data).expect("expected fogs.wdt to parse");

        assert_eq!(parsed.version, 2);
        assert_eq!(parsed.volumes.len(), 1);
        let volume = &parsed.volumes[0];
        assert_eq!(volume.model_fdid, 1_728_356);
        assert_eq!(volume.fog_id, 1_725);
        assert_eq!(volume.fog_level, 1);
        assert!((volume.position[0] - -3022.58).abs() < 0.05);
        assert!((volume.position[1] - 273.943).abs() < 0.05);
        assert!((volume.position[2] - 473.071).abs() < 0.05);
        assert!((volume.extents[0] - 235.947).abs() < 0.05);
        assert!((volume.extents[1] - 254.610).abs() < 0.05);
        assert!((volume.extents[2] - 86.853).abs() < 0.05);
        assert!((volume.density - 0.1).abs() < 0.0001);
    }
}
