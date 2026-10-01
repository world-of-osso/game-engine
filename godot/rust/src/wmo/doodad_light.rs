//! The light WebWowViewerCpp gives an M2 placed by a WMO (reference commit `1a8cccb`,
//! `~/Repos/WebWowViewerCpp/wowViewerLib/src/engine`):
//!
//! - `objects/wmo/wmoObject.cpp:98` starts every WMO doodad interior-lit
//!   (`setInteriorExteriorBlend(0)`); `wmoGroupObject.cpp:246-256` makes it exterior-lit
//!   once a group that references it is (`isInteriorLightingLit`, `wmoGroupObject.h:130-135`).
//! - `wmoObject.cpp:114-122`: when the first group that references it is an interior
//!   group, `applyLightingParamsToDoodad` (`:270-326`) and `applyColorFromMOLT`
//!   (`:186-268`) turn the MODD colour, MODD flags, MDDI intensity, MOLT light and the
//!   WMO ambient (`calculateAmbient`, `:1923-1966`) into the M2's interior ambient,
//!   direct colour and personal interior sun direction.
//! - `m2Object.cpp:1405-1432` hands them to the shader, where `calcLight`
//!   (`shaders/slang/common/commonLightFunctions.slang:103-197`) mixes interior and
//!   exterior light by the blend.

use game_engine_core::wmo::{self, WmoDoodad};
use glam::{Affine3A, Vec3};
use godot::{
    classes::{MeshInstance3D, Node3D, ShaderMaterial},
    prelude::*,
};

use super::assets::NativeWmoAsset;

/// MODD flag bits (`persistance/header/wmoFileHeader.h:196-212`, `SMODoodadDef`).
const NO_ADJUST_LIGHTING: u8 = 0x2;
const MOLT_DIRECT: u8 = 0x4;
const COLOR_IS_AMBIENT: u8 = 0x8;
const USE_INTENSITY: u8 = 0x10;
const NO_DIRECT: u8 = 0x40;
const NO_INTENSITY_COLOR: u8 = 0x80;
/// MODD colour alpha naming no MOLT light.
const NO_LIGHT: usize = 255;

/// The M2 shader's interior light of one WMO doodad (`m2Object.cpp:1405-1432`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DoodadLight {
    /// 0 lights the doodad as interior, 1 as exterior (`m_interiorExteriorBlend`).
    pub exterior_blend: f32,
    /// Interior ambient, horizon and ground ambient, in authored colour / 255.
    pub ambient: [Vec3; 3],
    pub direct: Vec3,
    /// WMO-local point (engine axes) its personal interior sun shines from, toward the
    /// doodad's box centre; `None` keeps the scene's interior sun.
    pub sun_source: Option<Vec3>,
}

/// Retail exterior lighting of a group: EXTERIOR (0x8), EXTERIOR_LIT (0x40) or not
/// INTERIOR (0x2000).
pub(crate) fn group_exterior_lit(flags: u32) -> bool {
    flags & 0x48 != 0 || flags & 0x2000 == 0
}

/// The WMO interior ambient, horizon and ground colours: the first MAVG of an active
/// doodad set (else MAVG 0), else MAVD 0, else the MOHD ambient; horizon and ground
/// only with MAVG/MAVD flag 1 (`wmoObject.cpp:1923-1966`).
pub(crate) fn wmo_ambient_colors(root: &wmo::WmoRootData, doodad_sets: &[u16]) -> [Vec3; 3] {
    let rgb = |color: [f32; 4]| Vec3::new(color[0], color[1], color[2]);
    let volume = root
        .global_ambient_volumes
        .iter()
        .find(|volume| doodad_sets.contains(&volume.doodad_set_id))
        .or(root.global_ambient_volumes.first())
        .or(root.ambient_volumes.first());
    match volume {
        Some(volume) if volume.flags & 1 != 0 => [
            rgb(volume.color_1),
            rgb(volume.color_2),
            rgb(volume.color_3),
        ],
        Some(volume) => [rgb(volume.color_1); 3],
        None => [rgb(root.ambient_color); 3],
    }
}

/// The light of `doodad`, placed by `asset` with active `doodad_sets`.
pub(crate) fn doodad_light(
    asset: &NativeWmoAsset,
    doodad: &WmoDoodad,
    doodad_sets: &[u16],
) -> DoodadLight {
    let header = |group: u16| {
        asset
            .groups
            .iter()
            .find(|loaded| loaded.index == u32::from(group))
            .map(|loaded| &loaded.group.header)
    };
    let exterior_lit = |group: u16| header(group).is_none_or(|h| group_exterior_lit(h.flags));
    let exterior_blend = if doodad.groups.iter().any(|&group| exterior_lit(group)) {
        1.0
    } else {
        0.0
    };
    // `fromGroupIndex` is the first group whose MODR loads the doodad.
    let first = doodad.groups.first().copied();
    match first.filter(|&group| !exterior_lit(group)).and_then(header) {
        Some(first) => apply_lighting(
            light_inputs(asset, doodad, first, doodad_sets),
            exterior_blend,
        ),
        None => DoodadLight::unlit(exterior_blend),
    }
}

impl DoodadLight {
    /// Nothing but the blend set (`m2Object.h` defaults).
    fn unlit(exterior_blend: f32) -> Self {
        Self {
            exterior_blend,
            ambient: [Vec3::ZERO; 3],
            direct: Vec3::ZERO,
            sun_source: None,
        }
    }
}

fn light_inputs<'a>(
    asset: &'a NativeWmoAsset,
    doodad: &WmoDoodad,
    first_group: &wmo::WmoGroupHeader,
    doodad_sets: &[u16],
) -> LightInputs<'a> {
    let box_center = (Vec3::from(first_group.bbox_min) + Vec3::from(first_group.bbox_max)) * 0.5;
    let intensity = asset
        .root
        .doodad_intensities
        .get(doodad.index as usize)
        .copied()
        .filter(|_| doodad.flags & USE_INTENSITY != 0)
        .unwrap_or(1.0);
    LightInputs {
        flags: doodad.flags,
        color: Vec3::new(doodad.color[0], doodad.color[1], doodad.color[2]),
        light: molt_index(doodad.color[3]),
        intensity,
        ambient: wmo_ambient_colors(&asset.root, doodad_sets),
        lights: &asset.root.lights,
        group_center: wmo_local(box_center),
    }
}

struct LightInputs<'a> {
    flags: u8,
    color: Vec3,
    /// MODD colour alpha: a MOLT index, or 255.
    light: usize,
    intensity: f32,
    ambient: [Vec3; 3],
    lights: &'a [wmo::WmoLight],
    /// WMO-local centre of the first referencing group's MOGP box.
    group_center: Vec3,
}

impl LightInputs<'_> {
    fn has(&self, bit: u8) -> bool {
        self.flags & bit != 0
    }

    /// Flag 0x4 or 0x40 (`hasDoodad0x4Flag`).
    fn has_direct_flag(&self) -> bool {
        self.has(MOLT_DIRECT) || self.has(NO_DIRECT)
    }

    fn molt(&self) -> Option<&wmo::WmoLight> {
        self.lights
            .get(self.light)
            .filter(|_| self.light != NO_LIGHT)
    }
}

/// `applyColorFromMOLT` then `applyLightingParamsToDoodad` (`wmoObject.cpp:186-326`).
fn apply_lighting(inputs: LightInputs, exterior_blend: f32) -> DoodadLight {
    let mut out = DoodadLight::unlit(exterior_blend);
    let (color, ambient) = apply_color_from_molt(&inputs, &mut out);
    if inputs.has_direct_flag() && inputs.has(COLOR_IS_AMBIENT) {
        return out;
    }
    let (color, ambient) = adjust_lighting(&inputs, color, ambient);
    if !inputs.has_direct_flag() {
        out.direct = color;
    }
    if !inputs.has(COLOR_IS_AMBIENT) {
        out.ambient = ambient;
    }
    out
}

/// `applyLightingParamsToDoodad`'s colour and ambient adjustment (`wmoObject.cpp:284-314`).
fn adjust_lighting(inputs: &LightInputs, color: Vec3, ambient: [Vec3; 3]) -> (Vec3, [Vec3; 3]) {
    let direct = if inputs.has(NO_INTENSITY_COLOR) {
        Vec3::ZERO
    } else {
        color * inputs.intensity.max(1.0)
    };
    if inputs.has(NO_ADJUST_LIGHTING) {
        if inputs.has(USE_INTENSITY) && !inputs.has_direct_flag() {
            return (add_direct_color_and_ambient(direct, ambient[0]), ambient);
        }
        return (color, ambient);
    }
    let (color, ambient) = if inputs.has(USE_INTENSITY) {
        let ambient = ambient.map(|ambient| add_direct_color_and_ambient(direct, ambient));
        (ambient[0], ambient)
    } else {
        (color, [color; 3])
    };
    (
        fix_direct_color(color, 0x70),
        ambient.map(|ambient| fix_ambient(ambient, 0x60)),
    )
}

/// `applyColorFromMOLT` (`wmoObject.cpp:186-268`): writes the MOLT-driven light into
/// `out` and returns the colour and interior ambients it leaves.
fn apply_color_from_molt(inputs: &LightInputs, out: &mut DoodadLight) -> (Vec3, [Vec3; 3]) {
    let (mut color, mut ambient) = (inputs.color, inputs.ambient);
    if inputs.has(COLOR_IS_AMBIENT) {
        ambient = [color; 3];
        out.ambient = ambient;
    }
    if inputs.has(NO_DIRECT) {
        color = Vec3::ZERO;
        out.direct = color;
    } else if let Some(light) = inputs.molt().filter(|_| inputs.has(MOLT_DIRECT)) {
        out.sun_source = Some(wmo_local(Vec3::from(light.position)));
        color = molt_color(light, color);
        if !inputs.has(NO_ADJUST_LIGHTING) {
            out.direct = fix_direct_color(color, 0x70);
        }
    }
    if inputs.has(NO_ADJUST_LIGHTING) {
        out.sun_source = if inputs.light == NO_LIGHT {
            Some(inputs.group_center)
        } else {
            inputs
                .molt()
                .map(|light| wmo_local(Vec3::from(light.position)))
        };
    }
    (color, ambient)
}

/// Sets `light` on the batch materials of `model`, a doodad of the WMO placed by
/// `world_from_wmo`, with its render box centred at world `center`. An exterior-lit
/// doodad keeps the M2 shader's exterior-only default.
pub(crate) fn bind_doodad_light(
    model: &Gd<Node3D>,
    light: &DoodadLight,
    world_from_wmo: Affine3A,
    center: Vec3,
) {
    if light.exterior_blend >= 1.0 {
        return;
    }
    let sun = light
        .sun_source
        .map(|source| (center - world_from_wmo.transform_point3(source)).normalize_or_zero())
        .filter(|direction| *direction != Vec3::ZERO);
    let meshes = model
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done();
    for node in meshes.iter_shared() {
        if let Some(material) = node.cast::<MeshInstance3D>().get_active_material(0) {
            set_light_parameters(material.cast(), light, sun);
        }
    }
}

fn set_light_parameters(mut material: Gd<ShaderMaterial>, light: &DoodadLight, sun: Option<Vec3>) {
    let vector = |value: Vec3| Vector3::from_array(value.to_array()).to_variant();
    material.set_shader_parameter("exterior_blend", &light.exterior_blend.to_variant());
    material.set_shader_parameter("interior_ambient", &vector(light.ambient[0]));
    material.set_shader_parameter("interior_horizon_ambient", &vector(light.ambient[1]));
    material.set_shader_parameter("interior_ground_ambient", &vector(light.ambient[2]));
    material.set_shader_parameter("interior_direct", &vector(light.direct));
    if let Some(sun) = sun {
        material.set_shader_parameter("interior_sun_direction", &vector(sun));
    }
}

/// The MOLT light colour times its intensity, normalised, when it exceeds 1; the
/// reference keeps `color` otherwise (`wmoObject.cpp:226-237`).
fn molt_color(light: &wmo::WmoLight, color: Vec3) -> Vec3 {
    let light_color = Vec3::new(light.color[0], light.color[1], light.color[2]) * light.intensity;
    let biggest = light_color.max_element();
    if biggest > 1.0 {
        (light_color / biggest).min(Vec3::ONE)
    } else {
        color
    }
}

fn molt_index(alpha: f32) -> usize {
    (alpha * 255.0).round() as usize
}

/// WMO file axes (Z up) to engine axes.
fn wmo_local(file: Vec3) -> Vec3 {
    Vec3::new(file.x, file.z, -file.y)
}

/// Its largest component, floored at 1/255 when near zero
/// (`wmoObject.cpp:154-169`, `:171-184`).
fn biggest_component(color: Vec3) -> f32 {
    let biggest = color.max_element();
    if biggest <= 0.00001 {
        1.0 / 255.0
    } else {
        biggest
    }
}

/// `fixDirectColor`: brightened to at least `limit` / 255 (an HSV value scale, which
/// scales every component alike).
fn fix_direct_color(color: Vec3, limit: u8) -> Vec3 {
    let biggest = biggest_component(color);
    let limit = f32::from(limit) / 255.0;
    if biggest < limit {
        color * (limit / biggest)
    } else {
        color
    }
}

/// `fixAmbient1`: darkened to at most `limit` / 255.
fn fix_ambient(ambient: Vec3, limit: u8) -> Vec3 {
    let biggest = biggest_component(ambient);
    let limit = f32::from(limit) / 255.0;
    if biggest > limit {
        ambient * (limit / biggest)
    } else {
        ambient
    }
}

/// `addDirectColorAndAmbient` (`wmoObject.cpp:139-152`).
fn add_direct_color_and_ambient(direct: Vec3, ambient: Vec3) -> Vec3 {
    let sum = ambient + direct;
    let biggest = sum.max_element();
    let sum = if biggest > 1.0 { sum / biggest } else { sum };
    sum.min(Vec3::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_wmo(fdid: u32) -> NativeWmoAsset {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let resolver = osso_asset_resolver::CascListfileResolver::new(
            osso_asset_resolver::AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        let placement = game_engine_core::adt::WmoPlacement {
            name_id: 0,
            unique_id: 0,
            position: [0.0; 3],
            rotation: [0.0; 3],
            extents_min: [0.0; 3],
            extents_max: [0.0; 3],
            flags: 0,
            doodad_set: 0,
            name_set: 0,
            scale: 1.0,
            fdid: Some(fdid),
            path: None,
        };
        super::super::assets::read_placement(&resolver, &data_root, &placement).unwrap()
    }

    fn light_of(asset: &NativeWmoAsset, index: u16) -> (WmoDoodad, DoodadLight) {
        asset
            .doodads(&[0, 0])
            .into_iter()
            .find(|(doodad, _)| doodad.index == index)
            .expect("placed doodad")
    }

    fn rgb(r: f32, g: f32, b: f32) -> Vec3 {
        Vec3::new(r, g, b) / 255.0
    }

    fn assert_near(actual: Vec3, expected: Vec3) {
        assert!(actual.abs_diff_eq(expected, 1e-5), "{actual} != {expected}");
    }

    /// Stockade (root 108631, every group but 26 INTERIOR 0x2000, MOHD ambient
    /// BGRA 19 19 19, no MAVG/MAVD, MDDI or 0x10 flags).
    /// MODD 100: flags 0x2, BGRA 56 4E 4D FF (no MOLT). Flag 0x2 skips the colour
    /// adjustment: direct = the MODD colour (77, 78, 86), ambient = MOHD (25, 25, 25),
    /// and the personal sun shines from its first group's MOGP box centre.
    #[test]
    fn stockade_doodad_takes_its_modd_colour_as_direct_light() {
        let asset = read_wmo(108_631);
        let (doodad, light) = light_of(&asset, 100);
        assert_eq!(doodad.flags, 0x2);
        assert_eq!(light.exterior_blend, 0.0, "groups {:?}", doodad.groups);
        assert_near(light.direct, rgb(77.0, 78.0, 86.0));
        for ambient in light.ambient {
            assert_near(ambient, rgb(25.0, 25.0, 25.0));
        }
        let group = &asset
            .groups
            .iter()
            .find(|group| group.index == u32::from(doodad.groups[0]))
            .unwrap()
            .group
            .header;
        let [x, y, z] = [0, 1, 2].map(|axis| (group.bbox_min[axis] + group.bbox_max[axis]) / 2.0);
        assert_near(light.sun_source.unwrap(), Vec3::new(x, z, -y));
    }

    /// Stockade MODD 3: flags 0x6, BGRA 19 19 19 00, so MOLT 0 (omni, BGRA 98 69 64,
    /// intensity 1, at (-159.42, -0.862, -4.484)). Flag 0x4 takes the light as the
    /// direct source; its colour (100, 105, 152) never exceeds 1 so the MODD colour
    /// stays; flag 0x2 then sets no direct colour. The personal sun shines from the
    /// light.
    #[test]
    fn stockade_molt_doodad_takes_its_sun_from_the_light() {
        let asset = read_wmo(108_631);
        let (doodad, light) = light_of(&asset, 3);
        assert_eq!(doodad.flags, 0x6);
        assert_eq!(light.exterior_blend, 0.0);
        assert_eq!(light.direct, Vec3::ZERO);
        for ambient in light.ambient {
            assert_near(ambient, rgb(25.0, 25.0, 25.0));
        }
        assert_near(
            light.sun_source.unwrap(),
            Vec3::new(-159.419_75, -4.484_417, 0.862_405_5),
        );
    }

    /// `sw_magicdistrict` (root 321999) Jail01 lamp MODD 32: flags 0x10, BGRA
    /// 5F 7D B3 FF, MDDI 0.703 (raised to 1); MAVG 0 (set 0, flags 0) ambient
    /// (33, 33, 33). Direct (179, 125, 95) is added to the ambient: (212, 158, 128),
    /// kept as direct (brighter than 0x70) and darkened to 0x60 as ambient.
    #[test]
    fn magic_district_lamp_adds_its_colour_to_the_interior_ambient() {
        let asset = read_wmo(321_999);
        let (doodad, light) = light_of(&asset, 32);
        assert_eq!(doodad.flags, 0x10);
        assert_eq!(light.exterior_blend, 0.0, "groups {:?}", doodad.groups);
        assert_near(light.direct, rgb(212.0, 158.0, 128.0));
        let scale = 96.0 / 212.0;
        for ambient in light.ambient {
            assert_near(ambient, rgb(212.0, 158.0, 128.0) * scale);
        }
        assert_eq!(light.sun_source, None);
    }

    /// A doodad referenced by an exterior group is exterior-lit and keeps the defaults.
    #[test]
    fn doodad_of_an_exterior_group_is_exterior_lit() {
        let asset = read_wmo(321_999);
        let exterior = asset
            .doodads(&[0, 0])
            .into_iter()
            .find(|(doodad, _)| {
                doodad.groups.iter().any(|&group| {
                    asset.groups.iter().any(|loaded| {
                        loaded.index == u32::from(group)
                            && group_exterior_lit(loaded.group.header.flags)
                    })
                })
            })
            .expect("an exterior doodad");
        assert_eq!(exterior.1.exterior_blend, 1.0);
    }
}
