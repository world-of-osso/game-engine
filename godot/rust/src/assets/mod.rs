//! Native M2/BLP conversion with authored batch resolution and materials.
pub(crate) mod appearance;
mod attachments;
pub(crate) mod creature;
pub(crate) mod equipment;
pub(crate) mod m2_lights;
pub(crate) mod material;
pub(crate) mod player;
mod player_request;
pub(crate) mod uv_animation;
use std::{cell::RefCell, collections::HashMap, fs, path::Path};

use crate::animation::WowAnimationPlayer;
use game_engine_core::{blp, m2};
use godot::{
    classes::{
        ArrayMesh, Image, ImageTexture, MeshInstance3D, Node3D, ProjectSettings, RefCounted,
        ShaderMaterial, Skeleton3D, Skin, image, mesh,
    },
    prelude::*,
};

use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct WowAssetLoader {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for WowAssetLoader {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl WowAssetLoader {
    #[func]
    fn decode_blp(&self, bytes: PackedByteArray) -> VarDictionary {
        result_image(blp::decode_rgba(bytes.as_slice()))
    }

    #[func]
    fn load_blp(&self, path: GString) -> VarDictionary {
        result_image(read_asset(&path).and_then(|data| blp::decode_rgba(&data)))
    }

    #[func]
    fn load_m2(&self, path: GString) -> VarDictionary {
        model_result(load_model_node(&path))
    }

    #[func]
    fn load_m2_with_skin_fdids(
        &self,
        path: GString,
        skin_fdids: PackedInt64Array,
    ) -> VarDictionary {
        let slots = parse_skin_texture_slots(skin_fdids.as_slice());
        model_result(slots.and_then(|slots| load_model_node_with_skin_fdids(&path, &slots)))
    }

    /// The player `race`/`sex`/`class` wearing `items` (`player_request::equipment`),
    /// built by the world's player loader: `{node}` or `{error}`.
    #[func]
    fn load_player(&self, race: i64, sex: i64, class: i64, items: VarArray) -> VarDictionary {
        let loaded = player_request::player(race, sex, class).and_then(|player| {
            let equipment = player_request::equipment(&items)?;
            player::load_player_model(&player_request::data_root(), &player, &equipment)
        });
        match loaded {
            Ok(node) => {
                let mut result = VarDictionary::new();
                result.set("node", &node);
                result
            }
            Err(error) => error_result(error),
        }
    }

    /// `load_player` with customization `choices`, a dictionary of ChrCustomizationOption
    /// ID to ChrCustomizationChoice ID, set as character creation and the barber do.
    #[func]
    fn load_player_customized(
        &self,
        race: i64,
        sex: i64,
        class: i64,
        items: VarArray,
        choices: VarDictionary,
    ) -> VarDictionary {
        let loaded = player_request::player(race, sex, class)
            .and_then(|player| player_request::customized(player, &choices))
            .and_then(|player| {
                let equipment = player_request::equipment(&items)?;
                player::load_player_model(&player_request::data_root(), &player, &equipment)
            });
        match loaded {
            Ok(node) => {
                let mut result = VarDictionary::new();
                result.set("node", &node);
                result
            }
            Err(error) => error_result(error),
        }
    }

    /// Place `model`'s weapons (`items`, as given to `load_player`) for sheath state
    /// `sheath` (0 sheathed, 1 melee drawn, 2 ranged drawn); "" or the error.
    #[func]
    fn place_player_weapons(&self, model: Gd<Node3D>, items: VarArray, sheath: i64) -> GString {
        let placed = player_request::sheath_state(sheath).and_then(|sheath| {
            let equipment = player_request::equipment(&items)?;
            player_request::place_player_weapons(&model, &equipment, sheath)
        });
        GString::from(placed.err().unwrap_or_default().as_str())
    }
}

fn parse_skin_texture_slots(values: &[i64]) -> Result<[u32; 3], String> {
    let [first, second, third] = values else {
        return Err("Expected exactly three creature skin texture FDIDs".into());
    };
    let convert = |value| u32::try_from(value).map_err(|_| format!("Invalid texture FDID {value}"));
    Ok([convert(*first)?, convert(*second)?, convert(*third)?])
}

fn model_result(result: Result<(Gd<Node3D>, PackedInt32Array), String>) -> VarDictionary {
    match result {
        Ok((node, missing)) => {
            let mut result = VarDictionary::new();
            result.set("node", &node);
            result.set("missing_texture_fdids", &missing);
            result
        }
        Err(error) => error_result(error),
    }
}

fn error_result(error: String) -> VarDictionary {
    let mut result = VarDictionary::new();
    result.set("error", error);
    result
}

fn global_path(path: &GString) -> String {
    ProjectSettings::singleton()
        .globalize_path(path)
        .to_string()
}

fn read_asset(path: &GString) -> Result<Vec<u8>, String> {
    fs::read(global_path(path)).map_err(|err| format!("Cannot read {path}: {err}"))
}

pub(crate) fn read_model(path: &GString) -> Result<m2::Model, String> {
    read_model_file(Path::new(&global_path(path)))
}

/// Parse the M2 at `model_path` with its `{stem}00.skin`, optional `{stem}.skel` and
/// external `.anim` files; no engine calls, so a worker thread can run it.
pub(crate) fn read_model_file(model_path: &Path) -> Result<m2::Model, String> {
    let read = |path: &Path| {
        fs::read(path).map_err(|err| format!("Cannot read {}: {err}", path.display()))
    };
    let model = read(model_path)?;
    let base = model_path.to_string_lossy();
    let stem = base.strip_suffix(".m2").ok_or("Expected .m2 path")?;
    let skin = read(Path::new(&format!("{stem}00.skin")))?;
    let skel_path = format!("{stem}.skel");
    let skeleton = if Path::new(&skel_path).exists() {
        Some(read(Path::new(&skel_path))?)
    } else {
        None
    };
    let resolver = model_asset_resolver(model_path)?;
    m2::parse_model_with_skeleton(&model, &skin, skeleton.as_deref(), |fdid| {
        read_animation_asset(model_path, fdid, &resolver)
    })
}

fn model_asset_resolver(model_path: &Path) -> Result<CascListfileResolver, String> {
    let data_root = model_path
        .parent()
        .and_then(Path::parent)
        .ok_or("Model path has no asset root")?;
    Ok(CascListfileResolver::new(
        AssetResolverConfig::new()
            .with_data_root(data_root)
            .with_shared_data_root(data_root),
    ))
}

fn read_animation_asset(
    model_path: &Path,
    fdid: u32,
    resolver: &CascListfileResolver,
) -> Option<Vec<u8>> {
    let path = model_path.with_file_name(format!("{fdid}.anim"));
    let cached = if path.is_file() {
        Some(path.clone())
    } else {
        resolver.ensure_cached(fdid, &path)
    };
    let loaded = cached
        .ok_or_else(|| format!("Cannot extract animation to {}", path.display()))
        .and_then(|path| fs::read(path).map_err(|error| error.to_string()));
    loaded
        .map_err(|error| {
            godot_warn!(
                "M2 {}: .anim FDID {fdid}: {error}; its sequence has no keyframes",
                model_path.display()
            );
        })
        .ok()
}

fn result_image(decoded: Result<blp::RgbaImage, String>) -> VarDictionary {
    match decoded.and_then(image_from_rgba) {
        Ok(image) => {
            let mut result = VarDictionary::new();
            result.set("image", &image);
            result
        }
        Err(error) => error_result(error),
    }
}

fn image_from_rgba(decoded: blp::RgbaImage) -> Result<Gd<Image>, String> {
    let pixels = PackedByteArray::from(decoded.pixels.as_slice());
    Image::create_from_data(
        decoded.width as i32,
        decoded.height as i32,
        false,
        image::Format::RGBA8,
        &pixels,
    )
    .ok_or_else(|| "Godot rejected decoded BLP image".into())
}

fn wow_vec3(value: [f32; 3]) -> Vector3 {
    Vector3::new(value[0], value[2], -value[1])
}

/// Root metadata: the M2 header bounding box in the model root's axes.
pub(crate) const M2_BOUNDS_META: &str = "m2_bounds";
/// Exact model-loader input identifier for semantic scene exports.
pub(crate) const M2_SOURCE_META: &str = "m2_source_path";
/// Batch metadata: the skin section's mesh part (geoset) ID.
const M2_MESH_PART_META: &str = "m2_mesh_part";

fn m2_bounds(model: &m2::Model) -> Aabb {
    let [min_x, min_y, min_z] = model.bounding_box_min;
    let [max_x, max_y, max_z] = model.bounding_box_max;
    // `wow_vec3` negates WoW Y into engine -Z, so the extremes swap on that axis.
    let min = Vector3::new(min_x, min_z, -max_y);
    let max = Vector3::new(max_x, max_z, -min_y);
    Aabb::new(min, max - min)
}

pub(crate) fn build_skeleton(bones: &[m2::Bone]) -> (Gd<Skeleton3D>, Option<Gd<Skin>>) {
    let mut skeleton = Skeleton3D::new_alloc();
    skeleton.set_name("Skeleton3D");
    // M2 skeletons have no SkeletonModifier3D; the default idle mode still runs an
    // internal process per skeleton every frame, about 20 ms for Stormwind's 8k doodads.
    skeleton.set_modifier_callback_mode_process(
        godot::classes::skeleton_3d::ModifierCallbackModeProcess::MANUAL,
    );
    if bones.is_empty() {
        return (skeleton, None);
    }
    let mut skin = Skin::new_gd();
    for i in 0..bones.len() {
        skeleton.add_bone(&format!("Bone{i}"));
    }
    for (i, bone) in bones.iter().enumerate() {
        let pivot = wow_vec3(bone.pivot);
        let parent = bone.parent_bone_id as i32;
        let local = if parent >= 0 {
            pivot - wow_vec3(bones[parent as usize].pivot)
        } else {
            pivot
        };
        skeleton.set_bone_parent(i as i32, parent);
        skeleton.set_bone_rest(i as i32, Transform3D::IDENTITY.translated(local));
        skeleton.set_bone_pose_position(i as i32, local);
        skin.add_bind(i as i32, Transform3D::IDENTITY.translated(-pivot));
    }
    (skeleton, Some(skin))
}

pub fn load_model_node(path: &GString) -> Result<(Gd<Node3D>, PackedInt32Array), String> {
    load_model_node_with_skin_fdids(path, &[0; 3])
}

pub(crate) fn load_model_node_with_skin_fdids(
    path: &GString,
    skin_texture_fdids: &[u32; 3],
) -> Result<(Gd<Node3D>, PackedInt32Array), String> {
    load_model_node_with_appearance(path, skin_texture_fdids, None)
}

pub(super) fn load_model_node_with_appearance(
    path: &GString,
    skin_texture_fdids: &[u32; 3],
    appearance: Option<&appearance::PreparedAppearance>,
) -> Result<(Gd<Node3D>, PackedInt32Array), String> {
    let model = read_model(path)?;
    build_model(&model, path, skin_texture_fdids, appearance)
}

pub(crate) fn build_model(
    model: &m2::Model,
    path: &GString,
    skin_texture_fdids: &[u32; 3],
    appearance: Option<&appearance::PreparedAppearance>,
) -> Result<(Gd<Node3D>, PackedInt32Array), String> {
    build_model_filtered(model, path, skin_texture_fdids, appearance, |_| true)
}

pub(super) fn build_model_filtered(
    model: &m2::Model,
    path: &GString,
    skin_texture_fdids: &[u32; 3],
    appearance: Option<&appearance::PreparedAppearance>,
    allowed: impl Fn(u16) -> bool,
) -> Result<(Gd<Node3D>, PackedInt32Array), String> {
    let mut missing = PackedInt32Array::new();
    let model_path = global_path(path);
    let resolver = model_asset_resolver(Path::new(&model_path))?;
    // Every batch is drawn: one whose opacity animates from zero appears later, and
    // the shader skips it while its opacity is zero (WebWowViewerCpp forEachVisibleMesh).
    let resolved: Vec<_> = m2::resolve_render_batches(model, skin_texture_fdids, true, |fdid| {
        resolver.resolve_path(fdid)
    })?
    .into_iter()
    .filter(|batch| allowed(batch.mesh_part_id))
    .collect();
    let tracks = game_engine_core::m2_material::MaterialTracks::of(model);
    let batches = resolved
        .iter()
        .map(|batch| {
            load_batch(
                model,
                &tracks,
                batch,
                skin_texture_fdids,
                path,
                &mut missing,
                appearance,
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mesh_parts: Vec<u16> = resolved.iter().map(|batch| batch.mesh_part_id).collect();
    let (mut skeleton, skin) = build_skeleton(&model.bones);
    if let Err(error) = attachments::add_attachment_nodes(&mut skeleton, model) {
        skeleton.free();
        return Err(error);
    }
    let player = if model.sequences.is_empty() {
        None
    } else {
        match WowAnimationPlayer::from_model(model, skeleton.clone()) {
            Ok(mut player) => {
                player.set_name("M2Animation");
                Some(player)
            }
            Err(error) => {
                skeleton.free();
                return Err(error);
            }
        }
    };
    let material_animation = uv_animation::WowMaterialAnimation::from_batches(
        tracks,
        batches
            .iter()
            .map(|batch| (batch.material.clone(), batch.binding.clone())),
    );
    let mut root = Node3D::new_alloc();
    root.set_meta(M2_BOUNDS_META, &m2_bounds(model).to_variant());
    root.set_meta(M2_SOURCE_META, &path.to_variant());
    root.add_child(&skeleton);
    for (batch_index, (batch, mesh_part)) in batches.into_iter().zip(mesh_parts).enumerate() {
        let LoadedBatch {
            mesh,
            material,
            visible,
            ..
        } = batch;
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_name(&format!("Batch{batch_index}"));
        instance.set_meta(M2_MESH_PART_META, &i64::from(mesh_part).to_variant());
        instance.set_mesh(&mesh);
        instance.set_visible(visible);
        if let Some(skin) = &skin {
            instance.set_skin(skin);
            instance.set_skeleton_path("../Skeleton3D");
        }
        // Bound as the material override, which GeometryInstance3D clears from its
        // RenderingServer instance before releasing: a surface override is released
        // first, and freeing a batch that held its last reference before the instance
        // was next drawn read the freed material (godotengine/godot#85817).
        instance.set_material_override(&material);
        root.add_child(&instance);
    }
    if let Some(player) = player {
        root.add_child(&player);
    }
    if let Some(mut animation) = material_animation {
        animation.set_name("M2MaterialAnimation");
        root.add_child(&animation);
    }
    if let Some(lights) = m2_lights::WowM2Lights::from_model(model) {
        root.add_child(&lights);
    }
    Ok((root, missing))
}

struct LoadedBatch {
    mesh: Gd<ArrayMesh>,
    material: Gd<ShaderMaterial>,
    binding: game_engine_core::m2_material::BatchBinding,
    visible: bool,
}

fn load_batch(
    model: &m2::Model,
    tracks: &game_engine_core::m2_material::MaterialTracks,
    batch: &game_engine_core::m2_batch_data::ResolvedBatch,
    skin_texture_fdids: &[u32; 3],
    path: &GString,
    missing: &mut PackedInt32Array,
    appearance: Option<&appearance::PreparedAppearance>,
) -> Result<LoadedBatch, String> {
    model.submeshes.get(batch.submesh_index).ok_or_else(|| {
        format!(
            "Batch {} references absent submesh",
            batch.source_unit_index
        )
    })?;
    let mesh = shared_batch_mesh(model, batch.submesh_index, path)?;
    check_required_replacement(batch, appearance)?;
    let (material, binding) = material::load_material(
        model,
        tracks,
        batch,
        skin_texture_fdids,
        path,
        missing,
        appearance.map(|appearance| &appearance.textures),
    )?;
    let visible = appearance.is_none_or(|appearance| {
        let visible = !appearance.hidden_geoset_ids.contains(&batch.mesh_part_id)
            && game_engine_core::npc_appearance_selection_data::npc_geoset_visible(
                batch.mesh_part_id,
                &appearance.selected_geosets,
                &appearance.authored_geosets,
            );
        game_engine_core::geoset_visibility_data::apply_exact_geoset_overrides(
            batch.mesh_part_id,
            visible,
            &appearance.equipment_geosets,
        )
    });
    Ok(LoadedBatch {
        mesh,
        material,
        binding,
        visible,
    })
}

/// A prepared body must bind its own body (1) and hair (6) textures.
fn check_required_replacement(
    batch: &game_engine_core::m2_batch_data::ResolvedBatch,
    appearance: Option<&appearance::PreparedAppearance>,
) -> Result<(), String> {
    let (Some(appearance), Some(kind)) = (appearance, batch.texture_type) else {
        return Ok(());
    };
    if matches!(kind, 1 | 6) && !appearance.textures.contains_key(&kind) {
        return Err(format!(
            "missing {} replacement texture type {kind} for batch {}",
            appearance.source, batch.source_unit_index
        ));
    }
    Ok(())
}

thread_local! {
    /// Batch meshes by model file and submesh: every placement and unit of a model draws
    /// the same vertex streams, so one mesh serves them all.
    static MESHES: RefCell<HashMap<(String, usize), Gd<ArrayMesh>>> =
        RefCell::new(HashMap::new());
}

fn shared_batch_mesh(
    model: &m2::Model,
    submesh_index: usize,
    path: &GString,
) -> Result<Gd<ArrayMesh>, String> {
    let key = (path.to_string(), submesh_index);
    if let Some(mesh) = MESHES.with_borrow(|meshes| meshes.get(&key).cloned()) {
        return Ok(mesh);
    }
    let sub = &model.submeshes[submesh_index];
    let mesh = build_batch_mesh(model, sub)?;
    MESHES.with_borrow_mut(|meshes| meshes.insert(key, mesh.clone()));
    Ok(mesh)
}

pub(crate) fn clear_shared_meshes() {
    MESHES.with_borrow_mut(HashMap::clear);
}

pub(crate) fn build_batch_mesh(
    model: &m2::Model,
    sub: &m2::Submesh,
) -> Result<Gd<ArrayMesh>, String> {
    let streams = m2::submesh_arrays(model, sub)?;
    let vectors = |values: &[[f32; 3]]| {
        let values: Vec<_> = values.iter().copied().map(Vector3::from_array).collect();
        PackedVector3Array::from(values.as_slice())
    };
    let uvs = |values: &[[f32; 2]]| {
        let values: Vec<_> = values.iter().map(|&[u, v]| Vector2::new(u, v)).collect();
        PackedVector2Array::from(values.as_slice())
    };
    let positions = vectors(&streams.positions);
    let normals = vectors(&streams.normals);
    let uv = uvs(&streams.uv);
    let uv2 = uvs(&streams.uv2);
    let bones = PackedInt32Array::from(streams.bones.as_slice());
    let weights = PackedFloat32Array::from(streams.weights.as_slice());
    let local_indices = PackedInt32Array::from(streams.indices.as_slice());
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        mesh::ArrayType::VERTEX.ord() as usize,
        &positions.to_variant(),
    );
    arrays.set(
        mesh::ArrayType::NORMAL.ord() as usize,
        &normals.to_variant(),
    );
    arrays.set(mesh::ArrayType::TEX_UV.ord() as usize, &uv.to_variant());
    arrays.set(mesh::ArrayType::TEX_UV2.ord() as usize, &uv2.to_variant());
    arrays.set(mesh::ArrayType::BONES.ord() as usize, &bones.to_variant());
    arrays.set(
        mesh::ArrayType::WEIGHTS.ord() as usize,
        &weights.to_variant(),
    );
    arrays.set(
        mesh::ArrayType::INDEX.ord() as usize,
        &local_indices.to_variant(),
    );
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
    Ok(mesh)
}
