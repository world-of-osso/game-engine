use bevy::mesh::{Indices, VertexAttributeValues};

pub fn estimate_indices_bytes(indices: &Indices) -> u64 {
    match indices {
        Indices::U16(values) => slice_bytes(values.as_slice()),
        Indices::U32(values) => slice_bytes(values.as_slice()),
    }
}

pub fn estimate_vertex_attribute_bytes(values: &VertexAttributeValues) -> u64 {
    match values {
        VertexAttributeValues::Float32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm8x4(values) => slice_bytes(values.as_slice()),
    }
}

pub fn slice_bytes<T>(values: &[T]) -> u64 {
    std::mem::size_of_val(values) as u64
}
