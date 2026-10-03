//! WDT MPHD header flags that select how a map's `_tex0` terrain textures are stored and blended,
//! and the single WMO of a map made of one (a WMO-only dungeon such as the Stockade).

use crate::asset::read_bytes::read_u32;

use super::adt::ChunkIter;
use super::adt_obj::{WmoPlacement, parse_modf};

/// wowdev WDT: `wdt_uses_global_map_obj`, the map is the one WMO its MODF places.
const MPHD_FLAG_GLOBAL_WMO: u32 = 0x1;

/// wowdev WDT: `adt_has_big_alpha`, "shader = 2".
const MPHD_FLAG_BIG_ALPHA: u32 = 0x4;
/// wowdev WDT: `adt_has_height_texturing`, "shader = 6"; also switches uncompressed MCAL to 4096 bytes.
const MPHD_FLAG_HEIGHT_TEXTURING: u32 = 0x80;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MphdFlags {
    pub raw: u32,
}

impl MphdFlags {
    /// Uncompressed MCAL layers are 4096 8-bit bytes instead of 2048 4-bit bytes.
    pub fn big_alpha(&self) -> bool {
        (self.raw & (MPHD_FLAG_BIG_ALPHA | MPHD_FLAG_HEIGHT_TEXTURING)) != 0
    }

    /// Terrain layers blend with `_h` height textures scaled by MTXP.
    pub fn height_texturing(&self) -> bool {
        (self.raw & MPHD_FLAG_HEIGHT_TEXTURING) != 0
    }
}

pub fn parse_wdt_mphd_flags(data: &[u8]) -> Result<MphdFlags, String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"DHPM" {
            return Ok(MphdFlags {
                raw: read_u32(payload, 0)?,
            });
        }
    }
    Err("WDT missing MPHD chunk".to_string())
}

/// The WMO of a global-WMO map (MPHD flag 0x1): the WDT's single MODF entry, named by
/// FileDataID. `None` for a map of ADT tiles.
pub fn parse_wdt_global_wmo(data: &[u8]) -> Result<Option<WmoPlacement>, String> {
    if parse_wdt_mphd_flags(data)?.raw & MPHD_FLAG_GLOBAL_WMO == 0 {
        return Ok(None);
    }
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"FDOM" {
            let placement = parse_modf(payload, &[], &[])?.into_iter().next();
            return placement
                .map(Some)
                .ok_or_else(|| "global-WMO WDT MODF is empty".to_string());
        }
    }
    Err("global-WMO WDT has no MODF".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wdt_payload(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut data = Vec::new();
        for (tag, payload) in chunks {
            data.extend_from_slice(*tag);
            data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            data.extend_from_slice(payload);
        }
        data
    }

    #[test]
    fn reads_mphd_flags_after_version_chunk() {
        let mut mphd = 0x3cau32.to_le_bytes().to_vec();
        mphd.extend_from_slice(&[0; 28]);
        let data = wdt_payload(&[(b"REVM", 18u32.to_le_bytes().to_vec()), (b"DHPM", mphd)]);

        let flags = parse_wdt_mphd_flags(&data).expect("expected MPHD");

        assert_eq!(flags.raw, 0x3ca);
        assert!(flags.big_alpha());
        assert!(flags.height_texturing());
    }

    #[test]
    fn height_texturing_flag_implies_big_alpha_and_big_alpha_alone_does_not_imply_height() {
        assert!(MphdFlags { raw: 0x80 }.big_alpha());
        assert!(MphdFlags { raw: 0x4 }.big_alpha());
        assert!(!MphdFlags { raw: 0x4 }.height_texturing());
        assert!(!MphdFlags { raw: 0x2 }.big_alpha());
    }

    #[test]
    fn stockade_wdt_places_its_prison_wmo_at_the_world_origin() {
        // world/maps/stormwindjail/stormwindjail.wdt (FDID 791060), extracted from local CASC.
        let data = std::fs::read("data/terrain/791060.wdt").expect("Stockade WDT in data/terrain");
        let wmo = parse_wdt_global_wmo(&data)
            .expect("parses")
            .expect("global WMO");
        // world/wmo/dungeon/az_stormwindprisons/stormwindjail.wmo.
        assert_eq!(wmo.fdid, Some(108_631));
        assert_eq!(wmo.position, [0.0, 0.0, 0.0]);
        assert_eq!(wmo.scale, 1.0);
    }

    #[test]
    fn eastern_kingdoms_wdt_has_no_global_wmo() {
        let data = std::fs::read("data/terrain/775971.wdt").expect("azeroth WDT in data/terrain");
        assert!(parse_wdt_global_wmo(&data).expect("parses").is_none());
    }

    #[test]
    fn missing_mphd_is_an_error() {
        let data = wdt_payload(&[(b"REVM", 18u32.to_le_bytes().to_vec())]);

        assert!(parse_wdt_mphd_flags(&data).is_err());
    }
}
