//! The map's WDL horizon (core `horizon`) around the camera: every WDL tile within the
//! client's horizon range that the terrain stream has not loaded draws in the scene fog's
//! colour, so distant mountains stand against the sky instead of ending at the loaded tiles.

use std::{collections::HashMap, path::PathBuf};

use game_engine_core::horizon::{HorizonTile, parse_wdl};
use godot::{
    classes::{
        ArrayMesh, MeshInstance3D, Node3D, ResourceLoader, Shader, ShaderMaterial,
        geometry_instance_3d::ShadowCastingSetting, mesh,
    },
    prelude::*,
};

use crate::{
    frame_error,
    lighting::TerrainLight,
    terrain::{assets::NativeTerrainAssets, streaming::StreamedTerrain},
};

const SHADER_PATH: &str = "res://shaders/horizon.gdshader";
/// `farclip` 777 × `horizonFarclipScale` 4 (solarityclient cvar definitions, client
/// 791170's horizon far plane).
const HORIZON_RANGE: f32 = 777.0 * 4.0;
/// Half a tile's diagonal.
const TILE_RADIUS: f32 = 533.333_3 * std::f32::consts::FRAC_1_SQRT_2;

type Tile = (u32, u32);

pub(crate) struct WorldHorizon {
    assets: NativeTerrainAssets,
    /// The map the tiles were read for.
    map: Option<String>,
    tiles: Vec<HorizonTile>,
    nodes: HashMap<Tile, Gd<MeshInstance3D>>,
    root: Option<Gd<Node3D>>,
    material: Option<Gd<ShaderMaterial>>,
    light: Option<TerrainLight>,
}

impl WorldHorizon {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            assets: NativeTerrainAssets::new(data_root),
            map: None,
            tiles: Vec::new(),
            nodes: HashMap::new(),
            root: None,
            material: None,
            light: None,
        }
    }

    /// Show the WDL tiles within range of `camera` that are not loaded as terrain.
    pub fn update(&mut self, parent: &mut Gd<Node3D>, terrain: &StreamedTerrain, camera: Vector3) {
        let Some(map) = terrain.map_name() else {
            return;
        };
        if self.map.as_deref() != Some(map) {
            self.reset();
            self.map = Some(map.to_owned());
            match self
                .assets
                .read_map_wdl(map)
                .and_then(|bytes| bytes.map_or(Ok(Vec::new()), |bytes| parse_wdl(&bytes)))
            {
                Ok(tiles) => self.tiles = tiles,
                Err(error) => frame_error::report_once(&format!("Horizon of {map}: {error}")),
            }
        }
        if let Err(error) = self.prepare(parent) {
            frame_error::report_once(&format!("Horizon: {error}"));
            return;
        }
        let eye = Vector2::new(camera.x, camera.z);
        for index in 0..self.tiles.len() {
            let tile = &self.tiles[index];
            let shown = !terrain.parsed_tiles.contains_key(&tile.tile)
                && tile_centre(tile.tile).distance_to(eye) <= HORIZON_RANGE + TILE_RADIUS;
            match (shown, self.nodes.get_mut(&tile.tile)) {
                (true, None) => {
                    let node = self.spawn(index);
                    self.nodes.insert(self.tiles[index].tile, node);
                }
                (shown, Some(node)) => node.set_visible(shown),
                (false, None) => {}
            }
        }
    }

    pub fn update_lighting(&mut self, light: &TerrainLight) {
        if let Some(material) = self.material.as_mut() {
            light.bind_scene_fog(material);
        }
        self.light = Some(light.clone());
    }

    /// WDL tiles drawn this frame.
    pub fn shown_count(&self) -> usize {
        self.nodes.values().filter(|node| node.is_visible()).count()
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.nodes.clear();
        self.tiles.clear();
        self.map = None;
    }

    fn prepare(&mut self, parent: &mut Gd<Node3D>) -> Result<(), String> {
        if self.material.is_none() {
            let shader = ResourceLoader::singleton()
                .load(SHADER_PATH)
                .ok_or_else(|| format!("Cannot load {SHADER_PATH}"))?
                .try_cast::<Shader>()
                .map_err(|_| format!("{SHADER_PATH} is not a Shader"))?;
            let mut material = ShaderMaterial::new_gd();
            material.set_shader(&shader);
            if let Some(light) = &self.light {
                light.bind_scene_fog(&mut material);
            }
            self.material = Some(material);
        }
        if self.root.is_none() {
            let mut root = Node3D::new_alloc();
            root.set_name("Horizon");
            parent.add_child(&root);
            self.root = Some(root);
        }
        Ok(())
    }

    fn spawn(&mut self, index: usize) -> Gd<MeshInstance3D> {
        let tile = &self.tiles[index];
        let positions: Vec<_> = tile
            .positions
            .iter()
            .copied()
            .map(Vector3::from_array)
            .collect();
        let indices: Vec<i32> = tile.indices.iter().map(|&index| i32::from(index)).collect();
        let mut arrays = VarArray::new();
        arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
        arrays.set(
            mesh::ArrayType::VERTEX.ord() as usize,
            &PackedVector3Array::from(positions.as_slice()).to_variant(),
        );
        arrays.set(
            mesh::ArrayType::INDEX.ord() as usize,
            &PackedInt32Array::from(indices.as_slice()).to_variant(),
        );
        let mut array_mesh = ArrayMesh::new_gd();
        array_mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
        let mut instance = MeshInstance3D::new_alloc();
        instance.set_name(&format!("Horizon{}_{}", tile.tile.0, tile.tile.1));
        instance.set_mesh(&array_mesh);
        instance.set_surface_override_material(0, self.material.as_ref().expect("prepared"));
        instance.set_cast_shadows_setting(ShadowCastingSetting::OFF);
        // The camera's far plane (1000 yd) is nearer than the horizon: keep Godot's frustum
        // culling from dropping tiles the shader draws at the far depth anyway.
        instance.set_extra_cull_margin(HORIZON_RANGE + TILE_RADIUS);
        self.root.as_mut().expect("prepared").add_child(&instance);
        instance
    }
}

/// Engine-space horizontal centre of tile `(tile_y, tile_x)`.
fn tile_centre((tile_y, tile_x): Tile) -> Vector2 {
    let size = 533.333_3;
    Vector2::new(
        32.0 * size - (tile_x as f32 + 0.5) * size,
        (tile_y as f32 + 0.5) * size - 32.0 * size,
    )
}
