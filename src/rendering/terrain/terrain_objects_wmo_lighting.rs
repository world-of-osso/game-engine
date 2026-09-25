//! Retail WMO lighting, per WebWowViewerCpp (`commonLightFunctions.slang` `calcLight`,
//! `wmoshader_text.slang`). MOCV is light, not albedo: the doubled fixed MOCV is added
//! to the ambient, `texture * (ambient + 2 * MOCV + sun)`. Exterior light is the shared
//! `RetailSceneLight`, interior light the WMO interior ambient without direct light;
//! the fixed MOCV alpha blends the two per vertex. StandardMaterial vertex color
//! multiplies base color, so these batches need their own fragment shader.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;

use crate::asset::wmo;

pub type WmoLitMaterial = ExtendedMaterial<StandardMaterial, WmoLighting>;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct WmoLighting {
    #[uniform(100)]
    pub params: WmoLightingParams,
    /// Second MOMT texture of a two-layer shader, sampled with UV_1.
    #[texture(101)]
    #[sampler(102)]
    pub second_texture: Option<Handle<Image>>,
    /// The shared exterior light, `crate::retail_light::RETAIL_SCENE_LIGHT_BUFFER`.
    #[storage(103, read_only)]
    pub scene_light: Handle<ShaderBuffer>,
}

#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct WmoLightingParams {
    /// Interior ambient, sRGB-encoded like MOCV.
    pub interior_ambient: Vec4,
    /// 1 when the group always takes exterior light, overriding the MOCV alpha.
    pub exterior_lit: u32,
    /// 1 for MOMT `F_UNLIT`: the texture alone.
    pub unlit: u32,
    /// MOMT shader of a two-layer blend by second MOCV alpha (6 or 13), else 0.
    pub two_layer_shader: u32,
    /// MOMT blend (GxBlend index), for the fog colour.
    pub blend_mode: u32,
    /// 1 for MOMT `F_UNFOGGED`.
    pub unfogged: u32,
}

/// Per-material MOMT state the lighting reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct WmoLitSurface {
    pub unlit: bool,
    pub unfogged: bool,
    pub blend_mode: u32,
}

/// Second layer of a Retail two-layer shader (see `WmoMaterialDef::blends_layers_by_second_mocv`).
#[derive(Debug, Clone)]
pub struct WmoSecondLayer {
    pub shader: u32,
    pub texture: Handle<Image>,
}

/// `WmoVertex` location of the second MOCV alpha in `wmo_lighting.wgsl`.
const SECOND_MOCV_SHADER_LOCATION: u32 = 8;

impl MaterialExtension for WmoLighting {
    fn vertex_shader() -> ShaderRef {
        "shaders/wmo_lighting.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/wmo_lighting.wgsl".into()
    }

    /// Feeds the second MOCV alpha to the forward pass. Prepass and shadow pipelines
    /// keep Bevy's vertex shader, whose attribute locations differ.
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Bevy labels these `pbr_prepass_pipeline` (depth/normal prepass and shadows).
        let is_prepass = descriptor
            .label
            .as_deref()
            .is_some_and(|label| label.contains("prepass"));
        if is_prepass || !layout.0.contains(wmo::WMO_BLEND_ALPHA_ATTRIBUTE) {
            return Ok(());
        }
        let forward_attributes = [
            (Mesh::ATTRIBUTE_POSITION, 0),
            (Mesh::ATTRIBUTE_NORMAL, 1),
            (Mesh::ATTRIBUTE_UV_0, 2),
            (Mesh::ATTRIBUTE_UV_1, 3),
            (Mesh::ATTRIBUTE_TANGENT, 4),
            (Mesh::ATTRIBUTE_COLOR, 5),
            (wmo::WMO_BLEND_ALPHA_ATTRIBUTE, SECOND_MOCV_SHADER_LOCATION),
        ];
        let attributes: Vec<_> = forward_attributes
            .into_iter()
            .filter(|(attribute, _)| layout.0.contains(attribute.clone()))
            .map(|(attribute, location)| attribute.at_shader_location(location))
            .collect();
        descriptor.vertex.buffers = vec![layout.0.get_layout(&attributes)?];
        descriptor.vertex.shader_defs.push("WMO_SECOND_MOCV".into());
        Ok(())
    }
}

const GROUP_EXTERIOR: u32 = 0x08;
const GROUP_EXTERIOR_LIT: u32 = 0x40;
const GROUP_INTERIOR: u32 = 0x2000;

/// WebWowViewerCpp `WmoGroupObject::isInteriorLightingLit`, negated.
pub(crate) fn wmo_group_is_exterior_lit(group_flags: u32) -> bool {
    group_flags & (GROUP_EXTERIOR | GROUP_EXTERIOR_LIT) != 0 || group_flags & GROUP_INTERIOR == 0
}

/// WebWowViewerCpp `WmoObject::calculateAmbient`: the first MAVG record for an active
/// doodad set, else the first MAVD, else the MOHD ambient. Only the base color (not
/// the horizon/ground colors of flag-1 records) is used.
pub(crate) fn wmo_interior_ambient(root: &wmo::WmoRootData, active_doodad_set: u16) -> [f32; 3] {
    let global = root
        .global_ambient_volumes
        .iter()
        .find(|volume| volume.doodad_set_id == 0 || volume.doodad_set_id == active_doodad_set)
        .or(root.global_ambient_volumes.first());
    let color = global
        .or(root.ambient_volumes.first())
        .map(|volume| volume.color_1)
        .unwrap_or(root.ambient_color);
    [color[0], color[1], color[2]]
}

pub(crate) fn wmo_lit_material(
    base: StandardMaterial,
    group_flags: u32,
    surface: WmoLitSurface,
    interior_ambient: [f32; 3],
    second_layer: Option<WmoSecondLayer>,
) -> WmoLitMaterial {
    let [r, g, b] = interior_ambient;
    let (two_layer_shader, second_texture) = match second_layer {
        Some(layer) => (layer.shader, Some(layer.texture)),
        None => (0, None),
    };
    WmoLitMaterial {
        // Fogged in authored space by the WMO shader.
        base: StandardMaterial {
            fog_enabled: false,
            ..base
        },
        extension: WmoLighting {
            params: WmoLightingParams {
                interior_ambient: Vec4::new(r, g, b, 1.0),
                exterior_lit: wmo_group_is_exterior_lit(group_flags) as u32,
                unlit: surface.unlit as u32,
                two_layer_shader,
                blend_mode: surface.blend_mode,
                unfogged: surface.unfogged as u32,
            },
            second_texture,
            scene_light: crate::retail_light::RETAIL_SCENE_LIGHT_BUFFER,
        },
    }
}

pub struct WmoLitMaterialPlugin;

impl Plugin for WmoLitMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WmoLitMaterial>::default());
    }
}
