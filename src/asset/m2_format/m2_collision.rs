//! Authored static collision geometry from the MD20 payload of an MD21 model.

use std::mem::size_of;

use super::{read_u16, read_vec3};
use crate::asset::read_bytes::read_m2_array_header;

const COLLISION_BOUNDS_OFFSET: usize = 0xBC;
const COLLISION_INDICES_OFFSET: usize = 0xD8;
const COLLISION_VERTICES_OFFSET: usize = 0xE0;
const VECTOR_BYTES: usize = 12;

/// Positions and bounds remain in WoW model-local coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct M2CollisionMesh {
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub vertices: Vec<[f32; 3]>,
    /// Triangle-list indices, three per face.
    pub indices: Vec<u16>,
}

pub(crate) fn parse_collision_mesh(md20: &[u8]) -> Result<Option<M2CollisionMesh>, String> {
    if md20.len() < COLLISION_VERTICES_OFFSET + 8 {
        return Err("M2 collision array headers are truncated".into());
    }
    let (index_count, index_offset) = read_m2_array_header(md20, COLLISION_INDICES_OFFSET)?;
    if index_count == 0 {
        return Ok(None);
    }
    if !index_count.is_multiple_of(3) {
        return Err(format!(
            "M2 collision index count {index_count} is not a triangle list"
        ));
    }
    let (vertex_count, vertex_offset) = read_m2_array_header(md20, COLLISION_VERTICES_OFFSET)?;
    let vertices = read_collision_vertices(md20, vertex_count, vertex_offset)?;
    let indices = read_collision_indices(md20, index_count, index_offset, vertex_count)?;
    let bounds_min = read_vec3(md20, COLLISION_BOUNDS_OFFSET)?;
    let bounds_max = read_vec3(md20, COLLISION_BOUNDS_OFFSET + VECTOR_BYTES)?;
    if !valid_bounds(bounds_min, bounds_max) {
        return Err("M2 collision bounds must be finite and ordered".into());
    }
    Ok(Some(M2CollisionMesh {
        bounds_min,
        bounds_max,
        vertices,
        indices,
    }))
}

fn valid_bounds(min: [f32; 3], max: [f32; 3]) -> bool {
    min.into_iter()
        .zip(max)
        .all(|(min, max)| min.is_finite() && max.is_finite() && min <= max)
}

fn validate_array(
    data: &[u8],
    count: usize,
    offset: usize,
    stride: usize,
    label: &str,
) -> Result<(), String> {
    let end = count
        .checked_mul(stride)
        .and_then(|bytes| offset.checked_add(bytes));
    if end.is_none_or(|end| data.get(offset..end).is_none()) {
        return Err(format!(
            "M2 collision {label} array is out of bounds: count={count}, offset={offset:#x}"
        ));
    }
    Ok(())
}

fn read_collision_vertices(
    data: &[u8],
    count: usize,
    offset: usize,
) -> Result<Vec<[f32; 3]>, String> {
    validate_array(data, count, offset, VECTOR_BYTES, "vertex")?;
    (0..count)
        .map(|index| {
            let position = read_vec3(data, offset + index * VECTOR_BYTES)?;
            if position.iter().any(|value| !value.is_finite()) {
                return Err(format!("M2 collision vertex {index} is not finite"));
            }
            Ok(position)
        })
        .collect()
}

fn read_collision_indices(
    data: &[u8],
    count: usize,
    offset: usize,
    vertex_count: usize,
) -> Result<Vec<u16>, String> {
    validate_array(data, count, offset, size_of::<u16>(), "index")?;
    (0..count).map(|index| {
        let vertex = read_u16(data, offset + index * size_of::<u16>())?;
        if usize::from(vertex) >= vertex_count {
            return Err(format!("M2 collision index {index} references vertex {vertex}, but only {vertex_count} exist"));
        }
        Ok(vertex)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle_data() -> Vec<u8> {
        let mut data = vec![0; 320];
        for (index, value) in [-1.0_f32, -1.0, 0.0, 1.0, 1.0, 0.0].into_iter().enumerate() {
            let offset = COLLISION_BOUNDS_OFFSET + index * 4;
            data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (offset, value) in [
            (COLLISION_INDICES_OFFSET, 3_u32),
            (COLLISION_INDICES_OFFSET + 4, 256),
            (COLLISION_VERTICES_OFFSET, 3),
            (COLLISION_VERTICES_OFFSET + 4, 272),
        ] {
            data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (index, value) in [0_u16, 1, 2].into_iter().enumerate() {
            data[256 + index * 2..258 + index * 2].copy_from_slice(&value.to_le_bytes());
        }
        for (index, vertex) in [[-1.0_f32, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]]
            .into_iter()
            .enumerate()
        {
            for (axis, value) in vertex.into_iter().enumerate() {
                let offset = 272 + index * VECTOR_BYTES + axis * 4;
                data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        data
    }

    #[test]
    fn authored_collision_parses_bounds_vertices_and_indices() {
        let mesh = parse_collision_mesh(&triangle_data()).unwrap().unwrap();
        assert_eq!(mesh.bounds_min, [-1.0, -1.0, 0.0]);
        assert_eq!(mesh.bounds_max, [1.0, 1.0, 0.0]);
        assert_eq!(
            mesh.vertices,
            [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]]
        );
        assert_eq!(mesh.indices, [0, 1, 2]);
    }

    #[test]
    fn authored_collision_empty_indices_mean_no_solid_geometry() {
        let mut data = triangle_data();
        data[COLLISION_INDICES_OFFSET..COLLISION_INDICES_OFFSET + 4]
            .copy_from_slice(&0_u32.to_le_bytes());
        assert!(parse_collision_mesh(&data).unwrap().is_none());
    }

    #[test]
    fn authored_collision_rejects_truncated_vertices() {
        assert!(
            parse_collision_mesh(&triangle_data()[..300])
                .unwrap_err()
                .contains("vertex array")
        );
    }

    #[test]
    fn authored_collision_rejects_invalid_vertex_reference() {
        let mut data = triangle_data();
        data[256..258].copy_from_slice(&7_u16.to_le_bytes());
        assert!(
            parse_collision_mesh(&data)
                .unwrap_err()
                .contains("references vertex 7")
        );
    }

    #[test]
    fn authored_collision_rejects_incomplete_triangle() {
        let mut data = triangle_data();
        data[COLLISION_INDICES_OFFSET..COLLISION_INDICES_OFFSET + 4]
            .copy_from_slice(&2_u32.to_le_bytes());
        assert!(
            parse_collision_mesh(&data)
                .unwrap_err()
                .contains("not a triangle list")
        );
    }
}
