//! Native WMO group meshes and authored shader materials.

use std::{collections::HashMap, fs, path::Path};

use game_engine_core::{
    asset::wmo_format::parser::WmoMaterialDef,
    blp, wmo,
    wmo_material_data::{composite_wmo_shader_layer, describe_wmo_shader},
};
use godot::{
    classes::{
        ArrayMesh, Image, ImageTexture, MeshInstance3D, Node3D, ResourceLoader, Shader,
        ShaderMaterial, image, mesh,
    },
    obj::{EngineBitfield, EngineEnum},
    prelude::*,
};
use osso_asset_resolver::CascListfileResolver;

use super::assets::NativeWmoAsset;
use crate::lighting::TerrainLight;

const SHADER_PATH: &str = "res://shaders/wmo.gdshader";
const SHADER_TYPE: &str = "shader_type spatial;";
const RENDER_MODE: &str =
    "render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;";

struct PreparedWmoBatch<'a> {
    group_index: u32,
    group_flags: u32,
    mesh: wmo::WmoMeshBatch,
    material: &'a WmoMaterialDef,
    interior_ambient: [f32; 3],
}

/// Returns unplaced WMO-local geometry; the caller owns the ADT MODF transform.
pub(crate) fn build_wmo_node(
    asset: &NativeWmoAsset,
    resolver: &CascListfileResolver,
    data_root: &Path,
    doodad_set: u16,
    light: Option<&TerrainLight>,
) -> Result<Gd<Node3D>, String> {
    let batches = prepare_wmo_batches(asset, doodad_set)?;
    let source = ResourceLoader::singleton()
        .load(SHADER_PATH)
        .ok_or_else(|| format!("Cannot load WMO shader {SHADER_PATH}"))?
        .try_cast::<Shader>()
        .map_err(|_| format!("WMO shader {SHADER_PATH} has wrong resource type"))?
        .get_code()
        .to_string();
    let mut composites = HashMap::new();
    let mut resources = Vec::with_capacity(batches.len());
    for (index, batch) in batches.iter().enumerate() {
        let context = format!(
            "WMO {} group {} batch {index}",
            asset.root_fdid, batch.group_index
        );
        let mesh = build_batch_mesh(&batch.mesh);
        let material =
            build_batch_material(batch, &source, resolver, data_root, &mut composites, light)
                .map_err(|error| format!("{context}: {error}"))?;
        resources.push((batch.group_index, mesh, material));
    }
    let mut root = Node3D::new_alloc();
    root.set_name(&format!("Wmo{}", asset.root_fdid));
    for (index, (group_index, mesh, material)) in resources.into_iter().enumerate() {
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_name(&format!("Group{group_index}_Batch{index}"));
        instance.set_mesh(&mesh);
        instance.set_surface_override_material(0, &material);
        root.add_child(&instance);
    }
    Ok(root)
}

fn prepare_wmo_batches(
    asset: &NativeWmoAsset,
    doodad_set: u16,
) -> Result<Vec<PreparedWmoBatch<'_>>, String> {
    let interior_ambient = wmo_interior_ambient(&asset.root, doodad_set);
    let mut prepared = Vec::new();
    for group in &asset.groups {
        if group.group.header.group_flags.antiportal {
            continue;
        }
        for batch in &group.batches {
            if !batch.indices.is_empty() {
                prepared.push(prepare_group_batch(asset, group, batch, interior_ambient)?);
            }
        }
    }
    Ok(prepared)
}

fn wmo_interior_ambient(root: &wmo::WmoRootData, doodad_set: u16) -> [f32; 3] {
    let global = root
        .global_ambient_volumes
        .iter()
        .find(|volume| volume.doodad_set_id == 0 || volume.doodad_set_id == doodad_set)
        .or(root.global_ambient_volumes.first());
    let ambient = global
        .or(root.ambient_volumes.first())
        .map(|volume| volume.color_1)
        .unwrap_or(root.ambient_color);
    [ambient[0], ambient[1], ambient[2]]
}

fn prepare_group_batch<'a>(
    asset: &'a NativeWmoAsset,
    group: &super::assets::NativeWmoGroup,
    batch: &wmo::WmoMeshBatch,
    interior_ambient: [f32; 3],
) -> Result<PreparedWmoBatch<'a>, String> {
    let material = asset
        .root
        .materials
        .get(batch.material_index as usize)
        .ok_or_else(|| {
            format!(
                "WMO {} group {} missing material {}",
                asset.root_fdid, group.index, batch.material_index
            )
        })?;
    // Original WmoLitMaterial shades shader 5 through retail_shade, not PBR;
    // its StandardMaterial roughness/reflectance do not affect that fragment.
    if !matches!(material.shader, 0 | 1 | 4 | 5 | 6 | 7 | 13 | 21) {
        return Err(format!(
            "WMO {} group {} material {} unsupported shader {}",
            asset.root_fdid, group.index, batch.material_index, material.shader
        ));
    }
    if !matches!(material.blend_mode, 0..=3) {
        return Err(format!(
            "WMO {} group {} material {} unsupported blend mode {}",
            asset.root_fdid, group.index, batch.material_index, material.blend_mode
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
        material,
        interior_ambient,
    })
}

fn build_batch_mesh(batch: &wmo::WmoMeshBatch) -> Gd<ArrayMesh> {
    let mut arrays = wmo_mesh_arrays(batch);
    let mut mesh = ArrayMesh::new_gd();
    if let Some(alphas) = &batch.second_color_blend_alphas {
        let flags = bind_second_mocv_alphas(&mut arrays, alphas);
        mesh.add_surface_from_arrays_ex(mesh::PrimitiveType::TRIANGLES, &arrays)
            .flags(flags)
            .done();
    } else {
        mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
    }
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
    let uvs = batch
        .uvs
        .iter()
        .map(|value| Vector2::new(value[0], value[1]))
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
        &PackedVector2Array::from(uvs.as_slice()).to_variant(),
    );
    arrays.set(
        mesh::ArrayType::INDEX.ord() as usize,
        &PackedInt32Array::from(indices.as_slice()).to_variant(),
    );
    if let Some(second) = &batch.second_uvs {
        let uv2 = second
            .iter()
            .map(|value| Vector2::new(value[0], value[1]))
            .collect::<Vec<_>>();
        arrays.set(
            mesh::ArrayType::TEX_UV2.ord() as usize,
            &PackedVector2Array::from(uv2.as_slice()).to_variant(),
        );
    }
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

fn bind_second_mocv_alphas(arrays: &mut VarArray, alphas: &[f32]) -> mesh::ArrayFormat {
    let custom = alphas
        .iter()
        .flat_map(|&alpha| [alpha, 0.0, 0.0, 0.0])
        .collect::<Vec<_>>();
    arrays.set(
        mesh::ArrayType::CUSTOM0.ord() as usize,
        &PackedFloat32Array::from(custom.as_slice()).to_variant(),
    );
    let format = mesh::ArrayCustomFormat::RGBA_FLOAT.ord() as u64;
    let shift = mesh::ArrayFormat::CUSTOM0_SHIFT.ord();
    mesh::ArrayFormat::try_from_ord(format << shift).expect("Godot custom format")
}

fn build_batch_material(
    batch: &PreparedWmoBatch<'_>,
    source: &str,
    resolver: &CascListfileResolver,
    data_root: &Path,
    composites: &mut HashMap<[u32; 3], Gd<ImageTexture>>,
    light: Option<&TerrainLight>,
) -> Result<Gd<ShaderMaterial>, String> {
    let authored = batch.material;
    let shader_code = shader_variant(source, authored)?;
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&crate::assets::material::shared_shader(&shader_code));
    let base = if authored.shader == 7 {
        read_shader_seven_texture(resolver, data_root, composites, authored)?
    } else {
        read_wmo_texture(resolver, data_root, authored.texture_fdid)?
    };
    material.set_shader_parameter("base_texture", &base.to_variant());
    if matches!(authored.shader, 6 | 13) {
        let second = read_wmo_texture(resolver, data_root, authored.texture_2_fdid)?;
        material.set_shader_parameter("second_texture", &second.to_variant());
    }
    let ambient = Vector3::from_array(batch.interior_ambient);
    for (name, value) in [
        ("interior_ambient", ambient.to_variant()),
        ("base_color", Color::WHITE.to_variant()),
        ("emissive", Vector3::ZERO.to_variant()),
        (
            "exterior_lit",
            ((batch.group_flags & 0x48 != 0) || (batch.group_flags & 0x2000 == 0)).to_variant(),
        ),
        ("unlit", authored.material_flags.unlit.to_variant()),
        ("unfogged", authored.material_flags.unfogged.to_variant()),
        ("has_mocv", batch.mesh.colors.is_some().to_variant()),
        (
            "has_second_mocv",
            batch.mesh.second_color_blend_alphas.is_some().to_variant(),
        ),
        ("has_uv2", batch.mesh.second_uvs.is_some().to_variant()),
        ("two_layer_shader", (authored.shader as i32).to_variant()),
        ("blend_mode", (authored.blend_mode as i32).to_variant()),
    ] {
        material.set_shader_parameter(name, &value);
    }
    if let Some(light) = light {
        light.bind_model(&mut material);
    }
    Ok(material)
}

fn shader_variant(source: &str, material: &WmoMaterialDef) -> Result<String, String> {
    if source.matches(RENDER_MODE).count() != 1 || !source.starts_with(SHADER_TYPE) {
        return Err("WMO shader type/render-mode declaration changed".into());
    }
    let cull = if material.material_flags.unculled {
        "cull_disabled"
    } else {
        "cull_back"
    };
    let render_mode = format!(
        "render_mode ambient_light_disabled, fog_disabled, specular_disabled, {cull}, blend_mix;"
    );
    let mut code = source.replace(RENDER_MODE, &render_mode);
    if matches!(material.blend_mode, 2 | 3) {
        code = code.replacen(
            SHADER_TYPE,
            &format!("{SHADER_TYPE}\n#define WMO_BLENDED"),
            1,
        );
    }
    if material.material_flags.clamp_s || material.material_flags.clamp_t {
        return clamp_wmo_shader_uv(code, material);
    }
    Ok(code)
}

fn clamp_wmo_shader_uv(mut code: String, material: &WmoMaterialDef) -> Result<String, String> {
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
        clamp_axis("x", material.material_flags.clamp_s),
        clamp_axis("y", material.material_flags.clamp_t)
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
    if fdid == 0 {
        return Err("WMO material has no authored texture FDID".into());
    }
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

fn read_shader_seven_texture(
    resolver: &CascListfileResolver,
    data_root: &Path,
    composites: &mut HashMap<[u32; 3], Gd<ImageTexture>>,
    authored: &WmoMaterialDef,
) -> Result<Gd<ImageTexture>, String> {
    let fdids = [
        authored.texture_fdid,
        authored.texture_2_fdid,
        authored.texture_3_fdid,
    ];
    if let Some(texture) = composites.get(&fdids) {
        return Ok(texture.clone());
    }
    let image = load_shader_seven_image(fdids, |fdid| read_wmo_image(resolver, data_root, fdid))?;
    let texture = create_wmo_texture(image, &format!("shader 7 FDIDs {fdids:?}"))?;
    composites.insert(fdids, texture.clone());
    Ok(texture)
}

fn load_shader_seven_image(
    fdids: [u32; 3],
    mut read_image: impl FnMut(u32) -> Result<blp::RgbaImage, String>,
) -> Result<blp::RgbaImage, String> {
    let [base_fdid, second_fdid, third_fdid] = fdids;
    let mut base =
        read_image(base_fdid).map_err(|error| format!("base FDID {base_fdid}: {error}"))?;
    let descriptor = describe_wmo_shader(7);
    for (fdid, mode) in [
        (second_fdid, descriptor.second_layer),
        (third_fdid, descriptor.third_layer),
    ] {
        if fdid == 0 {
            continue;
        }
        let overlay = read_image(fdid).map_err(|error| format!("overlay FDID {fdid}: {error}"))?;
        if (base.width, base.height) != (overlay.width, overlay.height) {
            return Err(format!(
                "WMO overlay FDID {fdid} {}x{} differs from base FDID {base_fdid} {}x{}",
                overlay.width, overlay.height, base.width, base.height
            ));
        }
        composite_wmo_shader_layer(&mut base.pixels, &overlay.pixels, mode);
    }
    Ok(base)
}

fn create_wmo_texture(image: blp::RgbaImage, label: &str) -> Result<Gd<ImageTexture>, String> {
    let godot_image = Image::create_from_data(
        image.width as i32,
        image.height as i32,
        false,
        image::Format::RGBA8,
        &PackedByteArray::from(image.pixels.as_slice()),
    )
    .ok_or_else(|| format!("Godot rejected WMO texture {label}"))?;
    ImageTexture::create_from_image(&godot_image)
        .ok_or_else(|| format!("Godot rejected WMO image {label}"))
}

fn read_wmo_image(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
) -> Result<blp::RgbaImage, String> {
    if fdid == 0 {
        return Err("WMO material has no authored texture FDID".into());
    }
    let destination = data_root.join("textures").join(format!("{fdid}.blp"));
    let path = resolver.ensure_cached(fdid, &destination).ok_or_else(|| {
        format!(
            "Local CASC WMO texture FDID {fdid} unavailable at {}",
            destination.display()
        )
    })?;
    let bytes = fs::read(&path)
        .map_err(|error| format!("WMO texture FDID {fdid} {}: {error}", path.display()))?;
    blp::decode_rgba(&bytes).map_err(|error| format!("WMO texture FDID {fdid}: {error}"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use game_engine_core::adt::WmoPlacement;
    use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

    use super::*;
    use crate::wmo::assets;

    fn campsite_asset() -> (NativeWmoAsset, CascListfileResolver, PathBuf) {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = CascListfileResolver::new(
            AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root)
                .with_cache_root(data_root.join("cache")),
        );
        let asset = assets::read_placement(&resolver, &data_root, &campsite_placement()).unwrap();
        (asset, resolver, data_root)
    }

    fn campsite_placement() -> WmoPlacement {
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
            fdid: Some(4_214_993),
            path: None,
        }
    }

    fn pixel(pixels: [u8; 4], width: u32, height: u32) -> blp::RgbaImage {
        blp::RgbaImage {
            pixels: pixels.to_vec(),
            width,
            height,
        }
    }

    #[test]
    fn shader_seven_blends_second_then_adds_third() {
        let image = load_shader_seven_image([10, 20, 30], |fdid| {
            Ok(match fdid {
                10 => pixel([100, 100, 100, 255], 1, 1),
                20 => pixel([200, 50, 0, 128], 1, 1),
                30 => pixel([20, 40, 80, 128], 1, 1),
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert_eq!(image.pixels, [160, 95, 90, 255]);
    }

    #[test]
    fn shader_seven_zero_overlays_leave_base_unchanged() {
        let mut loaded = Vec::new();
        let image = load_shader_seven_image([10, 0, 0], |fdid| {
            loaded.push(fdid);
            Ok(pixel([12, 34, 56, 255], 1, 1))
        })
        .unwrap();
        assert_eq!(loaded, [10]);
        assert_eq!(image.pixels, [12, 34, 56, 255]);
    }

    #[test]
    fn shader_seven_reports_missing_and_decode_errors_by_overlay_fdid() {
        for reason in ["missing", "decode failed"] {
            let error = load_shader_seven_image([10, 20, 0], |fdid| {
                if fdid == 20 {
                    Err(reason.to_owned())
                } else {
                    Ok(pixel([1, 2, 3, 255], 1, 1))
                }
            })
            .err()
            .unwrap();
            assert!(error.contains("20"), "{error}");
            assert!(error.contains(reason), "{error}");
        }
    }

    #[test]
    fn shader_seven_resizes_overlays_before_blending_and_adding() {
        let image = load_shader_seven_image([10, 20, 30], |fdid| {
            Ok(match fdid {
                10 => blp::RgbaImage {
                    pixels: [100, 100, 100, 255].repeat(4),
                    width: 2,
                    height: 2,
                },
                20 => pixel([200, 50, 0, 128], 1, 1),
                30 => pixel([20, 40, 80, 128], 1, 1),
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.pixels, [160, 95, 90, 255].repeat(4));
    }

    #[test]
    fn shader_seven_rejects_invalid_overlay_pixel_buffer() {
        let error = load_shader_seven_image([10, 20, 0], |fdid| {
            Ok(if fdid == 10 {
                pixel([100, 100, 100, 255], 1, 1)
            } else {
                pixel([200, 50, 0, 128], 2, 2)
            })
        })
        .err()
        .expect("invalid overlay buffer");
        assert!(error.contains("overlay FDID 20"), "{error}");
        assert!(error.contains("invalid RGBA buffer"), "{error}");
    }

    #[test]
    fn authored_wmo_108238_group_38_material_58_composites_resized_overlay() {
        let (_, resolver, data_root) = campsite_asset();
        let placement = WmoPlacement {
            fdid: Some(108_238),
            ..campsite_placement()
        };
        let asset = assets::read_placement(&resolver, &data_root, &placement).unwrap();
        let group = asset.groups.iter().find(|group| group.index == 38).unwrap();
        assert!(group.batches.iter().any(|batch| batch.material_index == 58));
        let batches = prepare_wmo_batches(&asset, 0).unwrap();
        assert!(
            batches
                .iter()
                .any(|batch| batch.group_index == 38 && batch.material.shader == 7)
        );
        let material = &asset.root.materials[58];
        assert_eq!(material.shader, 7);
        assert_eq!(material.texture_fdid, 948_125);
        assert_eq!(material.texture_2_fdid, 922_678);
        assert_eq!(material.texture_3_fdid, 0);
        let base = read_wmo_image(&resolver, &data_root, material.texture_fdid).unwrap();
        let overlay = read_wmo_image(&resolver, &data_root, material.texture_2_fdid).unwrap();
        assert_eq!((base.width, base.height), (512, 512));
        assert_eq!((overlay.width, overlay.height), (128, 128));
        let composite = load_shader_seven_image(
            [
                material.texture_fdid,
                material.texture_2_fdid,
                material.texture_3_fdid,
            ],
            |fdid| read_wmo_image(&resolver, &data_root, fdid),
        )
        .unwrap();
        assert_eq!((composite.width, composite.height), (512, 512));
        assert_ne!(composite.pixels, base.pixels);
    }

    #[test]
    fn campsite_wmo_has_renderable_authored_batches_without_unsupported_materials() {
        let (asset, _, _) = campsite_asset();
        let batches = prepare_wmo_batches(&asset, 0).unwrap();
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
        let prepared = prepare_wmo_batches(&asset, 0).unwrap();
        let group_batch = prepared.iter().find(|part| part.group_index == 0).unwrap();
        assert_eq!(group_batch.mesh.positions[0], batch.positions[0]);
        assert_eq!(group_batch.mesh.indices[0], batch.indices[0]);
        assert_eq!(group_batch.mesh.indices[1], batch.indices[2]);
        assert_eq!(group_batch.mesh.indices[2], batch.indices[1]);
    }

    /// Stormwind city WMO whose opaque materials use authored shader 4.
    #[test]
    fn opaque_shader_four_wmo_prepares_like_diffuse() {
        let (_, resolver, data_root) = campsite_asset();
        let placement = WmoPlacement {
            fdid: Some(111_538),
            ..campsite_placement()
        };
        let asset = assets::read_placement(&resolver, &data_root, &placement).unwrap();
        let batches = prepare_wmo_batches(&asset, 0).unwrap();
        assert!(batches.iter().any(|batch| batch.material.shader == 4));
    }

    #[test]
    fn unsupported_wmo_shader_is_contextual_error() {
        let (mut asset, _, _) = campsite_asset();
        let index = asset.groups[0].batches[0].material_index as usize;
        asset.root.materials[index].shader = 99;
        let error = prepare_wmo_batches(&asset, 0).err().unwrap();
        assert!(error.contains("shader 99"), "{error}");
        assert!(error.contains("4214993"), "{error}");
    }

    #[test]
    fn missing_wmo_texture_reports_fdid_and_local_casc_failure() {
        let (_, resolver, data_root) = campsite_asset();
        let error = read_wmo_image(&resolver, &data_root, 999_999_999)
            .err()
            .unwrap();
        assert!(error.contains("999999999"), "{error}");
        assert!(error.contains("Local CASC"), "{error}");
    }

    #[test]
    fn campsite_two_layer_textures_resolve_from_local_casc() {
        let (asset, resolver, data_root) = campsite_asset();
        let layers = prepare_wmo_batches(&asset, 0).unwrap();
        let layer = layers
            .iter()
            .find(|batch| batch.material.shader == 13)
            .unwrap();
        assert_ne!(layer.material.texture_fdid, 0);
        assert_ne!(layer.material.texture_2_fdid, 0);
        let first = read_wmo_image(&resolver, &data_root, layer.material.texture_fdid).unwrap();
        let second = read_wmo_image(&resolver, &data_root, layer.material.texture_2_fdid).unwrap();
        assert_eq!(
            first.pixels.len(),
            first.width as usize * first.height as usize * 4
        );
        assert_eq!(
            second.pixels.len(),
            second.width as usize * second.height as usize * 4
        );
    }
}
