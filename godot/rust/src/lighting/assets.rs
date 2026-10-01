//! Authored light catalogs loaded on the terrain asset worker.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use game_engine_core::{
    light_lookup_data::{
        LightEntry, LightParamsSlot, ZoneLight, light_params_blend, parse_zone_lights,
    },
    lighting_assets::{
        LightKeyframes, linear_to_authored_rgb, parse_light_csv, parse_light_data_csv,
    },
    retail_fog::{
        FogKeyframes, FogResult, parse_fog_keyframes, sample_fog_blend, sun_fog_direction,
    },
    retail_light_data::{RetailLightColors, RetailLightData, scene_light},
    sky_bodies::{LIGHT_PARAMS_HIDE_STARS, STARS_FDID, stars_alpha},
    sky_lightdata_data::{RetailFog, SkyColorSet, retail_fog, sample_light_blend},
};

use crate::assets::creature::{cache_model_files, cache_model_textures, local_resolver};

pub(crate) struct LightingCatalog {
    lights: Vec<LightEntry>,
    zone_lights: Vec<ZoneLight>,
    keyframes: LightKeyframes,
    fog_keyframes: FogKeyframes,
    /// `LightParams` Water/Ocean Shallow/Deep alphas and flags by LightParams ID.
    liquid_alphas: HashMap<u32, LiquidAlphas>,
    light_params_flags: HashMap<u32, u32>,
    pub data_root: PathBuf,
    /// The stars model, extracted from local CASC with its skin and textures.
    pub stars_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct LiquidAlphas {
    river: [f32; 2],
    ocean: [f32; 2],
}

pub(crate) struct LightingSample {
    pub retail: RetailLightData,
    pub fog: FogResult,
    /// World direction toward the sun disc, for the fog's sun scattering.
    pub fog_sun_direction: [f32; 3],
    pub sky: SkyColorSet<[f32; 3]>,
    pub water: WaterLight,
    /// The stars model's alpha, `None` when no stars are drawn.
    pub stars_alpha: Option<f32>,
}

/// Scene inputs of the retail water material in authored RGB (WebWowViewerCpp
/// DayNightLightHolder.cpp:928-937 liquid colours and alphas; MapSceneRenderer.cpp:194-203
/// close/far colour with shallow/deep alpha, specular = SunColor; :297-315 underwater fog
/// from the Light's underwater LightParams slot).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WaterLight {
    pub river_close: [f32; 4],
    pub river_far: [f32; 4],
    pub ocean_close: [f32; 4],
    pub ocean_far: [f32; 4],
    pub specular: [f32; 3],
    pub underwater_fog: RetailFog,
    pub underwater_fog_color: [f32; 3],
}

impl LightingCatalog {
    pub fn read(data_root: &Path) -> Result<Self, String> {
        let lights_path = data_root.join("Light.csv");
        let keyframes_path = data_root.join("LightData.csv");
        let lights = parse_light_csv(&read_text(&lights_path)?)
            .map_err(|error| format!("{}: {error}", lights_path.display()))?;
        let keyframes_text = read_text(&keyframes_path)?;
        let keyframes = parse_light_data_csv(&keyframes_text)
            .map_err(|error| format!("{}: {error}", keyframes_path.display()))?;
        let fog_keyframes = parse_fog_keyframes(&keyframes_text)
            .map_err(|error| format!("{}: {error}", keyframes_path.display()))?;
        let zone_lights = parse_zone_lights(
            &read_text(&data_root.join("ZoneLight.csv"))?,
            &read_text(&data_root.join("ZoneLightPoint.csv"))?,
        )
        .map_err(|error| format!("{}: {error}", data_root.display()))?;
        let (liquid_alphas, light_params_flags) =
            parse_light_params(&data_root.join("db2/12.1.0.69933/LightParams.csv"))?;
        let resolver = local_resolver(data_root);
        let stars_path = cache_model_files(&resolver, data_root, STARS_FDID)
            .map_err(|error| format!("Stars model {STARS_FDID}: {error}"))?;
        let stars = crate::assets::read_model_file(&stars_path)?;
        cache_model_textures(&resolver, data_root, &[0; 3], &stars)?;
        Ok(Self {
            lights,
            zone_lights,
            keyframes,
            fog_keyframes,
            liquid_alphas,
            light_params_flags,
            data_root: data_root.to_path_buf(),
            stars_path,
        })
    }

    pub fn sample(
        &self,
        map_id: u32,
        wow_position: [f32; 3],
        minutes: f32,
    ) -> Result<LightingSample, String> {
        let weights = self.blend_weights(map_id, wow_position, LightParamsSlot::Clear)?;
        let mut sky = self.sample_sky(&weights, minutes, map_id, wow_position)?;
        let alphas = self.blend_liquid_alphas(&weights)?;
        let retail = scene_light(&retail_colors(&sky), minutes);
        let fog = sample_fog_blend(
            &self.fog_keyframes,
            &self.light_params_flags,
            &weights,
            minutes,
        )
        .ok_or_else(|| format!("No authored fog for map {map_id} at {wow_position:?}"))?;
        // getLightResultsFromDB :850-851: the sky's fog band leans toward the end fog colour
        // by farClip / EndFogColorDistance.
        let toward_end = (FOG_FAR_CLIP / fog.end_fog_color_distance).clamp(0.0, 1.0);
        sky.fog_color = lerp_rgb(sky.fog_color, fog.end_fog_color, toward_end);
        let mut water = WaterLight {
            river_close: with_alpha(sky.river_close_color, alphas.river[0]),
            river_far: with_alpha(sky.river_far_color, alphas.river[1]),
            ocean_close: with_alpha(sky.ocean_close_color, alphas.ocean[0]),
            ocean_far: with_alpha(sky.ocean_far_color, alphas.ocean[1]),
            specular: linear_to_authored_rgb(sky.sun_color),
            underwater_fog: INERT_UNDERWATER_FOG,
            underwater_fog_color: [0.0; 3],
        };
        if let Some(underwater) = self.sample_underwater(map_id, wow_position, minutes) {
            water.underwater_fog = retail_fog(&underwater);
            water.underwater_fog_color = linear_to_authored_rgb(underwater.fog_color);
        }
        Ok(LightingSample {
            retail,
            fog,
            fog_sun_direction: sun_fog_direction(minutes),
            sky,
            water,
            stars_alpha: stars_alpha(minutes, self.blend_flag(&weights, LIGHT_PARAMS_HIDE_STARS)),
        })
    }

    /// LightParams flag `flag` as a blendable 0/1 overlaid by weight, as
    /// `calcLightParamResult` turns flags into `SkyBodyData` blends.
    fn blend_flag(&self, weights: &[(u32, f32)], flag: u32) -> f32 {
        weights.iter().fold(0.0, |blended, &(id, weight)| {
            let set = self.light_params_flags.get(&id).is_some_and(|f| f & flag != 0);
            lerp(blended, f32::from(u8::from(set)), weight)
        })
    }

    /// The underwater LightParams blend. Like the reference's day/night blend it skips
    /// LightParams without keyframes (21 authored Lights name such underwater slots); with none
    /// left the fog is inert (MapSceneRenderer.cpp:310-313 "no data -> inert fog").
    fn sample_underwater(
        &self,
        map_id: u32,
        wow_position: [f32; 3],
        minutes: f32,
    ) -> Option<SkyColorSet<[f32; 3]>> {
        let weights: Vec<_> = light_params_blend(
            &self.lights,
            &self.zone_lights,
            map_id,
            wow_position,
            LightParamsSlot::ClearUnderwater,
        )
        .iter()
        .map(|light| (light.light_params_id, light.weight))
        .collect();
        sample_light_blend(&self.keyframes, &weights, minutes, lerp_rgb)
    }

    fn blend_weights(
        &self,
        map_id: u32,
        wow_position: [f32; 3],
        slot: LightParamsSlot,
    ) -> Result<Vec<(u32, f32)>, String> {
        let blend = light_params_blend(&self.lights, &self.zone_lights, map_id, wow_position, slot);
        for light in &blend {
            if self
                .keyframes
                .get(&light.light_params_id)
                .is_none_or(Vec::is_empty)
            {
                return Err(format!(
                    "LightParams {} has no authored keyframes",
                    light.light_params_id
                ));
            }
        }
        Ok(blend
            .iter()
            .map(|light| (light.light_params_id, light.weight))
            .collect())
    }

    fn sample_sky(
        &self,
        weights: &[(u32, f32)],
        minutes: f32,
        map_id: u32,
        wow_position: [f32; 3],
    ) -> Result<SkyColorSet<[f32; 3]>, String> {
        sample_light_blend(&self.keyframes, weights, minutes, lerp_rgb)
            .ok_or_else(|| format!("No authored lighting for map {map_id} at {wow_position:?}"))
    }

    /// Overlays each LightParams' alphas by its weight, as `sample_light_blend` does colours.
    fn blend_liquid_alphas(&self, weights: &[(u32, f32)]) -> Result<LiquidAlphas, String> {
        let mut blended: Option<LiquidAlphas> = None;
        for &(id, weight) in weights {
            let alphas = *self
                .liquid_alphas
                .get(&id)
                .ok_or_else(|| format!("LightParams {id} has no DB2 row"))?;
            blended = Some(match blended {
                None => alphas,
                Some(base) => LiquidAlphas {
                    river: std::array::from_fn(|i| lerp(base.river[i], alphas.river[i], weight)),
                    ocean: std::array::from_fn(|i| lerp(base.ocean[i], alphas.ocean[i], weight)),
                },
            });
        }
        blended.ok_or_else(|| "No LightParams for liquid alphas".into())
    }
}

/// WebWowViewerCpp's default far clip (`config.h:119`), as `retail_fog` uses.
const FOG_FAR_CLIP: f32 = 1000.0;

type LightParamsRows = (HashMap<u32, LiquidAlphas>, HashMap<u32, u32>);

/// LightParams liquid alphas and flags by ID.
fn parse_light_params(path: &Path) -> Result<LightParamsRows, String> {
    let text = read_text(path)?;
    let mut lines = text.lines();
    let header: Vec<_> = lines.next().unwrap_or("").split(',').collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|column| *column == name)
            .ok_or_else(|| format!("{} has no {name} column", path.display()))
    };
    let columns = [
        column("ID")?,
        column("WaterShallowAlpha")?,
        column("WaterDeepAlpha")?,
        column("OceanShallowAlpha")?,
        column("OceanDeepAlpha")?,
        column("Flags")?,
    ];
    let mut alphas = HashMap::new();
    let mut flags = HashMap::new();
    lines
        .filter(|line| !line.is_empty())
        .map(|line| {
            let values: Vec<_> = line.split(',').collect();
            let number = |index: usize| {
                values
                    .get(index)
                    .and_then(|value| value.parse::<f32>().ok())
                    .ok_or_else(|| format!("{}: invalid row {line:?}", path.display()))
            };
            let id = number(columns[0])? as u32;
            let liquid = LiquidAlphas {
                river: [number(columns[1])?, number(columns[2])?],
                ocean: [number(columns[3])?, number(columns[4])?],
            };
            alphas.insert(id, liquid);
            flags.insert(id, number(columns[5])? as u32);
            Ok(())
        })
        .collect::<Result<(), String>>()?;
    Ok((alphas, flags))
}

/// MapSceneRenderer.cpp:312 underwater fog without data: start 0, end 1e8, density 0.
const INERT_UNDERWATER_FOG: RetailFog = RetailFog {
    start: 0.0,
    end: 100_000_000.0,
    density: 0.0,
};

fn with_alpha(linear: [f32; 3], alpha: f32) -> [f32; 4] {
    let [r, g, b] = linear_to_authored_rgb(linear);
    [r, g, b, alpha]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))
}

fn retail_colors(sky: &SkyColorSet<[f32; 3]>) -> RetailLightColors {
    RetailLightColors {
        ambient: linear_to_authored_rgb(sky.ambient_color),
        horizon_ambient: linear_to_authored_rgb(sky.horizon_ambient_color),
        ground_ambient: linear_to_authored_rgb(sky.ground_ambient_color),
        direct: linear_to_authored_rgb(sky.direct_color),
        fog_color: linear_to_authored_rgb(sky.fog_color),
        fog_start: sky.fog_start,
        fog_end: sky.fog_end,
    }
}

fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|index| lerp(a[index], b[index], t))
}
