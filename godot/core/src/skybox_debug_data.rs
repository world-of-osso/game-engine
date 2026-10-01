//! Cached authored skybox metadata decoded with the original WDC5 reader.
use std::{fs, path::Path};

#[path = "../../../src/rendering/lighting/light_lookup_wdc5.rs"]
mod wdc5;

use crate::light_lookup_types::{LightParamsFlags, LightSkyboxFlags, LightSkyboxMetadata};
use wdc5::ParsedWdc5Db2;

const LIGHT_SKYBOX_FDID: u32 = 1_308_501;
const LIGHT_PARAMS_FDID: u32 = 1_334_669;
const SKYBOX_LAYOUTS: &[u32] = &[0x9D49_56FF, 0x407F_EBCF, 0xD466_A5C2];
const PARAMS_LAYOUT: u32 = 0xCAE3_94E7;
const SKYBOX_FLAGS_FIELD: usize = 1;
const SKYBOX_MODEL_FIELD: usize = 2;
const PARAMS_SKYBOX_FIELD: usize = 3;
const PARAMS_FLAGS_FIELD: usize = 10;

pub fn read_light_skybox(data_root: &Path, id: u32) -> Result<(u32, LightSkyboxFlags), String> {
    let bytes = read_table(data_root, LIGHT_SKYBOX_FDID)?;
    let table = ParsedWdc5Db2::parse_any_layout(&bytes, SKYBOX_LAYOUTS)?;
    let row = table
        .rows()
        .into_iter()
        .find(|row| table.row_id(*row) == id)
        .ok_or_else(|| format!("LightSkybox {id} is absent from cached DB2"))?;
    let metadata = LightSkyboxMetadata {
        fdid: table.decode_field(row, SKYBOX_MODEL_FIELD),
        flags: LightSkyboxFlags::from_bits(table.decode_field(row, SKYBOX_FLAGS_FIELD)),
    };
    if metadata.fdid == 0 {
        return Err(format!("LightSkybox {id} has no authored model FileDataID"));
    }
    Ok((metadata.fdid, metadata.flags))
}

pub fn read_light_params_skybox(
    data_root: &Path,
    id: u32,
) -> Result<(u32, LightParamsFlags), String> {
    let bytes = read_table(data_root, LIGHT_PARAMS_FDID)?;
    let table = ParsedWdc5Db2::parse(&bytes, PARAMS_LAYOUT)?;
    let row = table
        .rows()
        .into_iter()
        .find(|row| table.row_id(*row) == id)
        .ok_or_else(|| format!("LightParams {id} is absent from cached DB2"))?;
    Ok((
        table.decode_field(row, PARAMS_SKYBOX_FIELD),
        LightParamsFlags::from_bits(table.decode_field(row, PARAMS_FLAGS_FIELD)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_authored_skybox_and_light_params_keep_original_ids_and_flags() {
        let root = Path::new("data");
        let (fdid, flags) = read_light_skybox(root, 653).unwrap();
        assert_eq!(fdid, 5_412_968);
        assert_eq!(flags.bits(), 0b01111);
        let (skybox_id, _) = read_light_params_skybox(root, 5615).unwrap();
        assert_eq!(skybox_id, 653);
    }

    #[test]
    fn unknown_cached_skybox_id_is_an_explicit_error_not_another_model() {
        let error = read_light_skybox(Path::new("data"), u32::MAX).unwrap_err();
        assert_eq!(error, "LightSkybox 4294967295 is absent from cached DB2");
    }
}

fn read_table(data_root: &Path, fdid: u32) -> Result<Vec<u8>, String> {
    let path = data_root.join("dbfilesclient").join(format!("{fdid}.db2"));
    fs::read(&path).map_err(|error| format!("Cannot read {}: {error}", path.display()))
}
