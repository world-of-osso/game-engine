//! WDT MPHD header flags that select how a map's `_tex0` terrain textures are stored and blended,
//! and the single WMO of a map made of one (a WMO-only dungeon such as the Stockade).

use std::collections::{BTreeMap, BTreeSet};

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

/// FileDataIDs in one MAID slot, indexed in MAIN order (second tile coordinate × 64 + first).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TileFileIds {
    pub root: u32,
    pub obj0: u32,
    pub obj1: u32,
    pub tex0: u32,
    pub lod: u32,
    pub map_texture: u32,
    pub map_normal: u32,
    pub minimap: u32,
}

#[derive(Debug, Clone, Default)]
pub struct WdtTiles {
    pub active: BTreeSet<(u32, u32)>,
    /// None means this WDT uses named listfile companions, not MAID.
    pub maid: Option<BTreeMap<(u32, u32), TileFileIds>>,
}

impl WdtTiles {
    pub fn file_ids(&self, first: u32, second: u32) -> Option<&TileFileIds> {
        self.maid.as_ref()?.get(&(first, second))
    }
}

pub fn parse_wdt_tiles(data: &[u8]) -> Result<WdtTiles, String> {
    let mut main = None;
    let mut maid = None;
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"NIAM" => main = Some(payload),
            b"DIAM" => maid = Some(payload),
            _ => {}
        }
    }
    let main = main.ok_or("WDT missing MAIN chunk")?;
    if main.len() != 4096 * 8 {
        return Err(format!("WDT MAIN has {} bytes, expected 32768", main.len()));
    }
    if let Some(maid) = maid
        && maid.len() != 4096 * 32
    {
        return Err(format!(
            "WDT MAID has {} bytes, expected 131072",
            maid.len()
        ));
    }
    let mut tiles = WdtTiles {
        active: BTreeSet::new(),
        maid: maid.map(|_| BTreeMap::new()),
    };
    for index in 0..4096 {
        if read_u32(main, index * 8)? & 1 == 0 {
            continue;
        }
        let tile = ((index % 64) as u32, (index / 64) as u32);
        tiles.active.insert(tile);
        if let (Some(payload), Some(ids)) = (maid, tiles.maid.as_mut()) {
            let offset = index * 32;
            ids.insert(
                tile,
                TileFileIds {
                    root: read_u32(payload, offset)?,
                    obj0: read_u32(payload, offset + 4)?,
                    obj1: read_u32(payload, offset + 8)?,
                    tex0: read_u32(payload, offset + 12)?,
                    lod: read_u32(payload, offset + 16)?,
                    map_texture: read_u32(payload, offset + 20)?,
                    map_normal: read_u32(payload, offset + 24)?,
                    minimap: read_u32(payload, offset + 28)?,
                },
            );
        }
    }
    Ok(tiles)
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
    fn zephras_maid_identifies_all_active_tiles_and_sample_companions() {
        let data = std::fs::read("data/terrain/7198644.wdt").unwrap();
        let tiles = parse_wdt_tiles(&data).unwrap();
        assert_eq!(tiles.active.len(), 72);
        let sample = tiles.file_ids(29, 26).unwrap();
        assert_eq!(sample.root, 7199999);
        assert_eq!(sample.obj0, 7200000);
        assert_eq!(sample.obj1, 7200001);
        assert_eq!(sample.tex0, 7200002);
        assert_eq!(sample.lod, 7200003);
        assert_eq!(sample.map_texture, 7199321);
        assert_eq!(sample.map_normal, 7199345);
        assert_eq!(sample.minimap, 7199297);
        assert!(tiles.file_ids(0, 0).is_none());
    }

    #[test]
    fn zephras_reader_coordinates_also_address_retail_maid_ids() {
        let data = std::fs::read("data/terrain/775971.wdt").unwrap();
        let tiles = parse_wdt_tiles(&data).unwrap();
        assert!(tiles.active.contains(&(32, 48)));
        assert_eq!(tiles.file_ids(32, 48).unwrap().root, 778027);
    }

    #[test]
    fn missing_mphd_is_an_error() {
        let data = wdt_payload(&[(b"REVM", 18u32.to_le_bytes().to_vec())]);

        assert!(parse_wdt_mphd_flags(&data).is_err());
    }
}
