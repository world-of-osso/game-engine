//! First authored camera snapshot from a version-274, chunked M2 file.

use super::{read_f32, read_u32, read_vec3};
use crate::asset::read_bytes::read_i32;

const CAMERA_ARRAY_OFFSET: usize = 0x110;
const CAMERA_RECORD_SIZE: usize = 116;
const TRACK_SIZE: usize = 20;
const ARRAY_ENTRY_SIZE: usize = 8;
const SPLINE_COMPONENTS: usize = 3;

/// Raw WoW model-local coordinates and diagonal field of view in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M2CameraSnapshot {
    pub camera_type: i32,
    pub position: [f32; 3],
    pub target: [f32; 3],
    pub roll: f32,
    pub fov: f32,
    pub near_clip: f32,
    pub far_clip: f32,
}

fn read_array(
    data: &[u8],
    offset: usize,
    entry_size: usize,
    label: &str,
) -> Result<(usize, usize), String> {
    let count = read_u32(data, offset).map_err(|error| format!("{label} count: {error}"))? as usize;
    let start =
        read_u32(data, offset + 4).map_err(|error| format!("{label} offset: {error}"))? as usize;
    let length = count
        .checked_mul(entry_size)
        .ok_or_else(|| format!("{label} length overflow"))?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| format!("{label} end overflow"))?;
    data.get(start..end)
        .ok_or_else(|| format!("{label} out of MD20 bounds at {start:#x}"))?;
    Ok((count, start))
}

fn find_first_spline_key(
    md20: &[u8],
    track: usize,
    key_size: usize,
    label: &str,
) -> Result<usize, String> {
    md20.get(track..track + TRACK_SIZE)
        .ok_or_else(|| format!("{label} track header truncated"))?;
    let (times_count, timestamps) = read_array(
        md20,
        track + 4,
        ARRAY_ENTRY_SIZE,
        &format!("{label} timestamps"),
    )?;
    let (values_count, values) = read_array(
        md20,
        track + 12,
        ARRAY_ENTRY_SIZE,
        &format!("{label} values"),
    )?;
    if times_count == 0 || times_count != values_count {
        return Err(format!(
            "{label} requires matching nonempty timestamp/value sequences"
        ));
    }
    let (time_keys, _) = read_array(md20, timestamps, 4, &format!("{label} first timestamps"))?;
    let (value_keys, first_key) =
        read_array(md20, values, key_size, &format!("{label} first values"))?;
    if time_keys == 0 || time_keys != value_keys {
        return Err(format!(
            "{label} first spline requires matching nonempty keys"
        ));
    }
    Ok(first_key)
}

fn read_first_spline<const N: usize>(
    md20: &[u8],
    track: usize,
    label: &str,
) -> Result<[f32; N], String> {
    let first_key = find_first_spline_key(md20, track, N * SPLINE_COMPONENTS * 4, label)?;
    let mut coordinates = [0.0; N];
    for (index, coordinate) in coordinates.iter_mut().enumerate() {
        *coordinate = read_f32(md20, first_key + index * 4)?;
    }
    Ok(coordinates)
}

fn parse_md20_camera(md20: &[u8]) -> Result<M2CameraSnapshot, String> {
    if md20.get(..4) != Some(b"MD20") {
        return Err("Invalid MD20 magic".into());
    }
    let version = read_u32(md20, 4)?;
    if version != 274 {
        return Err(format!(
            "Unsupported M2 camera version {version}; expected 274"
        ));
    }
    let (count, offset) = read_array(md20, CAMERA_ARRAY_OFFSET, CAMERA_RECORD_SIZE, "camera")?;
    if count == 0 {
        return Err("No authored camera in MD20".into());
    }
    let position_base =
        read_vec3(md20, offset + 32).map_err(|error| format!("camera position: {error}"))?;
    let target_base =
        read_vec3(md20, offset + 64).map_err(|error| format!("camera target: {error}"))?;
    let position_offset = read_first_spline::<3>(md20, offset + 12, "camera position")?;
    let target_offset = read_first_spline::<3>(md20, offset + 44, "camera target")?;
    let position = std::array::from_fn(|index| position_base[index] + position_offset[index]);
    let target = std::array::from_fn(|index| target_base[index] + target_offset[index]);
    Ok(M2CameraSnapshot {
        camera_type: read_i32(md20, offset)?,
        position,
        target,
        roll: read_first_spline::<1>(md20, offset + 76, "camera roll")?[0],
        fov: read_first_spline::<1>(md20, offset + 96, "camera fov")?[0],
        near_clip: read_f32(md20, offset + 8)?,
        far_clip: read_f32(md20, offset + 4)?,
    })
}

/// Extract camera index zero at its first track key; does not evaluate animation.
pub fn parse_camera_snapshot(m2_file: &[u8]) -> Result<M2CameraSnapshot, String> {
    let chunks = super::parse_chunks(m2_file)?;
    parse_md20_camera(chunks.md20)
}

#[cfg(test)]
mod tests {
    use super::parse_camera_snapshot;
    use std::path::PathBuf;

    const CAMERA_OFFSET: usize = 0x180;
    const MD20_SIZE: usize = 0x600;

    fn write_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_f32(data: &mut [u8], offset: usize, value: f32) {
        write_u32(data, offset, value.to_bits());
    }

    fn write_track(md20: &mut [u8], offset: usize, arrays: usize, key: &[f32]) {
        let timestamps = arrays;
        let values = arrays + 8;
        let timestamp = arrays + 16;
        let value = arrays + 20;
        write_u32(md20, offset + 4, 1);
        write_u32(md20, offset + 8, timestamps as u32);
        write_u32(md20, offset + 12, 1);
        write_u32(md20, offset + 16, values as u32);
        write_u32(md20, timestamps, 1);
        write_u32(md20, timestamps + 4, timestamp as u32);
        write_u32(md20, values, 1);
        write_u32(md20, values + 4, value as u32);
        for (index, coordinate) in key.iter().enumerate() {
            write_f32(md20, value + index * 4, *coordinate);
        }
    }

    fn model_with_camera() -> Vec<u8> {
        let mut md20 = vec![0; MD20_SIZE];
        md20[..4].copy_from_slice(b"MD20");
        write_u32(&mut md20, 4, 274);
        write_u32(&mut md20, 0x110, 1);
        write_u32(&mut md20, 0x114, CAMERA_OFFSET as u32);
        write_u32(&mut md20, CAMERA_OFFSET, (-1_i32) as u32);
        write_f32(&mut md20, CAMERA_OFFSET + 4, 500.0);
        write_f32(&mut md20, CAMERA_OFFSET + 8, 0.2);
        write_track(&mut md20, CAMERA_OFFSET + 12, 0x240, &[1.0, 2.0, 3.0]);
        write_track(&mut md20, CAMERA_OFFSET + 44, 0x300, &[4.0, 5.0, 6.0]);
        write_track(&mut md20, CAMERA_OFFSET + 76, 0x3c0, &[0.1]);
        write_track(&mut md20, CAMERA_OFFSET + 96, 0x420, &[0.8]);
        for (offset, coordinates) in [(32, [10.0, 20.0, 30.0]), (64, [40.0, 50.0, 60.0])] {
            for (index, coordinate) in coordinates.iter().enumerate() {
                write_f32(&mut md20, CAMERA_OFFSET + offset + index * 4, *coordinate);
            }
        }
        let mut file = b"MD21".to_vec();
        file.extend_from_slice(&(MD20_SIZE as u32).to_le_bytes());
        file.extend(md20);
        file
    }

    #[test]
    fn snapshot_uses_md20_relative_offsets_and_first_track_values() {
        let snapshot = parse_camera_snapshot(&model_with_camera()).unwrap();
        assert_eq!(snapshot.camera_type, -1);
        assert_eq!(snapshot.position, [11.0, 22.0, 33.0]);
        assert_eq!(snapshot.target, [44.0, 55.0, 66.0]);
        assert_eq!(snapshot.roll, 0.1);
        assert_eq!(snapshot.fov, 0.8);
        assert_eq!((snapshot.near_clip, snapshot.far_clip), (0.2, 500.0));
    }

    #[test]
    fn malformed_required_camera_data_fails_with_context() {
        let mut cases = Vec::new();
        let mut invalid_chunk = model_with_camera();
        invalid_chunk[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        cases.push(("chunk", invalid_chunk));
        let mut invalid_magic = model_with_camera();
        invalid_magic[8..12].copy_from_slice(b"NOPE");
        cases.push(("magic", invalid_magic));
        let mut invalid_version = model_with_camera();
        write_u32(&mut invalid_version, 8 + 4, 264);
        cases.push(("version", invalid_version));
        let mut no_camera = model_with_camera();
        write_u32(&mut no_camera, 8 + 0x110, 0);
        cases.push(("camera", no_camera));
        let mut invalid_record = model_with_camera();
        write_u32(&mut invalid_record, 8 + 0x114, (MD20_SIZE - 4) as u32);
        cases.push(("camera", invalid_record));
        let mut invalid_outer = model_with_camera();
        write_u32(&mut invalid_outer, 8 + CAMERA_OFFSET + 96 + 16, u32::MAX);
        cases.push(("fov", invalid_outer));
        let mut mismatched_keys = model_with_camera();
        write_u32(&mut mismatched_keys, 8 + 0x420, 2);
        cases.push(("fov", mismatched_keys));
        let mut no_fov_key = model_with_camera();
        write_u32(&mut no_fov_key, 8 + 0x420 + 8, 0);
        cases.push(("fov", no_fov_key));
        for (label, file) in cases {
            let error = parse_camera_snapshot(&file).unwrap_err();
            assert!(error.to_lowercase().contains(label), "{label}: {error}");
        }
    }

    #[test]
    fn authored_creation_cameras_match_inspected_values() {
        let expected = [
            (
                623712,
                [-4.364788, 3.848871, 0.5747803],
                [2.290539, -1.0632224, 1.0725104],
            ),
            (
                623714,
                [-4.9478707, 4.3812137, 0.65263426],
                [2.608921, -1.1962228, 1.0975605],
            ),
            (
                623716,
                [-4.9478707, 4.3812137, 0.65263426],
                [2.608921, -1.1962228, 1.2177819],
            ),
        ];
        for (id, position, target) in expected {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("data/models/{id}.m2"));
            let bytes =
                std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let camera = parse_camera_snapshot(&bytes).unwrap();
            assert_eq!(camera.camera_type, -1, "{id}");
            assert_eq!(camera.position, position, "{id}");
            assert_eq!(camera.target, target, "{id}");
            assert_eq!(camera.fov, 0.84570783, "{id}");
            assert_eq!(camera.roll, 0.0, "{id}");
            assert_eq!(camera.near_clip, 0.22222222, "{id}");
            assert_eq!(camera.far_clip, 513.9152, "{id}");
        }
    }
}
