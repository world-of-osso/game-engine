pub fn read_le(bytes: &[u8], offset: usize, len: usize) -> u64 {
    let mut value = 0u64;
    for (index, byte) in bytes[offset..offset + len].iter().enumerate() {
        value |= (*byte as u64) << (index * 8);
    }
    value
}

pub fn read_le_u16(bytes: &[u8], offset: usize) -> u16 {
    read_le(bytes, offset, 2) as u16
}

pub fn read_le_u32(bytes: &[u8], offset: usize) -> u32 {
    read_le(bytes, offset, 4) as u32
}
