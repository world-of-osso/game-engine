use core::mem::{self, size_of};

use bevy_asset::{AssetId, Assets, prelude::AssetChanged};
use bevy_camera::visibility::ViewVisibility;
use bevy_ecs::prelude::*;
use bevy_math::Mat4;
use bevy_mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_platform::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc,
};
use bevy_render::render_resource::{Buffer, BufferDescriptor};
use bevy_render::settings::WgpuLimits;
use bevy_render::sync_world::{MainEntity, MainEntityHashMap};
use bevy_render::{
    Extract,
    batching::NoAutomaticBatching,
    render_resource::BufferUsages,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_transform::prelude::GlobalTransform;
use offset_allocator::{Allocation, Allocator};
use tracing::error;

#[cfg(test)]
mod tests;

/// Maximum number of joints supported for skinned meshes.
///
/// It is used to allocate buffers.
/// The correctness of the value depends on the GPU/platform.
/// The current value is chosen because it is guaranteed to work everywhere.
/// To allow for bigger values, a check must be made for the limits
/// of the GPU at runtime, which would mean not using consts anymore.
pub const MAX_JOINTS: usize = 256;

/// The total number of joints we support.
///
/// This is 256 GiB worth of joint matrices, which we will never hit under any
/// reasonable circumstances.
const MAX_TOTAL_JOINTS: u32 = 1024 * 1024 * 1024;

/// The number of joints that we allocate at a time.
///
/// Some hardware requires that uniforms be allocated on 256-byte boundaries, so
/// we need to allocate 4 64-byte matrices at a time to satisfy alignment
/// requirements.
const JOINTS_PER_ALLOCATION_UNIT: u32 = (256 / size_of::<Mat4>()) as u32;

/// The location of the first joint matrix in the skin uniform buffer.
#[derive(Clone, Copy)]
pub struct SkinByteOffset {
    /// The byte offset of the first joint matrix.
    pub byte_offset: u32,
}

impl SkinByteOffset {
    /// Index to be in address space based on the size of a skin uniform.
    const fn from_index(index: usize) -> Self {
        SkinByteOffset {
            byte_offset: (index * size_of::<Mat4>()) as u32,
        }
    }

    /// Returns this skin index in elements (not bytes).
    ///
    /// Each element is a 4x4 matrix.
    pub fn index(&self) -> u32 {
        self.byte_offset / size_of::<Mat4>() as u32
    }
}

/// The GPU buffers containing joint matrices for all skinned meshes.
///
/// This is double-buffered: we store the joint matrices of each mesh for the
/// previous frame in addition to those of each mesh for the current frame. This
/// is for motion vector calculation. Every frame, we swap buffers and overwrite
/// the joint matrix buffer from two frames ago with the data for the current
/// frame.
///
/// Notes on implementation: see comment on top of the `extract_skins` system.
#[derive(Resource)]
pub struct SkinUniforms {
    /// The CPU-side buffer that stores the joint matrices for skinned meshes in
    /// the current frame.
    pub current_staging_buffer: Vec<Mat4>,
    /// The GPU-side buffer that stores the joint matrices for skinned meshes in
    /// the current frame.
    pub current_buffer: Buffer,
    /// The GPU-side buffer that stores the joint matrices for skinned meshes in
    /// the previous frame.
    pub prev_buffer: Buffer,
    /// The offset allocator that manages the placement of the joints within the
    /// [`Self::current_buffer`].
    allocator: Allocator,
    /// Per-mesh offsets and membership in a shared palette.
    skin_uniform_info: MainEntityHashMap<SkinUniformInfo>,
    /// Owns one allocation for each exact ordered-joint/inverse-bindpose identity.
    shared_palettes: HashMap<Arc<SkinPaletteKey>, SharedSkinPalette>,
    /// The number of joint matrices in allocated shared palettes.
    total_joints: usize,
}

pub fn skin_uniforms_from_world(device: Res<RenderDevice>, mut commands: Commands) {
    let buffer_usages = (if skins_use_uniform_buffers(&device.limits()) {
        BufferUsages::UNIFORM
    } else {
        BufferUsages::STORAGE
    }) | BufferUsages::COPY_DST;

    // Create the current and previous buffer with the minimum sizes.
    //
    // These will be swapped every frame.
    let current_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("skin uniform buffer"),
        size: MAX_JOINTS as u64 * size_of::<Mat4>() as u64,
        usage: buffer_usages,
        mapped_at_creation: false,
    });
    let prev_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("skin uniform buffer"),
        size: MAX_JOINTS as u64 * size_of::<Mat4>() as u64,
        usage: buffer_usages,
        mapped_at_creation: false,
    });

    let res = SkinUniforms {
        current_staging_buffer: vec![],
        current_buffer,
        prev_buffer,
        allocator: Allocator::new(MAX_TOTAL_JOINTS),
        skin_uniform_info: MainEntityHashMap::default(),
        shared_palettes: HashMap::default(),
        total_joints: 0,
    };

    commands.insert_resource(res);
}

impl SkinUniforms {
    /// Returns the current offset in joints of the skin in the buffer.
    pub fn skin_index(&self, skin: MainEntity) -> Option<u32> {
        self.skin_uniform_info
            .get(&skin)
            .map(SkinUniformInfo::offset)
    }

    /// Returns the current offset in bytes of the skin in the buffer.
    pub fn skin_byte_offset(&self, skin: MainEntity) -> Option<SkinByteOffset> {
        self.skin_uniform_info.get(&skin).map(|skin_uniform_info| {
            SkinByteOffset::from_index(skin_uniform_info.offset() as usize)
        })
    }

    /// Returns an iterator over all skins in the scene.
    pub fn all_skins(&self) -> impl Iterator<Item = &MainEntity> {
        self.skin_uniform_info.keys()
    }
}

/// Keeps mesh lookup independent of the number of joints in its palette.
struct SkinUniformInfo {
    joint_offset: u32,
    palette_key: Arc<SkinPaletteKey>,
}

impl SkinUniformInfo {
    fn offset(&self) -> u32 {
        self.joint_offset
    }
}

#[derive(PartialEq, Eq, Hash)]
struct SkinPaletteKey {
    joints: Vec<MainEntity>,
    inverse_bindposes: AssetId<SkinnedMeshInverseBindposes>,
}

struct SharedSkinPalette {
    allocation: Allocation,
    users: usize,
    needs_full_refresh: bool,
}

impl SharedSkinPalette {
    fn offset(&self) -> u32 {
        self.allocation.offset * JOINTS_PER_ALLOCATION_UNIT
    }
}

/// Returns true if skinning must use uniforms (and dynamic offsets) because
/// storage buffers aren't supported on the current platform.
pub fn skins_use_uniform_buffers(limits: &WgpuLimits) -> bool {
    bevy_render::storage_buffers_are_unsupported(limits)
}

/// Uploads the buffers containing the joints to the GPU.
pub fn prepare_skins(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    uniform: ResMut<SkinUniforms>,
) {
    let uniform = uniform.into_inner();

    if uniform.current_staging_buffer.is_empty() {
        return;
    }

    // Swap current and previous buffers.
    mem::swap(&mut uniform.current_buffer, &mut uniform.prev_buffer);

    // Resize the buffers if necessary. Include extra space equal to `MAX_JOINTS`
    // because we need to be able to bind a full uniform buffer's worth of data
    // if skins use uniform buffers on this platform.
    let needed_size = (uniform.current_staging_buffer.len() as u64 + MAX_JOINTS as u64)
        * size_of::<Mat4>() as u64;
    if uniform.current_buffer.size() < needed_size {
        let mut new_size = uniform.current_buffer.size();
        while new_size < needed_size {
            // 1.5× growth factor.
            new_size = (new_size + new_size / 2).next_multiple_of(4);
        }

        // Create the new buffers.
        let buffer_usages = if skins_use_uniform_buffers(&render_device.limits()) {
            BufferUsages::UNIFORM
        } else {
            BufferUsages::STORAGE
        } | BufferUsages::COPY_DST;
        uniform.current_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("skin uniform buffer"),
            usage: buffer_usages,
            size: new_size,
            mapped_at_creation: false,
        });
        uniform.prev_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("skin uniform buffer"),
            usage: buffer_usages,
            size: new_size,
            mapped_at_creation: false,
        });

        // We've created a new `prev_buffer` but we don't have the previous joint
        // data needed to fill it out correctly. Use the current joint data
        // instead.
        //
        // TODO: This is a bug - will cause motion blur to ignore joint movement
        // for one frame.
        render_queue.write_buffer(
            &uniform.prev_buffer,
            0,
            bytemuck::must_cast_slice(&uniform.current_staging_buffer[..]),
        );
    }

    // Write the data from `uniform.current_staging_buffer` into
    // `uniform.current_buffer`.
    render_queue.write_buffer(
        &uniform.current_buffer,
        0,
        bytemuck::must_cast_slice(&uniform.current_staging_buffer[..]),
    );

    // We don't need to write `uniform.prev_buffer` because we already wrote it
    // last frame, and the data should still be on the GPU.
}

// Notes on implementation:
// We define the uniform binding as an array<mat4x4<f32>, N> in the shader,
// where N is the maximum number of Mat4s we can fit in the uniform binding,
// which may be as little as 16kB or 64kB. But, we may not need all N.
// We may only need, for example, 10.
//
// If we used uniform buffers ‘normally’ then we would have to write a full
// binding of data for each dynamic offset binding, which is wasteful, makes
// the buffer much larger than it needs to be, and uses more memory bandwidth
// to transfer the data, which then costs frame time So @superdump came up
// with this design: just bind data at the specified offset and interpret
// the data at that offset as an array<T, N> regardless of what is there.
//
// So instead of writing N Mat4s when you only need 10, you write 10, and
// then pad up to the next dynamic offset alignment. Then write the next.
// And for the last dynamic offset binding, make sure there is a full binding
// of data after it so that the buffer is of size
// `last dynamic offset` + `array<mat4x4<f32>>`.
//
// Then when binding the first dynamic offset, the first 10 entries in the array
// are what you expect, but if you read the 11th you’re reading ‘invalid’ data
// which could be padding or could be from the next binding.
//
// In this way, we can pack ‘variable sized arrays’ into uniform buffer bindings
// which normally only support fixed size arrays. You just have to make sure
// in the shader that you only read the values that are valid for that binding.
pub fn extract_skins(
    skin_uniforms: ResMut<SkinUniforms>,
    changed_skinned_meshes: Extract<
        Query<
            (Entity, &ViewVisibility, &SkinnedMesh),
            Or<(
                Changed<ViewVisibility>,
                Changed<SkinnedMesh>,
                AssetChanged<SkinnedMesh>,
            )>,
        >,
    >,
    skinned_mesh_inverse_bindposes: Extract<Res<Assets<SkinnedMeshInverseBindposes>>>,
    changed_transforms: Extract<Query<(Entity, &GlobalTransform), Changed<GlobalTransform>>>,
    joints: Extract<Query<&GlobalTransform>>,
    mut removed_skinned_meshes_query: Extract<RemovedComponents<SkinnedMesh>>,
) {
    let skin_uniforms = skin_uniforms.into_inner();
    add_or_delete_skins(skin_uniforms, &changed_skinned_meshes);

    for skinned_mesh_entity in removed_skinned_meshes_query.read() {
        // A component removed and re-added this frame already has new membership.
        if !changed_skinned_meshes.contains(skinned_mesh_entity) {
            remove_skin(skin_uniforms, skinned_mesh_entity.into());
        }
    }

    extract_shared_palettes(
        skin_uniforms,
        &skinned_mesh_inverse_bindposes,
        &changed_transforms,
        &joints,
    );
}

/// Applies visibility, joint-list, and inverse-bindpose changes to mesh membership.
fn add_or_delete_skins(
    skin_uniforms: &mut SkinUniforms,
    changed_skinned_meshes: &Query<
        (Entity, &ViewVisibility, &SkinnedMesh),
        Or<(
            Changed<ViewVisibility>,
            Changed<SkinnedMesh>,
            AssetChanged<SkinnedMesh>,
        )>,
    >,
) {
    for (entity, visibility, skin) in changed_skinned_meshes {
        let entity = MainEntity::from(entity);
        remove_skin(skin_uniforms, entity);
        if visibility.get() {
            add_skin(entity, skin, skin_uniforms);
        }
    }
}

/// Writes each palette once after all membership changes have been applied.
fn extract_shared_palettes(
    skin_uniforms: &mut SkinUniforms,
    inverse_bindposes: &Assets<SkinnedMeshInverseBindposes>,
    changed_transforms: &Query<(Entity, &GlobalTransform), Changed<GlobalTransform>>,
    joints: &Query<&GlobalTransform>,
) {
    for (key, palette) in &mut skin_uniforms.shared_palettes {
        let bindposes = inverse_bindposes.get(key.inverse_bindposes);
        if palette.needs_full_refresh {
            initialize_palette(
                key,
                palette,
                &mut skin_uniforms.current_staging_buffer,
                bindposes,
                joints,
            );
            palette.needs_full_refresh = false;
        } else if let Some(bindposes) = bindposes {
            update_changed_palette(
                key,
                palette,
                &mut skin_uniforms.current_staging_buffer,
                bindposes,
                changed_transforms,
            );
        }
    }
}

fn initialize_palette(
    key: &SkinPaletteKey,
    palette: &SharedSkinPalette,
    staging_buffer: &mut Vec<Mat4>,
    bindposes: Option<&SkinnedMeshInverseBindposes>,
    joints: &Query<&GlobalTransform>,
) {
    let offset = palette.offset() as usize;
    let required_len = offset + key.joints.len();
    if staging_buffer.len() < required_len {
        staging_buffer.resize(required_len, Mat4::IDENTITY);
    }
    for (index, &joint) in key.joints.iter().enumerate() {
        let bindpose = bindposes.and_then(|poses| poses.get(index));
        staging_buffer[offset + index] = match (bindpose, joints.get(*joint)) {
            (Some(bindpose), Ok(transform)) => transform.affine() * *bindpose,
            _ => Mat4::IDENTITY,
        };
    }
}

fn update_changed_palette(
    key: &SkinPaletteKey,
    palette: &SharedSkinPalette,
    staging_buffer: &mut [Mat4],
    bindposes: &SkinnedMeshInverseBindposes,
    changed_transforms: &Query<(Entity, &GlobalTransform), Changed<GlobalTransform>>,
) {
    let offset = palette.offset() as usize;
    for (index, (&joint, bindpose)) in key.joints.iter().zip(bindposes.iter()).enumerate() {
        let Ok((_, transform)) = changed_transforms.get(*joint) else {
            continue;
        };
        staging_buffer[offset + index] = transform.affine() * *bindpose;
    }
}

/// Registers a mesh against the canonical key, retaining constant-time offset lookup.
fn add_skin(entity: MainEntity, skin: &SkinnedMesh, skin_uniforms: &mut SkinUniforms) {
    let key = Arc::new(SkinPaletteKey {
        joints: skin.joints.iter().copied().map(MainEntity::from).collect(),
        inverse_bindposes: skin.inverse_bindposes.id(),
    });
    let (key, palette) = match skin_uniforms.shared_palettes.entry(key) {
        Entry::Occupied(entry) => (Arc::clone(entry.key()), entry.into_mut()),
        Entry::Vacant(entry) => {
            let Some(palette) = allocate_palette(&mut skin_uniforms.allocator, entry.key(), entity)
            else {
                return;
            };
            skin_uniforms.total_joints += entry.key().joints.len();
            (Arc::clone(entry.key()), entry.insert(palette))
        }
    };
    palette.users += 1;
    // Reused membership may accompany an inverse-bindpose change without joint changes.
    palette.needs_full_refresh = true;
    skin_uniforms.skin_uniform_info.insert(
        entity,
        SkinUniformInfo {
            joint_offset: palette.offset(),
            palette_key: key,
        },
    );
}

fn allocate_palette(
    allocator: &mut Allocator,
    key: &SkinPaletteKey,
    entity: MainEntity,
) -> Option<SharedSkinPalette> {
    let units = key
        .joints
        .len()
        .div_ceil(JOINTS_PER_ALLOCATION_UNIT as usize) as u32;
    let Some(allocation) = allocator.allocate(units) else {
        error!(
            "Out of space for skin: {:?}. Tried to allocate space for {:?} joints.",
            entity,
            key.joints.len()
        );
        return None;
    };
    Some(SharedSkinPalette {
        allocation,
        users: 0,
        needs_full_refresh: true,
    })
}

/// Releases the allocation only after its final mesh membership is removed.
fn remove_skin(skin_uniforms: &mut SkinUniforms, entity: MainEntity) {
    let Some(info) = skin_uniforms.skin_uniform_info.remove(&entity) else {
        return;
    };
    let Entry::Occupied(mut entry) = skin_uniforms.shared_palettes.entry(info.palette_key) else {
        panic!("skin membership must reference an allocated palette");
    };
    entry.get_mut().users -= 1;
    if entry.get().users != 0 {
        return;
    }
    let (key, palette) = entry.remove_entry();
    skin_uniforms.allocator.free(palette.allocation);
    skin_uniforms.total_joints -= key.joints.len();
}

// NOTE: The skinned joints uniform buffer has to be bound at a dynamic offset per
// entity and so cannot currently be batched on WebGL 2.
pub fn no_automatic_skin_batching(
    mut commands: Commands,
    query: Query<Entity, (With<SkinnedMesh>, Without<NoAutomaticBatching>)>,
    render_device: Res<RenderDevice>,
) {
    if !skins_use_uniform_buffers(&render_device.limits()) {
        return;
    }

    for entity in &query {
        commands.entity(entity).try_insert(NoAutomaticBatching);
    }
}
