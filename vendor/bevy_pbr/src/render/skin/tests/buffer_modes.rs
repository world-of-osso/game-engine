//! Consume unmodified production skin buffers through a test-only render shader.

use bevy_render::render_resource::{
    BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource, BindingType,
    BufferBinding, BufferBindingType, BufferSize, ColorTargetState, ColorWrites, CommandEncoder,
    Extent3d, LoadOp, Operations, Origin3d, PipelineLayout, PipelineLayoutDescriptor,
    RawFragmentState, RawRenderPipelineDescriptor, RawVertexState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, ShaderModule, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, StoreOp, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
    Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, WgpuLimits, WgpuTextureView,
};

use super::*;

const MATRIX_BYTES: u64 = size_of::<Mat4>() as u64;
const UNIFORM_BINDING_BYTES: u64 = MAX_JOINTS as u64 * MATRIX_BYTES;
const READBACK_ROW_BYTES: u32 = 256;
const OUTPUT_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
const OUTPUT_SIZE: Extent3d = Extent3d {
    width: 4,
    height: 1,
    depth_or_array_layers: 1,
};

const READER_SHADER: &str = r"
@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(positions[index], 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return matrices[MATRIX_INDEX][u32(position.x)];
}
";

const UNIFORM_CHILD_TEST: &str = "WOO_SKIN_BUFFER_MODE_CHILD_TEST";

// Bevy caches storage support process-wide. Uniform cases must initialize their
// device in a fresh process, even when the parent runs the complete ignored suite.
fn run_uniform_test_in_child(name: &str) -> bool {
    let (_, module) = module_path!()
        .split_once("::")
        .expect("skin tests have a crate-qualified module path");
    let test_name = format!("{module}::{name}");
    if let Some(child_test) = std::env::var_os(UNIFORM_CHILD_TEST) {
        assert_eq!(
            child_test.to_str(),
            Some(test_name.as_str()),
            "GPU child guard must identify exactly the selected test",
        );
        return false;
    }

    let status = std::process::Command::new(
        std::env::current_exe().expect("resolve current GPU test executable"),
    )
    .arg(&test_name)
    .args(["--exact", "--ignored", "--nocapture", "--test-threads=1"])
    .env(UNIFORM_CHILD_TEST, &test_name)
    .status()
    .expect("start isolated uniform GPU test");
    assert!(
        status.success(),
        "isolated GPU test {test_name} failed: {status}"
    );
    true
}

#[test]
#[ignore = "requires a Vulkan device; explicit uniform skin binding run"]
fn shared_palette_uniform_binding_consumes_nonzero_dynamic_offsets() {
    if run_uniform_test_in_child("shared_palette_uniform_binding_consumes_nonzero_dynamic_offsets")
    {
        return;
    }
    let mut fixture = uniform_fixture();
    let device = fixture.render.resource::<RenderDevice>();
    assert_eq!(device.limits().max_storage_buffers_per_shader_stage, 0);
    assert!(skins_use_uniform_buffers(&device.limits()));
    let alignment = device.limits().min_uniform_buffer_offset_alignment;

    let first_joint = fixture.joint(6.0);
    let second_joint = fixture.joint(13.0);
    let first_bindposes = fixture.bindposes(&[-1.0]);
    let second_bindposes = fixture.bindposes(&[-3.0]);
    let first = fixture.mesh(&[first_joint], &first_bindposes);
    let second = fixture.mesh(&[second_joint], &second_bindposes);
    let sibling = fixture.mesh(&[second_joint], &second_bindposes);
    fixture.extract();
    fixture.upload();

    assert_eq!(fixture.offset(second), fixture.offset(sibling));
    assert_ne!(fixture.offset(first), fixture.offset(second));
    let offsets = [first, second].map(|mesh| {
        fixture
            .uniforms()
            .skin_byte_offset(mesh.into())
            .unwrap()
            .byte_offset
    });
    assert!(offsets.iter().any(|&offset| offset != 0));
    for offset in offsets {
        assert_eq!(offset % alignment, 0);
    }
    assert_rendered_history(&fixture, first, 0, 5.0, 5.0);
    assert_rendered_history(&fixture, second, 0, 10.0, 10.0);
    assert_rendered_history(&fixture, sibling, 0, 10.0, 10.0);

    fixture.move_joint(second_joint, 22.0);
    fixture.extract();
    fixture.upload();
    assert_rendered_history(&fixture, sibling, 0, 19.0, 10.0);
    assert_rendered_history(&fixture, first, 0, 5.0, 5.0);
}

#[test]
#[ignore = "requires a Vulkan device; explicit production skin buffer growth run"]
fn shared_palette_buffer_growth_resets_history_then_preserves_next_frame() {
    let fixture = SkinFixture::new();
    assert!(!skins_use_uniform_buffers(
        &fixture.render.resource::<RenderDevice>().limits()
    ));
    assert_buffer_growth_history(fixture);
}

#[test]
#[ignore = "requires a Vulkan device; explicit uniform skin buffer growth run"]
fn shared_palette_uniform_buffer_growth_resets_history_then_preserves_next_frame() {
    if run_uniform_test_in_child(
        "shared_palette_uniform_buffer_growth_resets_history_then_preserves_next_frame",
    ) {
        return;
    }
    let fixture = uniform_fixture();
    assert!(skins_use_uniform_buffers(
        &fixture.render.resource::<RenderDevice>().limits()
    ));
    assert_buffer_growth_history(fixture);
}

fn assert_buffer_growth_history(mut fixture: SkinFixture) {
    let joint = fixture.joint(3.0);
    let bindposes = fixture.bindposes(&[-1.0]);
    let mesh = fixture.mesh(&[joint], &bindposes);
    let sibling = fixture.mesh(&[joint], &bindposes);
    fixture.extract();
    fixture.upload();
    assert_rendered_history(&fixture, mesh, 0, 2.0, 2.0);
    let original_offset = fixture.offset(mesh);
    let initial_size = fixture.uniforms().current_buffer.size();

    fixture.move_joint(joint, 8.0);
    fixture.extract();
    fixture.upload();
    assert_rendered_history(&fixture, sibling, 0, 7.0, 2.0);
    assert_eq!(fixture.uniforms().current_buffer.size(), initial_size);

    let extra_joints: Vec<_> = (0..MAX_JOINTS)
        .map(|index| fixture.joint(20.0 + index as f32))
        .collect();
    let extra_bindposes = fixture.bindposes(&vec![-1.0; MAX_JOINTS]);
    let extra_mesh = fixture.mesh(&extra_joints, &extra_bindposes);
    fixture.move_joint(joint, 12.0);
    fixture.extract();
    fixture.upload();
    let grown_size = fixture.uniforms().current_buffer.size();
    assert!(grown_size > initial_size);
    assert_eq!(fixture.uniforms().prev_buffer.size(), grown_size);
    assert_eq!(fixture.offset(mesh), original_offset);
    assert_eq!(fixture.offset(sibling), original_offset);
    assert_rendered_history(&fixture, mesh, 0, 11.0, 11.0);
    let last_joint = (MAX_JOINTS - 1) as u32;
    let last_translation = 19.0 + last_joint as f32;
    assert_rendered_history(
        &fixture,
        extra_mesh,
        last_joint,
        last_translation,
        last_translation,
    );

    fixture.move_joint(joint, 18.0);
    fixture.move_joint(extra_joints[MAX_JOINTS - 1], 400.0);
    fixture.extract();
    fixture.upload();
    assert_eq!(fixture.uniforms().current_buffer.size(), grown_size);
    assert_eq!(fixture.uniforms().prev_buffer.size(), grown_size);
    assert_rendered_history(&fixture, sibling, 0, 17.0, 11.0);
    assert_rendered_history(&fixture, extra_mesh, last_joint, 399.0, last_translation);
}

fn uniform_fixture() -> SkinFixture {
    let mut main = App::new();
    main.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
    main.init_asset::<SkinnedMeshInverseBindposes>();
    static GPU: OnceLock<RenderResources> = OnceLock::new();
    let gpu = GPU.get_or_init(|| {
        let settings = WgpuSettings {
            constrained_limits: Some(WgpuLimits {
                max_storage_buffers_per_shader_stage: 0,
                ..Default::default()
            }),
            ..Default::default()
        };
        bevy_tasks::block_on(initialize_renderer(Backends::VULKAN, None, &settings))
    });
    SkinFixture::with_main_and_gpu(main, gpu)
}

fn assert_rendered_history(
    fixture: &SkinFixture,
    mesh: Entity,
    joint: u32,
    current: f32,
    previous: f32,
) {
    let uniforms = fixture.uniforms();
    let offset = uniforms.skin_byte_offset(mesh.into()).unwrap().byte_offset;
    for (buffer, expected) in [
        (&uniforms.current_buffer, current),
        (&uniforms.prev_buffer, previous),
    ] {
        assert!(!buffer.usage().contains(BufferUsages::COPY_SRC));
        assert_eq!(
            draw_matrix(&fixture.render, buffer, offset, joint),
            matrix_translation(expected),
        );
    }
}

fn draw_matrix(render: &World, source: &Buffer, byte_offset: u32, joint: u32) -> Mat4 {
    let device = render.resource::<RenderDevice>();
    let uniform = skins_use_uniform_buffers(&device.limits());
    let matrix_index = if uniform {
        joint
    } else {
        byte_offset / MATRIX_BYTES as u32 + joint
    };
    let (pipeline, layout) = create_reader_pipeline(device, uniform, matrix_index);
    let binding = create_skin_binding(device, &layout, source, uniform);
    let output = create_output_texture(device);
    let offsets = [byte_offset];
    let dynamic_offsets = if uniform { &offsets[..] } else { &[] };
    let readback = render_matrix_to_readback(render, &pipeline, &binding, &output, dynamic_offsets);
    read_matrix_readback(device, &readback)
}

fn create_reader_pipeline(
    device: &RenderDevice,
    uniform: bool,
    matrix_index: u32,
) -> (RenderPipeline, BindGroupLayout) {
    let layout = create_skin_layout(device, uniform);
    let shader = create_reader_shader(device, uniform, matrix_index);
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("skin matrix reader"),
        bind_group_layouts: &[Some(layout.value())],
        immediate_size: 0,
    });
    let pipeline = create_reader_render_pipeline(device, &shader, &pipeline_layout);
    (pipeline, layout)
}

fn create_skin_layout(device: &RenderDevice, uniform: bool) -> BindGroupLayout {
    let (ty, bytes) = if uniform {
        (BufferBindingType::Uniform, UNIFORM_BINDING_BYTES)
    } else {
        (BufferBindingType::Storage { read_only: true }, MATRIX_BYTES)
    };
    device.create_bind_group_layout(
        Some("production skin buffer reader"),
        &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty,
                has_dynamic_offset: uniform,
                min_binding_size: BufferSize::new(bytes),
            },
            count: None,
        }],
    )
}

fn create_reader_shader(device: &RenderDevice, uniform: bool, matrix_index: u32) -> ShaderModule {
    let binding = if uniform {
        format!("@group(0) @binding(0) var<uniform> matrices: array<mat4x4<f32>, {MAX_JOINTS}>;")
    } else {
        "@group(0) @binding(0) var<storage, read> matrices: array<mat4x4<f32>>;".to_string()
    };
    let source = format!("{binding}\nconst MATRIX_INDEX: u32 = {matrix_index}u;\n{READER_SHADER}");
    device.create_and_validate_shader_module(ShaderModuleDescriptor {
        label: Some("test-only skin matrix reader"),
        source: ShaderSource::Wgsl(source.into()),
    })
}

fn create_reader_render_pipeline(
    device: &RenderDevice,
    shader: &ShaderModule,
    layout: &PipelineLayout,
) -> RenderPipeline {
    device.create_render_pipeline(&RawRenderPipelineDescriptor {
        label: Some("skin matrix reader"),
        layout: Some(layout),
        vertex: RawVertexState {
            module: shader,
            entry_point: Some("vertex"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(RawFragmentState {
            module: shader,
            entry_point: Some("fragment"),
            targets: &[Some(ColorTargetState {
                format: OUTPUT_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_skin_binding(
    device: &RenderDevice,
    layout: &BindGroupLayout,
    source: &Buffer,
    uniform: bool,
) -> BindGroup {
    device.create_bind_group(
        Some("production skin buffer"),
        layout,
        &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(BufferBinding {
                buffer: source,
                offset: 0,
                size: if uniform {
                    BufferSize::new(UNIFORM_BINDING_BYTES)
                } else {
                    None
                },
            }),
        }],
    )
}

fn create_output_texture(device: &RenderDevice) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("skin matrix columns"),
        size: OUTPUT_SIZE,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: OUTPUT_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn render_matrix_to_readback(
    render: &World,
    pipeline: &RenderPipeline,
    binding: &BindGroup,
    output: &Texture,
    offsets: &[u32],
) -> Buffer {
    let device = render.resource::<RenderDevice>();
    let readback = device.create_buffer(&BufferDescriptor {
        label: Some("rendered skin matrix readback"),
        size: u64::from(READBACK_ROW_BYTES),
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let view = output.create_view(&TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("consume production skin matrix"),
    });
    record_reader_draw(&mut encoder, &view, pipeline, binding, offsets);
    copy_output_to_buffer(&mut encoder, output, &readback);
    render.resource::<RenderQueue>().submit([encoder.finish()]);
    readback
}

fn record_reader_draw(
    encoder: &mut CommandEncoder,
    view: &WgpuTextureView,
    pipeline: &RenderPipeline,
    binding: &BindGroup,
    offsets: &[u32],
) {
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("read skin matrix columns"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Default::default()),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(&**pipeline);
    pass.set_bind_group(0, &**binding, offsets);
    pass.draw(0..3, 0..1);
}

fn copy_output_to_buffer(encoder: &mut CommandEncoder, output: &Texture, readback: &Buffer) {
    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture: output,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: readback,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(READBACK_ROW_BYTES),
                rows_per_image: Some(1),
            },
        },
        OUTPUT_SIZE,
    );
}

fn read_matrix_readback(device: &RenderDevice, readback: &Buffer) -> Mat4 {
    map_readback(device, readback);
    let slice = readback.slice(..);
    let bytes = slice.get_mapped_range();
    let values = std::array::from_fn(|index| {
        let start = index * size_of::<f32>();
        f32::from_le_bytes(bytes[start..start + size_of::<f32>()].try_into().unwrap())
    });
    drop(bytes);
    readback.unmap();
    Mat4::from_cols_array(&values)
}

fn map_readback(device: &RenderDevice, readback: &Buffer) {
    let (sender, receiver) = std::sync::mpsc::channel();
    readback.slice(..).map_async(MapMode::Read, move |result| {
        sender.send(result).expect("matrix readback receiver")
    });
    device
        .poll(PollType::Wait {
            submission_index: None,
            timeout: Some(GPU_WAIT),
        })
        .expect("wait for rendered matrix readback");
    receiver
        .recv_timeout(GPU_WAIT)
        .expect("rendered matrix readback callback")
        .expect("map rendered matrix readback");
}
