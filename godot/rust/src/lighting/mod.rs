//! Native scene-light resources sampled from authored map-position/time data.

pub(crate) mod assets;

use game_engine_core::{
    asset::wmo_format::fog::WmoFogBlend,
    lighting_assets::{authored_to_linear_rgb, linear_to_authored_rgb},
    retail_fog::{FogResult, FogUniforms, blend_wmo_fog, fog_uniforms, wmo_fog},
    retail_light_data::RetailLightData,
    sky_cubemap_data,
};
use godot::{
    classes::{
        Cubemap, DirectionalLight3D, Environment, Image, Node3D, ShaderMaterial, WorldEnvironment,
        environment, image,
    },
    prelude::*,
};

use assets::{LightingCatalog, LightingSample, WaterLight};

type SkyStops = [[f32; 3]; 7];

#[derive(Clone)]
pub(crate) struct TerrainLight {
    retail: RetailLightData,
    fog: FogUniforms,
    cube: Gd<Cubemap>,
    water: WaterLight,
}

impl TerrainLight {
    /// The light of `sample` with its scene fog (the exterior or a WMO interior's).
    pub fn new(sample: LightingSample, fog: FogResult) -> Result<Self, String> {
        let sky = &sample.sky;
        let cube = create_cubemap([
            sky.sky_top,
            sky.sky_middle,
            sky.sky_band1,
            sky.sky_band2,
            sky.sky_smog,
            sky.fog_color,
            sky.fog_color,
        ])?;
        Ok(Self {
            retail: sample.retail,
            fog: fog_uniforms(&fog, sample.fog_sun_direction),
            cube,
            water: sample.water,
        })
    }

    pub fn bind(&self, material: &mut Gd<ShaderMaterial>) {
        self.bind_model(material);
        material.set_shader_parameter("environment_map", &self.cube.to_variant());
    }

    pub fn bind_model(&self, material: &mut Gd<ShaderMaterial>) {
        for (name, value) in [
            ("ambient", self.retail.ambient),
            ("horizon_ambient", self.retail.horizon_ambient),
            ("ground_ambient", self.retail.ground_ambient),
            ("direct", self.retail.direct),
            ("sun_direction", self.retail.sun_direction),
        ] {
            material.set_shader_parameter(name, &Vector3::from_array(value).to_variant());
        }
        bind_fog(material, &self.fog);
    }

    /// The model light plus the retail water scene inputs of `water.gdshader`.
    pub fn bind_water(&self, material: &mut Gd<ShaderMaterial>) {
        self.bind_model(material);
        let water = &self.water;
        for (name, value) in [
            ("river_close", water.river_close),
            ("river_far", water.river_far),
            ("ocean_close", water.ocean_close),
            ("ocean_far", water.ocean_far),
        ] {
            material.set_shader_parameter(name, &Vector4::from_array(value).to_variant());
        }
        for (name, value) in [
            ("specular_color", water.specular),
            ("underwater_fog_color", water.underwater_fog_color),
            (
                "underwater_fog",
                [
                    water.underwater_fog.start,
                    water.underwater_fog.end,
                    water.underwater_fog.density,
                ],
            ),
        ] {
            material.set_shader_parameter(name, &Vector3::from_array(value).to_variant());
        }
    }

    pub fn clear_model(material: &mut Gd<ShaderMaterial>) {
        for name in [
            "ambient",
            "horizon_ambient",
            "ground_ambient",
            "direct",
            "sun_direction",
        ]
        .into_iter()
        .chain(FOG_UNIFORMS)
        {
            material.set_shader_parameter(name, &Variant::nil());
        }
    }
}

#[derive(Default)]
pub(crate) struct WorldLighting {
    root: Option<Gd<Node3D>>,
    sun: Option<Gd<DirectionalLight3D>>,
    previous: Option<(RetailLightData, FogResult, SkyStops, WaterLight)>,
}

impl WorldLighting {
    /// Samples the authored light at `position`; `wmo_fog` is the MFOG fog of the WMO
    /// interior the camera is in.
    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        catalog: &LightingCatalog,
        map_id: u32,
        position: Vector3,
        minutes: f32,
        wmo_fog: Option<&WmoFogBlend>,
    ) -> Result<Option<TerrainLight>, String> {
        let sample = catalog.sample(map_id, [position.x, -position.z, position.y], minutes)?;
        let sky = &sample.sky;
        let stops = [
            sky.sky_top,
            sky.sky_middle,
            sky.sky_band1,
            sky.sky_band2,
            sky.sky_smog,
            sky.fog_color,
            sky.fog_color,
        ];
        let fog = apply_wmo_fog(&sample.fog, wmo_fog);
        let values = (sample.retail.clone(), fog, stops, sample.water.clone());
        if self.previous.as_ref() == Some(&values) {
            return Ok(None);
        }
        let direction = Vector3::from_array(sample.retail.sun_direction);
        let light = TerrainLight::new(sample, fog)?;
        self.attach_nodes(parent);
        self.sun
            .as_mut()
            .expect("attached sun")
            .look_at_from_position(Vector3::ZERO, direction);
        self.previous = Some(values);
        Ok(Some(light))
    }

    fn attach_nodes(&mut self, parent: &mut Gd<Node3D>) {
        if self.root.is_some() {
            return;
        }
        let mut root = Node3D::new_alloc();
        root.set_name("WorldLighting");
        parent.add_child(&root);
        let mut environment = Environment::new_gd();
        environment.set_ambient_source(environment::AmbientSource::DISABLED);
        environment.set_reflection_source(environment::ReflectionSource::DISABLED);
        environment.set_tonemapper(environment::ToneMapper::LINEAR);
        let mut environment_node = WorldEnvironment::new_alloc();
        environment_node.set_name("Environment");
        environment_node.set_environment(&environment);
        root.add_child(&environment_node);
        let mut sun = DirectionalLight3D::new_alloc();
        sun.set_name("Sun");
        sun.set_shadow(true);
        root.add_child(&sun);
        self.sun = Some(sun);
        self.root = Some(root);
    }

    pub fn reset(&mut self) {
        self.sun = None;
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.previous = None;
    }
}

/// DayNightLightHolder.cpp:491-497: inside a WMO interior group, the exterior fog mixes
/// toward the WMO's MFOG fog by `WmoFogBlend::weight` (`wmoFogDataToFogResult`,
/// `blendWmoFogIntoFogResult`). Colours mix in authored space, as the reference mixes its
/// byte colours. Underwater fog is not ported.
pub(crate) fn apply_wmo_fog(fog: &FogResult, wmo: Option<&WmoFogBlend>) -> FogResult {
    let Some(wmo) = wmo else {
        return *fog;
    };
    let color = wmo.fog.color.map(|channel| f32::from(channel) / 255.0);
    let authored = |fog: &FogResult| FogResult {
        fog_color: linear_to_authored_rgb(fog.fog_color),
        end_fog_color: linear_to_authored_rgb(fog.end_fog_color),
        sun_fog_color: linear_to_authored_rgb(fog.sun_fog_color),
        fog_height_color: linear_to_authored_rgb(fog.fog_height_color),
        ..*fog
    };
    let mut blended = blend_wmo_fog(
        &authored(fog),
        &wmo_fog(wmo.fog.end, wmo.fog.start_scalar, color),
        wmo.weight(),
    );
    for color in [
        &mut blended.fog_color,
        &mut blended.end_fog_color,
        &mut blended.sun_fog_color,
        &mut blended.fog_height_color,
    ] {
        *color = authored_to_linear_rgb(*color);
    }
    blended
}

/// Every uniform of `shaders/retail_fog.gdshaderinc`.
const FOG_UNIFORMS: [&str; 22] = [
    "fog_mode",
    "fog_opacity",
    "fog_color",
    "fog_end_color",
    "fog_height_color",
    "fog_height_end_color",
    "fog_sun_color",
    "fog_range",
    "fog_density",
    "fog_height_density",
    "fog_height",
    "fog_height_rate",
    "fog_z_scalar",
    "fog_legacy_scalar",
    "fog_main_range",
    "fog_color_range",
    "fog_height_coefficients",
    "fog_main_coefficients",
    "fog_height_density_coefficients",
    "fog_sun_direction",
    "fog_sun_angle",
    "fog_sun_percentage",
];

fn bind_fog(material: &mut Gd<ShaderMaterial>, fog: &FogUniforms) {
    let mut set = |name: &str, value: Variant| material.set_shader_parameter(name, &value);
    set("fog_mode", 1i32.to_variant());
    set("fog_opacity", 1.0f32.to_variant());
    for (name, value) in [
        ("fog_color", fog.color),
        ("fog_end_color", fog.end_color),
        ("fog_height_color", fog.height_color),
        ("fog_height_end_color", fog.height_end_color),
        ("fog_sun_color", fog.sun_color),
        ("fog_sun_direction", fog.sun_direction),
    ] {
        set(name, Vector3::from_array(value).to_variant());
    }
    for (name, value) in [
        ("fog_range", fog.range),
        ("fog_main_range", fog.main_range),
        ("fog_color_range", fog.color_range),
    ] {
        set(name, Vector2::from_array(value).to_variant());
    }
    for (name, value) in [
        ("fog_density", fog.density),
        ("fog_height_density", fog.height_density),
        ("fog_height", fog.height),
        ("fog_height_rate", fog.height_rate),
        ("fog_z_scalar", fog.z_scalar),
        ("fog_legacy_scalar", fog.legacy_scalar),
        ("fog_sun_angle", fog.sun_angle),
        ("fog_sun_percentage", fog.sun_percentage),
    ] {
        set(name, value.to_variant());
    }
    for (name, value) in [
        ("fog_height_coefficients", fog.height_coefficients),
        ("fog_main_coefficients", fog.main_coefficients),
        (
            "fog_height_density_coefficients",
            fog.height_density_coefficients,
        ),
    ] {
        set(name, Vector4::from_array(value).to_variant());
    }
}

fn create_cubemap(stops: SkyStops) -> Result<Gd<Cubemap>, String> {
    let pixels = sky_cubemap_data::build_sky_cubemap(&stops);
    let size = sky_cubemap_data::ENV_MAP_SIZE as i32;
    let face_bytes = size as usize * size as usize * 8;
    let mut faces = Array::<Gd<Image>>::new();
    for pixels in pixels.chunks_exact(face_bytes) {
        let image = Image::create_from_data(
            size,
            size,
            false,
            image::Format::RGBAH,
            &PackedByteArray::from(pixels),
        )
        .ok_or("Godot rejected authored linear sky cubemap face")?;
        faces.push(&image);
    }
    let mut cube = Cubemap::new_gd();
    let error = cube.create_from_images(&faces);
    if error != godot::global::Error::OK {
        return Err(format!("Godot rejected authored sky cubemap: {error:?}"));
    }
    Ok(cube)
}

#[cfg(test)]
mod tests {
    use game_engine_core::{
        asset::wmo_format::fog::{WmoFogBlend, WmoFogData},
        lighting_assets::{authored_to_linear_rgb, linear_to_authored_rgb},
    };

    use game_engine_core::retail_fog::fog_uniforms;

    use super::{apply_wmo_fog, assets::LightingCatalog};

    const CAVE: WmoFogData = WmoFogData {
        end: 578.0,
        start_scalar: 0.129,
        color: [21, 80, 99],
    };

    fn cave_at(dist_to_exit: f32) -> WmoFogBlend {
        WmoFogBlend {
            fog: CAVE,
            underwater: CAVE,
            dist_to_exit,
        }
    }

    /// Cultists' Quay at noon: LightParams 12 fog, replaced by the cave's MFOG record 0
    /// deep inside (legacy fog from 74.6 yd at density 1.5, the cave colour everywhere, no
    /// sun fog), and half of each 12.5 yd from a portal.
    #[test]
    fn cultists_quay_scene_fog_takes_the_cave_mfog() {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let catalog = LightingCatalog::read(&data_root).unwrap();
        let sample = catalog
            .sample(2837, [181.911_45, 2500.392_3, 94.236_43], 1440.0)
            .unwrap();
        let exterior = sample.fog;
        assert_eq!(apply_wmo_fog(&exterior, None), exterior);

        let deep = apply_wmo_fog(&exterior, Some(&cave_at(f32::MAX)));
        let uniforms = fog_uniforms(&deep, [0.0, 1.0, 0.0]);
        assert!((uniforms.range[0] - 74.562).abs() < 1e-3, "{uniforms:?}");
        assert_eq!(uniforms.range[1], 1000.0);
        assert!((uniforms.density - 0.000_75).abs() < 1e-9, "{uniforms:?}");
        assert_eq!(deep.legacy_fog_scalar, 1.0);
        assert_eq!((deep.sun_fog_angle, deep.sun_fog_strength), (0.0, 0.0));
        let expected = authored_to_linear_rgb([21.0, 80.0, 99.0].map(|byte| byte / 255.0));
        for color in [deep.fog_color, deep.end_fog_color, deep.fog_height_color] {
            for (channel, want) in color.iter().zip(expected) {
                assert!((channel - want).abs() < 1e-6, "{color:?} vs {expected:?}");
            }
        }

        let half = apply_wmo_fog(&exterior, Some(&cave_at(12.5)));
        assert!((half.fog_scaler - (exterior.fog_scaler + 0.074_562) / 2.0).abs() < 1e-5);
        assert!((half.fog_density - (exterior.fog_density + 1.5) / 2.0).abs() < 1e-5);
        let authored = linear_to_authored_rgb(exterior.fog_color);
        let mixed = linear_to_authored_rgb(half.fog_color);
        for channel in 0..3 {
            let want = (authored[channel] + [21.0, 80.0, 99.0][channel] / 255.0) / 2.0;
            assert!((mixed[channel] - want).abs() < 1e-5);
        }
    }
}
