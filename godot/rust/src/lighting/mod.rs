//! Native scene-light resources sampled from authored map-position/time data.

pub(crate) mod assets;
mod fog_cone;
mod planets;

use std::{collections::HashMap, ops::RangeInclusive};

use game_engine_core::{
    asset::wmo_format::fog::WmoFogBlend,
    lighting_assets::{authored_to_linear_rgb, linear_to_authored_rgb},
    retail_fog::{FogResult, FogUniforms, blend_wmo_fog, fog_uniforms, wmo_fog},
    retail_light_data::RetailLightData,
    sky_cubemap_data::{self, sky_dome_profile},
};
use godot::{
    classes::{
        Cubemap, DirectionalLight3D, Environment, Image, Node3D, RenderingServer, ResourceLoader,
        Shader, ShaderMaterial, Sky, WorldEnvironment, directional_light_3d, environment, image,
        sky,
    },
    prelude::*,
};

use assets::{LightingCatalog, LightingSample, WaterLight};
use game_engine_core::sky_bodies::{LIGHT_SKYBOX_FINAL_FOG, PlanetDraw, STARS_FDID, SkyboxDraw};

use crate::sky_model::SkyModel;

type SkyStops = [[f32; 3]; 7];
type LightValues = (
    RetailLightData,
    FogResult,
    SkyStops,
    WaterLight,
    Option<f32>,
    Vec<SkyboxDraw>,
    Vec<PlanetDraw>,
);

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

    /// Writes the scene light and fog: the global uniforms every `bind_model` material
    /// reads (`shaders/scene_light.gdshaderinc`, `shaders/retail_fog.gdshaderinc`), once
    /// per light change rather than per material, as WebWowViewerCpp fills its per-scene
    /// SceneWideParams (commonLightFunctions.slang:26-52) once per frame.
    pub fn bind_scene(&self) {
        let mut server = RenderingServer::singleton();
        for (name, value) in [
            ("scene_ambient", self.retail.ambient),
            ("scene_horizon_ambient", self.retail.horizon_ambient),
            ("scene_ground_ambient", self.retail.ground_ambient),
            ("scene_direct", self.retail.direct),
            ("scene_sun_direction", self.retail.sun_direction),
        ] {
            server.global_shader_parameter_set(name, &Vector3::from_array(value).to_variant());
        }
        bind_fog(&mut server, &self.fog);
    }

    /// The scene light and fog range `bind_scene` writes, for automation (scene-lit
    /// materials hold none of it).
    pub fn scene_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        for (name, value) in [
            ("ambient", self.retail.ambient),
            ("horizon_ambient", self.retail.horizon_ambient),
            ("ground_ambient", self.retail.ground_ambient),
            ("direct", self.retail.direct),
            ("sun_direction", self.retail.sun_direction),
            ("fog_color", self.fog.color),
        ] {
            state.set(name, Vector3::from_array(value));
        }
        state.set("fog_range", Vector2::from_array(self.fog.range));
        state
    }

    /// Lights `material` with the scene light and fog of `bind_scene`.
    pub fn bind_model(&self, material: &mut Gd<ShaderMaterial>) {
        material.set_shader_parameter("scene_light", &true.to_variant());
        self.bind_scene_fog(material);
    }

    /// Only the scene fog (`retail_fog.gdshaderinc`), for unlit materials such as particles.
    pub fn bind_scene_fog(&self, material: &mut Gd<ShaderMaterial>) {
        material.set_shader_parameter("fog_mode", &1i32.to_variant());
        material.set_shader_parameter("fog_opacity", &1.0f32.to_variant());
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

    /// Back to the material's own light, without fog.
    pub fn clear_model(material: &mut Gd<ShaderMaterial>) {
        for name in ["scene_light", "fog_mode", "fog_opacity"] {
            material.set_shader_parameter(name, &Variant::nil());
        }
    }
}

#[derive(Default)]
pub(crate) struct WorldLighting {
    root: Option<Gd<Node3D>>,
    sun: Option<Gd<DirectionalLight3D>>,
    environment: Option<Gd<Environment>>,
    sky: Option<Gd<ShaderMaterial>>,
    stars: Option<SkyModel>,
    /// LightSkybox models by FDID, with the day fraction a flag 0x1 skybox is held at.
    skyboxes: HashMap<u32, (SkyModel, Option<f32>)>,
    planets: Option<planets::Planets>,
    fog_cone: Option<fog_cone::FogCone>,
    previous: Option<LightValues>,
}

/// Render priorities of the sky models, in WebWowViewerCpp's sky-view draw order (dome,
/// stars, planets, skybox models, 0x4 fog cone; `ViewsObjects.cpp:40-55`). Godot draws
/// transparents by priority first, so these stay below the world's (0): the sky view draws
/// before the world, and water or particles in front of the far plane blend over it.
/// Godot's priorities start at -128; a model's batches take consecutive priorities in its band.
const STARS_PRIORITIES: RangeInclusive<i32> = -128..=-113;
pub(super) const PLANETS_PRIORITY: i32 = -112;
const SKYBOX_PRIORITIES: RangeInclusive<i32> = -111..=-2;
pub(super) const FOG_CONE_PRIORITY: i32 = -1;

const SKY_DOME_SHADER: &str = "res://shaders/sky_dome.gdshader";

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
        let stars_alpha = sample.stars_alpha;
        let values = (
            sample.retail.clone(),
            fog,
            stops,
            sample.water.clone(),
            stars_alpha,
            sample.skyboxes.clone(),
            sample.planets.clone(),
        );
        if self.previous.as_ref() == Some(&values) {
            return Ok(None);
        }
        let direction = Vector3::from_array(sample.retail.sun_direction);
        let light = TerrainLight::new(sample, fog)?;
        light.bind_scene();
        self.attach_nodes(parent)?;
        bind_sky_dome(self.sky.as_mut().expect("attached sky"), &stops, &light.fog);
        self.sync_stars(catalog, stars_alpha)?;
        self.sync_skyboxes(catalog, &values.5, minutes)?;
        self.sync_planets(catalog, &values.6, values.3.specular)?;
        let final_fog = values
            .5
            .iter()
            .any(|draw| draw.flags & LIGHT_SKYBOX_FINAL_FOG != 0);
        self.sync_fog_cone(final_fog, fog.end_fog_color, stops[5], &light.fog)?;
        self.sun
            .as_mut()
            .expect("attached sun")
            .look_at_from_position(Vector3::ZERO, direction);
        self.previous = Some(values);
        Ok(Some(light))
    }

    /// map.cpp: the stars model draws in the sky view while `stars.enabled`, at the
    /// night alpha.
    fn sync_stars(&mut self, catalog: &LightingCatalog, alpha: Option<f32>) -> Result<(), String> {
        if self.stars.is_none() && alpha.is_some() {
            let mut stars =
                SkyModel::load_model(&catalog.data_root, &catalog.stars_path, STARS_FDID, None)?;
            stars.place_render_priorities(STARS_PRIORITIES)?;
            self.root
                .as_mut()
                .expect("attached root")
                .add_child(&stars.node);
            self.stars = Some(stars);
        }
        if let Some(stars) = self.stars.as_mut() {
            stars.node.set_visible(alpha.is_some());
            stars.set_alpha(alpha.unwrap_or(0.0));
        }
        Ok(())
    }

    /// map.cpp: the Light's LightSkybox models draw in the sky view at their collected
    /// alphas; a flag 0x1 skybox holds its animation at the day's fraction.
    fn sync_skyboxes(
        &mut self,
        catalog: &LightingCatalog,
        draws: &[SkyboxDraw],
        minutes: f32,
    ) -> Result<(), String> {
        for draw in draws {
            let fraction = (draw.flags & 1 != 0).then_some(minutes.rem_euclid(2880.0) / 2880.0);
            if self
                .skyboxes
                .get(&draw.fdid)
                .is_some_and(|(_, held)| *held != fraction)
            {
                let (stale, _) = self.skyboxes.remove(&draw.fdid).expect("present");
                stale.node.free();
            }
            if !self.skyboxes.contains_key(&draw.fdid) {
                let path = assets::cache_sky_model(&catalog.data_root, draw.fdid)
                    .map_err(|error| format!("Skybox {}: {error}", draw.fdid))?;
                let mut model = match fraction {
                    Some(fraction) => {
                        SkyModel::load_at_fraction(&catalog.data_root, &path, draw.fdid, fraction)?
                    }
                    None => SkyModel::load_model(&catalog.data_root, &path, draw.fdid, None)?,
                };
                model
                    .place_render_priorities(SKYBOX_PRIORITIES)
                    .map_err(|error| format!("Skybox {}: {error}", draw.fdid))?;
                self.root
                    .as_mut()
                    .expect("attached root")
                    .add_child(&model.node);
                self.skyboxes.insert(draw.fdid, (model, fraction));
            }
            let (model, _) = self.skyboxes.get_mut(&draw.fdid).expect("loaded above");
            model.node.set_visible(true);
            model.set_alpha(draw.alpha);
        }
        for (fdid, (model, _)) in &mut self.skyboxes {
            if !draws.iter().any(|draw| draw.fdid == *fdid) {
                model.node.set_visible(false);
            }
        }
        Ok(())
    }

    /// map.cpp: the sun and moon discs draw in the sky view while visible.
    fn sync_planets(
        &mut self,
        catalog: &LightingCatalog,
        draws: &[PlanetDraw],
        color: [f32; 3],
    ) -> Result<(), String> {
        if self.planets.is_none() {
            let root = self.root.as_mut().expect("attached root");
            self.planets = Some(planets::Planets::load(root, &catalog.data_root)?);
        }
        self.planets
            .as_mut()
            .expect("loaded above")
            .sync(draws, color);
        Ok(())
    }

    /// map.cpp: the 0x4 fog cone draws after the skybox models while one has flag 0x4.
    fn sync_fog_cone(
        &mut self,
        shown: bool,
        end_fog_color: [f32; 3],
        sky_fog: [f32; 3],
        fog: &FogUniforms,
    ) -> Result<(), String> {
        if self.fog_cone.is_none() {
            let root = self.root.as_mut().expect("attached root");
            self.fog_cone = Some(fog_cone::FogCone::load(root)?);
        }
        self.fog_cone
            .as_mut()
            .expect("loaded above")
            .sync(shown, end_fog_color, sky_fog, fog);
        Ok(())
    }

    /// Sky models sit on the camera (WebWowViewerCpp's sky view drops the view
    /// translation) and play their animation at `time_ms`.
    pub fn place_sky(&mut self, camera: Vector3, time_ms: u32) {
        if let Some(stars) = self.stars.as_mut().filter(|stars| stars.node.is_visible()) {
            stars.node.set_global_position(camera);
            stars.sample(time_ms);
        }
        if let Some(planets) = self.planets.as_mut() {
            planets.place(camera);
        }
        if let Some(cone) = self.fog_cone.as_mut() {
            cone.place(camera);
        }
        for (model, fraction) in self.skyboxes.values_mut() {
            if model.node.is_visible() {
                model.node.set_global_position(camera);
                if fraction.is_none() {
                    model.sample(time_ms);
                }
            }
        }
    }

    fn attach_nodes(&mut self, parent: &mut Gd<Node3D>) -> Result<(), String> {
        if self.root.is_some() {
            return Ok(());
        }
        let sky_material = sky_dome_material()?;
        let mut root = Node3D::new_alloc();
        root.set_name("WorldLighting");
        parent.add_child(&root);
        let mut environment = Environment::new_gd();
        environment.set_ambient_source(environment::AmbientSource::DISABLED);
        environment.set_reflection_source(environment::ReflectionSource::DISABLED);
        environment.set_tonemapper(environment::ToneMapper::LINEAR);
        let mut sky = Sky::new_gd();
        sky.set_material(&sky_material);
        sky.set_radiance_size(sky::RadianceSize::SIZE_32);
        environment.set_sky(&sky);
        environment.set_background(environment::BgMode::SKY);
        let mut environment_node = WorldEnvironment::new_alloc();
        environment_node.set_name("Environment");
        environment_node.set_environment(&environment);
        root.add_child(&environment_node);
        let mut sun = DirectionalLight3D::new_alloc();
        sun.set_name("Sun");
        sun.set_shadow(true);
        // Two cascades, as the retail client writes `shadowNumCascades 2` for shadow
        // quality Medium (`graphicsShadowQuality 2`, `_retail_/WTF/Config.wtf`).
        sun.set_shadow_mode(directional_light_3d::ShadowMode::PARALLEL_2_SPLITS);
        root.add_child(&sun);
        self.sun = Some(sun);
        self.environment = Some(environment);
        self.sky = Some(sky_material);
        self.root = Some(root);
        Ok(())
    }

    /// The attached sun, once the first sample placed it.
    pub fn sun(&self) -> Option<Gd<DirectionalLight3D>> {
        self.sun.clone()
    }

    pub fn set_ghost_grading(&mut self, ghost: bool) {
        if let Some(environment) = self.environment.as_mut() {
            environment.set_adjustment_enabled(ghost);
            environment.set_adjustment_saturation(if ghost { 0.2 } else { 1.0 });
        }
    }

    pub fn reset(&mut self) {
        self.sun = None;
        self.environment = None;
        self.sky = None;
        self.stars = None;
        self.skyboxes.clear();
        self.planets = None;
        self.fog_cone = None;
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.previous = None;
    }
}

fn sky_dome_material() -> Result<Gd<ShaderMaterial>, String> {
    let shader = ResourceLoader::singleton()
        .load(SKY_DOME_SHADER)
        .and_then(|resource| resource.try_cast::<Shader>().ok())
        .ok_or_else(|| format!("Sky dome shader {SKY_DOME_SHADER} failed to load"))?;
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    let points: PackedVector2Array = sky_dome_profile()
        .iter()
        .map(|point| Vector2::new(point.horizontal, point.height))
        .collect();
    material.set_shader_parameter("dome_points", &points.to_variant());
    Ok(material)
}

/// The exterior sky dome's ring colours (map.cpp `Map::updateBuffers` skyColor[0..5])
/// and the scene sun fog it scatters (skyConus.frag.slang).
fn bind_sky_dome(material: &mut Gd<ShaderMaterial>, stops: &SkyStops, fog: &FogUniforms) {
    let stops: PackedVector3Array = stops.iter().copied().map(Vector3::from_array).collect();
    material.set_shader_parameter("sky_stops", &stops.to_variant());
    for (name, value) in [
        ("fog_sun_direction", fog.sun_direction),
        ("fog_sun_color", fog.sun_color),
    ] {
        material.set_shader_parameter(name, &Vector3::from_array(value).to_variant());
    }
    for (name, value) in [
        ("fog_sun_angle", fog.sun_angle),
        ("fog_sun_percentage", fog.sun_percentage),
    ] {
        material.set_shader_parameter(name, &value.to_variant());
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

fn bind_fog(server: &mut Gd<RenderingServer>, fog: &FogUniforms) {
    let mut set = |name: &str, value: Variant| server.global_shader_parameter_set(name, &value);
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
