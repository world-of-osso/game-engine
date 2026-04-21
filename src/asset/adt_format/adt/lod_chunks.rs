use super::*;

struct PendingLodLiquidGroup<'a> {
    header: Option<&'a [u8]>,
    indices: Option<&'a [u8]>,
}

impl<'a> PendingLodLiquidGroup<'a> {
    fn empty() -> Self {
        Self {
            header: None,
            indices: None,
        }
    }
}

pub(super) fn collect_lod_chunks(data: &[u8]) -> Result<LodRootChunks<'_>, String> {
    let mut root_chunks = empty_lod_root_chunks();
    let mut pending_liquid = PendingLodLiquidGroup::empty();

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        route_lod_chunk(&mut root_chunks, tag, payload, &mut pending_liquid)?;
    }

    ensure_complete_lod_liquid_group(&pending_liquid)?;
    ensure_required_lod_chunks_present(&root_chunks)?;

    Ok(root_chunks)
}

fn empty_lod_root_chunks<'a>() -> LodRootChunks<'a> {
    LodRootChunks {
        mver: None,
        mlhd: None,
        mlvh: None,
        mlll: None,
        mlnd: None,
        mlvi: None,
        mlsi: None,
        mlld: None,
        mldd: None,
        mldx: None,
        mlmd: None,
        mlmx: None,
        liquid_groups: Vec::new(),
    }
}

fn route_lod_chunk<'a>(
    root_chunks: &mut LodRootChunks<'a>,
    tag: &[u8; 4],
    payload: &'a [u8],
    pending_liquid: &mut PendingLodLiquidGroup<'a>,
) -> Result<(), String> {
    match tag {
        b"REVM" => root_chunks.mver = Some(payload),
        b"DHLM" => root_chunks.mlhd = Some(payload),
        b"HVLM" => root_chunks.mlvh = Some(payload),
        b"LLLM" => root_chunks.mlll = Some(payload),
        b"DNLM" => root_chunks.mlnd = Some(payload),
        b"IVLM" => root_chunks.mlvi = Some(payload),
        b"ISLM" => root_chunks.mlsi = Some(payload),
        b"DLLM" => root_chunks.mlld = Some(payload),
        b"DDLM" => root_chunks.mldd = Some(payload),
        b"XDLM" => root_chunks.mldx = Some(payload),
        b"DMLM" => root_chunks.mlmd = Some(payload),
        b"XMLM" => root_chunks.mlmx = Some(payload),
        b"NLLM" => {
            pending_liquid.header = Some(payload);
            pending_liquid.indices = None;
        }
        b"ILLM" => {
            if pending_liquid.header.is_some() {
                pending_liquid.indices = Some(payload);
            }
        }
        b"VLLM" => {
            push_lod_liquid_group(root_chunks, payload, pending_liquid)?;
        }
        _ => {}
    }

    Ok(())
}

fn push_lod_liquid_group<'a>(
    root_chunks: &mut LodRootChunks<'a>,
    vertices: &'a [u8],
    pending_liquid: &mut PendingLodLiquidGroup<'a>,
) -> Result<(), String> {
    let Some(header) = pending_liquid.header.take() else {
        return Err("VLLM encountered before NLLM in _lod.adt file".to_string());
    };
    let Some(indices) = pending_liquid.indices.take() else {
        return Err("VLLM encountered before ILLM in _lod.adt file".to_string());
    };
    root_chunks.liquid_groups.push(LodLiquidChunkGroup {
        header,
        indices,
        vertices,
    });

    Ok(())
}

fn ensure_complete_lod_liquid_group(
    pending_liquid: &PendingLodLiquidGroup<'_>,
) -> Result<(), String> {
    if pending_liquid.header.is_some() || pending_liquid.indices.is_some() {
        return Err("Incomplete MLLN/MLLI/MLLV liquid group in _lod.adt file".to_string());
    }

    Ok(())
}

fn ensure_required_lod_chunks_present(root_chunks: &LodRootChunks<'_>) -> Result<(), String> {
    ensure_lod_chunk_present(
        root_chunks.mver,
        "No REVM (MVER) chunk found in _lod.adt file",
    )?;
    ensure_lod_chunk_present(
        root_chunks.mlhd,
        "No DHLM (MLHD) chunk found in _lod.adt file",
    )?;
    ensure_lod_chunk_present(
        root_chunks.mlvh,
        "No HVLM (MLVH) chunk found in _lod.adt file",
    )?;
    ensure_lod_chunk_present(
        root_chunks.mlll,
        "No LLLM (MLLL) chunk found in _lod.adt file",
    )?;
    ensure_lod_chunk_present(
        root_chunks.mlnd,
        "No DNLM (MLND) chunk found in _lod.adt file",
    )?;
    ensure_lod_chunk_present(
        root_chunks.mlvi,
        "No IVLM (MLVI) chunk found in _lod.adt file",
    )?;
    ensure_lod_chunk_present(
        root_chunks.mlsi,
        "No ISLM (MLSI) chunk found in _lod.adt file",
    )?;
    Ok(())
}

fn ensure_lod_chunk_present(chunk: Option<&[u8]>, missing_error: &str) -> Result<(), String> {
    if chunk.is_none() {
        return Err(missing_error.to_string());
    }
    Ok(())
}
