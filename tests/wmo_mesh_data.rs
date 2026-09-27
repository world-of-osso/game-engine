use bevy::mesh::{Indices, Mesh, MeshVertexAttribute};
use game_engine::asset::wmo::*;

#[test]
fn portable_abbey_batches_match_bevy_mesh_attributes() {
    let abbey = std::fs::read("data/models/107075.wmo").unwrap();
    let abbey_root = load_wmo_root(&std::fs::read("data/models/107074.wmo").unwrap()).unwrap();
    let data = &abbey;
    let root = &abbey_root;
    {
        let payload = find_mogp(data).unwrap();
        let header = parse_mogp_header(payload).unwrap();
        let raw = parse_group_subchunks(&payload[MOGP_HEADER_SIZE..]).unwrap();
        let portable = game_engine::asset::wmo_format::mesh_data::build_group_batches(
            &header,
            &raw,
            Some(root),
        );
        let bevy = load_wmo_group_with_root(data, Some(root)).unwrap();
        assert_eq!(portable.len(), bevy.batches.len());
        for (plain, rendered) in portable.iter().zip(&bevy.batches) {
            assert_eq!(plain.material_index, rendered.material_index);
            assert_eq!(plain.batch_type, rendered.batch_type);
            assert_eq!(
                plain.uses_second_color_blend_alpha,
                rendered.uses_second_color_blend_alpha
            );
            assert_eq!(plain.uses_second_uv_set, rendered.uses_second_uv_set);
            assert_eq!(plain.uses_third_uv_set, rendered.uses_third_uv_set);
            assert_eq!(
                plain.uses_generated_tangents,
                rendered.uses_generated_tangents
            );
            assert_eq!(plain.has_vertex_color, rendered.has_vertex_color);
            let mesh = &rendered.mesh;
            assert!(
                matches!(mesh.indices(), Some(Indices::U32(indices)) if indices == &plain.indices)
            );
            assert_eq!(
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                    .unwrap()
                    .as_float3(),
                Some(plain.positions.as_slice())
            );
            assert_eq!(
                mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3(),
                Some(plain.normals.as_slice())
            );
            assert_eq!(
                mesh_float2(mesh, Mesh::ATTRIBUTE_UV_0),
                Some(plain.uvs.as_slice())
            );
            assert_eq!(
                mesh_float2(mesh, Mesh::ATTRIBUTE_UV_1),
                plain.second_uvs.as_deref()
            );
            assert_eq!(
                mesh_float2(mesh, WMO_THIRD_UV_ATTRIBUTE),
                plain.third_uvs.as_deref()
            );
            assert_eq!(
                mesh_float1(mesh, WMO_BLEND_ALPHA_ATTRIBUTE),
                plain.second_color_blend_alphas.as_deref()
            );
            assert_eq!(
                mesh_float4(mesh, Mesh::ATTRIBUTE_COLOR),
                plain.colors.as_deref()
            );
        }
    }
}

fn mesh_float1(mesh: &Mesh, attribute: MeshVertexAttribute) -> Option<&[f32]> {
    match mesh.attribute(attribute) {
        Some(bevy::mesh::VertexAttributeValues::Float32(values)) => Some(values),
        None => None,
        _ => panic!("unexpected mesh attribute format"),
    }
}

fn mesh_float2(mesh: &Mesh, attribute: MeshVertexAttribute) -> Option<&[[f32; 2]]> {
    match mesh.attribute(attribute) {
        Some(bevy::mesh::VertexAttributeValues::Float32x2(values)) => Some(values),
        None => None,
        _ => panic!("unexpected mesh attribute format"),
    }
}

fn mesh_float4(mesh: &Mesh, attribute: MeshVertexAttribute) -> Option<&[[f32; 4]]> {
    match mesh.attribute(attribute) {
        Some(bevy::mesh::VertexAttributeValues::Float32x4(values)) => Some(values),
        None => None,
        _ => panic!("unexpected mesh attribute format"),
    }
}
