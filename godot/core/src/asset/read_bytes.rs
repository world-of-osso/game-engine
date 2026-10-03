pub fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

pub fn read_i32(data: &[u8], off: usize) -> Result<i32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_i32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(i32::from_le_bytes(bytes))
}

pub fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

/// Read an M2Array header (count u32 + offset u32) starting at `header_off`.
/// Returns `(0, 0)` if the data is too short to contain the header.
pub fn read_m2_array_header(data: &[u8], header_off: usize) -> Result<(usize, usize), String> {
    if data.len() < header_off + 8 {
        return Ok((0, 0));
    }
    let count = read_u32(data, header_off)? as usize;
    let offset = read_u32(data, header_off + 4)? as usize;
    Ok((count, offset))
}

pub fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_u16 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

pub fn read_i8(data: &[u8], off: usize) -> Result<i8, String> {
    data.get(off)
        .copied()
        .map(|b| b as i8)
        .ok_or_else(|| format!("read_i8 out of bounds at {off:#x}"))
}

pub fn read_i16(data: &[u8], off: usize) -> Result<i16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_i16 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(i16::from_le_bytes(bytes))
}

pub const FIXED16_SCALE: f32 = 32767.0;

pub fn fixed16_to_f32(raw: i16) -> f32 {
    raw as f32 / FIXED16_SCALE
}

pub fn unorm16_to_f32(raw: u16) -> f32 {
    (raw as f32 / FIXED16_SCALE).clamp(0.0, 1.0)
}

pub fn read_vec3(data: &[u8], off: usize) -> Result<[f32; 3], String> {
    Ok([
        read_f32(data, off)?,
        read_f32(data, off + 4)?,
        read_f32(data, off + 8)?,
    ])
}

pub fn read_quat_i16(data: &[u8], off: usize) -> Result<[i16; 4], String> {
    Ok([
        read_i16(data, off)?,
        read_i16(data, off + 2)?,
        read_i16(data, off + 4)?,
        read_i16(data, off + 6)?,
    ])
}
