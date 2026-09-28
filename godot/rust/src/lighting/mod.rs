//! Native scene-light resources sampled from authored map-position/time data.

pub(crate) mod assets;

use game_engine_core::{
    retail_light_data::RetailLightData, sky_cubemap_data, sky_lightdata_data::RetailFog,
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
    previous: Option<(RetailLightData, RetailFog, SkyStops)>,
}

impl WorldLighting {
    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        catalog: &LightingCatalog,
        map_id: u32,
        position: Vector3,
        minutes: f32,
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
        let values = (sample.retail.clone(), sample.fog, stops);
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
            fog: sample.fog,
            fog_color: sky.fog_color,
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
