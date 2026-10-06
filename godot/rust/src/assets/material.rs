//! Authored M2 batch material: shader variants plus the original CPU texture-composition route.

use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use game_engine_core::{
    blp, m2,
    m2_batch_data::{OverlayScale, ResolvedBatch, TextureOverlay},
    m2_material::{self, BatchBinding, MaterialTracks, TextureMatrix},
    m2_texture_composite_data,
};
use godot::{
    classes::{
        Image, ImageTexture, MeshInstance3D, Node3D, ProjectSettings, ResourceLoader, Shader,
        ShaderMaterial, geometry_instance_3d::ShadowCastingSetting, image,
    },
    prelude::*,
};

const SHADER_PATH: &str = "res://shaders/m2.gdshader";
const RENDER_MODE: &str =
    "render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;";
/// The shader's texture uniforms, one per batch texture slot.
const TEXTURE_SLOTS: [&str; 4] = [
    "base_texture",
    "second_texture",
    "third_texture",
    "fourth_texture",
];
type DecodedTexture = (Vec<u8>, u32, u32);

thread_local! {
    /// Compiled shaders by exact variant source, shared by every material.
    static SHADERS: RefCell<HashMap<String, Gd<Shader>>> = RefCell::new(HashMap::new());
    /// One GPU texture per texture file or per distinct composite, shared by every
    /// placement: building textures per material uploaded a copy for every doodad.
    static TEXTURES: RefCell<HashMap<TextureKey, Gd<ImageTexture>>> =
        RefCell::new(HashMap::new());
    /// `batch_shaders` by pipeline state.
    static BATCH_SHADERS: RefCell<HashMap<Pipeline, (Gd<Shader>, Option<Gd<Shader>>)>> =
        RefCell::new(HashMap::new());
}

/// A batch's base texture: the file itself, or the file with its character overlays
/// composited onto it on the CPU.
#[derive(Clone, PartialEq, Eq, Hash)]
struct TextureKey {
    dir: PathBuf,
    fdid: u32,
    overlays: Vec<TextureOverlay>,
}

impl TextureKey {
    fn plain(fdid: u32, dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            fdid,
            overlays: Vec::new(),
        }
    }
}

/// One Godot `Shader` per distinct source: each new resource is compiled
/// separately, which dominated model loading when created per batch.
pub(crate) fn shared_shader(code: &str) -> Gd<Shader> {
    SHADERS.with_borrow_mut(|shaders| {
        shaders
            .entry(code.to_owned())
            .or_insert_with(|| {
                let mut shader = Shader::new_gd();
                shader.set_code(code);
                shader
            })
            .clone()
    })
}

pub(crate) fn clear_shared_shaders() {
    SHADERS.with_borrow_mut(HashMap::clear);
    TEXTURES.with_borrow_mut(HashMap::clear);
    BATCH_SHADERS.with_borrow_mut(HashMap::clear);
}

/// Compiles `pipeline`'s shader and, when opaque, its scenery-fade shader: Godot
/// compiles a shader when a material first takes it.
pub(crate) fn compile_pipeline(pipeline: Pipeline) -> Result<(), String> {
    let (shader, fade) = batch_shaders(pipeline)?;
    for shader in std::iter::once(shader).chain(fade) {
        // The RID creates the rendering server's shader, which compiles it.
        shader.get_rid();
    }
    Ok(())
}

/// The batch's material and the binding its animation samples.
pub(super) fn load_material(
    model: &m2::Model,
    tracks: &MaterialTracks,
    batch: &ResolvedBatch,
    skin_texture_fdids: &[u32; 3],
    path: &GString,
    missing: &mut PackedInt32Array,
    replacements: Option<&HashMap<u32, Gd<ImageTexture>>>,
) -> Result<(Gd<ShaderMaterial>, BatchBinding), String> {
    let _material = crate::profile::span(|| "phase.material.m2".to_owned());
    let texture_dir = Path::new(
        &ProjectSettings::singleton()
            .globalize_path(path)
            .to_string(),
    )
    .parent()
    .and_then(Path::parent)
    .ok_or("Model path has no asset root")?
    .join("textures");
    let unit = model
        .batches
        .get(batch.source_unit_index)
        .ok_or("Resolved batch has no skin batch")?;
    let binding = m2_material::batch_binding(model, unit, skin_texture_fdids)?;
    let pipeline = Pipeline::of(batch)?;
    let (shader, fade) = batch_shaders(pipeline)?;
    let span = crate::profile::span(|| "material.set_shader".to_owned());
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    drop(span);
    let _span = crate::profile::span(|| "material.parameters".to_owned());
    if let Some(fade) = fade {
        material.set_meta(SCENERY_FADE_SHADER_META, &fade.to_variant());
    }
    // Every slot of a replaceable type takes that type's texture, the second eye slot
    // as much as the first (WebWowViewerCpp getBlpTextureData per texture index).
    for (slot, fdid) in binding.textures.iter().enumerate() {
        let replacement = replacements
            .zip(binding.texture_types.get(slot).filter(|kind| **kind != 0))
            .and_then(|(textures, kind)| textures.get(kind));
        let texture = match (slot, replacement) {
            (_, Some(texture)) => Some(texture.clone()),
            (0, None) => fdid
                .map(|fdid| base_texture(fdid, &batch.overlays, &texture_dir, missing))
                .transpose()?
                .flatten(),
            _ => fdid
                .map(|fdid| shared_texture(fdid, &texture_dir, missing))
                .transpose()?
                .flatten(),
        };
        if let Some(texture) = texture {
            material.set_shader_parameter(TEXTURE_SLOTS[slot], &texture.to_variant());
        }
    }
    material.set_meta(
        "m2_texture_count",
        &i64::try_from(binding.textures.len())
            .expect("at most four textures")
            .to_variant(),
    );
    material.set_shader_parameter(
        "pixel_shader",
        &i32::from(binding.pixel_shader).to_variant(),
    );
    material.set_shader_parameter(
        "vertex_shader",
        &i32::from(binding.vertex_shader).to_variant(),
    );
    material.set_shader_parameter("texture_wrap", &(binding.texture_wrap as i32).to_variant());
    material.set_shader_parameter("render_flags", &i32::from(batch.render_flags).to_variant());
    material.set_shader_parameter("gx_blend", &i32::from(pipeline.gx_blend).to_variant());
    apply_sample(
        &mut material,
        &m2_material::sample_material(tracks, &binding, 0),
        None,
    );
    Ok((material, binding))
}

/// Sets the animated inputs of a batch material at one time that differ from `applied`,
/// the sample last set on it (every input when `None`). Each set makes Godot rebuild the
/// material's uniform buffer, so unchanged inputs are not set again.
pub(super) fn apply_sample(
    material: &mut Gd<ShaderMaterial>,
    sample: &m2_material::MaterialSample,
    applied: Option<&m2_material::MaterialSample>,
) {
    if applied.is_none_or(|applied| applied.mesh_color != sample.mesh_color) {
        material.set_shader_parameter(
            "mesh_color",
            &Vector3::from_array(sample.mesh_color).to_variant(),
        );
    }
    if applied.is_none_or(|applied| applied.opacity != sample.opacity) {
        material.set_shader_parameter("transparency", &sample.opacity.to_variant());
    }
    if applied.is_none_or(|applied| applied.texture_weights != sample.texture_weights) {
        material.set_shader_parameter(
            "texture_weights",
            &Vector3::from_array(sample.texture_weights).to_variant(),
        );
    }
    for (index, name) in ["texture_matrix_1", "texture_matrix_2"]
        .into_iter()
        .enumerate()
    {
        let matrix = sample.texture_matrices[index];
        if applied.is_none_or(|applied| applied.texture_matrices[index] != matrix) {
            material.set_shader_parameter(name, &texture_basis(matrix).to_variant());
        }
    }
}

/// A texture matrix as the shader's `mat3` acting on (u, v, 1).
fn texture_basis([m00, m10, m01, m11, tx, ty]: TextureMatrix) -> Basis {
    Basis::from_cols(
        Vector3::new(m00, m10, 0.0),
        Vector3::new(m01, m11, 0.0),
        Vector3::new(tx, ty, 1.0),
    )
}

/// The GPU state WebWowViewer's createM2Material derives from a batch's material:
/// GX blend, backface culling off for render flag 0x4, depth test off for 0x8 and depth
/// write off for 0x10.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Pipeline {
    gx_blend: u8,
    two_sided: bool,
    depth_test: bool,
    depth_write: bool,
}

impl Pipeline {
    fn of(batch: &ResolvedBatch) -> Result<Self, String> {
        Ok(Self {
            gx_blend: m2_material::gx_blend(batch.blend_mode)?,
            two_sided: batch.render_flags & 0x4 != 0,
            depth_test: batch.render_flags & 0x8 == 0,
            depth_write: batch.render_flags & 0x10 == 0,
        })
    }

    /// GX blend, then two-sided, depth test and depth write as 0/1.
    pub(crate) fn line(self) -> String {
        format!(
            "{} {} {} {}",
            self.gx_blend,
            u8::from(self.two_sided),
            u8::from(self.depth_test),
            u8::from(self.depth_write)
        )
    }

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let flag = |field: &str| match field {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        };
        match line.split(' ').collect::<Vec<_>>()[..] {
            [gx_blend, two_sided, depth_test, depth_write] => Some(Self {
                gx_blend: gx_blend.parse().ok()?,
                two_sided: flag(two_sided)?,
                depth_test: flag(depth_test)?,
                depth_write: flag(depth_write)?,
            }),
            _ => None,
        }
    }

    fn opaque(self) -> bool {
        self.gx_blend <= 1
    }

    /// Godot blend state for the GX blend and its GL factors: Alpha (SRC_ALPHA,
    /// 1-SRC_ALPHA), NoAlphaAdd (ONE, ONE), Add (SRC_ALPHA, ONE), Mod (DST_COLOR, ZERO),
    /// Mod2x (DST_COLOR, SRC_COLOR), BlendAdd (ONE, 1-SRC_ALPHA). Mod2x multiplies by
    /// twice the output (the shader doubles it); NoAlphaAdd writes alpha 1 so the add
    /// ignores it.
    fn blend(self) -> &'static str {
        match self.gx_blend {
            0..=2 => "blend_mix",
            3 | 10 => "blend_add",
            4 | 5 => "blend_mul",
            _ => "blend_premul_alpha",
        }
    }

    /// `depth_write` decides the draw for blended batches too: the reference keeps
    /// writing depth for them unless flag 0x10 clears it, so later triangles of the same
    /// batch behind earlier ones fail the depth test.
    fn render_mode(self, fading: bool) -> String {
        let cull = if self.two_sided {
            "cull_disabled"
        } else {
            "cull_back"
        };
        let depth = match (self.depth_write, self.opaque() && !fading) {
            (false, _) => ", depth_draw_never",
            (true, true) => "",
            (true, false) => ", depth_draw_always",
        };
        let test = if self.depth_test {
            ""
        } else {
            ", depth_test_disabled"
        };
        format!(
            "render_mode ambient_light_disabled, fog_disabled, specular_disabled, {cull}, {}{depth}{test};",
            self.blend()
        )
    }
}

/// A batch's shader and, when opaque, its scenery-fade shader, made once per pipeline.
fn batch_shaders(pipeline: Pipeline) -> Result<(Gd<Shader>, Option<Gd<Shader>>), String> {
    if let Some(shaders) = BATCH_SHADERS.with_borrow(|shaders| shaders.get(&pipeline).cloned()) {
        return Ok(shaders);
    }
    let _span = crate::profile::span(|| format!("material.shaders {pipeline:?}"));
    let source = ResourceLoader::singleton()
        .load(SHADER_PATH)
        .ok_or_else(|| format!("Cannot load M2 shader {SHADER_PATH}"))?
        .try_cast::<Shader>()
        .map_err(|_| format!("M2 shader {SHADER_PATH} has wrong resource type"))?
        .get_code()
        .to_string();
    let shader = shared_shader(&shader_variant(&source, pipeline)?);
    let fade = scenery_fade_variant(&source, pipeline)?.map(|fade| shared_shader(&fade));
    BATCH_SHADERS
        .with_borrow_mut(|shaders| shaders.insert(pipeline, (shader.clone(), fade.clone())));
    crate::shader_warmup::record(crate::shader_warmup::UsedShader::M2(pipeline));
    Ok((shader, fade))
}

/// Metadata of an opaque (blend 0/1) M2 material: the shader variant a placement
/// swaps in while it fades by distance. Godot draws any material that writes ALPHA
/// in its transparent pass, so the authored opaque variant writes none and only
/// fading placements take the blended one (solarityclient `with_runtime_alpha_fade`).
pub(crate) const SCENERY_FADE_SHADER_META: &str = "scenery_fade_shader";

/// A placed model's batch materials, faded together by retail scenery distance.
pub(crate) struct SceneryFade {
    /// Each batch mesh with its authored shadow casting, material, authored shader
    /// and, when opaque, its fade shader.
    batches: Vec<FadedBatch>,
    opacity: f32,
}

struct FadedBatch {
    mesh: Gd<MeshInstance3D>,
    shadows: ShadowCastingSetting,
    material: Gd<ShaderMaterial>,
    authored: Gd<Shader>,
    fade: Option<Gd<Shader>>,
}

impl SceneryFade {
    /// The `Batch*` mesh materials under a `build_model` root.
    pub fn from_model(root: &Gd<Node3D>) -> Self {
        let batches = root
            .get_children()
            .iter_shared()
            .filter_map(|child| child.try_cast::<MeshInstance3D>().ok())
            .filter_map(|mesh| {
                let material: Gd<ShaderMaterial> = mesh.get_active_material(0)?.try_cast().ok()?;
                let authored = material.get_shader()?;
                let fade = material
                    .has_meta(SCENERY_FADE_SHADER_META)
                    .then(|| material.get_meta(SCENERY_FADE_SHADER_META).try_to().ok())
                    .flatten();
                Some(FadedBatch {
                    shadows: mesh.get_cast_shadows_setting(),
                    mesh,
                    material,
                    authored,
                    fade,
                })
            })
            .collect();
        Self {
            batches,
            opacity: 1.0,
        }
    }

    /// Opaque batches blend only while `opacity` is below 1. A fading placement casts
    /// no shadow: retail admits scenery shadows only within the fade-start radius
    /// (solarityclient `SceneryDistance::admits_shadow`).
    pub fn set_opacity(&mut self, opacity: f32) {
        if opacity == self.opacity {
            return;
        }
        let fading = opacity < 1.0;
        let swap = fading != (self.opacity < 1.0);
        for batch in &mut self.batches {
            if swap {
                if let Some(fade) = &batch.fade {
                    batch
                        .material
                        .set_shader(if fading { fade } else { &batch.authored });
                }
                batch.mesh.set_cast_shadows_setting(if fading {
                    ShadowCastingSetting::OFF
                } else {
                    batch.shadows
                });
            }
            batch
                .material
                .set_shader_parameter("scenery_opacity", &opacity.to_variant());
        }
        self.opacity = opacity;
    }
}

fn shader_variant(source: &str, pipeline: Pipeline) -> Result<String, String> {
    let variant = with_render_mode(source, pipeline, false)?;
    let alpha = match pipeline.gx_blend {
        0 | 1 => "",
        10 => "ALPHA = 1.0;",
        _ => return Ok(variant),
    };
    replace_alpha(&variant, alpha)
}

/// The blended variant of an opaque batch: source-alpha blending by the placement's
/// `scenery_opacity` alone, as the opaque combiners ignore texture alpha. Blended
/// batches already multiply their alpha by it. It keeps the opaque depth write, as the
/// fade only enables blending: without it the overlapping cards of a fading tree
/// compound their coverage toward opaque.
fn scenery_fade_variant(source: &str, pipeline: Pipeline) -> Result<Option<String>, String> {
    if !pipeline.opaque() {
        return Ok(None);
    }
    let variant = with_render_mode(source, pipeline, true)?;
    replace_alpha(&variant, "ALPHA = scenery_opacity;").map(Some)
}

fn replace_alpha(variant: &str, alpha: &str) -> Result<String, String> {
    const AUTHORED: &str = "ALPHA = color.a * scenery_opacity;";
    if variant.matches(AUTHORED).count() != 1 {
        return Err("Expected one M2 alpha output".into());
    }
    Ok(variant.replace(AUTHORED, alpha))
}

fn with_render_mode(source: &str, pipeline: Pipeline, fading: bool) -> Result<String, String> {
    let count = source.matches(RENDER_MODE).count();
    if count != 1 {
        return Err(format!(
            "Expected one M2 render_mode declaration, found {count}"
        ));
    }
    Ok(source.replace(RENDER_MODE, &pipeline.render_mode(fading)))
}

fn texture_path(fdid: u32, dir: &Path) -> PathBuf {
    dir.join(format!("{fdid}.blp"))
}

pub(crate) fn load_texture(
    fdid: u32,
    dir: &Path,
    missing: &mut PackedInt32Array,
) -> Result<Option<DecodedTexture>, String> {
    let path = texture_path(fdid, dir);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            missing.push(fdid as i32);
            return Ok(None);
        }
        Err(error) => return Err(format!("Cannot read texture {fdid}: {error}")),
    };
    let rgba = blp::decode_rgba(&bytes).map_err(|error| format!("Texture {fdid}: {error}"))?;
    Ok(Some((rgba.pixels, rgba.width, rgba.height)))
}

/// A batch's base texture: the file, with its character overlays composited on the CPU.
fn base_texture(
    fdid: u32,
    overlays: &[TextureOverlay],
    dir: &Path,
    missing: &mut PackedInt32Array,
) -> Result<Option<Gd<ImageTexture>>, String> {
    if overlays.is_empty() {
        return shared_texture(fdid, dir, missing);
    }
    let key = TextureKey {
        overlays: overlays.to_vec(),
        ..TextureKey::plain(fdid, dir)
    };
    if let Some(texture) = TEXTURES.with_borrow(|textures| textures.get(&key).cloned()) {
        return Ok(Some(texture));
    }
    let _span = crate::profile::span(|| format!("material.base_texture {fdid}"));
    let Some((mut pixels, width, height)) = load_texture(fdid, dir, missing)? else {
        return Ok(None);
    };
    let missing_before = missing.len();
    compose_overlays(&mut pixels, width, &key, missing)?;
    let texture = texture_from_rgba(&pixels, width, height)?;
    // A composite missing a layer is reported again to every model that uses it.
    if missing.len() == missing_before {
        TEXTURES.with_borrow_mut(|textures| textures.insert(key, texture.clone()));
    }
    Ok(Some(texture))
}

/// The texture file as authored, block-compressed when it is DXT, shared by every user.
pub(crate) fn shared_texture(
    fdid: u32,
    dir: &Path,
    missing: &mut PackedInt32Array,
) -> Result<Option<Gd<ImageTexture>>, String> {
    let key = TextureKey::plain(fdid, dir);
    if let Some(texture) = TEXTURES.with_borrow(|textures| textures.get(&key).cloned()) {
        return Ok(Some(texture));
    }
    let _span = crate::profile::span(|| format!("material.shared_texture {fdid}"));
    let io = crate::profile::span(|| "phase.asset_io.blp_main".to_owned());
    let bytes = match fs::read(texture_path(fdid, dir)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            missing.push(fdid as i32);
            return Ok(None);
        }
        Err(error) => return Err(format!("Cannot read texture {fdid}: {error}")),
    };
    drop(io);
    let decode = crate::profile::span(|| "phase.blp_decode.main".to_owned());
    let image = blp::decode_gpu(&bytes).map_err(|error| format!("Texture {fdid}: {error}"))?;
    drop(decode);
    let texture = texture_from_gpu_image(image)?;
    TEXTURES.with_borrow_mut(|textures| textures.insert(key, texture.clone()));
    Ok(Some(texture))
}

/// Make `image`, decoded off the main thread, the shared texture of file `fdid` in
/// `dir`, so the first material or particle that uses it does no file work.
pub(crate) fn insert_shared_texture(
    fdid: u32,
    dir: &Path,
    image: blp::GpuImage,
) -> Result<(), String> {
    let key = TextureKey::plain(fdid, dir);
    if TEXTURES.with_borrow(|textures| textures.contains_key(&key)) {
        return Ok(());
    }
    let texture =
        texture_from_gpu_image(image).map_err(|error| format!("Texture {fdid}: {error}"))?;
    TEXTURES.with_borrow_mut(|textures| textures.insert(key, texture));
    Ok(())
}

pub(crate) fn texture_from_gpu_image(image: blp::GpuImage) -> Result<Gd<ImageTexture>, String> {
    let _upload = crate::profile::span(|| "phase.texture_upload.objects".to_owned());
    let format = match image.format {
        blp::GpuFormat::Dxt1 => image::Format::DXT1,
        blp::GpuFormat::Dxt3 => image::Format::DXT3,
        blp::GpuFormat::Dxt5 => image::Format::DXT5,
        blp::GpuFormat::Rgba8 => image::Format::RGBA8,
    };
    let (width, height) = (image.width, image.height);
    let godot_image = Image::create_from_data(
        width as i32,
        height as i32,
        image.mipmaps,
        format,
        &PackedByteArray::from(image.data.as_slice()),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} {format:?} image"))?;
    ImageTexture::create_from_image(&godot_image)
        .ok_or_else(|| format!("Godot rejected {width}x{height} {format:?} texture"))
}

fn compose_overlays(
    pixels: &mut [u8],
    width: u32,
    key: &TextureKey,
    missing: &mut PackedInt32Array,
) -> Result<(), String> {
    for overlay in &key.overlays {
        if let Some((bytes, w, h)) = load_texture(overlay.fdid, &key.dir, missing)? {
            m2_texture_composite_data::composite_overlay_pixels(
                pixels,
                width,
                &bytes,
                w,
                h,
                overlay.x,
                overlay.y,
                overlay.scale == OverlayScale::Uniform2x,
            );
        }
    }
    Ok(())
}

pub(crate) fn texture_from_rgba(
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<Gd<ImageTexture>, String> {
    let image = Image::create_from_data(
        width as i32,
        height as i32,
        false,
        image::Format::RGBA8,
        &PackedByteArray::from(pixels),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} M2 image"))?;
    ImageTexture::create_from_image(&image).ok_or_else(|| "Godot rejected M2 texture".into())
}
