use binrw::BinRead;

pub struct WmoRootData {
    pub n_groups: u32,
    pub flags: WmoRootFlags,
    pub ambient_color: [f32; 4],
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub materials: Vec<WmoMaterialDef>,
    pub lights: Vec<WmoLight>,
    pub doodad_sets: Vec<WmoDoodadSet>,
    pub group_names: Vec<WmoGroupName>,
    pub doodad_names: Vec<WmoDoodadName>,
    pub doodad_file_ids: Vec<u32>,
    pub doodad_defs: Vec<WmoDoodadDef>,
    pub fogs: Vec<WmoFog>,
    pub visible_block_vertices: Vec<[f32; 3]>,
    pub visible_blocks: Vec<WmoVisibleBlock>,
    pub convex_volume_planes: Vec<WmoConvexVolumePlane>,
    pub group_file_data_ids: Vec<u32>,
    pub global_ambient_volumes: Vec<WmoAmbientVolume>,
    pub ambient_volumes: Vec<WmoAmbientVolume>,
    pub baked_ambient_box_volumes: Vec<WmoAmbientBoxVolume>,
    pub dynamic_lights: Vec<WmoNewLight>,
    pub portals: Vec<WmoPortal>,
    pub portal_refs: Vec<WmoPortalRef>,
    pub group_infos: Vec<WmoGroupInfo>,
    pub skybox_wow_path: Option<String>,
}

pub struct WmoPortal {
    pub vertices: Vec<[f32; 3]>,
    pub normal: [f32; 3],
}

pub struct WmoPortalRef {
    pub portal_index: u16,
    pub group_index: u16,
    pub side: i16,
}

pub struct WmoGroupInfo {
    pub flags: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

pub struct WmoGroupHeader {
    pub group_name_offset: u32,
    pub descriptive_group_name_offset: u32,
    pub flags: u32,
    pub group_flags: WmoGroupFlags,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub portal_start: u16,
    pub portal_count: u16,
    pub trans_batch_count: u16,
    pub int_batch_count: u16,
    pub ext_batch_count: u16,
    pub batch_type_d: u16,
    pub fog_ids: [u8; 4],
    pub group_liquid: u32,
    pub unique_id: u32,
    pub flags2: u32,
    pub parent_split_group_index: i16,
    pub next_split_child_group_index: i16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WmoGroupFlags {
    pub exterior: bool,
    pub interior: bool,
}

impl WmoGroupFlags {
    const EXTERIOR: u32 = 0x8;
    const INTERIOR: u32 = 0x2000;

    pub fn from_bits(bits: u32) -> Self {
        Self {
            exterior: bits & Self::EXTERIOR != 0,
            interior: bits & Self::INTERIOR != 0,
        }
    }
}

pub struct WmoMaterialDef {
    pub texture_fdid: u32,
    pub texture_2_fdid: u32,
    pub texture_3_fdid: u32,
    pub flags: u32,
    pub material_flags: WmoMaterialFlags,
    pub sidn_color: [f32; 4],
    pub diff_color: [f32; 4],
    pub ground_type: u32,
    pub blend_mode: u32,
    pub shader: u32,
    pub uv_translation_speed: Option<[[f32; 2]; 2]>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WmoMaterialFlags {
    pub unlit: bool,
    pub unfogged: bool,
    pub unculled: bool,
    pub exterior_light: bool,
    pub sidn: bool,
    pub window: bool,
    pub clamp_s: bool,
    pub clamp_t: bool,
}

impl WmoMaterialDef {
    const UNLIT_FLAG: u32 = 0x01;
    const UNFOGGED_FLAG: u32 = 0x02;
    const UNCULLED_FLAG: u32 = 0x04;
    const EXTERIOR_LIGHT_FLAG: u32 = 0x08;
    const SIDN_WINDOW_FLAG: u32 = 0x10;
    const CLAMP_S_FLAG: u32 = 0x20;
    const CLAMP_T_FLAG: u32 = 0x40;
    const SECOND_COLOR_FLAG: u32 = 0x0100_0000;
    const SECOND_UV_FLAG: u32 = 0x0200_0000;

    pub fn uses_second_color_blend_alpha(&self) -> bool {
        self.flags & Self::SECOND_COLOR_FLAG != 0
    }

    pub fn uses_second_uv_set(&self) -> bool {
        self.flags & Self::SECOND_UV_FLAG != 0 && matches!(self.shader, 6..=9 | 11..=15)
    }

    pub fn uses_generated_tangents(&self) -> bool {
        matches!(self.shader, 10 | 14)
    }

    pub fn uses_third_uv_set(&self) -> bool {
        self.flags & 0x4000_0000 != 0 && self.shader == 18
    }
}

impl WmoMaterialFlags {
    pub fn from_bits(bits: u32) -> Self {
        let sidn_window = bits & WmoMaterialDef::SIDN_WINDOW_FLAG != 0;
        Self {
            unlit: bits & WmoMaterialDef::UNLIT_FLAG != 0,
            unfogged: bits & WmoMaterialDef::UNFOGGED_FLAG != 0,
            unculled: bits & WmoMaterialDef::UNCULLED_FLAG != 0,
            exterior_light: bits & WmoMaterialDef::EXTERIOR_LIGHT_FLAG != 0,
            sidn: sidn_window,
            window: sidn_window,
            clamp_s: bits & WmoMaterialDef::CLAMP_S_FLAG != 0,
            clamp_t: bits & WmoMaterialDef::CLAMP_T_FLAG != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WmoRootFlags {
    pub do_not_attenuate_vertices: bool,
    pub use_unified_render_path: bool,
    pub use_liquid_type_dbc_id: bool,
    pub do_not_fix_vertex_color_alpha: bool,
}

impl WmoRootFlags {
    pub fn from_bits(bits: u16) -> Self {
        Self {
            do_not_attenuate_vertices: bits & 0x1 != 0,
            use_unified_render_path: bits & 0x2 != 0,
            use_liquid_type_dbc_id: bits & 0x4 != 0,
            do_not_fix_vertex_color_alpha: bits & 0x8 != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WmoLightType {
    Omni = 0,
    Spot = 1,
    Directional = 2,
    Ambient = 3,
}

impl WmoLightType {
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Spot,
            2 => Self::Directional,
            3 => Self::Ambient,
            _ => Self::Omni,
        }
    }
}

pub struct WmoLight {
    pub light_type: WmoLightType,
    pub use_attenuation: bool,
    pub color: [f32; 4],
    pub position: [f32; 3],
    pub intensity: f32,
    pub rotation: [f32; 4],
    pub attenuation_start: f32,
    pub attenuation_end: f32,
}

pub struct WmoDoodadSet {
    pub name: String,
    pub start_doodad: u32,
    pub n_doodads: u32,
}

pub struct WmoDoodadName {
    pub offset: u32,
    pub name: String,
}

pub struct WmoGroupName {
    pub offset: u32,
    pub name: String,
    pub is_antiportal: bool,
}

pub struct WmoDoodadDef {
    pub name_offset: u32,
    pub flags: u8,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: f32,
    pub color: [f32; 4],
}

pub struct WmoFog {
    pub flags: u32,
    pub position: [f32; 3],
    pub smaller_radius: f32,
    pub larger_radius: f32,
    pub fog_end: f32,
    pub fog_start_multiplier: f32,
    pub color_1: [f32; 4],
    pub underwater_fog_end: f32,
    pub underwater_fog_start_multiplier: f32,
    pub color_2: [f32; 4],
}

pub struct WmoVisibleBlock {
    pub start_vertex: u16,
    pub vertex_count: u16,
}

pub struct WmoConvexVolumePlane {
    pub normal: [f32; 3],
    pub distance: f32,
    pub flags: u32,
}

pub struct WmoMaterialUvTransform {
    pub translation_speed: [[f32; 2]; 2],
}

pub struct WmoAmbientVolume {
    pub position: [f32; 3],
    pub start: f32,
    pub end: f32,
    pub color_1: [f32; 4],
    pub color_2: [f32; 4],
    pub color_3: [f32; 4],
    pub flags: u32,
    pub doodad_set_id: u16,
}

pub struct WmoAmbientBoxVolume {
    pub planes: [[f32; 4]; 6],
    pub end: f32,
    pub color_1: [f32; 4],
    pub color_2: [f32; 4],
    pub color_3: [f32; 4],
    pub flags: u32,
    pub doodad_set_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WmoNewLightType {
    Point = 0,
    Spot = 1,
}

impl WmoNewLightType {
    pub fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Spot,
            _ => Self::Point,
        }
    }
}

pub struct WmoNewLight {
    pub light_type: WmoNewLightType,
    pub light_index: i32,
    pub flags: i32,
    pub doodad_set: i32,
    pub inner_color: [f32; 4],
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub attenuation_start: f32,
    pub attenuation_end: f32,
    pub intensity: f32,
    pub outer_color: [f32; 4],
}

pub struct RawGroupData {
    pub triangle_materials: Vec<WmoTriangleMaterial>,
    pub doodad_refs: Vec<u16>,
    pub light_refs: Vec<u16>,
    pub bsp_nodes: Vec<WmoBspNode>,
    pub bsp_face_refs: Vec<u16>,
    pub liquid: Option<WmoLiquid>,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub second_uvs: Vec<[f32; 2]>,
    pub third_uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub second_color_blend_alphas: Vec<f32>,
    pub indices: Vec<u16>,
    pub batches: Vec<RawBatch>,
}

pub struct WmoTriangleMaterial {
    pub flags: u8,
    pub material_id: u8,
}

pub struct WmoBspNode {
    pub flags: u16,
    pub neg_child: i16,
    pub pos_child: i16,
    pub face_count: u16,
    pub face_start: u32,
    pub plane_dist: f32,
}

pub struct WmoLiquid {
    pub header: WmoLiquidHeader,
    pub vertices: Vec<WmoLiquidVertex>,
    pub tiles: Vec<WmoLiquidTile>,
}

pub struct WmoLiquidHeader {
    pub x_verts: i32,
    pub y_verts: i32,
    pub x_tiles: i32,
    pub y_tiles: i32,
    pub position: [f32; 3],
    pub material_id: i16,
}

pub struct WmoLiquidVertex {
    pub raw: [u8; 4],
    pub height: f32,
}

pub struct WmoLiquidTile {
    pub liquid_type: u8,
    pub fishable: bool,
    pub shared: bool,
}

pub struct RawBatch {
    pub start_index: u32,
    pub count: u16,
    pub min_index: u16,
    pub max_index: u16,
    pub material_id: u16,
}

pub const MOHD_HEADER_SIZE: usize = 64;
pub const MOMT_ENTRY_SIZE: usize = 64;
pub const MOLT_ENTRY_SIZE: usize = 48;
pub const MODS_ENTRY_SIZE: usize = 32;
pub const MODI_ENTRY_SIZE: usize = 4;
pub const MODD_ENTRY_SIZE: usize = 40;
pub const MFOG_ENTRY_SIZE: usize = 48;
pub const MOVB_ENTRY_SIZE: usize = 4;
pub const MCVP_ENTRY_SIZE: usize = 20;
pub const MOUV_ENTRY_SIZE: usize = 16;
pub const MAVD_ENTRY_SIZE: usize = 48;
pub const MBVD_ENTRY_SIZE: usize = 128;
pub const MNLD_ENTRY_SIZE: usize = 60;
pub const MOPY_ENTRY_SIZE: usize = 2;
pub const MOBN_ENTRY_SIZE: usize = 16;
pub const MOBR_ENTRY_SIZE: usize = 2;
pub const MLIQ_HEADER_SIZE: usize = 30;
pub const MLIQ_VERTEX_SIZE: usize = 8;
pub const MLIQ_TILE_SIZE: usize = 1;
pub const MOPT_ENTRY_SIZE: usize = 20;
pub const MOPR_ENTRY_SIZE: usize = 8;
pub const MOGI_ENTRY_SIZE: usize = 32;
pub const VEC3_ENTRY_SIZE: usize = 12;
pub const VEC2_ENTRY_SIZE: usize = 8;
pub const MOBA_ENTRY_SIZE: usize = 24;

#[derive(BinRead)]
#[br(little)]
pub struct MohdHeader {
    pub _n_materials: u32,
    pub n_groups: u32,
    pub _n_portals: u32,
    pub _n_lights: u32,
    pub _n_models: u32,
    pub _n_doodads: u32,
    pub _n_sets: u32,
    pub ambient_color: [u8; 4],
    pub _wmo_id: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub flags: u16,
    pub _n_lod: u16,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoMaterialDef {
    pub flags: u32,
    pub shader: u32,
    pub blend_mode: u32,
    pub texture_fdid: u32,
    pub _sidn_emissive_color: u32,
    pub _frame_sidn_runtime_data: [u32; 2],
    pub texture_2_fdid: u32,
    pub _diff_color: u32,
    pub texture_3_fdid: u32,
    pub _color_2: u32,
    pub _terrain_type: u32,
    pub _texture_3_flags: u32,
    pub _run_time_data: [u32; 3],
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoLight {
    pub light_type: u8,
    pub use_attenuation: u8,
    pub _padding: [u8; 2],
    pub color: [u8; 4],
    pub position: [f32; 3],
    pub intensity: f32,
    pub rotation: [f32; 4],
    pub attenuation_start: f32,
    pub attenuation_end: f32,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoDoodadSet {
    pub name: [u8; 20],
    pub start_doodad: u32,
    pub n_doodads: u32,
    pub _unused: u32,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoDoodadDef {
    pub name_index_and_flags: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: f32,
    pub color: [u8; 4],
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoFog {
    pub flags: u32,
    pub position: [f32; 3],
    pub smaller_radius: f32,
    pub larger_radius: f32,
    pub fog_end: f32,
    pub fog_start_multiplier: f32,
    pub color_1: [u8; 4],
    pub underwater_fog_end: f32,
    pub underwater_fog_start_multiplier: f32,
    pub color_2: [u8; 4],
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoVisibleBlock {
    pub start_vertex: u16,
    pub vertex_count: u16,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoConvexVolumePlane {
    pub normal: [f32; 3],
    pub distance: f32,
    pub flags: u32,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoMaterialUvTransform {
    pub translation_speed: [[f32; 2]; 2],
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoAmbientVolume {
    pub position: [f32; 3],
    pub start: f32,
    pub end: f32,
    pub color_1: [u8; 4],
    pub color_2: [u8; 4],
    pub color_3: [u8; 4],
    pub flags: u32,
    pub doodad_set_id: u16,
    pub _padding: [u8; 10],
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoAmbientBoxVolume {
    pub planes: [[f32; 4]; 6],
    pub end: f32,
    pub color_1: [u8; 4],
    pub color_2: [u8; 4],
    pub color_3: [u8; 4],
    pub flags: u32,
    pub doodad_set_id: u16,
    pub _padding: [u8; 10],
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoNewLight {
    pub light_type: i32,
    pub light_index: i32,
    pub flags: i32,
    pub doodad_set: i32,
    pub inner_color: [u8; 4],
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub attenuation_start: f32,
    pub attenuation_end: f32,
    pub intensity: f32,
    pub outer_color: [u8; 4],
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoPortal {
    pub start_vertex: u16,
    pub vert_count: u16,
    pub normal: [f32; 3],
    pub _unknown: f32,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoPortalRef {
    pub portal_index: u16,
    pub group_index: u16,
    pub side: i16,
    pub _padding: u16,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoGroupInfo {
    pub flags: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub _name_offset: u32,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawBatchEntry {
    pub _possible_box_1: [u8; 10],
    pub material_id_large: u16,
    pub start_index: u32,
    pub count: u16,
    pub min_index: u16,
    pub max_index: u16,
    pub _possible_box_2: u8,
    pub material_id_small: u8,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoGroupHeader {
    pub group_name_offset: u32,
    pub descriptive_group_name_offset: u32,
    pub flags: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub portal_start: u16,
    pub portal_count: u16,
    pub trans_batch_count: u16,
    pub int_batch_count: u16,
    pub ext_batch_count: u16,
    pub batch_type_d: u16,
    pub fog_ids: [u8; 4],
    pub group_liquid: u32,
    pub unique_id: u32,
    pub flags2: u32,
    pub parent_split_group_index: i16,
    pub next_split_child_group_index: i16,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoTriangleMaterial {
    pub flags: u8,
    pub material_id: u8,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoBspNode {
    pub flags: u16,
    pub neg_child: i16,
    pub pos_child: i16,
    pub face_count: u16,
    pub face_start: u32,
    pub plane_dist: f32,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoLiquidHeader {
    pub x_verts: i32,
    pub y_verts: i32,
    pub x_tiles: i32,
    pub y_tiles: i32,
    pub position: [f32; 3],
    pub material_id: i16,
}

#[derive(BinRead)]
#[br(little)]
pub struct RawWmoLiquidVertex {
    pub raw: [u8; 4],
    pub height: f32,
}

pub const MOGP_HEADER_SIZE: usize = 68;
