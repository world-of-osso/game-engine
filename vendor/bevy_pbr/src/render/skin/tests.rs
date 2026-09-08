//! Explicit Vulkan integration tests for the production skin extraction boundary.
//!
//! Run with `cargo test -p bevy_pbr --lib render::skin::tests:: -- --ignored --test-threads=1`.
//! These tests require an available Vulkan device; initialization failures are not skipped.

mod buffer_modes;
mod characterization;

use std::{mem, sync::OnceLock, time::Duration};

use bevy_app::{App, TaskPoolPlugin};
use bevy_asset::{AssetApp, AssetPlugin, Handle};
use bevy_ecs::system::{RunSystemOnce, SystemId};
use bevy_math::Vec3;
use bevy_render::{
    MainWorld,
    render_resource::{CommandEncoderDescriptor, MapMode, PollType},
    renderer::initialize_renderer,
    settings::{Backends, RenderResources, WgpuSettings},
};

use super::*;

const GPU_WAIT: Duration = Duration::from_secs(10);

struct SkinFixture {
    main: App,
    render: World,
    extract: SystemId,
}

impl SkinFixture {
    fn new() -> Self {
        let mut main = App::new();
        main.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
        main.init_asset::<SkinnedMeshInverseBindposes>();
        Self::with_main_and_gpu(main, gpu_resources())
    }

    fn with_main_and_gpu(main: App, gpu: &RenderResources) -> Self {
        let mut render = World::new();
        render.insert_resource(gpu.0.clone());
        render.insert_resource(gpu.1.clone());
        render
            .run_system_once(skin_uniforms_from_world)
            .expect("initialize production skin buffers");
        let extract = render.register_system(extract_skins);
        Self {
            main,
            render,
            extract,
        }
    }

    fn joint(&mut self, x: f32) -> Entity {
        self.main.world_mut().spawn(global_translation(x)).id()
    }

    fn bindposes(&mut self, translations: &[f32]) -> Handle<SkinnedMeshInverseBindposes> {
        self.main
            .world_mut()
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .add(bindpose_asset(translations))
    }

    fn mesh(
        &mut self,
        joints: &[Entity],
        bindposes: &Handle<SkinnedMeshInverseBindposes>,
    ) -> Entity {
        self.main
            .world_mut()
            .spawn((
                SkinnedMesh {
                    inverse_bindposes: bindposes.clone(),
                    joints: joints.to_vec(),
                },
                ViewVisibility::VISIBLE,
            ))
            .id()
    }

    fn extract(&mut self) {
        self.main.update();
        let mut main_world = MainWorld::default();
        mem::swap(&mut *main_world, self.main.world_mut());
        self.render.insert_resource(main_world);
        let result = self.render.run_system(self.extract);
        let mut main_world = self
            .render
            .remove_resource::<MainWorld>()
            .expect("restore main world");
        mem::swap(&mut *main_world, self.main.world_mut());
        result.expect("run production skin extraction");
    }

    fn offset(&self, mesh: Entity) -> u32 {
        self.uniforms()
            .skin_index(mesh.into())
            .expect("visible skin offset")
    }

    fn uniforms(&self) -> &SkinUniforms {
        self.render.resource::<SkinUniforms>()
    }

    fn assert_palette(&self, mesh: Entity, translations: &[f32]) {
        let offset = self.offset(mesh) as usize;
        let actual = &self.uniforms().current_staging_buffer[offset..offset + translations.len()];
        let expected: Vec<_> = translations
            .iter()
            .map(|&x| matrix_translation(x))
            .collect();
        assert_eq!(actual, expected);
    }

    fn move_joint(&mut self, joint: Entity, x: f32) {
        *self
            .main
            .world_mut()
            .get_mut::<GlobalTransform>(joint)
            .expect("joint") = global_translation(x);
    }

    fn visibility(&mut self, mesh: Entity, visible: bool) {
        *self
            .main
            .world_mut()
            .get_mut::<ViewVisibility>(mesh)
            .expect("mesh visibility") = if visible {
            ViewVisibility::VISIBLE
        } else {
            ViewVisibility::HIDDEN
        };
    }

    fn upload(&mut self) {
        self.render
            .run_system_once(prepare_skins)
            .expect("upload production skin buffers");
    }
}

fn gpu_resources() -> &'static RenderResources {
    static GPU: OnceLock<RenderResources> = OnceLock::new();
    GPU.get_or_init(|| {
        bevy_tasks::block_on(initialize_renderer(
            Backends::VULKAN,
            None,
            &WgpuSettings::default(),
        ))
    })
}

fn global_translation(x: f32) -> GlobalTransform {
    GlobalTransform::from_translation(Vec3::new(x, 0.0, 0.0))
}

fn matrix_translation(x: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(x, 0.0, 0.0))
}

fn bindpose_asset(translations: &[f32]) -> SkinnedMeshInverseBindposes {
    translations
        .iter()
        .map(|&x| matrix_translation(x))
        .collect::<Vec<_>>()
        .into()
}

#[test]
#[ignore = "requires a Vulkan device; explicit skin palette integration run"]
fn shared_palette_identical_inputs_use_one_shader_offset() {
    let mut fixture = SkinFixture::new();
    let joints = [fixture.joint(3.0), fixture.joint(9.0)];
    let bindposes = fixture.bindposes(&[-1.0, -2.0]);
    let first = fixture.mesh(&joints, &bindposes);
    let second = fixture.mesh(&joints, &bindposes);
    fixture.extract();

    fixture.assert_palette(first, &[2.0, 7.0]);
    fixture.assert_palette(second, &[2.0, 7.0]);
    assert_eq!(fixture.offset(first), fixture.offset(second));
    assert_eq!(fixture.uniforms().all_skins().count(), 2);
    assert_eq!(
        fixture
            .uniforms()
            .skin_byte_offset(first.into())
            .unwrap()
            .index(),
        fixture.offset(first)
    );
}

#[test]
#[ignore = "requires a Vulkan device; explicit skin palette integration run"]
fn shared_palette_different_order_or_asset_identity_stays_distinct() {
    let mut fixture = SkinFixture::new();
    let joints = [fixture.joint(3.0), fixture.joint(9.0)];
    let bindposes = fixture.bindposes(&[-1.0, -2.0]);
    let other_asset = fixture.bindposes(&[-1.0, -2.0]);
    let first = fixture.mesh(&joints, &bindposes);
    let reordered = fixture.mesh(&[joints[1], joints[0]], &bindposes);
    let different_asset = fixture.mesh(&joints, &other_asset);
    fixture.extract();

    fixture.assert_palette(first, &[2.0, 7.0]);
    fixture.assert_palette(reordered, &[8.0, 1.0]);
    fixture.assert_palette(different_asset, &[2.0, 7.0]);
    assert_ne!(fixture.offset(first), fixture.offset(reordered));
    assert_ne!(fixture.offset(first), fixture.offset(different_asset));
    assert_ne!(fixture.offset(reordered), fixture.offset(different_asset));
}

#[test]
#[ignore = "requires a Vulkan device; explicit skin palette integration run"]
fn shared_palette_hiding_one_mesh_preserves_survivor_and_rejoins() {
    let mut fixture = SkinFixture::new();
    let joint = fixture.joint(4.0);
    let bindposes = fixture.bindposes(&[-1.0]);
    let first = fixture.mesh(&[joint], &bindposes);
    let second = fixture.mesh(&[joint], &bindposes);
    fixture.extract();
    let survivor_offset = fixture.offset(second);

    fixture.visibility(first, false);
    fixture.move_joint(joint, 8.0);
    fixture.extract();
    assert!(fixture.uniforms().skin_index(first.into()).is_none());
    assert_eq!(fixture.offset(second), survivor_offset);
    fixture.assert_palette(second, &[7.0]);

    fixture.visibility(first, true);
    fixture.extract();
    fixture.assert_palette(first, &[7.0]);
    assert_eq!(fixture.offset(second), survivor_offset);
    assert_eq!(fixture.offset(first), fixture.offset(second));
}

#[test]
#[ignore = "requires a Vulkan device; explicit skin palette integration run"]
fn shared_palette_removal_and_readdition_preserve_other_mesh() {
    let mut fixture = SkinFixture::new();
    let joint = fixture.joint(5.0);
    let bindposes = fixture.bindposes(&[-2.0]);
    let first = fixture.mesh(&[joint], &bindposes);
    let second = fixture.mesh(&[joint], &bindposes);
    fixture.extract();
    let survivor_offset = fixture.offset(second);

    fixture
        .main
        .world_mut()
        .entity_mut(first)
        .remove::<SkinnedMesh>();
    fixture.extract();
    assert!(fixture.uniforms().skin_index(first.into()).is_none());
    assert_eq!(fixture.offset(second), survivor_offset);
    fixture.assert_palette(second, &[3.0]);

    fixture
        .main
        .world_mut()
        .entity_mut(first)
        .insert(SkinnedMesh {
            joints: vec![joint],
            inverse_bindposes: bindposes.clone(),
        });
    fixture.extract();
    assert_eq!(fixture.offset(second), survivor_offset);
    fixture.assert_palette(first, &[3.0]);
    assert_eq!(fixture.offset(first), fixture.offset(second));
}

#[test]
#[ignore = "requires a Vulkan device; explicit skin palette integration run"]
fn shared_palette_rebinding_one_mesh_does_not_rebind_its_survivor() {
    let mut fixture = SkinFixture::new();
    let first_joint = fixture.joint(5.0);
    let replacement_joint = fixture.joint(12.0);
    let bindposes = fixture.bindposes(&[-2.0]);
    let first = fixture.mesh(&[first_joint], &bindposes);
    let second = fixture.mesh(&[first_joint], &bindposes);
    fixture.extract();
    let survivor_offset = fixture.offset(second);

    fixture
        .main
        .world_mut()
        .get_mut::<SkinnedMesh>(first)
        .unwrap()
        .joints = vec![replacement_joint];
    fixture.move_joint(first_joint, 7.0);
    fixture.extract();
    assert_eq!(fixture.offset(second), survivor_offset);
    assert_ne!(fixture.offset(first), fixture.offset(second));
    fixture.assert_palette(first, &[10.0]);
    fixture.assert_palette(second, &[5.0]);

    fixture
        .main
        .world_mut()
        .get_mut::<SkinnedMesh>(first)
        .unwrap()
        .joints = vec![first_joint];
    fixture.extract();
    fixture.assert_palette(first, &[5.0]);
    assert_eq!(fixture.offset(first), fixture.offset(second));
}

#[test]
#[ignore = "requires a Vulkan device; explicit skin palette integration run"]
fn shared_palette_asset_change_refreshes_unchanged_joint_transforms() {
    let mut fixture = SkinFixture::new();
    let joint = fixture.joint(9.0);
    let bindposes = fixture.bindposes(&[-2.0]);
    let first = fixture.mesh(&[joint], &bindposes);
    let second = fixture.mesh(&[joint], &bindposes);
    fixture.extract();
    fixture.extract();

    *fixture
        .main
        .world_mut()
        .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
        .get_mut(&bindposes)
        .unwrap() = bindpose_asset(&[-5.0]);
    fixture.extract();
    fixture.assert_palette(first, &[4.0]);
    fixture.assert_palette(second, &[4.0]);
    assert_eq!(fixture.offset(first), fixture.offset(second));
}

#[test]
#[ignore = "requires a Vulkan device; explicit skin palette integration run"]
fn shared_palette_same_frame_additions_receive_current_joint_pose() {
    let mut fixture = SkinFixture::new();
    let joint = fixture.joint(3.0);
    let bindposes = fixture.bindposes(&[-1.0]);
    let first = fixture.mesh(&[joint], &bindposes);
    fixture.extract();
    fixture.move_joint(joint, 10.0);
    let second = fixture.mesh(&[joint], &bindposes);
    let third = fixture.mesh(&[joint], &bindposes);
    fixture.extract();

    for mesh in [first, second, third] {
        fixture.assert_palette(mesh, &[9.0]);
    }
    assert_eq!(fixture.offset(first), fixture.offset(second));
    assert_eq!(fixture.offset(first), fixture.offset(third));
}

#[test]
#[ignore = "requires a Vulkan device; explicit skin palette integration run"]
fn shared_palette_last_removal_clears_mesh_lookup_and_readded_data_is_fresh() {
    let mut fixture = SkinFixture::new();
    let joint = fixture.joint(4.0);
    let bindposes = fixture.bindposes(&[-1.0]);
    let first = fixture.mesh(&[joint], &bindposes);
    let second = fixture.mesh(&[joint], &bindposes);
    fixture.extract();
    fixture.main.world_mut().despawn(first);
    fixture.main.world_mut().despawn(second);
    fixture.extract();
    assert_eq!(fixture.uniforms().all_skins().count(), 0);
    assert!(fixture.uniforms().skin_index(first.into()).is_none());
    assert!(fixture.uniforms().skin_index(second.into()).is_none());

    fixture.move_joint(joint, 15.0);
    let replacement = fixture.mesh(&[joint], &bindposes);
    fixture.extract();
    fixture.assert_palette(replacement, &[14.0]);
    assert_eq!(fixture.uniforms().all_skins().count(), 1);
}

#[test]
#[ignore = "requires a Vulkan device; explicit GPU skin upload/history run"]
fn shared_palette_gpu_upload_preserves_previous_frame_after_one_user_hides() {
    let mut fixture = SkinFixture::new();
    let joint = fixture.joint(3.0);
    let bindposes = fixture.bindposes(&[-1.0]);
    let first = fixture.mesh(&[joint], &bindposes);
    let second = fixture.mesh(&[joint], &bindposes);
    fixture.extract();
    install_copyable_history_buffers(&mut fixture.render);
    fixture.upload();
    fixture.upload();
    let offset = fixture.offset(second);
    assert_gpu_matrix(&fixture.render, false, offset, 2.0);
    assert_gpu_matrix(&fixture.render, true, offset, 2.0);

    fixture.visibility(first, false);
    fixture.move_joint(joint, 8.0);
    fixture.extract();
    fixture.upload();
    assert_eq!(fixture.offset(second), offset);
    assert_gpu_matrix(&fixture.render, false, offset, 7.0);
    assert_gpu_matrix(&fixture.render, true, offset, 2.0);

    fixture.move_joint(joint, 11.0);
    fixture.extract();
    fixture.upload();
    assert_gpu_matrix(&fixture.render, false, offset, 10.0);
    assert_gpu_matrix(&fixture.render, true, offset, 7.0);
}

// Production buffers lack COPY_SRC. Extra usage enables observation only; all
// palette bytes come from prepare_skins. Pre-sizing deliberately excludes resize behavior.
fn install_copyable_history_buffers(render: &mut World) {
    let device = render.resource::<RenderDevice>().clone();
    let mut uniforms = render.resource_mut::<SkinUniforms>();
    let usage = uniforms.current_buffer.usage() | BufferUsages::COPY_SRC;
    let size =
        (uniforms.current_staging_buffer.len() + MAX_JOINTS * 2) as u64 * size_of::<Mat4>() as u64;
    let descriptor = BufferDescriptor {
        label: Some("skin history test"),
        size,
        usage,
        mapped_at_creation: false,
    };
    uniforms.current_buffer = device.create_buffer(&descriptor);
    uniforms.prev_buffer = device.create_buffer(&descriptor);
}

fn assert_gpu_matrix(render: &World, previous: bool, offset: u32, translation: f32) {
    let uniforms = render.resource::<SkinUniforms>();
    let source = if previous {
        &uniforms.prev_buffer
    } else {
        &uniforms.current_buffer
    };
    let device = render.resource::<RenderDevice>();
    let queue = render.resource::<RenderQueue>();
    let readback = device.create_buffer(&BufferDescriptor {
        label: Some("skin palette readback"),
        size: size_of::<Mat4>() as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("read skin matrix"),
    });
    encoder.copy_buffer_to_buffer(
        source,
        u64::from(offset) * size_of::<Mat4>() as u64,
        &readback,
        0,
        size_of::<Mat4>() as u64,
    );
    queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(MapMode::Read, move |result| {
        sender.send(result).expect("readback receiver")
    });
    device
        .poll(PollType::Wait {
            submission_index: None,
            timeout: Some(GPU_WAIT),
        })
        .expect("wait for skin readback");
    receiver
        .recv_timeout(GPU_WAIT)
        .expect("skin readback callback")
        .expect("map skin readback");
    let bytes = slice.get_mapped_range();
    assert_eq!(
        &*bytes,
        bytemuck::bytes_of(&matrix_translation(translation))
    );
    drop(bytes);
    readback.unmap();
}
