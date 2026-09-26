//! Native GPU resources from retained split-ADT data; no world-readiness decisions.

use std::collections::{BTreeSet, HashMap};

use game_engine_core::{adt, blp};
use godot::{
    classes::{
        Image, ImageTexture, MeshInstance3D, Node3D, ResourceLoader, Shader, ShaderMaterial, image,
    },
    prelude::*,
};

use super::{assets::NativeTerrainTile, streaming::StreamedTerrain};

type Tile = (u32, u32);

#[derive(Default)]
pub(crate) struct TerrainMaterials {
    root: Option<Gd<Node3D>>,
    attached: BTreeSet<Tile>,
    textures: HashMap<u32, Gd<ImageTexture>>,
    shader: Option<Gd<Shader>>,
}

impl TerrainMaterials {
    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
    ) -> Result<(), String> {
        for (&tile, parsed) in &terrain.parsed_tiles {
            if self.attached.contains(&tile) {
                continue;
            }
            let node = self
                .build_tile(tile, parsed)
                .map_err(|error| format!("Terrain ({}, {}): {error}", tile.0, tile.1))?;
            let root = self.root.get_or_insert_with(|| {
                let mut root = Node3D::new_alloc();
                root.set_name("WorldTerrain");
                parent.add_child(&root);
                root
            });
            root.add_child(&node);
            self.attached.insert(tile);
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.attached.clear();
        self.textures.clear();
    }

    fn shader(&mut self) -> Result<Gd<Shader>, String> {
        if let Some(shader) = &self.shader {
            return Ok(shader.clone());
        }
        let resource = ResourceLoader::singleton()
            .load("res://shaders/terrain.gdshader")
            .ok_or("Cannot load native terrain shader")?;
        let shader = resource
            .try_cast::<Shader>()
            .map_err(|_| "Native terrain shader resource has wrong type")?;
        self.shader = Some(shader.clone());
        Ok(shader)
    }

    fn build_tile(&mut self, tile: Tile, parsed: &NativeTerrainTile) -> Result<Gd<Node3D>, String> {
        let tex = parsed.tex.as_ref().ok_or("Missing texture companion")?;
        if tex.chunk_layers.len() != parsed.root.chunks.len() {
            return Err(format!(
                "Root/texture chunk counts differ: {}/{}",
                parsed.root.chunks.len(),
                tex.chunk_layers.len()
            ));
        }
        let shader = self.shader()?;
        let mut chunks = Vec::new();
        // Texture chunks are stored by encounter order, not by root chunk coordinates.
        for (chunk, layers) in parsed.root.chunks.iter().zip(&tex.chunk_layers) {
            let geometry = adt::chunk_geometry(chunk, Some(tile));
            if geometry.indices.is_empty() {
                continue;
            }
            let material = self.build_material(parsed, &layers.layers, &shader)?;
            let mesh = super::build_mesh(geometry, &chunk.vertex_colors);
            chunks.push((
                format!("Chunk{}_{}", chunk.index_x, chunk.index_y),
                mesh,
                material,
            ));
        }
        // Allocate manual-lifetime nodes only after all fallible resource construction.
        let mut root = Node3D::new_alloc();
        root.set_name(&format!("Tile{}_{}", tile.0, tile.1));
        for (name, mesh, material) in chunks {
            let mut instance = MeshInstance3D::new_alloc();
            instance.set_name(&name);
            instance.set_mesh(&mesh);
            instance.set_surface_override_material(0, &material);
            root.add_child(&instance);
        }
        Ok(root)
    }

    fn build_material(
        &mut self,
        parsed: &NativeTerrainTile,
        layers: &[adt::TextureLayer],
        shader: &Gd<Shader>,
    ) -> Result<Gd<ShaderMaterial>, String> {
        if !(1..=4).contains(&layers.len()) {
            return Err(format!(
                "Expected 1–4 authored texture layers, got {}",
                layers.len()
            ));
        }
        let tex = parsed.tex.as_ref().ok_or("Missing texture companion")?;
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(shader);
        let config = Vector4::new(
            layers.len() as f32,
            adt::terrain_blend_mode(tex.map_flags) as u32 as f32,
            adt::terrain_texture_repeat(tex.texture_amplifier),
            0.0,
        );
        material.set_shader_parameter("config", &config.to_variant());
        let has_height = self.bind_layer_textures(&mut material, parsed, layers)?;
        let params = adt::texture_layer_params(&tex.texture_params, layers, has_height);
        let animation = adt::terrain_layer_animation_params(layers);
        for slot in 0..4 {
            material.set_shader_parameter(
                &format!("layer_params_{slot}"),
                &vector4(params[slot]).to_variant(),
            );
            material.set_shader_parameter(
                &format!("animation_params_{slot}"),
                &vector4(animation[slot]).to_variant(),
            );
        }
        let alpha = texture_from_rgba(64, 64, &adt::pack_alpha_map_bytes(layers))?;
        material.set_shader_parameter("alpha_packed", &alpha.to_variant());
        Ok(material)
    }

    fn bind_layer_textures(
        &mut self,
        material: &mut Gd<ShaderMaterial>,
        parsed: &NativeTerrainTile,
        layers: &[adt::TextureLayer],
    ) -> Result<[bool; 4], String> {
        let mut has_height = [false; 4];
        for (slot, layer) in layers.iter().enumerate() {
            let images = parsed
                .textures
                .get(&layer.texture_index)
                .ok_or_else(|| format!("Texture index {} was not decoded", layer.texture_index))?;
            let diffuse = self.texture(images.diffuse_fdid, &images.diffuse)?;
            material.set_shader_parameter(&format!("ground_{slot}"), &diffuse.to_variant());
            if let Some(height) = &images.height {
                let texture = self.texture(height.fdid, &height.image)?;
                material.set_shader_parameter(&format!("height_{slot}"), &texture.to_variant());
                has_height[slot] = true;
            }
        }
        Ok(has_height)
    }

    fn texture(&mut self, fdid: u32, pixels: &blp::RgbaImage) -> Result<Gd<ImageTexture>, String> {
        if let Some(texture) = self.textures.get(&fdid) {
            return Ok(texture.clone());
        }
        let texture = texture_from_rgba(pixels.width, pixels.height, &pixels.pixels)
            .map_err(|error| format!("Texture FDID {fdid}: {error}"))?;
        self.textures.insert(fdid, texture.clone());
        Ok(texture)
    }
}

fn vector4(values: [f32; 4]) -> Vector4 {
    Vector4::new(values[0], values[1], values[2], values[3])
}

fn texture_from_rgba(width: u32, height: u32, pixels: &[u8]) -> Result<Gd<ImageTexture>, String> {
    let image = Image::create_from_data(
        width as i32,
        height as i32,
        false,
        image::Format::RGBA8,
        &PackedByteArray::from(pixels),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} terrain image"))?;
    ImageTexture::create_from_image(&image).ok_or_else(|| "Godot rejected terrain texture".into())
}
