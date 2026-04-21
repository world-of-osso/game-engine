use bevy::mesh::{Indices, VertexAttributeValues};

pub fn estimate_indices_bytes(indices: &Indices) -> u64 {
    match indices {
        Indices::U16(values) => slice_bytes(values.as_slice()),
        Indices::U32(values) => slice_bytes(values.as_slice()),
    }
}

pub fn estimate_vertex_attribute_bytes(values: &VertexAttributeValues) -> u64 {
    values.get_bytes().len() as u64
}

pub fn slice_bytes<T>(values: &[T]) -> u64 {
    std::mem::size_of_val(values) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_vertex_attribute_bytes_matches_vertex_bytes() {
        let values = VertexAttributeValues::Float32x3(vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
        assert_eq!(
            estimate_vertex_attribute_bytes(&values),
            values.get_bytes().len() as u64
        );
    }

    #[test]
    fn estimate_indices_bytes_matches_index_storage() {
        let indices = Indices::U16(vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(estimate_indices_bytes(&indices), 12);
    }
}
