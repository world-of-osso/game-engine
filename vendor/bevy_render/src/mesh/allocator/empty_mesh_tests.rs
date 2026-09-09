use super::*;
use bevy_asset::RenderAssetUsages;
use bevy_log::{tracing, tracing_subscriber};
use bevy_mesh::{Indices, PrimitiveTopology};
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct LogCapture(Arc<Mutex<Vec<u8>>>);
impl Write for LogCapture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn triangle(empty: bool) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let positions = if empty {
        vec![]
    } else {
        vec![[1.0f32, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(if empty { vec![] } else { vec![0, 1, 2] }));
    mesh
}

fn read_bytes(
    device: &RenderDevice,
    queue: &RenderQueue,
    slice: MeshBufferSlice<'_>,
    stride: u64,
) -> Vec<u8> {
    let size = u64::from(slice.range.end - slice.range.start) * stride;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mesh allocator regression readback"),
        size,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(
        slice.buffer,
        u64::from(slice.range.start) * stride,
        &staging,
        0,
        size,
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap()
        });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    receiver.recv().unwrap().unwrap();
    let bytes = staging.slice(..).get_mapped_range().to_vec();
    staging.unmap();
    bytes
}

#[test]
fn empty_mesh_upload_and_valid_empty_valid_lifecycle() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        bevy_tasks::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("GPU adapter required");
    let (device, queue) =
        bevy_tasks::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let device = RenderDevice::new(crate::renderer::WgpuWrapper::new(device));
    let queue = RenderQueue(Arc::new(crate::renderer::WgpuWrapper::new(queue)));
    let mut allocator = MeshAllocator {
        slab_allocator: SlabAllocator::new(),
        general_vertex_slabs_supported: true,
    };
    let settings = MeshAllocatorSettings::default();
    let mut layouts = MeshVertexBufferLayouts::default();
    let empty_id = AssetId::<Mesh>::Uuid {
        uuid: bevy_asset::uuid::Uuid::from_u128(1),
    };
    let changing_id = AssetId::<Mesh>::Uuid {
        uuid: bevy_asset::uuid::Uuid::from_u128(2),
    };
    let capture = LogCapture(Arc::new(Mutex::new(Vec::new())));
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    for empty in [false, true, false] {
        let mut extracted = ExtractedAssets::<RenderMesh>::default();
        extracted.extracted = vec![(empty_id, triangle(true)), (changing_id, triangle(empty))];
        extracted.modified.insert(changing_id);
        extracted.modified.insert(empty_id);
        allocator.free_meshes(&extracted);
        allocator.allocate_meshes(&settings, &extracted, &mut layouts, &device, &queue);
        assert!(allocator.mesh_vertex_slice(&empty_id).is_none());
        assert!(allocator.mesh_index_slice(&empty_id).is_none());
        if empty {
            assert!(allocator.mesh_vertex_slice(&changing_id).is_none());
            assert!(allocator.mesh_index_slice(&changing_id).is_none());
        } else {
            let vertices = read_bytes(
                &device,
                &queue,
                allocator.mesh_vertex_slice(&changing_id).unwrap(),
                12,
            );
            let expected: Vec<u8> = (1..=9).flat_map(|n| (n as f32).to_ne_bytes()).collect();
            assert_eq!(vertices, expected);
            let indices = read_bytes(
                &device,
                &queue,
                allocator.mesh_index_slice(&changing_id).unwrap(),
                4,
            );
            let expected: Vec<u8> = [0u32, 1, 2]
                .into_iter()
                .flat_map(u32::to_ne_bytes)
                .collect();
            assert_eq!(indices, expected);
        }
    }
    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(!logs.contains("Use-after-free"), "{logs}");
}
