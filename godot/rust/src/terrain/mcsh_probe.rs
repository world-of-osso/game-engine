//! Cache-only S3 acceptance entry. It never changes runtime WDT tile selection.
use std::{collections::BTreeMap, fs, path::Path, sync::Arc};

use game_engine_core::{adt, blp, wdt};
use godot::{
    classes::{IRefCounted, MeshInstance3D, RefCounted},
    prelude::*,
};

use super::{
    assets::NativeTerrainTile,
    material::TerrainMaterials,
    textures::{TerrainLayerTextures, TerrainTexture},
};

const CHUNK: usize = 7;

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct WowTerrainShadowProbe {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for WowTerrainShadowProbe {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl WowTerrainShadowProbe {
    /// 0 zero, 1 all set, 2 left-half set, 3 missing, 4 authentic cached payload.
    #[func]
    fn chunk(&self, mask_mode: i32) -> VarDictionary {
        let mut result = VarDictionary::new();
        match read_fixture_tile().and_then(|tile| build_fixture_chunk(tile, mask_mode)) {
            Ok((node, source, mask)) => {
                result.set("node", &node);
                result.set("source_mask", &PackedByteArray::from(source.as_slice()));
                result.set(
                    "mask",
                    &PackedByteArray::from(mask.as_ref().map_or(&[][..], |m| m.as_slice())),
                );
            }
            Err(error) => result.set("error", error),
        }
        result
    }
}

type FixtureChunk = (Gd<MeshInstance3D>, [u8; 512], Option<[u8; 512]>);

fn build_fixture_chunk(mut tile: NativeTerrainTile, mode: i32) -> Result<FixtureChunk, String> {
    let source = tile.root.chunks[CHUNK]
        .shadow_map
        .ok_or("fixture chunk lacks MCSH")?;
    let mask = match mode {
        0 => Some([0; 512]),
        1 => Some([255; 512]),
        2 => Some(std::array::from_fn(|i| if i % 8 < 4 { 255 } else { 0 })),
        3 => None,
        4 => Some(source),
        _ => return Err(format!("unknown MCSH fixture mask mode {mode}")),
    };
    tile.root.chunks[CHUNK].shadow_map = mask;
    let shader = crate::shader_warmup::load_shader("res://shaders/terrain.gdshader")?;
    let layers = &tile
        .tex
        .as_ref()
        .ok_or("fixture texture companion missing")?
        .chunk_layers[CHUNK]
        .layers;
    let mut resources = TerrainMaterials::default();
    let material = resources.build_material(
        &tile,
        layers,
        tile.root.chunks[CHUNK].shadow_map.as_ref(),
        &shader,
    )?;
    let chunk = &tile.root.chunks[CHUNK];
    let geometry = adt::chunk_geometry(chunk, Some((31, 36)));
    let mesh = super::build_mesh(geometry, &chunk.vertex_colors);
    let mut node = MeshInstance3D::new_alloc();
    node.set_mesh(&mesh);
    node.set_material_override(&material);
    node.set_position(-mesh.get_aabb().center());
    Ok((node, source, mask))
}

fn read_fixture_tile() -> Result<NativeTerrainTile, String> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let root_path = data.join("terrain/2703_31_36.adt");
    let tex_path = data.join("terrain/2703_31_36_tex0.adt");
    let root_bytes = fs::read(&root_path).map_err(|e| format!("{}: {e}", root_path.display()))?;
    let tex_bytes = fs::read(&tex_path).map_err(|e| format!("{}: {e}", tex_path.display()))?;
    // Active Map.csv row2703 WdtFileDataID; no inactive-tile runtime bypass.
    let map = fs::read(data.join("terrain/5493025.wdt")).map_err(|e| e.to_string())?;
    let flags = wdt::parse_wdt_mphd_flags(&map)?;
    let root = adt::parse_root_for_tile(&root_bytes, 31, 36, Some(&tex_bytes))?;
    let tex = adt::parse_tex(&tex_bytes, flags, &root)?;
    let textures = read_fixture_textures(&data, &tex)?;
    Ok(NativeTerrainTile {
        root_path,
        tex_path: Some(tex_path),
        obj_path: None,
        root,
        tex: Some(tex),
        obj: None,
        textures,
        chunk_surfaces: BTreeMap::new(),
        wmo_floors: Vec::new(),
        m2_floors: Vec::new(),
        wmo_surfaces: Vec::new(),
        liquid_materials: BTreeMap::new(),
    })
}

fn read_fixture_image(data: &Path, fdid: u32) -> Result<Arc<blp::RgbaImage>, String> {
    let path = data.join("textures").join(format!("{fdid}.blp"));
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    blp::decode_rgba(&bytes).map(Arc::new)
}

fn read_fixture_textures(
    data: &Path,
    tex: &adt::AdtTexData,
) -> Result<BTreeMap<u32, TerrainLayerTextures>, String> {
    tex.chunk_layers[CHUNK]
        .layers
        .iter()
        .take(4)
        .map(|layer| {
            let index = layer.texture_index as usize;
            let fdid = *tex
                .texture_fdids
                .get(index)
                .ok_or("fixture layer has no MDID")?;
            let diffuse = read_fixture_image(data, fdid)?;
            let height = tex
                .height_texture_fdids
                .get(index)
                .copied()
                .filter(|&id| id != 0)
                .map(|fdid| {
                    read_fixture_image(data, fdid).map(|image| TerrainTexture { fdid, image })
                })
                .transpose()?;
            Ok((
                layer.texture_index,
                TerrainLayerTextures {
                    diffuse_fdid: fdid,
                    diffuse,
                    height,
                },
            ))
        })
        .collect()
}
