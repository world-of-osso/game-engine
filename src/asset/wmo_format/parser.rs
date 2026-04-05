use std::io::Cursor;

use binrw::BinRead;

use crate::asset::adt::ChunkIter;

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

    fn from_bits(bits: u32) -> Self {
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
}

impl WmoMaterialFlags {
    fn from_bits(bits: u32) -> Self {
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
    fn from_bits(bits: u16) -> Self {
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
    fn from_raw(raw: u8) -> Self {
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
    fn from_raw(raw: i32) -> Self {
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

const MOHD_HEADER_SIZE: usize = 64;
const MOMT_ENTRY_SIZE: usize = 64;
const MOLT_ENTRY_SIZE: usize = 48;
const MODS_ENTRY_SIZE: usize = 32;
const MODI_ENTRY_SIZE: usize = 4;
const MODD_ENTRY_SIZE: usize = 40;
const MFOG_ENTRY_SIZE: usize = 48;
const MOVB_ENTRY_SIZE: usize = 4;
const MCVP_ENTRY_SIZE: usize = 20;
const MOUV_ENTRY_SIZE: usize = 16;
const MAVD_ENTRY_SIZE: usize = 48;
const MBVD_ENTRY_SIZE: usize = 128;
const MNLD_ENTRY_SIZE: usize = 60;
const MOPY_ENTRY_SIZE: usize = 2;
const MOBN_ENTRY_SIZE: usize = 16;
const MOBR_ENTRY_SIZE: usize = 2;
const MLIQ_HEADER_SIZE: usize = 30;
const MLIQ_VERTEX_SIZE: usize = 8;
const MLIQ_TILE_SIZE: usize = 1;
const MOPT_ENTRY_SIZE: usize = 20;
const MOPR_ENTRY_SIZE: usize = 8;
const MOGI_ENTRY_SIZE: usize = 32;
const VEC3_ENTRY_SIZE: usize = 12;
const VEC2_ENTRY_SIZE: usize = 8;
const MOBA_ENTRY_SIZE: usize = 24;

#[derive(BinRead)]
#[br(little)]
struct MohdHeader {
    _n_materials: u32,
    n_groups: u32,
    _n_portals: u32,
    _n_lights: u32,
    _n_models: u32,
    _n_doodads: u32,
    _n_sets: u32,
    ambient_color: [u8; 4],
    _wmo_id: u32,
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
    flags: u16,
    _n_lod: u16,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoMaterialDef {
    flags: u32,
    shader: u32,
    blend_mode: u32,
    texture_fdid: u32,
    _sidn_emissive_color: u32,
    _frame_sidn_runtime_data: [u32; 2],
    texture_2_fdid: u32,
    _diff_color: u32,
    texture_3_fdid: u32,
    _color_2: u32,
    _terrain_type: u32,
    _texture_3_flags: u32,
    _run_time_data: [u32; 3],
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoLight {
    light_type: u8,
    use_attenuation: u8,
    _padding: [u8; 2],
    color: [u8; 4],
    position: [f32; 3],
    intensity: f32,
    rotation: [f32; 4],
    attenuation_start: f32,
    attenuation_end: f32,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoDoodadSet {
    name: [u8; 20],
    start_doodad: u32,
    n_doodads: u32,
    _unused: u32,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoDoodadDef {
    name_index_and_flags: u32,
    position: [f32; 3],
    rotation: [f32; 4],
    scale: f32,
    color: [u8; 4],
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoFog {
    flags: u32,
    position: [f32; 3],
    smaller_radius: f32,
    larger_radius: f32,
    fog_end: f32,
    fog_start_multiplier: f32,
    color_1: [u8; 4],
    underwater_fog_end: f32,
    underwater_fog_start_multiplier: f32,
    color_2: [u8; 4],
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoVisibleBlock {
    start_vertex: u16,
    vertex_count: u16,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoConvexVolumePlane {
    normal: [f32; 3],
    distance: f32,
    flags: u32,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoMaterialUvTransform {
    translation_speed: [[f32; 2]; 2],
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoAmbientVolume {
    position: [f32; 3],
    start: f32,
    end: f32,
    color_1: [u8; 4],
    color_2: [u8; 4],
    color_3: [u8; 4],
    flags: u32,
    doodad_set_id: u16,
    _padding: [u8; 10],
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoAmbientBoxVolume {
    planes: [[f32; 4]; 6],
    end: f32,
    color_1: [u8; 4],
    color_2: [u8; 4],
    color_3: [u8; 4],
    flags: u32,
    doodad_set_id: u16,
    _padding: [u8; 10],
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoNewLight {
    light_type: i32,
    light_index: i32,
    flags: i32,
    doodad_set: i32,
    inner_color: [u8; 4],
    position: [f32; 3],
    rotation: [f32; 3],
    attenuation_start: f32,
    attenuation_end: f32,
    intensity: f32,
    outer_color: [u8; 4],
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoPortal {
    start_vertex: u16,
    vert_count: u16,
    normal: [f32; 3],
    _unknown: f32,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoPortalRef {
    portal_index: u16,
    group_index: u16,
    side: i16,
    _padding: u16,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoGroupInfo {
    flags: u32,
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
    _name_offset: u32,
}

#[derive(BinRead)]
#[br(little)]
struct RawBatchEntry {
    _possible_box_1: [u8; 10],
    material_id_large: u16,
    start_index: u32,
    count: u16,
    min_index: u16,
    max_index: u16,
    _possible_box_2: u8,
    material_id_small: u8,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoGroupHeader {
    group_name_offset: u32,
    descriptive_group_name_offset: u32,
    flags: u32,
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
    portal_start: u16,
    portal_count: u16,
    trans_batch_count: u16,
    int_batch_count: u16,
    ext_batch_count: u16,
    batch_type_d: u16,
    fog_ids: [u8; 4],
    group_liquid: u32,
    unique_id: u32,
    flags2: u32,
    parent_split_group_index: i16,
    next_split_child_group_index: i16,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoTriangleMaterial {
    flags: u8,
    material_id: u8,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoBspNode {
    flags: u16,
    neg_child: i16,
    pos_child: i16,
    face_count: u16,
    face_start: u32,
    plane_dist: f32,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoLiquidHeader {
    x_verts: i32,
    y_verts: i32,
    x_tiles: i32,
    y_tiles: i32,
    position: [f32; 3],
    material_id: i16,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoLiquidVertex {
    raw: [u8; 4],
    height: f32,
}

pub const MOGP_HEADER_SIZE: usize = 68;

pub fn wmo_local_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [-x, z, y]
}

type PortalsAndRanges = (Vec<WmoPortal>, Vec<(u16, u16)>);

fn parse_binrw_entries<T>(data: &[u8], entry_size: usize, label: &str) -> Result<Vec<T>, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let count = data.len() / entry_size;
    let byte_len = count
        .checked_mul(entry_size)
        .ok_or_else(|| format!("{label} byte length overflow"))?;
    let slice = data
        .get(..byte_len)
        .ok_or_else(|| format!("{label} data out of bounds"))?;
    let mut cursor = Cursor::new(slice);
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        entries.push(
            T::read_le(&mut cursor).map_err(|err| {
                format!("{label} {i} parse failed at {:#x}: {err}", i * entry_size)
            })?,
        );
    }
    Ok(entries)
}

fn parse_binrw_value<T>(data: &[u8], byte_len: usize, label: &str) -> Result<T, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let slice = data
        .get(..byte_len)
        .ok_or_else(|| format!("{label} too small: {} bytes", data.len()))?;
    T::read_le(&mut Cursor::new(slice)).map_err(|err| format!("{label} parse failed: {err}"))
}

pub fn load_wmo_root(data: &[u8]) -> Result<WmoRootData, String> {
    let mut accum = WmoRootAccum::default();
    load_wmo_root_chunks(data, &mut accum)?;
    Ok(finalize_wmo_root_data(accum))
}

fn load_wmo_root_chunks(data: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        apply_root_chunk(tag, payload, accum)?;
    }
    Ok(())
}

fn finalize_wmo_root_data(mut accum: WmoRootAccum) -> WmoRootData {
    apply_material_uv_transforms(&mut accum.materials, &accum.material_uv_transforms);
    resolve_portal_vertices(&mut accum.portals, &accum.mopt_raw, &accum.portal_vertices);
    WmoRootData {
        n_groups: accum.n_groups,
        flags: accum.flags,
        ambient_color: accum.ambient_color,
        bbox_min: accum.bbox_min,
        bbox_max: accum.bbox_max,
        materials: accum.materials,
        lights: accum.lights,
        doodad_sets: accum.doodad_sets,
        group_names: accum.group_names,
        doodad_names: accum.doodad_names,
        doodad_file_ids: accum.doodad_file_ids,
        doodad_defs: accum.doodad_defs,
        fogs: accum.fogs,
        visible_block_vertices: accum.visible_block_vertices,
        visible_blocks: accum.visible_blocks,
        convex_volume_planes: accum.convex_volume_planes,
        group_file_data_ids: accum.group_file_data_ids,
        global_ambient_volumes: accum.global_ambient_volumes,
        ambient_volumes: accum.ambient_volumes,
        baked_ambient_box_volumes: accum.baked_ambient_box_volumes,
        dynamic_lights: accum.dynamic_lights,
        portals: accum.portals,
        portal_refs: accum.portal_refs,
        group_infos: accum.group_infos,
        skybox_wow_path: accum.skybox_wow_path,
    }
}

#[derive(Default)]
struct WmoRootAccum {
    n_groups: u32,
    flags: WmoRootFlags,
    ambient_color: [f32; 4],
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
    materials: Vec<WmoMaterialDef>,
    material_uv_transforms: Vec<WmoMaterialUvTransform>,
    lights: Vec<WmoLight>,
    doodad_sets: Vec<WmoDoodadSet>,
    group_names: Vec<WmoGroupName>,
    doodad_names: Vec<WmoDoodadName>,
    doodad_file_ids: Vec<u32>,
    doodad_defs: Vec<WmoDoodadDef>,
    fogs: Vec<WmoFog>,
    visible_block_vertices: Vec<[f32; 3]>,
    visible_blocks: Vec<WmoVisibleBlock>,
    convex_volume_planes: Vec<WmoConvexVolumePlane>,
    group_file_data_ids: Vec<u32>,
    global_ambient_volumes: Vec<WmoAmbientVolume>,
    ambient_volumes: Vec<WmoAmbientVolume>,
    baked_ambient_box_volumes: Vec<WmoAmbientBoxVolume>,
    dynamic_lights: Vec<WmoNewLight>,
    portals: Vec<WmoPortal>,
    mopt_raw: Vec<(u16, u16)>,
    portal_refs: Vec<WmoPortalRef>,
    group_infos: Vec<WmoGroupInfo>,
    skybox_wow_path: Option<String>,
    portal_vertices: Vec<[f32; 3]>,
}

fn apply_root_chunk(tag: &[u8], payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    match tag {
        b"DHOM" => {
            let header: MohdHeader = parse_binrw_value(payload, MOHD_HEADER_SIZE, "MOHD")?;
            accum.n_groups = header.n_groups;
            accum.flags = WmoRootFlags::from_bits(header.flags);
            accum.ambient_color = parse_bgra_color(header.ambient_color);
            accum.bbox_min = header.bbox_min;
            accum.bbox_max = header.bbox_max;
        }
        b"TMOM" => accum.materials = parse_momt(payload)?,
        b"VUOM" => accum.material_uv_transforms = parse_mouv(payload)?,
        b"TLOM" => accum.lights = parse_molt(payload)?,
        b"SDOM" => accum.doodad_sets = parse_mods(payload)?,
        b"NGOM" => accum.group_names = parse_mogn(payload)?,
        b"NDOM" => accum.doodad_names = parse_modn(payload)?,
        b"IDOM" => accum.doodad_file_ids = parse_modi(payload)?,
        b"DDOM" => accum.doodad_defs = parse_modd(payload)?,
        b"GFOM" | b"GOFM" => accum.fogs = parse_mfog(payload)?,
        b"DIFG" => accum.group_file_data_ids = parse_gfid(payload)?,
        b"GVAM" => accum.global_ambient_volumes = parse_mavd(payload)?,
        b"DVAM" => accum.ambient_volumes = parse_mavd(payload)?,
        b"DVBM" => accum.baked_ambient_box_volumes = parse_mbvd(payload)?,
        b"DNLM" => accum.dynamic_lights = parse_mnld(payload)?,
        b"VVOM" => accum.visible_block_vertices = parse_vec3_array(payload)?,
        b"VBOM" | b"BVOM" => accum.visible_blocks = parse_movb(payload)?,
        b"PVCM" => accum.convex_volume_planes = parse_mcvp(payload)?,
        b"VPOM" => accum.portal_vertices = parse_vec3_array(payload)?,
        b"TPOM" => {
            let (p, raw) = parse_mopt(payload)?;
            accum.portals = p;
            accum.mopt_raw = raw;
        }
        b"RPOM" => accum.portal_refs = parse_mopr(payload)?,
        b"IGOM" => accum.group_infos = parse_mogi(payload)?,
        b"BSOM" => accum.skybox_wow_path = parse_c_string(payload),
        _ => {}
    }
    Ok(())
}

fn parse_c_string(data: &[u8]) -> Option<String> {
    let nul = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let bytes = &data[..nul];
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn parse_fixed_c_string(bytes: &[u8]) -> String {
    parse_c_string(bytes).unwrap_or_default()
}

pub fn parse_momt(data: &[u8]) -> Result<Vec<WmoMaterialDef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoMaterialDef>(data, MOMT_ENTRY_SIZE, "MOMT")?
            .into_iter()
            .map(|mat| WmoMaterialDef {
                texture_fdid: mat.texture_fdid,
                texture_2_fdid: mat.texture_2_fdid,
                texture_3_fdid: mat.texture_3_fdid,
                flags: mat.flags,
                material_flags: WmoMaterialFlags::from_bits(mat.flags),
                sidn_color: parse_bgra_color(mat._sidn_emissive_color.to_le_bytes()),
                blend_mode: mat.blend_mode,
                shader: mat.shader,
                uv_translation_speed: None,
            })
            .collect(),
    )
}

pub fn parse_mouv(data: &[u8]) -> Result<Vec<WmoMaterialUvTransform>, String> {
    Ok(
        parse_binrw_entries::<RawWmoMaterialUvTransform>(data, MOUV_ENTRY_SIZE, "MOUV")?
            .into_iter()
            .map(|transform| WmoMaterialUvTransform {
                translation_speed: transform.translation_speed,
            })
            .collect(),
    )
}

pub fn parse_molt(data: &[u8]) -> Result<Vec<WmoLight>, String> {
    Ok(
        parse_binrw_entries::<RawWmoLight>(data, MOLT_ENTRY_SIZE, "MOLT")?
            .into_iter()
            .map(|light| WmoLight {
                light_type: WmoLightType::from_raw(light.light_type),
                use_attenuation: light.use_attenuation != 0,
                color: parse_bgra_color(light.color),
                position: light.position,
                intensity: light.intensity,
                rotation: light.rotation,
                attenuation_start: light.attenuation_start,
                attenuation_end: light.attenuation_end,
            })
            .collect(),
    )
}

pub fn parse_mods(data: &[u8]) -> Result<Vec<WmoDoodadSet>, String> {
    Ok(
        parse_binrw_entries::<RawWmoDoodadSet>(data, MODS_ENTRY_SIZE, "MODS")?
            .into_iter()
            .map(|set| WmoDoodadSet {
                name: parse_fixed_c_string(&set.name),
                start_doodad: set.start_doodad,
                n_doodads: set.n_doodads,
            })
            .collect(),
    )
}

pub fn parse_modn(data: &[u8]) -> Result<Vec<WmoDoodadName>, String> {
    let mut names = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let remaining = &data[offset..];
        let Some(name) = parse_c_string(remaining) else {
            break;
        };
        let byte_len = name.len() + 1;
        names.push(WmoDoodadName {
            offset: offset as u32,
            name,
        });
        offset += byte_len;
    }

    Ok(names)
}

pub fn parse_mogn(data: &[u8]) -> Result<Vec<WmoGroupName>, String> {
    let mut names = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let remaining = &data[offset..];
        let Some(name) = parse_c_string(remaining) else {
            break;
        };
        let byte_len = name.len() + 1;
        let is_antiportal = name.to_ascii_lowercase().contains("antiportal");
        names.push(WmoGroupName {
            offset: offset as u32,
            name,
            is_antiportal,
        });
        offset += byte_len;
    }

    Ok(names)
}

pub fn parse_modi(data: &[u8]) -> Result<Vec<u32>, String> {
    Ok(data
        .chunks_exact(MODI_ENTRY_SIZE)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

pub fn parse_gfid(data: &[u8]) -> Result<Vec<u32>, String> {
    Ok(data
        .chunks_exact(MODI_ENTRY_SIZE)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

pub fn parse_mavd(data: &[u8]) -> Result<Vec<WmoAmbientVolume>, String> {
    Ok(
        parse_binrw_entries::<RawWmoAmbientVolume>(data, MAVD_ENTRY_SIZE, "MAVD")?
            .into_iter()
            .map(|volume| WmoAmbientVolume {
                position: volume.position,
                start: volume.start,
                end: volume.end,
                color_1: parse_bgra_color(volume.color_1),
                color_2: parse_bgra_color(volume.color_2),
                color_3: parse_bgra_color(volume.color_3),
                flags: volume.flags,
                doodad_set_id: volume.doodad_set_id,
            })
            .collect(),
    )
}

pub fn parse_mbvd(data: &[u8]) -> Result<Vec<WmoAmbientBoxVolume>, String> {
    Ok(
        parse_binrw_entries::<RawWmoAmbientBoxVolume>(data, MBVD_ENTRY_SIZE, "MBVD")?
            .into_iter()
            .map(|volume| WmoAmbientBoxVolume {
                planes: volume.planes,
                end: volume.end,
                color_1: parse_bgra_color(volume.color_1),
                color_2: parse_bgra_color(volume.color_2),
                color_3: parse_bgra_color(volume.color_3),
                flags: volume.flags,
                doodad_set_id: volume.doodad_set_id,
            })
            .collect(),
    )
}

pub fn parse_mnld(data: &[u8]) -> Result<Vec<WmoNewLight>, String> {
    Ok(
        parse_binrw_entries::<RawWmoNewLight>(data, MNLD_ENTRY_SIZE, "MNLD")?
            .into_iter()
            .map(|light| WmoNewLight {
                light_type: WmoNewLightType::from_raw(light.light_type),
                light_index: light.light_index,
                flags: light.flags,
                doodad_set: light.doodad_set,
                inner_color: parse_bgra_color(light.inner_color),
                position: light.position,
                rotation: light.rotation,
                attenuation_start: light.attenuation_start,
                attenuation_end: light.attenuation_end,
                intensity: light.intensity,
                outer_color: parse_bgra_color(light.outer_color),
            })
            .collect(),
    )
}

pub fn parse_modd(data: &[u8]) -> Result<Vec<WmoDoodadDef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoDoodadDef>(data, MODD_ENTRY_SIZE, "MODD")?
            .into_iter()
            .map(|doodad| WmoDoodadDef {
                name_offset: doodad.name_index_and_flags & 0x00FF_FFFF,
                flags: (doodad.name_index_and_flags >> 24) as u8,
                position: doodad.position,
                rotation: doodad.rotation,
                scale: doodad.scale,
                color: parse_bgra_color(doodad.color),
            })
            .collect(),
    )
}

pub fn parse_mfog(data: &[u8]) -> Result<Vec<WmoFog>, String> {
    Ok(
        parse_binrw_entries::<RawWmoFog>(data, MFOG_ENTRY_SIZE, "MFOG")?
            .into_iter()
            .map(|fog| WmoFog {
                flags: fog.flags,
                position: fog.position,
                smaller_radius: fog.smaller_radius,
                larger_radius: fog.larger_radius,
                fog_end: fog.fog_end,
                fog_start_multiplier: fog.fog_start_multiplier,
                color_1: parse_bgra_color(fog.color_1),
                underwater_fog_end: fog.underwater_fog_end,
                underwater_fog_start_multiplier: fog.underwater_fog_start_multiplier,
                color_2: parse_bgra_color(fog.color_2),
            })
            .collect(),
    )
}

pub fn parse_movb(data: &[u8]) -> Result<Vec<WmoVisibleBlock>, String> {
    Ok(
        parse_binrw_entries::<RawWmoVisibleBlock>(data, MOVB_ENTRY_SIZE, "MOVB")?
            .into_iter()
            .map(|block| WmoVisibleBlock {
                start_vertex: block.start_vertex,
                vertex_count: block.vertex_count,
            })
            .collect(),
    )
}

fn apply_material_uv_transforms(
    materials: &mut [WmoMaterialDef],
    transforms: &[WmoMaterialUvTransform],
) {
    for (material, transform) in materials.iter_mut().zip(transforms.iter()) {
        material.uv_translation_speed = Some(transform.translation_speed);
    }
}

pub fn parse_mcvp(data: &[u8]) -> Result<Vec<WmoConvexVolumePlane>, String> {
    Ok(
        parse_binrw_entries::<RawWmoConvexVolumePlane>(data, MCVP_ENTRY_SIZE, "MCVP")?
            .into_iter()
            .map(|plane| WmoConvexVolumePlane {
                normal: plane.normal,
                distance: plane.distance,
                flags: plane.flags,
            })
            .collect(),
    )
}

fn parse_mopt(data: &[u8]) -> Result<PortalsAndRanges, String> {
    let raw = parse_binrw_entries::<RawWmoPortal>(data, MOPT_ENTRY_SIZE, "MOPT")?;
    let mut portals = Vec::with_capacity(raw.len());
    let mut ranges = Vec::with_capacity(raw.len());
    for portal in raw {
        portals.push(WmoPortal {
            vertices: Vec::new(),
            normal: portal.normal,
        });
        ranges.push((portal.start_vertex, portal.vert_count));
    }
    Ok((portals, ranges))
}

fn resolve_portal_vertices(
    portals: &mut [WmoPortal],
    ranges: &[(u16, u16)],
    vertices: &[[f32; 3]],
) {
    for (portal, &(start, count)) in portals.iter_mut().zip(ranges.iter()) {
        let s = start as usize;
        let e = (s + count as usize).min(vertices.len());
        if s < vertices.len() {
            portal.vertices = vertices[s..e].to_vec();
        }
    }
}

fn parse_mopr(data: &[u8]) -> Result<Vec<WmoPortalRef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoPortalRef>(data, MOPR_ENTRY_SIZE, "MOPR")?
            .into_iter()
            .map(|portal_ref| WmoPortalRef {
                portal_index: portal_ref.portal_index,
                group_index: portal_ref.group_index,
                side: portal_ref.side,
            })
            .collect(),
    )
}

fn parse_mogi(data: &[u8]) -> Result<Vec<WmoGroupInfo>, String> {
    Ok(
        parse_binrw_entries::<RawWmoGroupInfo>(data, MOGI_ENTRY_SIZE, "MOGI")?
            .into_iter()
            .map(|group| WmoGroupInfo {
                flags: group.flags,
                bbox_min: group.bbox_min,
                bbox_max: group.bbox_max,
            })
            .collect(),
    )
}

pub fn find_mogp(data: &[u8]) -> Result<&[u8], String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"PGOM" {
            return Ok(payload);
        }
    }
    Err("No MOGP chunk found in WMO group file".to_string())
}

pub fn parse_mogp_header(data: &[u8]) -> Result<WmoGroupHeader, String> {
    let header: RawWmoGroupHeader = parse_binrw_value(data, MOGP_HEADER_SIZE, "MOGP")?;
    Ok(WmoGroupHeader {
        group_name_offset: header.group_name_offset,
        descriptive_group_name_offset: header.descriptive_group_name_offset,
        flags: header.flags,
        group_flags: WmoGroupFlags::from_bits(header.flags),
        bbox_min: header.bbox_min,
        bbox_max: header.bbox_max,
        portal_start: header.portal_start,
        portal_count: header.portal_count,
        trans_batch_count: header.trans_batch_count,
        int_batch_count: header.int_batch_count,
        ext_batch_count: header.ext_batch_count,
        batch_type_d: header.batch_type_d,
        fog_ids: header.fog_ids,
        group_liquid: header.group_liquid,
        unique_id: header.unique_id,
        flags2: header.flags2,
        parent_split_group_index: header.parent_split_group_index,
        next_split_child_group_index: header.next_split_child_group_index,
    })
}

pub fn parse_group_subchunks(data: &[u8]) -> Result<RawGroupData, String> {
    let mut triangle_materials = Vec::new();
    let mut doodad_refs = Vec::new();
    let mut light_refs = Vec::new();
    let mut bsp_nodes = Vec::new();
    let mut bsp_face_refs = Vec::new();
    let mut liquid = None;
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut second_uvs = Vec::new();
    let mut colors = Vec::new();
    let mut second_color_blend_alphas = Vec::new();
    let mut indices = Vec::new();
    let mut batches = Vec::new();

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"YPOM" => triangle_materials = parse_mopy(payload)?,
            b"RDOM" => doodad_refs = parse_u16_array(payload),
            b"RLOM" => light_refs = parse_u16_array(payload),
            b"NBOM" => bsp_nodes = parse_mobn(payload)?,
            b"RBOM" => bsp_face_refs = parse_mobr(payload)?,
            b"QILM" => liquid = Some(parse_mliq(payload)?),
            b"TVOM" => vertices = parse_vec3_array(payload)?,
            b"RNOM" => normals = parse_vec3_array(payload)?,
            b"VTOM" => {
                if uvs.is_empty() {
                    uvs = parse_vec2_array(payload)?;
                } else {
                    second_uvs = parse_vec2_array(payload)?;
                }
            }
            b"VCOM" => {
                if colors.is_empty() {
                    colors = parse_mocv(payload);
                } else {
                    second_color_blend_alphas = parse_mocv_alpha(payload);
                }
            }
            b"IVOM" => indices = parse_u16_array(payload),
            b"ABOM" => batches = parse_moba(payload)?,
            _ => {}
        }
    }

    if vertices.is_empty() {
        return Err("WMO group missing MOVT (vertices)".to_string());
    }
    if indices.is_empty() {
        return Err("WMO group missing MOVI (indices)".to_string());
    }

    Ok(RawGroupData {
        triangle_materials,
        doodad_refs,
        light_refs,
        bsp_nodes,
        bsp_face_refs,
        liquid,
        vertices,
        normals,
        uvs,
        second_uvs,
        colors,
        second_color_blend_alphas,
        indices,
        batches,
    })
}

pub fn parse_mopy(data: &[u8]) -> Result<Vec<WmoTriangleMaterial>, String> {
    Ok(
        parse_binrw_entries::<RawWmoTriangleMaterial>(data, MOPY_ENTRY_SIZE, "MOPY")?
            .into_iter()
            .map(|entry| WmoTriangleMaterial {
                flags: entry.flags,
                material_id: entry.material_id,
            })
            .collect(),
    )
}

pub fn parse_mobn(data: &[u8]) -> Result<Vec<WmoBspNode>, String> {
    Ok(
        parse_binrw_entries::<RawWmoBspNode>(data, MOBN_ENTRY_SIZE, "MOBN")?
            .into_iter()
            .map(|entry| WmoBspNode {
                flags: entry.flags,
                neg_child: entry.neg_child,
                pos_child: entry.pos_child,
                face_count: entry.face_count,
                face_start: entry.face_start,
                plane_dist: entry.plane_dist,
            })
            .collect(),
    )
}

pub fn parse_mobr(data: &[u8]) -> Result<Vec<u16>, String> {
    parse_binrw_entries(data, MOBR_ENTRY_SIZE, "MOBR")
}

pub fn parse_mliq(data: &[u8]) -> Result<WmoLiquid, String> {
    let header: RawWmoLiquidHeader = parse_binrw_value(data, MLIQ_HEADER_SIZE, "MLIQ")?;
    let vertex_count = header
        .x_verts
        .checked_mul(header.y_verts)
        .ok_or_else(|| "MLIQ vertex count overflow".to_string())? as usize;
    let tile_count = header
        .x_tiles
        .checked_mul(header.y_tiles)
        .ok_or_else(|| "MLIQ tile count overflow".to_string())? as usize;
    let vertices_offset = MLIQ_HEADER_SIZE;
    let vertices_end = vertices_offset
        .checked_add(vertex_count * MLIQ_VERTEX_SIZE)
        .ok_or_else(|| "MLIQ vertex byte length overflow".to_string())?;
    let vertices_data = data
        .get(vertices_offset..vertices_end)
        .ok_or_else(|| format!("MLIQ missing vertex payload: {} bytes", data.len()))?;
    let tiles_end = vertices_end
        .checked_add(tile_count * MLIQ_TILE_SIZE)
        .ok_or_else(|| "MLIQ tile byte length overflow".to_string())?;
    let tiles_data = data
        .get(vertices_end..tiles_end)
        .ok_or_else(|| format!("MLIQ missing tile payload: {} bytes", data.len()))?;

    Ok(WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: header.x_verts,
            y_verts: header.y_verts,
            x_tiles: header.x_tiles,
            y_tiles: header.y_tiles,
            position: header.position,
            material_id: header.material_id,
        },
        vertices: parse_binrw_entries::<RawWmoLiquidVertex>(
            vertices_data,
            MLIQ_VERTEX_SIZE,
            "MLIQ vertices",
        )?
        .into_iter()
        .map(|vertex| WmoLiquidVertex {
            raw: vertex.raw,
            height: vertex.height,
        })
        .collect(),
        tiles: tiles_data
            .iter()
            .copied()
            .map(|tile| WmoLiquidTile {
                liquid_type: tile & 0x3F,
                fishable: tile & 0x40 != 0,
                shared: tile & 0x80 != 0,
            })
            .collect(),
    })
}

fn parse_vec3_array(data: &[u8]) -> Result<Vec<[f32; 3]>, String> {
    parse_binrw_entries(data, VEC3_ENTRY_SIZE, "vec3 array")
}

fn parse_vec2_array(data: &[u8]) -> Result<Vec<[f32; 2]>, String> {
    parse_binrw_entries(data, VEC2_ENTRY_SIZE, "vec2 array")
}

fn parse_u16_array(data: &[u8]) -> Vec<u16> {
    data.chunks_exact(2)
        .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn parse_bgra_color(color: [u8; 4]) -> [f32; 4] {
    [
        color[2] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[0] as f32 / 255.0,
        color[3] as f32 / 255.0,
    ]
}

fn parse_mocv(data: &[u8]) -> Vec<[f32; 4]> {
    data.chunks_exact(4)
        .map(|c| parse_bgra_color(c.try_into().unwrap()))
        .collect()
}

fn parse_mocv_alpha(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(4).map(|c| c[3] as f32 / 255.0).collect()
}

pub fn parse_moba(data: &[u8]) -> Result<Vec<RawBatch>, String> {
    Ok(
        parse_binrw_entries::<RawBatchEntry>(data, MOBA_ENTRY_SIZE, "MOBA")?
            .into_iter()
            .map(|batch| {
                let material_id = if batch.material_id_small == 0xFF {
                    batch.material_id_large
                } else {
                    batch.material_id_small as u16
                };
                RawBatch {
                    start_index: batch.start_index,
                    count: batch.count,
                    min_index: batch.min_index,
                    max_index: batch.max_index,
                    material_id,
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_momt_entry_size() {
        let data = vec![0u8; 64];
        let mats = parse_momt(&data).unwrap();
        assert_eq!(mats.len(), 1);
        assert_eq!(mats[0].texture_fdid, 0);
        assert_eq!(mats[0].material_flags, WmoMaterialFlags::default());
        assert_eq!(mats[0].sidn_color, [0.0; 4]);
        assert_eq!(mats[0].uv_translation_speed, None);
    }

    #[test]
    fn parse_momt_reads_named_material_flags() {
        let mut data = vec![0_u8; MOMT_ENTRY_SIZE];
        let flags = 0x7F_u32;
        data[0..4].copy_from_slice(&flags.to_le_bytes());

        let mats = parse_momt(&data).expect("parse MOMT");

        assert_eq!(mats.len(), 1);
        assert_eq!(mats[0].flags, flags);
        assert_eq!(
            mats[0].material_flags,
            WmoMaterialFlags {
                unlit: true,
                unfogged: true,
                unculled: true,
                exterior_light: true,
                sidn: true,
                window: true,
                clamp_s: true,
                clamp_t: true,
            }
        );
    }

    #[test]
    fn parse_momt_reads_sidn_color() {
        let mut data = vec![0_u8; MOMT_ENTRY_SIZE];
        data[16..20].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);

        let mats = parse_momt(&data).expect("parse MOMT");

        assert_eq!(mats.len(), 1);
        assert_eq!(
            mats[0].sidn_color,
            [
                0x33 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x11 as f32 / 255.0,
                0x44 as f32 / 255.0,
            ]
        );
    }

    #[test]
    fn parse_mouv_reads_material_uv_translation_speeds() {
        let mut data = Vec::new();
        for value in [1.0_f32, 2.0, 3.0, 4.0, -1.0, -2.0, -3.0, -4.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let transforms = parse_mouv(&data).expect("parse MOUV");

        assert_eq!(transforms.len(), 2);
        assert_eq!(transforms[0].translation_speed, [[1.0, 2.0], [3.0, 4.0]]);
        assert_eq!(
            transforms[1].translation_speed,
            [[-1.0, -2.0], [-3.0, -4.0]]
        );
    }

    #[test]
    fn load_wmo_root_reads_mouv_uv_translation_speeds() {
        let mut data = Vec::new();

        data.extend_from_slice(b"VUOM");
        data.extend_from_slice(&(MOUV_ENTRY_SIZE as u32).to_le_bytes());
        for value in [0.25_f32, 0.5, 0.75, 1.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TMOM");
        data.extend_from_slice(&(MOMT_ENTRY_SIZE as u32).to_le_bytes());
        let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
        momt[4..8].copy_from_slice(&6_u32.to_le_bytes());
        momt[12..16].copy_from_slice(&123_u32.to_le_bytes());
        data.extend_from_slice(&momt);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.materials.len(), 1);
        assert_eq!(root.materials[0].shader, 6);
        assert_eq!(root.materials[0].texture_fdid, 123);
        assert_eq!(
            root.materials[0].uv_translation_speed,
            Some([[0.25, 0.5], [0.75, 1.0]])
        );
    }

    #[test]
    fn load_wmo_root_reads_momt_sidn_color() {
        let mut data = Vec::new();

        data.extend_from_slice(b"TMOM");
        data.extend_from_slice(&(MOMT_ENTRY_SIZE as u32).to_le_bytes());
        let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
        momt[16..20].copy_from_slice(&[0x10, 0x20, 0x30, 0x40]);
        data.extend_from_slice(&momt);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.materials.len(), 1);
        assert_eq!(
            root.materials[0].sidn_color,
            [
                0x30 as f32 / 255.0,
                0x20 as f32 / 255.0,
                0x10 as f32 / 255.0,
                0x40 as f32 / 255.0,
            ]
        );
    }

    #[test]
    fn load_wmo_root_reads_momt_material_flags() {
        let mut data = Vec::new();

        data.extend_from_slice(b"TMOM");
        data.extend_from_slice(&(MOMT_ENTRY_SIZE as u32).to_le_bytes());
        let mut momt = vec![0_u8; MOMT_ENTRY_SIZE];
        let flags = 0x2F_u32;
        momt[0..4].copy_from_slice(&flags.to_le_bytes());
        momt[12..16].copy_from_slice(&123_u32.to_le_bytes());
        data.extend_from_slice(&momt);

        let root = load_wmo_root(&data).expect("parse WMO root");
        let material = &root.materials[0];

        assert_eq!(material.flags, flags);
        assert_eq!(material.texture_fdid, 123);
        assert_eq!(
            material.material_flags,
            WmoMaterialFlags {
                unlit: true,
                unfogged: true,
                unculled: true,
                exterior_light: true,
                sidn: false,
                window: false,
                clamp_s: true,
                clamp_t: false,
            }
        );
    }

    #[test]
    fn parse_gfid_reads_group_file_data_ids() {
        let mut data = Vec::new();
        data.extend_from_slice(&101_u32.to_le_bytes());
        data.extend_from_slice(&202_u32.to_le_bytes());
        data.extend_from_slice(&303_u32.to_le_bytes());

        let group_file_data_ids = parse_gfid(&data).expect("parse GFID");

        assert_eq!(group_file_data_ids, vec![101, 202, 303]);
    }

    #[test]
    fn load_wmo_root_reads_gfid_group_file_data_ids() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DIFG");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        data.extend_from_slice(&1001_u32.to_le_bytes());
        data.extend_from_slice(&1002_u32.to_le_bytes());
        data.extend_from_slice(&1003_u32.to_le_bytes());

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.group_file_data_ids, vec![1001, 1002, 1003]);
    }

    #[test]
    fn parse_mavd_reads_ambient_volume_entries() {
        let mut data = Vec::new();
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&4.5_f32.to_le_bytes());
        data.extend_from_slice(&9.5_f32.to_le_bytes());
        data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
        data.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);
        data.extend_from_slice(&[0x90, 0xA0, 0xB0, 0xC0]);
        data.extend_from_slice(&7_u32.to_le_bytes());
        data.extend_from_slice(&12_u16.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);

        let volumes = parse_mavd(&data).expect("parse MAVD");

        assert_eq!(volumes.len(), 1);
        let volume = &volumes[0];
        assert_eq!(volume.position, [1.0, 2.0, 3.0]);
        assert_eq!(volume.start, 4.5);
        assert_eq!(volume.end, 9.5);
        assert_eq!(
            volume.color_1,
            [
                0x30 as f32 / 255.0,
                0x20 as f32 / 255.0,
                0x10 as f32 / 255.0,
                0x40 as f32 / 255.0,
            ]
        );
        assert_eq!(volume.flags, 7);
        assert_eq!(volume.doodad_set_id, 12);
    }

    #[test]
    fn parse_mbvd_reads_baked_ambient_box_volumes() {
        let mut data = Vec::new();
        for value in [
            1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0,
        ] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&25.0_f32.to_le_bytes());
        data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        data.extend_from_slice(&[0x55, 0x66, 0x77, 0x88]);
        data.extend_from_slice(&[0x99, 0xAA, 0xBB, 0xCC]);
        data.extend_from_slice(&9_u32.to_le_bytes());
        data.extend_from_slice(&4_u16.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);

        let volumes = parse_mbvd(&data).expect("parse MBVD");

        assert_eq!(volumes.len(), 1);
        let volume = &volumes[0];
        assert_eq!(volume.planes[0], [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(volume.planes[5], [21.0, 22.0, 23.0, 24.0]);
        assert_eq!(volume.end, 25.0);
        assert_eq!(volume.flags, 9);
        assert_eq!(volume.doodad_set_id, 4);
    }

    #[test]
    fn load_wmo_root_reads_ambient_volume_chunks() {
        let mut data = Vec::new();

        data.extend_from_slice(b"GVAM");
        data.extend_from_slice(&(MAVD_ENTRY_SIZE as u32).to_le_bytes());
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&2.0_f32.to_le_bytes());
        data.extend_from_slice(&8.0_f32.to_le_bytes());
        data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
        data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        data.extend_from_slice(&[0x12, 0x23, 0x34, 0x45]);
        data.extend_from_slice(&1_u32.to_le_bytes());
        data.extend_from_slice(&2_u16.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);

        data.extend_from_slice(b"DVAM");
        data.extend_from_slice(&(MAVD_ENTRY_SIZE as u32).to_le_bytes());
        for value in [40.0_f32, 50.0, 60.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&3.0_f32.to_le_bytes());
        data.extend_from_slice(&9.0_f32.to_le_bytes());
        data.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);
        data.extend_from_slice(&[0x51, 0x61, 0x71, 0x81]);
        data.extend_from_slice(&[0x52, 0x62, 0x72, 0x82]);
        data.extend_from_slice(&3_u32.to_le_bytes());
        data.extend_from_slice(&4_u16.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);

        data.extend_from_slice(b"DVBM");
        data.extend_from_slice(&(MBVD_ENTRY_SIZE as u32).to_le_bytes());
        for value in [
            1.0_f32, 0.0, 0.0, 5.0, -1.0, 0.0, 0.0, 6.0, 0.0, 1.0, 0.0, 7.0, 0.0, -1.0, 0.0, 8.0,
            0.0, 0.0, 1.0, 9.0, 0.0, 0.0, -1.0, 10.0,
        ] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&11.0_f32.to_le_bytes());
        data.extend_from_slice(&[0x90, 0xA0, 0xB0, 0xC0]);
        data.extend_from_slice(&[0x91, 0xA1, 0xB1, 0xC1]);
        data.extend_from_slice(&[0x92, 0xA2, 0xB2, 0xC2]);
        data.extend_from_slice(&5_u32.to_le_bytes());
        data.extend_from_slice(&6_u16.to_le_bytes());
        data.extend_from_slice(&[0_u8; 10]);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.global_ambient_volumes.len(), 1);
        assert_eq!(root.global_ambient_volumes[0].position, [10.0, 20.0, 30.0]);
        assert_eq!(root.ambient_volumes.len(), 1);
        assert_eq!(root.ambient_volumes[0].position, [40.0, 50.0, 60.0]);
        assert_eq!(root.baked_ambient_box_volumes.len(), 1);
        assert_eq!(
            root.baked_ambient_box_volumes[0].planes[0],
            [1.0, 0.0, 0.0, 5.0]
        );
        assert_eq!(root.baked_ambient_box_volumes[0].end, 11.0);
    }

    #[test]
    fn parse_mnld_reads_dynamic_lights() {
        let mut data = Vec::new();
        data.extend_from_slice(&1_i32.to_le_bytes());
        data.extend_from_slice(&22_i32.to_le_bytes());
        data.extend_from_slice(&3_i32.to_le_bytes());
        data.extend_from_slice(&4_i32.to_le_bytes());
        data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.1_f32, 0.2, 0.3] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&5.5_f32.to_le_bytes());
        data.extend_from_slice(&9.5_f32.to_le_bytes());
        data.extend_from_slice(&2.25_f32.to_le_bytes());
        data.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);

        let lights = parse_mnld(&data).expect("parse MNLD");

        assert_eq!(lights.len(), 1);
        let light = &lights[0];
        assert_eq!(light.light_type, WmoNewLightType::Spot);
        assert_eq!(light.light_index, 22);
        assert_eq!(light.flags, 3);
        assert_eq!(light.doodad_set, 4);
        assert_eq!(
            light.inner_color,
            [
                0x30 as f32 / 255.0,
                0x20 as f32 / 255.0,
                0x10 as f32 / 255.0,
                0x40 as f32 / 255.0,
            ]
        );
        assert_eq!(light.position, [1.0, 2.0, 3.0]);
        assert_eq!(light.rotation, [0.1, 0.2, 0.3]);
        assert_eq!(light.attenuation_start, 5.5);
        assert_eq!(light.attenuation_end, 9.5);
        assert_eq!(light.intensity, 2.25);
        assert_eq!(
            light.outer_color,
            [
                0x70 as f32 / 255.0,
                0x60 as f32 / 255.0,
                0x50 as f32 / 255.0,
                0x80 as f32 / 255.0,
            ]
        );
    }

    #[test]
    fn load_wmo_root_reads_mnld_dynamic_lights() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DNLM");
        data.extend_from_slice(&(MNLD_ENTRY_SIZE as u32).to_le_bytes());
        data.extend_from_slice(&0_i32.to_le_bytes());
        data.extend_from_slice(&11_i32.to_le_bytes());
        data.extend_from_slice(&5_i32.to_le_bytes());
        data.extend_from_slice(&6_i32.to_le_bytes());
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0_f32, 1.0, 0.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&3.0_f32.to_le_bytes());
        data.extend_from_slice(&7.0_f32.to_le_bytes());
        data.extend_from_slice(&1.5_f32.to_le_bytes());
        data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.dynamic_lights.len(), 1);
        let light = &root.dynamic_lights[0];
        assert_eq!(light.light_type, WmoNewLightType::Point);
        assert_eq!(light.light_index, 11);
        assert_eq!(light.flags, 5);
        assert_eq!(light.doodad_set, 6);
        assert_eq!(light.position, [10.0, 20.0, 30.0]);
        assert_eq!(light.rotation, [0.0, 1.0, 0.0]);
        assert_eq!(light.attenuation_start, 3.0);
        assert_eq!(light.attenuation_end, 7.0);
        assert_eq!(light.intensity, 1.5);
    }

    #[test]
    fn parse_mogp_header_reads_group_fields() {
        let mut data = Vec::new();
        data.extend_from_slice(&12_u32.to_le_bytes());
        data.extend_from_slice(&34_u32.to_le_bytes());
        data.extend_from_slice(&0x0102_0304_u32.to_le_bytes());
        for value in [-1.0_f32, -2.0, -3.0, 4.0, 5.0, 6.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&7_u16.to_le_bytes());
        data.extend_from_slice(&8_u16.to_le_bytes());
        data.extend_from_slice(&9_u16.to_le_bytes());
        data.extend_from_slice(&10_u16.to_le_bytes());
        data.extend_from_slice(&11_u16.to_le_bytes());
        data.extend_from_slice(&12_u16.to_le_bytes());
        data.extend_from_slice(&[1_u8, 2, 3, 4]);
        data.extend_from_slice(&13_u32.to_le_bytes());
        data.extend_from_slice(&14_u32.to_le_bytes());
        data.extend_from_slice(&15_u32.to_le_bytes());
        data.extend_from_slice(&(-16_i16).to_le_bytes());
        data.extend_from_slice(&17_i16.to_le_bytes());

        let header = parse_mogp_header(&data).expect("parse MOGP header");

        assert_eq!(header.group_name_offset, 12);
        assert_eq!(header.descriptive_group_name_offset, 34);
        assert_eq!(header.flags, 0x0102_0304);
        assert_eq!(
            header.group_flags,
            WmoGroupFlags {
                exterior: false,
                interior: false,
            }
        );
        assert_eq!(header.bbox_min, [-1.0, -2.0, -3.0]);
        assert_eq!(header.bbox_max, [4.0, 5.0, 6.0]);
        assert_eq!(header.portal_start, 7);
        assert_eq!(header.portal_count, 8);
        assert_eq!(header.trans_batch_count, 9);
        assert_eq!(header.int_batch_count, 10);
        assert_eq!(header.ext_batch_count, 11);
        assert_eq!(header.batch_type_d, 12);
        assert_eq!(header.fog_ids, [1, 2, 3, 4]);
        assert_eq!(header.group_liquid, 13);
        assert_eq!(header.unique_id, 14);
        assert_eq!(header.flags2, 15);
        assert_eq!(header.parent_split_group_index, -16);
        assert_eq!(header.next_split_child_group_index, 17);
    }

    #[test]
    fn parse_mogp_header_reads_indoor_and_outdoor_group_flags() {
        let mut interior_data = vec![0_u8; MOGP_HEADER_SIZE];
        interior_data[8..12].copy_from_slice(&0x2000_u32.to_le_bytes());
        let interior = parse_mogp_header(&interior_data).expect("parse interior MOGP");

        assert_eq!(
            interior.group_flags,
            WmoGroupFlags {
                exterior: false,
                interior: true,
            }
        );

        let mut exterior_data = vec![0_u8; MOGP_HEADER_SIZE];
        exterior_data[8..12].copy_from_slice(&0x8_u32.to_le_bytes());
        let exterior = parse_mogp_header(&exterior_data).expect("parse exterior MOGP");

        assert_eq!(
            exterior.group_flags,
            WmoGroupFlags {
                exterior: true,
                interior: false,
            }
        );
    }

    #[test]
    fn parse_mopy_reads_triangle_material_info() {
        let data = [0x20_u8, 0x05, 0x08, 0xFF];

        let materials = parse_mopy(&data).expect("parse MOPY");

        assert_eq!(materials.len(), 2);
        assert_eq!(materials[0].flags, 0x20);
        assert_eq!(materials[0].material_id, 0x05);
        assert_eq!(materials[1].flags, 0x08);
        assert_eq!(materials[1].material_id, 0xFF);
    }

    #[test]
    fn parse_mliq_reads_liquid_header_vertices_and_tiles() {
        let mut data = Vec::new();
        data.extend_from_slice(&2_i32.to_le_bytes());
        data.extend_from_slice(&2_i32.to_le_bytes());
        data.extend_from_slice(&1_i32.to_le_bytes());
        data.extend_from_slice(&1_i32.to_le_bytes());
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&7_i16.to_le_bytes());
        for (raw, height) in [
            ([1_u8, 2, 3, 4], 100.0_f32),
            ([5_u8, 6, 7, 8], 101.0),
            ([9_u8, 10, 11, 12], 102.0),
            ([13_u8, 14, 15, 16], 103.0),
        ] {
            data.extend_from_slice(&raw);
            data.extend_from_slice(&height.to_le_bytes());
        }
        data.push(0b1100_0101);

        let liquid = parse_mliq(&data).expect("parse MLIQ");

        assert_eq!(liquid.header.x_verts, 2);
        assert_eq!(liquid.header.y_verts, 2);
        assert_eq!(liquid.header.x_tiles, 1);
        assert_eq!(liquid.header.y_tiles, 1);
        assert_eq!(liquid.header.position, [10.0, 20.0, 30.0]);
        assert_eq!(liquid.header.material_id, 7);
        assert_eq!(liquid.vertices.len(), 4);
        assert_eq!(liquid.vertices[0].raw, [1, 2, 3, 4]);
        assert_eq!(liquid.vertices[3].height, 103.0);
        assert_eq!(liquid.tiles.len(), 1);
        assert_eq!(liquid.tiles[0].liquid_type, 5);
        assert!(liquid.tiles[0].fishable);
        assert!(liquid.tiles[0].shared);
    }

    #[test]
    fn parse_group_subchunks_reads_modr_doodad_refs() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RDOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [3_u16, 7, 11] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = parse_group_subchunks(&data).expect("parse group subchunks");

        assert_eq!(group.doodad_refs, vec![3, 7, 11]);
    }

    #[test]
    fn parse_group_subchunks_reads_molr_light_refs() {
        let mut data = Vec::new();
        data.extend_from_slice(b"RLOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [2_u16, 5, 8] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = parse_group_subchunks(&data).expect("parse group subchunks");

        assert_eq!(group.light_refs, vec![2, 5, 8]);
    }

    #[test]
    fn parse_mobn_reads_bsp_nodes() {
        let mut data = Vec::new();
        data.extend_from_slice(&0x0006_u16.to_le_bytes());
        data.extend_from_slice(&(-1_i16).to_le_bytes());
        data.extend_from_slice(&5_i16.to_le_bytes());
        data.extend_from_slice(&12_u16.to_le_bytes());
        data.extend_from_slice(&34_u32.to_le_bytes());
        data.extend_from_slice(&1.5_f32.to_le_bytes());

        let nodes = parse_mobn(&data).expect("parse MOBN");

        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];
        assert_eq!(node.flags, 0x0006);
        assert_eq!(node.neg_child, -1);
        assert_eq!(node.pos_child, 5);
        assert_eq!(node.face_count, 12);
        assert_eq!(node.face_start, 34);
        assert_eq!(node.plane_dist, 1.5);
    }

    #[test]
    fn parse_group_subchunks_reads_mobn_and_mobr_bsp_data() {
        let mut data = Vec::new();
        data.extend_from_slice(b"NBOM");
        data.extend_from_slice(&(MOBN_ENTRY_SIZE as u32).to_le_bytes());
        data.extend_from_slice(&0x0004_u16.to_le_bytes());
        data.extend_from_slice(&(-1_i16).to_le_bytes());
        data.extend_from_slice(&(-1_i16).to_le_bytes());
        data.extend_from_slice(&3_u16.to_le_bytes());
        data.extend_from_slice(&7_u32.to_le_bytes());
        data.extend_from_slice(&12.5_f32.to_le_bytes());

        data.extend_from_slice(b"RBOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [4_u16, 8, 9] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = parse_group_subchunks(&data).expect("parse group subchunks");

        assert_eq!(group.bsp_nodes.len(), 1);
        assert_eq!(group.bsp_nodes[0].flags, 0x0004);
        assert_eq!(group.bsp_nodes[0].face_count, 3);
        assert_eq!(group.bsp_nodes[0].face_start, 7);
        assert_eq!(group.bsp_nodes[0].plane_dist, 12.5);
        assert_eq!(group.bsp_face_refs, vec![4, 8, 9]);
    }

    #[test]
    fn parse_group_subchunks_preserves_second_motv_uv_set() {
        let mut data = Vec::new();
        data.extend_from_slice(b"VTOM");
        data.extend_from_slice(&(8_u32).to_le_bytes());
        for value in [1.0_f32, 2.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"VTOM");
        data.extend_from_slice(&(8_u32).to_le_bytes());
        for value in [3.0_f32, 4.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = parse_group_subchunks(&data).expect("parse group subchunks");

        assert_eq!(group.uvs, vec![[1.0, 2.0]]);
        assert_eq!(group.second_uvs, vec![[3.0, 4.0]]);
    }

    #[test]
    fn parse_group_subchunks_preserves_second_mocv_alpha_values() {
        let mut data = Vec::new();
        data.extend_from_slice(b"VCOM");
        data.extend_from_slice(&(4_u32).to_le_bytes());
        data.extend_from_slice(&[1_u8, 2, 3, 4]);

        data.extend_from_slice(b"VCOM");
        data.extend_from_slice(&(8_u32).to_le_bytes());
        data.extend_from_slice(&[5_u8, 6, 7, 64]);
        data.extend_from_slice(&[8_u8, 9, 10, 192]);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(24_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = parse_group_subchunks(&data).expect("parse group subchunks");

        assert_eq!(group.colors.len(), 1);
        assert_eq!(
            group.second_color_blend_alphas,
            vec![64.0 / 255.0, 192.0 / 255.0]
        );
    }

    #[test]
    fn parse_root_chunk_reads_mosb_skybox_name() {
        let mut accum = WmoRootAccum::default();

        apply_root_chunk(b"BSOM", b"environments/stars/deathskybox.m2\0", &mut accum)
            .expect("parse MOSB chunk");

        assert_eq!(
            accum.skybox_wow_path.as_deref(),
            Some("environments/stars/deathskybox.m2")
        );
    }

    #[test]
    fn parse_root_chunk_reads_mohd_flags() {
        let mut accum = WmoRootAccum::default();
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        mohd[4..8].copy_from_slice(&7_u32.to_le_bytes());
        mohd[60..62].copy_from_slice(&0x000F_u16.to_le_bytes());

        apply_root_chunk(b"DHOM", &mohd, &mut accum).expect("parse MOHD chunk");

        assert_eq!(accum.n_groups, 7);
        assert_eq!(
            accum.flags,
            WmoRootFlags {
                do_not_attenuate_vertices: true,
                use_unified_render_path: true,
                use_liquid_type_dbc_id: true,
                do_not_fix_vertex_color_alpha: true,
            }
        );
    }

    #[test]
    fn parse_root_chunk_reads_mohd_ambient_color() {
        let mut accum = WmoRootAccum::default();
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        mohd[28..32].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]);

        apply_root_chunk(b"DHOM", &mohd, &mut accum).expect("parse MOHD chunk");

        assert_eq!(
            accum.ambient_color,
            [
                0x33 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x11 as f32 / 255.0,
                0x44 as f32 / 255.0,
            ]
        );
    }

    #[test]
    fn parse_root_chunk_reads_mohd_bounding_box() {
        let mut accum = WmoRootAccum::default();
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        for (offset, value) in [
            (36usize, -1.0_f32),
            (40, -2.0),
            (44, -3.0),
            (48, 4.0),
            (52, 5.0),
            (56, 6.0),
        ] {
            mohd[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }

        apply_root_chunk(b"DHOM", &mohd, &mut accum).expect("parse MOHD chunk");

        assert_eq!(accum.bbox_min, [-1.0, -2.0, -3.0]);
        assert_eq!(accum.bbox_max, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn parse_molt_reads_light_fields() {
        let mut data = Vec::new();
        data.push(1);
        data.push(1);
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&4.5_f32.to_le_bytes());
        for value in [0.1_f32, 0.2, 0.3, 0.4] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&5.5_f32.to_le_bytes());
        data.extend_from_slice(&9.5_f32.to_le_bytes());

        let lights = parse_molt(&data).expect("parse MOLT");

        assert_eq!(lights.len(), 1);
        let light = &lights[0];
        assert_eq!(light.light_type, WmoLightType::Spot);
        assert!(light.use_attenuation);
        assert_eq!(
            light.color,
            [
                0x30 as f32 / 255.0,
                0x20 as f32 / 255.0,
                0x10 as f32 / 255.0,
                0x40 as f32 / 255.0,
            ]
        );
        assert_eq!(light.position, [1.0, 2.0, 3.0]);
        assert_eq!(light.intensity, 4.5);
        assert_eq!(light.rotation, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(light.attenuation_start, 5.5);
        assert_eq!(light.attenuation_end, 9.5);
    }

    #[test]
    fn load_wmo_root_reads_molt_lights() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DHOM");
        data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        mohd[4..8].copy_from_slice(&1_u32.to_le_bytes());
        mohd[12..16].copy_from_slice(&1_u32.to_le_bytes());
        data.extend_from_slice(&mohd);

        data.extend_from_slice(b"TLOM");
        data.extend_from_slice(&(MOLT_ENTRY_SIZE as u32).to_le_bytes());
        data.push(2);
        data.push(0);
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&2.25_f32.to_le_bytes());
        for value in [0.0_f32, 0.0, 1.0, 0.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&3.0_f32.to_le_bytes());
        data.extend_from_slice(&7.0_f32.to_le_bytes());

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.n_groups, 1);
        assert_eq!(root.lights.len(), 1);
        let light = &root.lights[0];
        assert_eq!(light.light_type, WmoLightType::Directional);
        assert!(!light.use_attenuation);
        assert_eq!(light.position, [10.0, 20.0, 30.0]);
        assert_eq!(light.intensity, 2.25);
        assert_eq!(light.rotation, [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(light.attenuation_start, 3.0);
        assert_eq!(light.attenuation_end, 7.0);
    }

    #[test]
    fn load_wmo_root_reads_mohd_flags() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DHOM");
        data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        mohd[4..8].copy_from_slice(&2_u32.to_le_bytes());
        mohd[60..62].copy_from_slice(&0x000A_u16.to_le_bytes());
        data.extend_from_slice(&mohd);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.n_groups, 2);
        assert_eq!(
            root.flags,
            WmoRootFlags {
                do_not_attenuate_vertices: false,
                use_unified_render_path: true,
                use_liquid_type_dbc_id: false,
                do_not_fix_vertex_color_alpha: true,
            }
        );
    }

    #[test]
    fn load_wmo_root_reads_mohd_ambient_color() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DHOM");
        data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        mohd[28..32].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        data.extend_from_slice(&mohd);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(
            root.ambient_color,
            [
                0xCC as f32 / 255.0,
                0xBB as f32 / 255.0,
                0xAA as f32 / 255.0,
                0xDD as f32 / 255.0,
            ]
        );
    }

    #[test]
    fn load_wmo_root_reads_mohd_bounding_box() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DHOM");
        data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        for (offset, value) in [
            (36usize, -10.0_f32),
            (40, -20.0),
            (44, -30.0),
            (48, 40.0),
            (52, 50.0),
            (56, 60.0),
        ] {
            mohd[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&mohd);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.bbox_min, [-10.0, -20.0, -30.0]);
        assert_eq!(root.bbox_max, [40.0, 50.0, 60.0]);
    }

    #[test]
    fn parse_mfog_reads_fog_entries() {
        let mut data = Vec::new();
        data.extend_from_slice(&7_u32.to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&4.5_f32.to_le_bytes());
        data.extend_from_slice(&9.5_f32.to_le_bytes());
        data.extend_from_slice(&12.0_f32.to_le_bytes());
        data.extend_from_slice(&0.25_f32.to_le_bytes());
        data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
        data.extend_from_slice(&18.0_f32.to_le_bytes());
        data.extend_from_slice(&0.5_f32.to_le_bytes());
        data.extend_from_slice(&[0x50, 0x60, 0x70, 0x80]);

        let fogs = parse_mfog(&data).expect("parse MFOG");

        assert_eq!(fogs.len(), 1);
        let fog = &fogs[0];
        assert_eq!(fog.flags, 7);
        assert_eq!(fog.position, [1.0, 2.0, 3.0]);
        assert_eq!(fog.smaller_radius, 4.5);
        assert_eq!(fog.larger_radius, 9.5);
        assert_eq!(fog.fog_end, 12.0);
        assert_eq!(fog.fog_start_multiplier, 0.25);
        assert_eq!(
            fog.color_1,
            [
                0x30 as f32 / 255.0,
                0x20 as f32 / 255.0,
                0x10 as f32 / 255.0,
                0x40 as f32 / 255.0,
            ]
        );
        assert_eq!(fog.underwater_fog_end, 18.0);
        assert_eq!(fog.underwater_fog_start_multiplier, 0.5);
        assert_eq!(
            fog.color_2,
            [
                0x70 as f32 / 255.0,
                0x60 as f32 / 255.0,
                0x50 as f32 / 255.0,
                0x80 as f32 / 255.0,
            ]
        );
    }

    #[test]
    fn load_wmo_root_reads_mfog_entries() {
        let mut data = Vec::new();

        data.extend_from_slice(b"GFOM");
        data.extend_from_slice(&(MFOG_ENTRY_SIZE as u32).to_le_bytes());
        data.extend_from_slice(&3_u32.to_le_bytes());
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&6.0_f32.to_le_bytes());
        data.extend_from_slice(&14.0_f32.to_le_bytes());
        data.extend_from_slice(&22.0_f32.to_le_bytes());
        data.extend_from_slice(&0.4_f32.to_le_bytes());
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        data.extend_from_slice(&33.0_f32.to_le_bytes());
        data.extend_from_slice(&0.6_f32.to_le_bytes());
        data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.fogs.len(), 1);
        let fog = &root.fogs[0];
        assert_eq!(fog.flags, 3);
        assert_eq!(fog.position, [10.0, 20.0, 30.0]);
        assert_eq!(fog.smaller_radius, 6.0);
        assert_eq!(fog.larger_radius, 14.0);
        assert_eq!(fog.fog_end, 22.0);
        assert_eq!(fog.underwater_fog_end, 33.0);
    }

    #[test]
    fn parse_mogn_preserves_offsets_and_antiportal_names() {
        let data = b"EntryHall\0antiportal01\0";

        let names = parse_mogn(data).expect("parse MOGN");

        assert_eq!(names.len(), 2);
        assert_eq!(names[0].offset, 0);
        assert_eq!(names[0].name, "EntryHall");
        assert!(!names[0].is_antiportal);
        assert_eq!(names[1].offset, 10);
        assert_eq!(names[1].name, "antiportal01");
        assert!(names[1].is_antiportal);
    }

    #[test]
    fn load_wmo_root_reads_mogn_group_names() {
        let mut data = Vec::new();

        data.extend_from_slice(b"NGOM");
        data.extend_from_slice(&(23_u32).to_le_bytes());
        data.extend_from_slice(b"EntryHall\0antiportal01\0");

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.group_names.len(), 2);
        assert_eq!(root.group_names[0].name, "EntryHall");
        assert!(!root.group_names[0].is_antiportal);
        assert_eq!(root.group_names[1].offset, 10);
        assert_eq!(root.group_names[1].name, "antiportal01");
        assert!(root.group_names[1].is_antiportal);
    }

    #[test]
    fn parse_movb_reads_visible_blocks() {
        let mut data = Vec::new();
        data.extend_from_slice(&3_u16.to_le_bytes());
        data.extend_from_slice(&6_u16.to_le_bytes());
        data.extend_from_slice(&9_u16.to_le_bytes());
        data.extend_from_slice(&12_u16.to_le_bytes());

        let blocks = parse_movb(&data).expect("parse MOVB");

        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].start_vertex, 3);
        assert_eq!(blocks[0].vertex_count, 6);
        assert_eq!(blocks[1].start_vertex, 9);
        assert_eq!(blocks[1].vertex_count, 12);
    }

    #[test]
    fn load_wmo_root_reads_visible_volumes() {
        let mut data = Vec::new();

        data.extend_from_slice(b"VVOM");
        data.extend_from_slice(&(24_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"VBOM");
        data.extend_from_slice(&(8_u32).to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&2_u16.to_le_bytes());
        data.extend_from_slice(&2_u16.to_le_bytes());
        data.extend_from_slice(&2_u16.to_le_bytes());

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(
            root.visible_block_vertices,
            vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
        );
        assert_eq!(root.visible_blocks.len(), 2);
        assert_eq!(root.visible_blocks[0].start_vertex, 0);
        assert_eq!(root.visible_blocks[0].vertex_count, 2);
        assert_eq!(root.visible_blocks[1].start_vertex, 2);
        assert_eq!(root.visible_blocks[1].vertex_count, 2);
    }

    #[test]
    fn parse_mcvp_reads_convex_volume_planes() {
        let mut data = Vec::new();
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&4.5_f32.to_le_bytes());
        data.extend_from_slice(&7_u32.to_le_bytes());
        for value in [-1.0_f32, -2.0, -3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&8.5_f32.to_le_bytes());
        data.extend_from_slice(&9_u32.to_le_bytes());

        let planes = parse_mcvp(&data).expect("parse MCVP");

        assert_eq!(planes.len(), 2);
        assert_eq!(planes[0].normal, [1.0, 2.0, 3.0]);
        assert_eq!(planes[0].distance, 4.5);
        assert_eq!(planes[0].flags, 7);
        assert_eq!(planes[1].normal, [-1.0, -2.0, -3.0]);
        assert_eq!(planes[1].distance, 8.5);
        assert_eq!(planes[1].flags, 9);
    }

    #[test]
    fn load_wmo_root_reads_mcvp_convex_volume_planes() {
        let mut data = Vec::new();

        data.extend_from_slice(b"PVCM");
        data.extend_from_slice(&(MCVP_ENTRY_SIZE as u32).to_le_bytes());
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&40.0_f32.to_le_bytes());
        data.extend_from_slice(&5_u32.to_le_bytes());

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.convex_volume_planes.len(), 1);
        assert_eq!(root.convex_volume_planes[0].normal, [10.0, 20.0, 30.0]);
        assert_eq!(root.convex_volume_planes[0].distance, 40.0);
        assert_eq!(root.convex_volume_planes[0].flags, 5);
    }

    #[test]
    fn parse_mods_reads_doodad_sets() {
        let mut data = Vec::new();
        let mut name = [0_u8; 20];
        name[..14].copy_from_slice(b"$DefaultGlobal");
        data.extend_from_slice(&name);
        data.extend_from_slice(&4_u32.to_le_bytes());
        data.extend_from_slice(&9_u32.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());

        let sets = parse_mods(&data).expect("parse MODS");

        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].name, "$DefaultGlobal");
        assert_eq!(sets[0].start_doodad, 4);
        assert_eq!(sets[0].n_doodads, 9);
    }

    #[test]
    fn load_wmo_root_reads_mods_doodad_sets() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DHOM");
        data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        mohd[24..28].copy_from_slice(&2_u32.to_le_bytes());
        data.extend_from_slice(&mohd);

        data.extend_from_slice(b"SDOM");
        data.extend_from_slice(&(64_u32).to_le_bytes());

        let mut first_name = [0_u8; 20];
        first_name[..14].copy_from_slice(b"$DefaultGlobal");
        data.extend_from_slice(&first_name);
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&3_u32.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());

        let mut second_name = [0_u8; 20];
        second_name[..7].copy_from_slice(b"FirePit");
        data.extend_from_slice(&second_name);
        data.extend_from_slice(&3_u32.to_le_bytes());
        data.extend_from_slice(&5_u32.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.doodad_sets.len(), 2);
        assert_eq!(root.doodad_sets[0].name, "$DefaultGlobal");
        assert_eq!(root.doodad_sets[0].start_doodad, 0);
        assert_eq!(root.doodad_sets[0].n_doodads, 3);
        assert_eq!(root.doodad_sets[1].name, "FirePit");
        assert_eq!(root.doodad_sets[1].start_doodad, 3);
        assert_eq!(root.doodad_sets[1].n_doodads, 5);
    }

    #[test]
    fn parse_modn_preserves_chunk_relative_name_offsets() {
        let data = b"torch01.m2\0barrel02.m2\0";

        let names = parse_modn(data).expect("parse MODN");

        assert_eq!(names.len(), 2);
        assert_eq!(names[0].offset, 0);
        assert_eq!(names[0].name, "torch01.m2");
        assert_eq!(names[1].offset, 11);
        assert_eq!(names[1].name, "barrel02.m2");
    }

    #[test]
    fn parse_modi_reads_doodad_file_ids() {
        let mut data = Vec::new();
        data.extend_from_slice(&1001_u32.to_le_bytes());
        data.extend_from_slice(&2002_u32.to_le_bytes());

        let ids = parse_modi(&data).expect("parse MODI");

        assert_eq!(ids, vec![1001, 2002]);
    }

    #[test]
    fn load_wmo_root_reads_modn_and_modi_doodad_sources() {
        let mut data = Vec::new();

        data.extend_from_slice(b"NDOM");
        data.extend_from_slice(&(23_u32).to_le_bytes());
        data.extend_from_slice(b"torch01.m2\0barrel02.m2\0");

        data.extend_from_slice(b"IDOM");
        data.extend_from_slice(&(8_u32).to_le_bytes());
        data.extend_from_slice(&1001_u32.to_le_bytes());
        data.extend_from_slice(&2002_u32.to_le_bytes());

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.doodad_names.len(), 2);
        assert_eq!(root.doodad_names[0].offset, 0);
        assert_eq!(root.doodad_names[0].name, "torch01.m2");
        assert_eq!(root.doodad_names[1].offset, 11);
        assert_eq!(root.doodad_names[1].name, "barrel02.m2");
        assert_eq!(root.doodad_file_ids, vec![1001, 2002]);
    }

    #[test]
    fn parse_modd_reads_doodad_definitions() {
        let mut data = Vec::new();
        data.extend_from_slice(&0x1200002A_u32.to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.1_f32, 0.2, 0.3, 0.4] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&1.5_f32.to_le_bytes());
        data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);

        let doodads = parse_modd(&data).expect("parse MODD");

        assert_eq!(doodads.len(), 1);
        let doodad = &doodads[0];
        assert_eq!(doodad.name_offset, 0x2A);
        assert_eq!(doodad.flags, 0x12);
        assert_eq!(doodad.position, [1.0, 2.0, 3.0]);
        assert_eq!(doodad.rotation, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(doodad.scale, 1.5);
        assert_eq!(
            doodad.color,
            [
                0x33 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x11 as f32 / 255.0,
                0x44 as f32 / 255.0,
            ]
        );
    }

    #[test]
    fn load_wmo_root_reads_modd_doodad_definitions() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DDOM");
        data.extend_from_slice(&(40_u32).to_le_bytes());
        data.extend_from_slice(&0x0100000B_u32.to_le_bytes());
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        for value in [0.0_f32, 0.0, 1.0, 0.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&0.75_f32.to_le_bytes());
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.doodad_defs.len(), 1);
        let doodad = &root.doodad_defs[0];
        assert_eq!(doodad.name_offset, 11);
        assert_eq!(doodad.flags, 1);
        assert_eq!(doodad.position, [10.0, 20.0, 30.0]);
        assert_eq!(doodad.rotation, [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(doodad.scale, 0.75);
    }

    #[test]
    fn parse_moba_entry_size() {
        let data = vec![0u8; 24];
        let batches = parse_moba(&data).unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].material_id, 0);
    }

    #[test]
    fn parse_mocv_bgra_to_rgba() {
        let colors = parse_mocv(&[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(colors.len(), 1);
        assert_eq!(
            colors[0],
            [
                0x33 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x11 as f32 / 255.0,
                0x44 as f32 / 255.0,
            ]
        );
    }
}
