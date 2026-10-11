//! Narrow decoder for the D4 Appearance GeoChunk layout documented in d4data.

#[derive(Debug, PartialEq, Eq)]
pub struct MeshCount {
    pub chunk: usize,
    pub lod: usize,
    pub subobject: usize,
    pub segment: usize,
    pub flags: u32,
    pub hash: u32,
    pub vertices: usize,
    pub triangles: usize,
}

// Field offsets and record sizes: DiabloTools/d4data AppearanceDefinition,
// Structure, GeoChunk, GeoSet, SubObject and SubObjectSegment (MIT).
const SNO_HEADER_SIZE: usize = 16;
const APPEARANCE_SIZE: usize = 312;
const CHUNKS_FIELD: usize = SNO_HEADER_SIZE + 8 + 32;
const CHUNK_SIZE: usize = 88;
const LODS_FIELD: usize = 24;
const VERTEX_BUFFERS_FIELD: usize = 40;
const INDEX_BUFFERS_FIELD: usize = 56;
const LOD_SIZE: usize = 40;
const SUBOBJECT_SIZE: usize = 240;
const SUBOBJECT_HASH: usize = 100;
const VERTEX_BUFFER_INDEX: usize = 104;
const INDEX_BUFFER_INDEX: usize = 108;
const SEGMENTS_FIELD: usize = 200;
const SEGMENT_SIZE: usize = 32;
const VERTEX_COUNT: usize = 16;
const VERTEX_OFFSET: usize = 20;
const INDEX_COUNT: usize = 24;
const INDEX_OFFSET: usize = 28;
const VERTEX_BUFFER_SIZE: usize = 80;
const VERTEX_DATA_FIELD: usize = 48;
const INDEX_BUFFER_SIZE: usize = 24;
const INDEX_WIDTH: usize = 2;
const TRIANGLE_CORNERS: usize = 3;
const VERTEX_FORMAT: u32 = 6;
const VERTEX_STRIDE: usize = 44;

struct VertexBuffer<'a> {
    bytes: &'a [u8],
    stride: usize,
}

struct ChunkBuffers<'a> {
    vertices: Vec<VertexBuffer<'a>>,
    indices: Vec<&'a [u8]>,
}

pub fn count_triangles(meta: &[u8], payload: &[u8]) -> Result<Vec<MeshCount>, String> {
    validate_asset_pair(meta, payload)?;
    let chunks = read_records(meta, CHUNKS_FIELD, CHUNK_SIZE)?;
    let mut counts = Vec::new();
    for (chunk_index, chunk) in chunks.into_iter().enumerate() {
        counts.extend(count_chunk(meta, payload, chunk, chunk_index)?);
    }
    if counts.is_empty() {
        return Err("No supported geometry segments in Appearance".into());
    }
    Ok(counts)
}

fn validate_asset_pair(meta: &[u8], payload: &[u8]) -> Result<(), String> {
    if read_u32(meta, 0)? != 0xdead_beef {
        return Err("Expected Appearance metadata magic 0xDEADBEEF".into());
    }
    read_span(meta, SNO_HEADER_SIZE, APPEARANCE_SIZE)?;
    if read_u32(meta, SNO_HEADER_SIZE)? != read_u32(payload, 0)? {
        return Err("Metadata and payload SNO ID differ".into());
    }
    Ok(())
}

fn count_chunk(
    meta: &[u8],
    payload: &[u8],
    chunk: usize,
    chunk_index: usize,
) -> Result<Vec<MeshCount>, String> {
    let vertices = read_records(meta, chunk + VERTEX_BUFFERS_FIELD, VERTEX_BUFFER_SIZE)?
        .into_iter()
        .map(|at| read_vertex_buffer(meta, payload, at))
        .collect::<Result<_, _>>()?;
    let indices = read_records(meta, chunk + INDEX_BUFFERS_FIELD, INDEX_BUFFER_SIZE)?
        .into_iter()
        .map(|at| read_index_buffer(meta, payload, at))
        .collect::<Result<_, _>>()?;
    let buffers = ChunkBuffers { vertices, indices };
    let mut counts = Vec::new();
    for (lod_index, lod) in read_records(meta, chunk + LODS_FIELD, LOD_SIZE)?
        .into_iter()
        .enumerate()
    {
        for (subobject_index, subobject) in read_records(meta, lod, SUBOBJECT_SIZE)?
            .into_iter()
            .enumerate()
        {
            let location = [chunk_index, lod_index, subobject_index];
            counts.extend(count_subobject(meta, &buffers, subobject, location)?);
        }
    }
    Ok(counts)
}

fn read_vertex_buffer<'a>(
    meta: &[u8],
    payload: &'a [u8],
    at: usize,
) -> Result<VertexBuffer<'a>, String> {
    let stride = read_u32(meta, at + 4)? as usize;
    if read_u32(meta, at)? != VERTEX_FORMAT || stride != VERTEX_STRIDE {
        return Err("Unsupported vertex format/stride; expected format 6, stride 44".into());
    }
    // vfid, vbid and baid must describe inline payload data, not external resources.
    for field in [40, 64, 68] {
        if read_u32(meta, at + field)? != u32::MAX {
            return Err("Unsupported external vertex buffer".into());
        }
    }
    let bytes = read_payload_blob(meta, payload, at + VERTEX_DATA_FIELD)?;
    if !bytes.len().is_multiple_of(stride) {
        return Err("Vertex buffer byte length is not divisible by stride".into());
    }
    Ok(VertexBuffer { bytes, stride })
}

fn read_index_buffer<'a>(meta: &[u8], payload: &'a [u8], at: usize) -> Result<&'a [u8], String> {
    if read_u32(meta, at + 16)? != u32::MAX {
        return Err("Unsupported external index buffer".into());
    }
    let bytes = read_payload_blob(meta, payload, at)?;
    if !bytes.len().is_multiple_of(INDEX_WIDTH) {
        return Err("Index buffer length is not divisible by 16-bit index width".into());
    }
    Ok(bytes)
}

fn count_subobject(
    meta: &[u8],
    buffers: &ChunkBuffers<'_>,
    at: usize,
    location: [usize; 3],
) -> Result<Vec<MeshCount>, String> {
    let (vertices, indices) = select_subobject_buffers(meta, buffers, at)?;
    let flags = read_u32(meta, at)?;
    let hash = read_u32(meta, at + SUBOBJECT_HASH)?;
    let mut counts = Vec::new();
    for (segment, record) in read_records(meta, at + SEGMENTS_FIELD, SEGMENT_SIZE)?
        .into_iter()
        .enumerate()
    {
        let (vertex_count, triangles) =
            count_segment(meta, vertices, indices, record).map_err(|error| {
                format!(
                    "chunk {} LOD {} subobject {} segment {segment}: {error}",
                    location[0], location[1], location[2]
                )
            })?;
        counts.push(MeshCount {
            chunk: location[0],
            lod: location[1],
            subobject: location[2],
            segment,
            flags,
            hash,
            vertices: vertex_count,
            triangles,
        });
    }
    Ok(counts)
}

fn select_subobject_buffers<'a>(
    meta: &[u8],
    buffers: &'a ChunkBuffers<'_>,
    at: usize,
) -> Result<(&'a VertexBuffer<'a>, &'a [u8]), String> {
    let vb_index = read_nonnegative(meta, at + VERTEX_BUFFER_INDEX)?;
    let ib_index = read_nonnegative(meta, at + INDEX_BUFFER_INDEX)?;
    let vertices = buffers
        .vertices
        .get(vb_index)
        .ok_or("Invalid vertex buffer index")?;
    let indices = buffers
        .indices
        .get(ib_index)
        .ok_or("Invalid index buffer index")?;
    Ok((vertices, indices))
}

fn count_segment(
    meta: &[u8],
    vertices: &VertexBuffer<'_>,
    indices: &[u8],
    at: usize,
) -> Result<(usize, usize), String> {
    let vertex_count = read_nonnegative(meta, at + VERTEX_COUNT)?;
    let vertex_offset = read_nonnegative(meta, at + VERTEX_OFFSET)?;
    let index_count = read_nonnegative(meta, at + INDEX_COUNT)?;
    let index_offset = read_nonnegative(meta, at + INDEX_OFFSET)?;
    if !index_count.is_multiple_of(TRIANGLE_CORNERS) {
        return Err("Index count does not describe a complete triangle list".into());
    }
    if !vertex_offset.is_multiple_of(vertices.stride) {
        return Err("Vertex byte offset is not aligned to stride".into());
    }
    read_span(
        vertices.bytes,
        vertex_offset,
        checked_scale(vertex_count, vertices.stride)?,
    )?;
    let index_bytes = read_span(
        indices,
        checked_scale(index_offset, INDEX_WIDTH)?,
        checked_scale(index_count, INDEX_WIDTH)?,
    )?;
    validate_segment_indices(index_bytes, vertex_count)?;
    Ok((vertex_count, index_count / TRIANGLE_CORNERS))
}

fn validate_segment_indices(index_bytes: &[u8], vertex_count: usize) -> Result<(), String> {
    for index in index_bytes.chunks_exact(INDEX_WIDTH) {
        let value = u16::from_le_bytes([index[0], index[1]]) as usize;
        if value >= vertex_count {
            return Err(format!(
                "Triangle index {value} outside {vertex_count} segment vertices"
            ));
        }
    }
    Ok(())
}

fn checked_scale(count: usize, width: usize) -> Result<usize, String> {
    count
        .checked_mul(width)
        .ok_or_else(|| "Byte range overflow".into())
}

fn read_span(bytes: &[u8], start: usize, length: usize) -> Result<&[u8], String> {
    let end = start.checked_add(length).ok_or("Byte range overflow")?;
    bytes
        .get(start..end)
        .ok_or_else(|| format!("Byte range {start}..{end} exceeds bounds {}", bytes.len()))
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, String> {
    let word = read_span(bytes, at, 4)?;
    Ok(u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
}

fn read_nonnegative(bytes: &[u8], at: usize) -> Result<usize, String> {
    let value = read_u32(bytes, at)? as i32;
    usize::try_from(value).map_err(|_| format!("Unsupported negative count/offset {value} at {at}"))
}

fn read_array_range(meta: &[u8], field: usize) -> Result<(usize, usize), String> {
    if read_span(meta, field, 8)? != [0; 8] {
        return Err(format!("Unsupported serialized array padding at {field}"));
    }
    Ok((
        read_u32(meta, field + 8)? as usize,
        read_u32(meta, field + 12)? as usize,
    ))
}

fn read_records(meta: &[u8], field: usize, record_size: usize) -> Result<Vec<usize>, String> {
    let (offset, size) = read_array_range(meta, field)?;
    if !size.is_multiple_of(record_size) {
        return Err(format!(
            "Array at {field} is not divisible by record size {record_size}"
        ));
    }
    let start = offset
        .checked_add(SNO_HEADER_SIZE)
        .ok_or("Metadata offset overflow")?;
    read_span(meta, start, size)?;
    Ok((start..start + size).step_by(record_size).collect())
}

fn read_payload_blob<'a>(meta: &[u8], payload: &'a [u8], field: usize) -> Result<&'a [u8], String> {
    let (offset, size) = read_array_range(meta, field)?;
    read_span(payload, offset, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_array(meta: &mut [u8], field: usize, target: usize, length: usize) {
        put_u32(meta, field + 8, (target - 16) as u32);
        put_u32(meta, field + 12, length as u32);
    }

    fn fixture() -> (Vec<u8>, Vec<u8>) {
        let mut meta = vec![0; 1400];
        let mut payload = vec![0; 320];
        put_u32(&mut meta, 0, 0xdead_beef);
        put_u32(&mut meta, 16, 123);
        put_u32(&mut payload, 0, 123);
        put_array(&mut meta, 56, 328, 88);
        put_array(&mut meta, 352, 416, 80);
        put_array(&mut meta, 368, 1000, 80);
        put_array(&mut meta, 384, 1080, 24);
        put_array(&mut meta, 416, 496, 240);
        put_array(&mut meta, 456, 736, 240);
        for (subobject, segment, vertex_count, index_count, index_offset) in
            [(496, 1120, 4, 6, 0), (736, 1152, 3, 3, 6)]
        {
            put_u32(&mut meta, subobject, 0x200000);
            put_u32(&mut meta, subobject + 100, 0xaabbccdd);
            put_array(&mut meta, subobject + 200, segment, 32);
            put_u32(&mut meta, segment + 16, vertex_count);
            put_u32(&mut meta, segment + 24, index_count);
            put_u32(&mut meta, segment + 28, index_offset);
        }
        put_u32(&mut meta, 1000, 6);
        put_u32(&mut meta, 1004, 44);
        put_u32(&mut meta, 1040, u32::MAX);
        put_u32(&mut meta, 1056, 128);
        put_u32(&mut meta, 1060, 176);
        put_u32(&mut meta, 1064, u32::MAX);
        put_u32(&mut meta, 1068, u32::MAX);
        put_u32(&mut meta, 1088, 32);
        put_u32(&mut meta, 1092, 18);
        put_u32(&mut meta, 1096, u32::MAX);
        for (index, value) in [0u16, 1, 2, 0, 2, 3, 0, 1, 2].iter().enumerate() {
            payload[32 + index * 2..34 + index * 2].copy_from_slice(&value.to_le_bytes());
        }
        (meta, payload)
    }

    #[test]
    fn counts_each_lod_segment_without_combining_alternatives() {
        let (meta, payload) = fixture();
        let counts = count_triangles(&meta, &payload).unwrap();
        assert_eq!(
            counts,
            vec![
                MeshCount {
                    chunk: 0,
                    lod: 0,
                    subobject: 0,
                    segment: 0,
                    flags: 0x200000,
                    hash: 0xaabbccdd,
                    vertices: 4,
                    triangles: 2
                },
                MeshCount {
                    chunk: 0,
                    lod: 1,
                    subobject: 0,
                    segment: 0,
                    flags: 0x200000,
                    hash: 0xaabbccdd,
                    vertices: 3,
                    triangles: 1
                },
            ]
        );
    }

    #[test]
    fn counts_multiple_segments_without_merging_their_vertex_ranges() {
        let (mut meta, payload) = fixture();
        let second_segment = meta[1152..1184].to_vec();
        meta[1184..1216].copy_from_slice(&second_segment);
        put_array(&mut meta, 696, 1120, 64);
        put_array(&mut meta, 936, 1184, 32);
        let counts = count_triangles(&meta, &payload).unwrap();
        assert_eq!(counts.len(), 3);
        assert_eq!(
            (counts[0].lod, counts[0].segment, counts[0].triangles),
            (0, 0, 2)
        );
        assert_eq!(
            (counts[1].lod, counts[1].segment, counts[1].triangles),
            (0, 1, 1)
        );
        assert_eq!(
            (counts[2].lod, counts[2].segment, counts[2].triangles),
            (1, 0, 1)
        );
    }

    #[test]
    fn interprets_vertex_offsets_in_bytes_and_index_offsets_in_elements() {
        let (mut meta, mut payload) = fixture();
        put_u32(&mut meta, 1136, 3);
        put_u32(&mut meta, 1140, 44);
        payload[42..44].copy_from_slice(&1u16.to_le_bytes());
        let counts = count_triangles(&meta, &payload).unwrap();
        assert_eq!((counts[0].vertices, counts[0].triangles), (3, 2));
        assert_eq!((counts[1].vertices, counts[1].triangles), (3, 1));
    }

    #[test]
    fn rejects_negative_segment_ranges() {
        let (mut meta, payload) = fixture();
        put_u32(&mut meta, 1136, u32::MAX);
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("negative")
        );
    }

    #[test]
    fn rejects_missing_buffer_references() {
        let (mut meta, payload) = fixture();
        put_u32(&mut meta, 600, 2);
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("buffer index")
        );
    }

    #[test]
    fn rejects_wrong_metadata_magic() {
        let (mut meta, payload) = fixture();
        put_u32(&mut meta, 0, 0);
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("magic")
        );
    }

    #[test]
    fn rejects_out_of_range_indices() {
        let (meta, mut payload) = fixture();
        payload[32..34].copy_from_slice(&4u16.to_le_bytes());
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("index 4")
        );
    }

    #[test]
    fn rejects_a_different_payload_asset() {
        let (meta, mut payload) = fixture();
        put_u32(&mut payload, 0, 999);
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("SNO ID")
        );
    }

    #[test]
    fn rejects_incomplete_triangles() {
        let (mut meta, payload) = fixture();
        put_u32(&mut meta, 1144, 5);
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("triangle list")
        );
    }

    #[test]
    fn rejects_truncated_metadata_and_payload() {
        let (meta, payload) = fixture();
        assert!(
            count_triangles(&meta[..1100], &payload)
                .unwrap_err()
                .contains("bounds")
        );
        assert!(
            count_triangles(&meta, &payload[..150])
                .unwrap_err()
                .contains("bounds")
        );
    }

    #[test]
    fn rejects_external_vertex_buffers() {
        let (mut meta, payload) = fixture();
        put_u32(&mut meta, 1064, 7);
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("external")
        );
    }

    #[test]
    fn rejects_unsupported_vertex_format() {
        let (mut meta, payload) = fixture();
        put_u32(&mut meta, 1000, 99);
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("vertex format")
        );
    }

    #[test]
    fn rejects_malformed_record_array() {
        let (mut meta, payload) = fixture();
        put_u32(&mut meta, 364, 79);
        assert!(
            count_triangles(&meta, &payload)
                .unwrap_err()
                .contains("record size")
        );
    }
}
