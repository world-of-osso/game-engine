//! Native WMO group meshes and authored shader materials.
//!
//! Every retail MOMT shader id maps to WebWowViewerCpp's vertex/pixel shader pair
//! (`wowViewerLib/src/engine/objects/iWmoApi.h` `wmoMaterialShader`), whose math
//! `shaders/wmo.gdshader` ports from `commonWMOMaterial.slang`.

use std::{collections::BTreeSet, ops::RangeInclusive, path::Path};

use game_engine_core::{asset::wmo_format::parser::WmoMaterialDef, wmo};
use godot::{
    classes::{
        ArrayMesh, Image, ImageTexture, MeshInstance3D, Node3D, ResourceLoader, Shader,
        ShaderMaterial, geometry_instance_3d::ShadowCastingSetting, image, mesh,
    },
    obj::{EngineBitfield, EngineEnum},
    prelude::*,
};
use osso_asset_resolver::CascListfileResolver;

use super::assets::{NativeWmoAsset, group_casts_shadow};
use crate::lighting::TerrainLight;

const SHADER_PATH: &str = "res://shaders/wmo.gdshader";
const SHADER_TYPE: &str = "shader_type spatial;";
const RENDER_MODE: &str =
    "render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;";

/// `iWmoApi.h` `wmoMaterialShader[MAX_WMO_SHADERS]`: MOMT shader id ->
/// (WmoVertexShader, WmoPixelShader). 10 waterWindow and 14 submarineWindow are
/// (None, None) = (-1, -1), which the reference still draws through its `-1`
/// branches (`commonWMOMaterial.slang` `calcWMOVertMat`/`caclWMOFragMat` case -1).
const RETAIL_WMO_SHADERS: [(i32, i32); 24] = [
    (0, 0),   // 0 MapObjDiffuse: Diffuse_T1 / MapObjDiffuse
    (3, 1),   // 1 MapObjSpecular: Specular_T1 / MapObjSpecular
    (3, 2),   // 2 MapObjMetal: Specular_T1 / MapObjMetal
    (1, 3),   // 3 MapObjEnv: Diffuse_T1_Refl / MapObjEnv
    (0, 4),   // 4 MapObjOpaque: Diffuse_T1 / MapObjOpaque
    (1, 5),   // 5 MapObjEnvMetal: Diffuse_T1_Refl / MapObjEnvMetal
    (4, 6),   // 6 MapObjTwoLayerDiffuse: Diffuse_Comp / TwoLayerDiffuse
    (0, 7),   // 7 MapObjTwoLayerEnvMetal: Diffuse_T1 / TwoLayerEnvMetal
    (6, 8),   // 8 TwoLayerTerrain: Diffuse_Comp_Terrain / TwoLayerTerrain
    (4, 9),   // 9 MapObjDiffuseEmissive: Diffuse_Comp / DiffuseEmissive
    (-1, -1), // 10 waterWindow: None / None
    (2, 10),  // 11 MapObjMaskedEnvMetal: Diffuse_T1_Env_T2 / MaskedEnvMetal
    (2, 11),  // 12 MapObjEnvMetalEmissive: Diffuse_T1_Env_T2 / EnvMetalEmissive
    (4, 12),  // 13 TwoLayerDiffuseOpaque: Diffuse_Comp / TwoLayerDiffuseOpaque
    (-1, -1), // 14 submarineWindow: None / None
    (4, 13),  // 15 TwoLayerDiffuseEmissive: Diffuse_Comp / TwoLayerDiffuseEmissive
    (0, 0),   // 16 MapObjDiffuseTerrain: Diffuse_T1 / MapObjDiffuse
    (2, 14),  // 17: Diffuse_T1_Env_T2 / AdditiveMaskedEnvMetal
    (7, 15),  // 18: Diffuse_CompAlpha / TwoLayerDiffuseMod2x
    (4, 16),  // 19: Diffuse_Comp / TwoLayerDiffuseMod2xNA
    (7, 17),  // 20: Diffuse_CompAlpha / TwoLayerDiffuseAlpha
    (0, 18),  // 21: Diffuse_T1 / MapObjLod
    (8, 19),  // 22: MapObjParallax / MapObjParallax
    (0, 20),  // 23: Diffuse_T1 / MapObjDFShader
];

/// Shader uniforms for the nine WebWowViewerCpp WMO texture slots, in
/// `WMOMaterialTemplate.textures` order.
const TEXTURE_UNIFORMS: [&str; 9] = [
    "base_texture",
    "second_texture",
    "third_texture",
    "texture_4",
    "texture_5",
    "texture_6",
    "texture_7",
    "texture_8",
    "texture_9",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RetailWmoShader {
    vertex: i32,
    pixel: i32,
}

fn retail_wmo_shader(momt_shader: u32) -> Result<RetailWmoShader, String> {
    let (vertex, pixel) = RETAIL_WMO_SHADERS
        .get(momt_shader as usize)
        .copied()
        .ok_or_else(|| {
            format!("unsupported shader {momt_shader} (retail MOMT shaders are 0..=23)")
        })?;
    Ok(RetailWmoShader { vertex, pixel })
}

/// Texture slots each pixel shader samples in `caclWMOFragMat` (bit N = slot N).
/// Unread slots stay black, so their FDIDs need not be loaded.
fn pixel_shader_texture_slots(pixel: i32) -> u16 {
    match pixel {
        0 | 1 | 2 | 4 | 18 => 0b1,
        -1 | 3 | 5 | 6 | 8 | 9 | 12 | 13 | 16 => 0b11,
        7 | 10 | 11 | 14 | 15 | 17 => 0b111,
        19 => 0b11_1101,
        20 => 0b1_1111_1111,
        _ => unreachable!("pixel shader {pixel} is not in RETAIL_WMO_SHADERS"),
    }
}

/// Vertex streams the retail shaders read beyond MOCV, as WebWowViewerCpp
/// binds them (`wmoGroupGeom.cpp` `getVBO`): every MOTV set, the second MOCV and
/// MOC2 the group file carries, whatever the material flags. Missing streams use
/// the reference defaults in the shader: UV (1,1), MOCV2 alpha 1, MOC2 (0,0,0,1).
#[derive(Default)]
struct RetailWmoStreams {
    uv2: Option<Vec<[f32; 2]>>,
    uv3: Option<Vec<[f32; 2]>>,
    uv4: Option<Vec<[f32; 2]>>,
    second_mocv_alpha: Option<Vec<f32>>,
    moc2: Option<Vec<[f32; 4]>>,
}

fn retail_streams(raw: &wmo::RawGroupData, range: RangeInclusive<usize>) -> RetailWmoStreams {
    fn slice<T: Clone>(values: &[T], range: &RangeInclusive<usize>) -> Option<Vec<T>> {
        values.get(range.clone()).map(<[T]>::to_vec)
    }
    RetailWmoStreams {
        uv2: slice(&raw.second_uvs, &range),
        uv3: slice(&raw.third_uvs, &range),
        uv4: slice(&raw.fourth_uvs, &range),
        second_mocv_alpha: slice(&raw.second_color_blend_alphas, &range),
        moc2: slice(&raw.moc2_colors, &range),
    }
}

/// The group vertex range a shared mesh batch was sliced from
/// (`wmo_format::mesh_data` `build_split_group_batch`/`build_whole_group_batch`).
fn batch_vertex_range(raw: &wmo::RawGroupData, batch_index: usize) -> RangeInclusive<usize> {
    let last = raw.vertices.len().saturating_sub(1);
    match raw.batches.get(batch_index) {
        Some(batch) => batch.min_index as usize..=(batch.max_index as usize).min(last),
        None => 0..=last,
    }
}

struct PreparedWmoBatch<'a> {
    group_index: u32,
    group_flags: u32,
    mesh: wmo::WmoMeshBatch,
    streams: RetailWmoStreams,
    material: &'a WmoMaterialDef,
    shader: RetailWmoShader,
    interior_ambient: [f32; 3],
}

/// Unplaced WMO-local geometry plus the batches that could not be built; the
/// caller owns the ADT MODF transform and reports each batch error.
pub(crate) struct WmoNode {
    pub node: Gd<Node3D>,
    pub batch_errors: Vec<String>,
}

pub(crate) fn build_wmo_node(
    asset: &NativeWmoAsset,
    resolver: &CascListfileResolver,
    data_root: &Path,
    doodad_sets: &[u16],
    light: Option<&TerrainLight>,
) -> Result<WmoNode, String> {
    let mut build = WmoBuild::new(asset, doodad_sets)?;
    build.step(asset, resolver, data_root, light, || false);
    Ok(build.finish())
}

/// A WMO node built some batches at a time, so a large WMO can spread over frames.
pub(crate) struct WmoBuild {
    root: Gd<Node3D>,
    /// Renderable (group, batch) indices into the asset, in draw order.
    batches: Vec<(usize, usize)>,
    next: usize,
    /// Batches prepared so far: the `Group{g}_Batch{i}` index of the next one.
    prepared: usize,
    interior_ambient: [f32; 3],
    source: String,
    black: Gd<ImageTexture>,
    batch_errors: Vec<String>,
}

impl WmoBuild {
    pub(crate) fn new(asset: &NativeWmoAsset, doodad_sets: &[u16]) -> Result<Self, String> {
        let source = load_shader_source()?;
        let black = black_pixel_texture()?;
        let batches = renderable_batches(asset);
        let mut root = Node3D::new_alloc();
        root.set_name(&format!("Wmo{}", asset.root_fdid));
        Ok(Self {
            root,
            batches,
            next: 0,
            prepared: 0,
            interior_ambient: wmo_interior_ambient(&asset.root, doodad_sets),
            source,
            black,
            batch_errors: Vec::new(),
        })
    }

    /// Build batches until `stop` asks to (checked before each) or none are left;
    /// `true` once every batch is built. One bad batch must not hide the rest.
    pub(crate) fn step(
        &mut self,
        asset: &NativeWmoAsset,
        resolver: &CascListfileResolver,
        data_root: &Path,
        light: Option<&TerrainLight>,
        stop: impl Fn() -> bool,
    ) -> bool {
        while let Some(&(group_index, batch_index)) = self.batches.get(self.next) {
            if stop() {
                return false;
            }
            self.next += 1;
            let group = &asset.groups[group_index];
            let batch = &group.batches[batch_index];
            let prepared = match prepare_group_batch(
                asset,
                group,
                batch_index,
                batch,
                self.interior_ambient,
            ) {
                Ok(prepared) => prepared,
                Err(error) => {
                    self.batch_errors.push(error);
                    continue;
                }
            };
            let index = self.prepared;
            self.prepared += 1;
            let material = build_batch_material(
                &prepared,
                &self.source,
                resolver,
                data_root,
                &self.black,
                light,
            );
            let material = match material {
                Ok(material) => material,
                Err(error) => {
                    self.batch_errors.push(format!(
                        "WMO {} group {} batch {index}: {error}",
                        asset.root_fdid, prepared.group_index
                    ));
                    continue;
                }
            };
            let mut instance = MeshInstance3D::new_alloc();
            instance.set_name(&format!("Group{}_Batch{index}", prepared.group_index));
            instance.set_mesh(&build_batch_mesh(&prepared));
            instance.set_surface_override_material(0, &material);
            if !group_casts_shadow(prepared.group_flags) {
                instance.set_cast_shadows_setting(ShadowCastingSetting::OFF);
            }
            self.root.add_child(&instance);
        }
        true
    }

    /// The built node, unplaced; the caller owns it from here.
    pub(crate) fn finish(self) -> WmoNode {
        WmoNode {
            node: self.root,
            batch_errors: self.batch_errors,
        }
    }

    /// Free a build that will not finish.
    pub(crate) fn abandon(self) {
        self.root.free();
    }
}

/// Every texture FDID the WMO's batch materials sample (`build_batch_material`).
pub(crate) fn texture_fdids(asset: &NativeWmoAsset) -> BTreeSet<u32> {
    let mut fdids = BTreeSet::new();
    let groups = asset
        .groups
        .iter()
        .filter(|group| !group.group.header.group_flags.antiportal);
    for batch in groups.flat_map(|group| &group.batches) {
        let Some(material) = asset.root.materials.get(batch.material_index as usize) else {
            continue;
        };
        let Ok(shader) = retail_wmo_shader(material.shader) else {
            continue;
        };
        let slots = pixel_shader_texture_slots(shader.pixel);
        let used = material
            .retail_texture_fdids(shader.pixel)
            .into_iter()
            .enumerate()
            .filter(|&(slot, fdid)| fdid != 0 && slots & (1 << slot) != 0);
        fdids.extend(used.map(|(_, fdid)| fdid));
    }
    fdids
}

/// (group, batch) indices of the batches with triangles, outside antiportal groups.
fn renderable_batches(asset: &NativeWmoAsset) -> Vec<(usize, usize)> {
    asset
        .groups
        .iter()
        .enumerate()
        .filter(|(_, group)| !group.group.header.group_flags.antiportal)
        .flat_map(|(group_index, group)| {
            group
                .batches
                .iter()
                .enumerate()
                .filter(|(_, batch)| !batch.indices.is_empty())
                .map(move |(batch_index, _)| (group_index, batch_index))
        })
        .collect()
}

/// Renderable batches plus one error per batch that cannot be drawn.
#[cfg(test)]
fn prepare_wmo_batches<'a>(
    asset: &'a NativeWmoAsset,
    doodad_sets: &[u16],
) -> (Vec<PreparedWmoBatch<'a>>, Vec<String>) {
    let interior_ambient = wmo_interior_ambient(&asset.root, doodad_sets);
    let mut prepared = Vec::new();
    let mut errors = Vec::new();
    for (group_index, batch_index) in renderable_batches(asset) {
        let group = &asset.groups[group_index];
        let batch = &group.batches[batch_index];
        match prepare_group_batch(asset, group, batch_index, batch, interior_ambient) {
            Ok(batch) => prepared.push(batch),
            Err(error) => errors.push(error),
        }
    }
    (prepared, errors)
}

fn wmo_interior_ambient(root: &wmo::WmoRootData, doodad_sets: &[u16]) -> [f32; 3] {
    super::doodad_light::wmo_ambient_colors(root, doodad_sets)[0].to_array()
}

fn prepare_group_batch<'a>(
    asset: &'a NativeWmoAsset,
    group: &super::assets::NativeWmoGroup,
    batch_index: usize,
    batch: &wmo::WmoMeshBatch,
    interior_ambient: [f32; 3],
) -> Result<PreparedWmoBatch<'a>, String> {
    let context = format!(
        "WMO {} group {} material {}",
        asset.root_fdid, group.index, batch.material_index
    );
    let material = asset
        .root
        .materials
        .get(batch.material_index as usize)
        .ok_or_else(|| format!("{context} missing"))?;
    let shader =
        retail_wmo_shader(material.shader).map_err(|error| format!("{context} {error}"))?;
    if !matches!(material.blend_mode, 0..=3) {
        return Err(format!(
            "{context} unsupported blend mode {}",
            material.blend_mode
        ));
    }
    let invalid_index = batch
        .indices
        .iter()
        .any(|index| *index as usize >= batch.positions.len());
    if batch.indices.len() % 3 != 0 || invalid_index {
        return Err(format!(
            "WMO {} group {} has invalid batch triangles",
            asset.root_fdid, group.index
        ));
    }
    let raw = &group.group.geometry;
    let streams = retail_streams(raw, batch_vertex_range(raw, batch_index));
    let mut mesh = batch.clone();
    // Shared WMO batches already apply the sole [x,z,-y] conversion.
    // Godot's front-face convention is opposite Bevy's for these indices.
    for triangle in mesh.indices.chunks_exact_mut(3) {
        triangle.swap(1, 2);
    }
    Ok(PreparedWmoBatch {
        group_index: group.index,
        group_flags: group.group.header.flags,
        mesh,
        streams,
        material,
        shader,
        interior_ambient,
    })
}

fn build_batch_mesh(batch: &PreparedWmoBatch<'_>) -> Gd<ArrayMesh> {
    let mut arrays = wmo_mesh_arrays(&batch.mesh);
    let flags = bind_retail_streams(&mut arrays, &batch.streams);
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays_ex(mesh::PrimitiveType::TRIANGLES, &arrays)
        .flags(flags)
        .done();
    mesh
}

fn wmo_mesh_arrays(batch: &wmo::WmoMeshBatch) -> VarArray {
    let positions = batch
        .positions
        .iter()
        .map(|value| Vector3::from_array(*value))
        .collect::<Vec<_>>();
    let normals = batch
        .normals
        .iter()
        .map(|value| Vector3::from_array(*value))
        .collect::<Vec<_>>();
    let indices = batch
        .indices
        .iter()
        .map(|&index| index as i32)
        .collect::<Vec<_>>();
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        mesh::ArrayType::VERTEX.ord() as usize,
        &PackedVector3Array::from(positions.as_slice()).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::NORMAL.ord() as usize,
        &PackedVector3Array::from(normals.as_slice()).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::TEX_UV.ord() as usize,
        &uv_array(&batch.uvs).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::INDEX.ord() as usize,
        &PackedInt32Array::from(indices.as_slice()).to_variant(),
    );
    if let Some(colors) = &batch.colors {
        let colors = colors
            .iter()
            .map(|color| Color::from_rgba(color[0], color[1], color[2], color[3]))
            .collect::<Vec<_>>();
        arrays.set(
            mesh::ArrayType::COLOR.ord() as usize,
            &PackedColorArray::from(colors.as_slice()).to_variant(),
        );
    }
    arrays
}

fn uv_array(uvs: &[[f32; 2]]) -> PackedVector2Array {
    let uvs = uvs
        .iter()
        .map(|value| Vector2::new(value[0], value[1]))
        .collect::<Vec<_>>();
    PackedVector2Array::from(uvs.as_slice())
}

/// UV2 = MOTV2; CUSTOM0 = (MOTV3, MOTV4); CUSTOM1 = MOC2 RGBA8; CUSTOM2.x = MOCV2 alpha.
fn bind_retail_streams(arrays: &mut VarArray, streams: &RetailWmoStreams) -> mesh::ArrayFormat {
    if let Some(uv2) = &streams.uv2 {
        arrays.set(
            mesh::ArrayType::TEX_UV2.ord() as usize,
            &uv_array(uv2).to_variant(),
        );
    }
    let mut format = 0_u64;
    let mut custom = |slot: mesh::ArrayType,
                      shift: mesh::ArrayFormat,
                      kind: mesh::ArrayCustomFormat,
                      value: Variant| {
        arrays.set(slot.ord() as usize, &value);
        format |= (kind.ord() as u64) << shift.ord();
    };
    if streams.uv3.is_some() || streams.uv4.is_some() {
        let count = streams
            .uv3
            .as_ref()
            .or(streams.uv4.as_ref())
            .map_or(0, Vec::len);
        let uv3 = streams.uv3.as_deref();
        let uv4 = streams.uv4.as_deref();
        let packed = (0..count)
            .flat_map(|index| {
                let [u3, v3] = uv3.map_or([1.0; 2], |set| set[index]);
                let [u4, v4] = uv4.map_or([1.0; 2], |set| set[index]);
                [u3, v3, u4, v4]
            })
            .collect::<Vec<_>>();
        custom(
            mesh::ArrayType::CUSTOM0,
            mesh::ArrayFormat::CUSTOM0_SHIFT,
            mesh::ArrayCustomFormat::RGBA_FLOAT,
            PackedFloat32Array::from(packed.as_slice()).to_variant(),
        );
    }
    if let Some(moc2) = &streams.moc2 {
        let bytes = moc2
            .iter()
            .flat_map(|color| color.map(|channel| (channel * 255.0).round() as u8))
            .collect::<Vec<_>>();
        custom(
            mesh::ArrayType::CUSTOM1,
            mesh::ArrayFormat::CUSTOM1_SHIFT,
            mesh::ArrayCustomFormat::RGBA8_UNORM,
            PackedByteArray::from(bytes.as_slice()).to_variant(),
        );
    }
    if let Some(alphas) = &streams.second_mocv_alpha {
        custom(
            mesh::ArrayType::CUSTOM2,
            mesh::ArrayFormat::CUSTOM2_SHIFT,
            mesh::ArrayCustomFormat::R_FLOAT,
            PackedFloat32Array::from(alphas.as_slice()).to_variant(),
        );
    }
    mesh::ArrayFormat::try_from_ord(format).expect("Godot custom format")
}

fn build_batch_material(
    batch: &PreparedWmoBatch<'_>,
    source: &str,
    resolver: &CascListfileResolver,
    data_root: &Path,
    black: &Gd<ImageTexture>,
    light: Option<&TerrainLight>,
) -> Result<Gd<ShaderMaterial>, String> {
    let authored = batch.material;
    let span = crate::profile::span(|| "wmo.set_shader".to_owned());
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&material_shader(source, WmoShaderKey::of(authored))?);
    drop(span);
    let slots = pixel_shader_texture_slots(batch.shader.pixel);
    let fdids = authored.retail_texture_fdids(batch.shader.pixel);
    for (slot, (name, fdid)) in TEXTURE_UNIFORMS.into_iter().zip(fdids).enumerate() {
        // WebWowViewerCpp binds its black pixel for FDID 0
        // (`GDescriptorSet.cpp` texture_internal, `GDeviceVulkan.cpp` m_blackPixelTexture).
        let texture = if fdid == 0 || slots & (1 << slot) == 0 {
            black.clone()
        } else {
            read_wmo_texture(resolver, data_root, fdid)
                .map_err(|error| format!("texture slot {}: {error}", slot + 1))?
        };
        material.set_shader_parameter(name, &texture.to_variant());
    }
    let ambient = Vector3::from_array(batch.interior_ambient);
    let streams = &batch.streams;
    for (name, value) in [
        ("interior_ambient", ambient.to_variant()),
        ("base_color", Color::WHITE.to_variant()),
        ("emissive", Vector3::ZERO.to_variant()),
        (
            "exterior_lit",
            super::doodad_light::group_exterior_lit(batch.group_flags).to_variant(),
        ),
        ("unlit", authored.material_flags.unlit.to_variant()),
        ("unfogged", authored.material_flags.unfogged.to_variant()),
        ("has_mocv", batch.mesh.colors.is_some().to_variant()),
        (
            "has_second_mocv",
            streams.second_mocv_alpha.is_some().to_variant(),
        ),
        ("has_uv2", streams.uv2.is_some().to_variant()),
        ("has_uv3", streams.uv3.is_some().to_variant()),
        ("has_uv4", streams.uv4.is_some().to_variant()),
        ("has_moc2", streams.moc2.is_some().to_variant()),
        ("vertex_shader", batch.shader.vertex.to_variant()),
        ("pixel_shader", batch.shader.pixel.to_variant()),
        ("blend_mode", (authored.blend_mode as i32).to_variant()),
    ] {
        material.set_shader_parameter(name, &value);
    }
    if let Some(light) = light {
        light.bind_model(&mut material);
    }
    Ok(material)
}

thread_local! {
    /// WMO shaders by `WmoShaderKey`.
    static SHADERS: std::cell::RefCell<std::collections::HashMap<WmoShaderKey, Gd<Shader>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// The only material inputs `shader_variant` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct WmoShaderKey {
    two_sided: bool,
    blended: bool,
    clamp_s: bool,
    clamp_t: bool,
}

impl WmoShaderKey {
    fn of(material: &WmoMaterialDef) -> Self {
        let flags = &material.material_flags;
        Self {
            two_sided: flags.unculled,
            blended: matches!(material.blend_mode, 2 | 3),
            clamp_s: flags.clamp_s,
            clamp_t: flags.clamp_t,
        }
    }

    /// Two-sided, blended, clamp S and clamp T as 0/1.
    pub(crate) fn line(self) -> String {
        [self.two_sided, self.blended, self.clamp_s, self.clamp_t]
            .map(|flag| u8::from(flag).to_string())
            .join(" ")
    }

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let flags: Vec<bool> = line
            .split(' ')
            .map(|field| match field {
                "0" => Some(false),
                "1" => Some(true),
                _ => None,
            })
            .collect::<Option<_>>()?;
        match flags[..] {
            [two_sided, blended, clamp_s, clamp_t] => Some(Self {
                two_sided,
                blended,
                clamp_s,
                clamp_t,
            }),
            _ => None,
        }
    }
}

fn load_shader_source() -> Result<String, String> {
    Ok(ResourceLoader::singleton()
        .load(SHADER_PATH)
        .ok_or_else(|| format!("Cannot load WMO shader {SHADER_PATH}"))?
        .try_cast::<Shader>()
        .map_err(|_| format!("WMO shader {SHADER_PATH} has wrong resource type"))?
        .get_code()
        .to_string())
}

fn material_shader(source: &str, key: WmoShaderKey) -> Result<Gd<Shader>, String> {
    if let Some(shader) = SHADERS.with_borrow(|shaders| shaders.get(&key).cloned()) {
        return Ok(shader);
    }
    let shader = crate::assets::material::shared_shader(&shader_variant(source, key)?);
    SHADERS.with_borrow_mut(|shaders| shaders.insert(key, shader.clone()));
    crate::shader_warmup::record(crate::shader_warmup::UsedShader::Wmo(key));
    Ok(shader)
}

/// Compiles `key`'s shader: Godot compiles a shader when a material first takes it.
pub(crate) fn compile_shader(key: WmoShaderKey) -> Result<(), String> {
    // The RID creates the rendering server's shader, which compiles it.
    material_shader(&load_shader_source()?, key)?.get_rid();
    Ok(())
}

pub(crate) fn clear_shaders() {
    SHADERS.with_borrow_mut(std::collections::HashMap::clear);
}

fn shader_variant(source: &str, key: WmoShaderKey) -> Result<String, String> {
    if source.matches(RENDER_MODE).count() != 1 || !source.starts_with(SHADER_TYPE) {
        return Err("WMO shader type/render-mode declaration changed".into());
    }
    let cull = if key.two_sided {
        "cull_disabled"
    } else {
        "cull_back"
    };
    let render_mode = format!(
        "render_mode ambient_light_disabled, fog_disabled, specular_disabled, {cull}, blend_mix;"
    );
    let mut code = source.replace(RENDER_MODE, &render_mode);
    if key.blended {
        code = code.replacen(
            SHADER_TYPE,
            &format!("{SHADER_TYPE}\n#define WMO_BLENDED"),
            1,
        );
    }
    if key.clamp_s || key.clamp_t {
        return clamp_wmo_shader_uv(code, key);
    }
    Ok(code)
}

fn clamp_wmo_shader_uv(mut code: String, key: WmoShaderKey) -> Result<String, String> {
    for (original, replacement) in [
        (
            "texture(base_texture, UV)",
            "texture(base_texture, wmo_clamp_uv(UV, base_texture))",
        ),
        (
            "texture(second_texture, second_uv)",
            "texture(second_texture, wmo_clamp_uv(second_uv, second_texture))",
        ),
    ] {
        if code.matches(original).count() != 1 {
            return Err(format!("WMO shader sampler expression changed: {original}"));
        }
        code = code.replace(original, replacement);
    }
    let clamp_axis = |axis: &str, enabled: bool| {
        if enabled {
            format!("clamp(uv.{axis}, half_pixel.{axis}, 1.0 - half_pixel.{axis})")
        } else {
            format!("uv.{axis}")
        }
    };
    let uv = format!(
        "vec2 wmo_clamp_uv(vec2 uv, sampler2D tex) {{ vec2 half_pixel = vec2(0.5) / vec2(textureSize(tex, 0)); return vec2({}, {}); }}\n",
        clamp_axis("x", key.clamp_s),
        clamp_axis("y", key.clamp_t)
    );
    if code.matches("void vertex() {").count() != 1 {
        return Err("WMO shader vertex entry changed".into());
    }
    Ok(code.replace("void vertex() {", &format!("{uv}void vertex() {{")))
}

/// The authored texture, shared with every WMO and model that uses it and
/// block-compressed when it is DXT.
fn read_wmo_texture(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
) -> Result<Gd<ImageTexture>, String> {
    let dir = data_root.join("textures");
    let destination = dir.join(format!("{fdid}.blp"));
    resolver.ensure_cached(fdid, &destination).ok_or_else(|| {
        format!(
            "Local CASC WMO texture FDID {fdid} unavailable at {}",
            destination.display()
        )
    })?;
    let mut missing = PackedInt32Array::new();
    crate::assets::material::shared_texture(fdid, &dir, &mut missing)?.ok_or_else(|| {
        format!(
            "WMO texture FDID {fdid} missing at {}",
            destination.display()
        )
    })
}

/// WebWowViewerCpp's `m_blackPixelTexture`: one RGBA (0,0,0,0) texel.
fn black_pixel_texture() -> Result<Gd<ImageTexture>, String> {
    let image = Image::create_from_data(
        1,
        1,
        false,
        image::Format::RGBA8,
        &PackedByteArray::from([0_u8; 4].as_slice()),
    )
    .ok_or("Godot rejected the WMO black pixel image")?;
    ImageTexture::create_from_image(&image)
        .ok_or_else(|| "Godot rejected the WMO black pixel".into())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use game_engine_core::{adt::WmoPlacement, blp};
    use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

    use super::*;
    use crate::wmo::assets;

    fn resolver() -> (CascListfileResolver, PathBuf) {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = CascListfileResolver::new(
            AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        (resolver, data_root)
    }

    fn placement(fdid: u32) -> WmoPlacement {
        WmoPlacement {
            name_id: 0,
            unique_id: 48_366_671,
            position: [-2985.072, 446.524, -423.364],
            rotation: [0.0; 3],
            extents_min: [0.0; 3],
            extents_max: [0.0; 3],
            flags: 0xc,
            doodad_set: 0,
            name_set: 0,
            scale: 1.0,
            fdid: Some(fdid),
            path: None,
        }
    }

    fn read_asset(fdid: u32) -> (NativeWmoAsset, CascListfileResolver, PathBuf) {
        let (resolver, data_root) = resolver();
        let asset = assets::read_placement(&resolver, &data_root, &placement(fdid)).unwrap();
        (asset, resolver, data_root)
    }

    fn campsite_asset() -> (NativeWmoAsset, CascListfileResolver, PathBuf) {
        read_asset(4_214_993)
    }

    fn prepared(asset: &NativeWmoAsset) -> Vec<PreparedWmoBatch<'_>> {
        let (batches, errors) = prepare_wmo_batches(asset, &[0]);
        assert!(errors.is_empty(), "{errors:?}");
        batches
    }

    /// Every texture a prepared batch's pixel shader samples resolves from local
    /// CASC and decodes.
    fn assert_batch_textures_decode(
        batches: &[PreparedWmoBatch<'_>],
        resolver: &CascListfileResolver,
        data_root: &Path,
    ) {
        for batch in batches {
            let slots = pixel_shader_texture_slots(batch.shader.pixel);
            let fdids = batch.material.retail_texture_fdids(batch.shader.pixel);
            for (slot, fdid) in fdids.into_iter().enumerate() {
                if fdid == 0 || slots & (1 << slot) == 0 {
                    continue;
                }
                let path = data_root.join("textures").join(format!("{fdid}.blp"));
                let path = resolver
                    .ensure_cached(fdid, &path)
                    .unwrap_or_else(|| panic!("texture FDID {fdid} not in local CASC"));
                let image = blp::decode_rgba(&std::fs::read(path).unwrap()).unwrap();
                assert!(image.width > 0 && image.height > 0, "FDID {fdid}");
            }
        }
    }

    #[test]
    fn retail_table_matches_web_wow_viewer_pairs_for_every_momt_shader() {
        let expected = [
            (2, (3, 2)),
            (7, (0, 7)),
            (9, (4, 9)),
            (10, (-1, -1)),
            (12, (2, 11)),
            (14, (-1, -1)),
            (16, (0, 0)),
            (22, (8, 19)),
            (23, (0, 20)),
        ];
        for (momt, (vertex, pixel)) in expected {
            assert_eq!(
                retail_wmo_shader(momt).unwrap(),
                RetailWmoShader { vertex, pixel },
                "MOMT {momt}"
            );
        }
        for momt in 0..24 {
            let shader = retail_wmo_shader(momt).unwrap();
            assert_ne!(pixel_shader_texture_slots(shader.pixel), 0);
        }
        assert!(
            retail_wmo_shader(24)
                .unwrap_err()
                .contains("unsupported shader 24")
        );
    }

    /// Character-select campsite WMOs whose ground is one MOMT 23
    /// (MapObjDFShader) batch, previously rejected as "unsupported shader 23".
    #[test]
    fn campsite_df_shader_wmos_prepare_with_four_uv_sets_and_moc2() {
        for fdid in [4_907_674, 4_684_716, 4_684_717, 5_484_842, 4_883_307] {
            let (asset, resolver, data_root) = read_asset(fdid);
            let batches = prepared(&asset);
            let ground = batches
                .iter()
                .find(|batch| batch.material.shader == 23)
                .unwrap_or_else(|| panic!("WMO {fdid} has no shader 23 batch"));
            assert_eq!(
                ground.shader,
                RetailWmoShader {
                    vertex: 0,
                    pixel: 20
                }
            );
            let vertices = ground.mesh.positions.len();
            let streams = &ground.streams;
            for set in [&streams.uv2, &streams.uv3, &streams.uv4] {
                assert_eq!(set.as_ref().map(Vec::len), Some(vertices), "WMO {fdid}");
            }
            assert_eq!(streams.moc2.as_ref().map(Vec::len), Some(vertices));
            assert!(ground.mesh.colors.is_none());
            assert_batch_textures_decode(&batches, &resolver, &data_root);
        }
    }

    /// Stormwind portal room 8sw_portalroom01: MOMT 9, 12, 7, 5, 13 and 23.
    #[test]
    fn stormwind_portal_room_prepares_every_material() {
        let (asset, resolver, data_root) = read_asset(2_320_850);
        let batches = prepared(&asset);
        for shader in [5, 7, 9, 12, 13, 23] {
            assert!(
                batches.iter().any(|batch| batch.material.shader == shader),
                "shader {shader}"
            );
        }
        let emissive = batches
            .iter()
            .find(|batch| batch.material.shader == 9)
            .unwrap();
        // MapObjDiffuseEmissive reads MOTV2 and the second MOCV alpha.
        assert!(emissive.streams.uv2.is_some());
        assert!(emissive.streams.second_mocv_alpha.is_some());
        assert_batch_textures_decode(&batches, &resolver, &data_root);
    }

    /// Garrison farm: group 0 is EXTERIOR, group 1 only INTERIOR. Only the doodads
    /// group 0's MODR references (2-8 of the default set) cast shadows; group 1's
    /// default and interior-set doodads (0, 1, 9-13, 29-82) do not.
    #[test]
    fn garrison_farm_casts_shadows_from_exterior_group_and_its_doodads_only() {
        let (asset, _, _) = read_asset(892_927);
        let shadows = asset.shadow_groups();
        assert!(shadows.casts(&[0]));
        assert!(!shadows.casts(&[1]));
        let doodads = asset.doodads(&[0, 2]);
        let (casting, silent): (Vec<_>, Vec<_>) =
            doodads.iter().partition(|(doodad, _)| shadows.casts(&doodad.groups));
        let indices = |doodads: &[&assets::LitDoodad]| -> Vec<u16> {
            doodads.iter().map(|(doodad, _)| doodad.index).collect()
        };
        assert_eq!(indices(&casting), (2..=8).collect::<Vec<_>>());
        let mut expected: Vec<u16> = vec![0, 1];
        expected.extend(9..=13);
        expected.extend(29..=82);
        assert_eq!(indices(&silent), expected);
    }

    /// Garrison farm: MOMT 16 (MapObjDiffuseTerrain) and 7.
    #[test]
    fn garrison_farm_prepares_diffuse_terrain_and_two_layer_env_metal() {
        let (asset, resolver, data_root) = read_asset(892_927);
        let batches = prepared(&asset);
        let terrain = batches
            .iter()
            .find(|batch| batch.material.shader == 16)
            .unwrap();
        assert_eq!(
            terrain.shader,
            RetailWmoShader {
                vertex: 0,
                pixel: 0
            }
        );
        assert!(batches.iter().any(|batch| batch.material.shader == 7));
        assert_batch_textures_decode(&batches, &resolver, &data_root);
    }

    /// Campsite 5/25 building: MOMT 2 (MapObjMetal), 15 and 22 (MapObjParallax).
    #[test]
    fn campsite_building_prepares_metal_and_parallax() {
        let (asset, resolver, data_root) = read_asset(6_357_544);
        let batches = prepared(&asset);
        for shader in [2, 15, 22] {
            assert!(
                batches.iter().any(|batch| batch.material.shader == shader),
                "shader {shader}"
            );
        }
        assert_batch_textures_decode(&batches, &resolver, &data_root);
    }

    #[test]
    fn missing_second_uv_set_is_absent_not_defaulted_on_the_cpu() {
        let (asset, _, _) = read_asset(892_927);
        let batches = prepared(&asset);
        // Group 892930 has one MOTV and one MOCV.
        let group = asset
            .groups
            .iter()
            .find(|group| group.fdid == 892_930)
            .unwrap();
        let batch = batches
            .iter()
            .find(|batch| batch.group_index == group.index)
            .unwrap();
        assert!(batch.streams.uv2.is_none());
        assert!(batch.streams.second_mocv_alpha.is_none());
        assert!(batch.streams.moc2.is_none());
    }

    #[test]
    fn campsite_wmo_has_renderable_authored_batches_without_unsupported_materials() {
        let (asset, _, _) = campsite_asset();
        let batches = prepared(&asset);
        assert_eq!(asset.root.n_groups, 1);
        assert_eq!(asset.groups.len(), 1);
        assert!(!batches.is_empty());
        assert!(batches.iter().any(|batch| batch.material.shader == 13));
        assert!(batches.iter().all(|batch| batch.mesh.indices.len() >= 3));
    }

    #[test]
    fn campsite_group_vertices_are_swizzled_once_and_godot_winding_is_reversed() {
        let (asset, _, _) = campsite_asset();
        let batch = &asset.groups[0].batches[0];
        let raw = &asset.groups[0].group.geometry;
        let source = raw.vertices[raw.batches[0].min_index as usize];
        assert_eq!(batch.positions[0], [source[0], source[2], -source[1]]);
        let prepared = prepared(&asset);
        let group_batch = prepared.iter().find(|part| part.group_index == 0).unwrap();
        assert_eq!(group_batch.mesh.positions[0], batch.positions[0]);
        assert_eq!(group_batch.mesh.indices[0], batch.indices[0]);
        assert_eq!(group_batch.mesh.indices[1], batch.indices[2]);
        assert_eq!(group_batch.mesh.indices[2], batch.indices[1]);
    }

    /// Stormwind city WMO whose opaque materials use authored shader 4.
    #[test]
    fn opaque_shader_four_wmo_prepares_like_diffuse() {
        let (asset, _, _) = read_asset(111_538);
        let batches = prepared(&asset);
        assert!(batches.iter().any(|batch| batch.material.shader == 4));
    }

    /// An out-of-table shader drops only its own batches, with context.
    #[test]
    fn unsupported_wmo_shader_drops_only_its_batch() {
        let (mut asset, _, _) = campsite_asset();
        let index = asset.groups[0].batches[0].material_index as usize;
        asset.root.materials[index].shader = 99;
        let total = asset.groups[0]
            .batches
            .iter()
            .filter(|batch| !batch.indices.is_empty())
            .count();
        let affected = asset.groups[0]
            .batches
            .iter()
            .filter(|batch| !batch.indices.is_empty() && batch.material_index as usize == index)
            .count();
        let (batches, errors) = prepare_wmo_batches(&asset, &[0]);
        assert_eq!(errors.len(), affected);
        assert_eq!(batches.len(), total - affected);
        assert!(!batches.is_empty());
        assert!(errors[0].contains("shader 99"), "{}", errors[0]);
        assert!(errors[0].contains("4214993"), "{}", errors[0]);
    }

    #[test]
    fn missing_wmo_texture_reports_fdid_and_local_casc_failure() {
        let (_, resolver, data_root) = campsite_asset();
        let error = read_wmo_texture(&resolver, &data_root, 999_999_999)
            .err()
            .unwrap();
        assert!(error.contains("999999999"), "{error}");
        assert!(error.contains("Local CASC"), "{error}");
    }
}
