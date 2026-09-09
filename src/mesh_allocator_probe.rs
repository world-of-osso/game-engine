use bevy::{
    prelude::*,
    render::{
        Render, RenderApp,
        mesh::{RenderMesh, allocator::allocate_and_free_meshes},
        render_asset::ExtractedAssets,
    },
};

pub fn register_mesh_allocator_probe(app: &mut App) {
    if std::env::var_os("MESH_ALLOCATOR_PROBE").is_none() {
        return;
    }
    app.sub_app_mut(RenderApp).add_systems(
        Render,
        log_extracted_mesh_sizes.before(allocate_and_free_meshes),
    );
}

fn log_extracted_mesh_sizes(meshes: Res<ExtractedAssets<RenderMesh>>) {
    for (id, mesh) in &meshes.extracted {
        let vertices = mesh.get_vertex_buffer_size();
        let indices = mesh.get_index_buffer_bytes().map(<[u8]>::len);
        eprintln!("MESH_ALLOCATOR_PROBE id={id:?} vertex_bytes={vertices} index_bytes={indices:?}");
    }
}
