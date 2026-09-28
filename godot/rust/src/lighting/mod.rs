//! Native scene-light resources sampled from authored map-position/time data.

pub(crate) mod assets;

use game_engine_core::{
    asset::wmo_format::fog::WmoFogBlend,
    lighting_assets::{authored_to_linear_rgb, linear_to_authored_rgb},
    retail_light_data::RetailLightData,
    sky_cubemap_data,
    sky_lightdata_data::{RetailFog, blend_wmo_fog, wmo_retail_fog},
};
use godot::{
    classes::{
        Cubemap, DirectionalLight3D, Environment, Image, Node3D, ShaderMaterial, WorldEnvironment,
        environment, image,
    },
    prelude::*,
};

use assets::LightingCatalog;

type SkyStops = [[f32; 3]; 7];

#[derive(Clone)]
pub(crate) struct TerrainLight {
    retail: RetailLightData,
    fog: RetailFog,
    fog_color: [f32; 3],
    cube: Gd<Cubemap>,
}

impl TerrainLight {
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
            ("fog_color", self.fog_color),
        ] {
            material.set_shader_parameter(name, &Vector3::from_array(value).to_variant());
        }
        let range = Vector2::new(self.fog.start, self.fog.end);
        material.set_shader_parameter("fog_range", &range.to_variant());
        material.set_shader_parameter("fog_density", &self.fog.density.to_variant());
        material.set_shader_parameter("fog_opacity", &1.0f32.to_variant());
        material.set_shader_parameter("fog_mode", &1i32.to_variant());
    }

    pub fn clear_model(material: &mut Gd<ShaderMaterial>) {
        for name in [
            "ambient",
            "horizon_ambient",
            "ground_ambient",
            "direct",
            "sun_direction",
            "fog_color",
            "fog_range",
            "fog_density",
            "fog_opacity",
            "fog_mode",
        ] {
            material.set_shader_parameter(name, &Variant::nil());
        }
    }
}

#[derive(Default)]
pub(crate) struct WorldLighting {
    root: Option<Gd<Node3D>>,
    sun: Option<Gd<DirectionalLight3D>>,
    previous: Option<(RetailLightData, RetailFog, [f32; 3], SkyStops)>,
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
        let (fog, fog_color) = apply_wmo_fog(sample.fog, sky.fog_color, wmo_fog);
        let values = (sample.retail.clone(), fog, fog_color, stops);
        if self.previous.as_ref() == Some(&values) {
            return Ok(None);
        }
        let cube = create_cubemap(stops)?;
        self.attach_nodes(parent);
        let direction = Vector3::from_array(sample.retail.sun_direction);
        self.sun
            .as_mut()
            .expect("attached sun")
            .look_at_from_position(Vector3::ZERO, direction);
        self.previous = Some(values);
        Ok(Some(TerrainLight {
            retail: sample.retail,
            fog,
            fog_color,
            cube,
        }))
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

/// DayNightLightHolder.cpp:491-497: inside a WMO interior group, the exterior fog
/// (`fog_color` linear) mixes toward the WMO's MFOG fog by `WmoFogBlend::weight`. Colours
/// mix in authored space, as the reference mixes its byte colours
/// (`blendWmoFogIntoFogResult` :712-716). Underwater fog is not ported.
pub(crate) fn apply_wmo_fog(
    fog: RetailFog,
    fog_color: [f32; 3],
    wmo: Option<&WmoFogBlend>,
) -> (RetailFog, [f32; 3]) {
    let Some(wmo) = wmo else {
        return (fog, fog_color);
    };
    let weight = wmo.weight();
    let fog = blend_wmo_fog(
        fog,
        wmo_retail_fog(wmo.fog.end, wmo.fog.start_scalar),
        weight,
    );
    let exterior = linear_to_authored_rgb(fog_color);
    let authored = std::array::from_fn(|channel| {
        let target = f32::from(wmo.fog.color[channel]) / 255.0;
        exterior[channel] + (target - exterior[channel]) * weight
    });
    (fog, authored_to_linear_rgb(authored))
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
    /// deep inside, and half of each 12.5 yd from a portal.
    #[test]
    fn cultists_quay_scene_fog_takes_the_cave_mfog() {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let catalog = LightingCatalog::read(&data_root).unwrap();
        let sample = catalog
            .sample(2837, [181.911_45, 2500.392_3, 94.236_43], 1440.0)
            .unwrap();
        let exterior = (sample.fog, sample.sky.fog_color);
        assert_eq!(apply_wmo_fog(exterior.0, exterior.1, None), exterior);

        let (fog, color) = apply_wmo_fog(exterior.0, exterior.1, Some(&cave_at(f32::MAX)));
        assert!((fog.start - 74.562).abs() < 1e-3, "{fog:?}");
        assert_eq!(fog.end, 1000.0);
        assert!((fog.density - 0.000_75).abs() < 1e-9, "{fog:?}");
        let expected = authored_to_linear_rgb([21.0, 80.0, 99.0].map(|byte| byte / 255.0));
        for (channel, want) in color.iter().zip(expected) {
            assert!((channel - want).abs() < 1e-6, "{color:?} vs {expected:?}");
        }

        let (half, half_color) = apply_wmo_fog(exterior.0, exterior.1, Some(&cave_at(12.5)));
        assert!((half.start - (exterior.0.start + 74.562) / 2.0).abs() < 1e-3);
        assert!((half.density - (exterior.0.density + 0.000_75) / 2.0).abs() < 1e-9);
        let authored = linear_to_authored_rgb(exterior.1);
        let mixed = linear_to_authored_rgb(half_color);
        for channel in 0..3 {
            let want = (authored[channel] + [21.0, 80.0, 99.0][channel] / 255.0) / 2.0;
            assert!((mixed[channel] - want).abs() < 1e-5);
        }
    }
}
