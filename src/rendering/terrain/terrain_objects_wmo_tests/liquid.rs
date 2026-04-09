use super::*;

#[test]
fn build_wmo_liquid_mesh_skips_empty_tiles_and_uses_vertex_heights() {
    let liquid = wmo::WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 3,
            y_verts: 2,
            x_tiles: 2,
            y_tiles: 1,
            position: [10.0, 20.0, 30.0],
            material_id: 7,
        },
        vertices: (30..36)
            .map(|h| WmoLiquidVertex {
                raw: [0; 4],
                height: h as f32,
            })
            .collect(),
        tiles: vec![
            WmoLiquidTile {
                liquid_type: 3,
                fishable: false,
                shared: false,
            },
            WmoLiquidTile {
                liquid_type: 0x0F,
                fishable: false,
                shared: false,
            },
        ],
    };

    let mesh = build_wmo_liquid_mesh(&liquid);
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("expected wmo liquid positions");
    };
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
        mesh.attribute(Mesh::ATTRIBUTE_COLOR)
    else {
        panic!("expected wmo liquid colors");
    };
    assert_eq!(positions.len(), 4);
    assert_eq!(colors.len(), 4);
    assert_eq!(positions[0], [-10.0, 29.0, 20.0]);
    assert_eq!(positions[1], [-(10.0 + WMO_LIQUID_TILE_SIZE), 30.0, 20.0]);
    assert_eq!(positions[2], [-10.0, 32.0, 20.0 + WMO_LIQUID_TILE_SIZE]);
    assert_eq!(colors[0], [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(mesh.indices().unwrap().len(), 6);
}

#[test]
fn wmo_liquid_geometry_empty_tiles() {
    let liquid = wmo::WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 1,
            y_verts: 1,
            x_tiles: 0,
            y_tiles: 0,
            position: [0.0, 0.0, 0.0],
            material_id: 0,
        },
        vertices: vec![WmoLiquidVertex {
            raw: [0; 4],
            height: 10.0,
        }],
        tiles: vec![],
    };
    let (positions, _, _, _, indices) = build_wmo_liquid_geometry(&liquid);
    assert!(positions.is_empty());
    assert!(indices.is_empty());
}

#[test]
fn wmo_liquid_tile_exists_marks_0x0f_as_empty() {
    let liquid = wmo::WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 3,
            y_verts: 2,
            x_tiles: 2,
            y_tiles: 1,
            position: [0.0; 3],
            material_id: 0,
        },
        vertices: (0..6)
            .map(|_| WmoLiquidVertex {
                raw: [0; 4],
                height: 1.0,
            })
            .collect(),
        tiles: vec![
            WmoLiquidTile {
                liquid_type: 3,
                fishable: false,
                shared: false,
            },
            WmoLiquidTile {
                liquid_type: 0x0F,
                fishable: false,
                shared: false,
            },
        ],
    };
    assert!(wmo_liquid_tile_exists(&liquid, 0, 0));
    assert!(!wmo_liquid_tile_exists(&liquid, 0, 1));
}

#[test]
fn wmo_liquid_height_from_vertex() {
    let liquid = wmo::WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 2,
            y_verts: 2,
            x_tiles: 1,
            y_tiles: 1,
            position: [0.0; 3],
            material_id: 0,
        },
        vertices: vec![
            WmoLiquidVertex {
                raw: [0; 4],
                height: 10.0,
            },
            WmoLiquidVertex {
                raw: [0; 4],
                height: 20.0,
            },
            WmoLiquidVertex {
                raw: [0; 4],
                height: 30.0,
            },
            WmoLiquidVertex {
                raw: [0; 4],
                height: 40.0,
            },
        ],
        tiles: vec![WmoLiquidTile {
            liquid_type: 1,
            fishable: false,
            shared: false,
        }],
    };
    assert!((wmo_liquid_height(&liquid, 0, 0) - 10.0).abs() < 0.01);
    assert!((wmo_liquid_height(&liquid, 0, 1) - 20.0).abs() < 0.01);
    assert!((wmo_liquid_height(&liquid, 1, 0) - 30.0).abs() < 0.01);
    assert!((wmo_liquid_height(&liquid, 1, 1) - 40.0).abs() < 0.01);
}

#[test]
fn wmo_liquid_geometry_uvs_normalized() {
    let liquid = wmo::WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 3,
            y_verts: 3,
            x_tiles: 2,
            y_tiles: 2,
            position: [0.0; 3],
            material_id: 0,
        },
        vertices: (0..9)
            .map(|_| WmoLiquidVertex {
                raw: [0; 4],
                height: 1.0,
            })
            .collect(),
        tiles: (0..4)
            .map(|_| WmoLiquidTile {
                liquid_type: 1,
                fishable: false,
                shared: false,
            })
            .collect(),
    };
    let (_, _, uvs, _, _) = build_wmo_liquid_geometry(&liquid);
    for uv in &uvs {
        assert!(uv[0] >= 0.0 && uv[0] <= 1.0, "u out of range: {}", uv[0]);
        assert!(uv[1] >= 0.0 && uv[1] <= 1.0, "v out of range: {}", uv[1]);
    }
}

#[test]
fn wmo_liquid_geometry_normals_up() {
    let liquid = wmo::WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 2,
            y_verts: 2,
            x_tiles: 1,
            y_tiles: 1,
            position: [0.0; 3],
            material_id: 0,
        },
        vertices: (0..4)
            .map(|_| WmoLiquidVertex {
                raw: [0; 4],
                height: 5.0,
            })
            .collect(),
        tiles: vec![WmoLiquidTile {
            liquid_type: 1,
            fishable: false,
            shared: false,
        }],
    };
    let (_, normals, _, _, _) = build_wmo_liquid_geometry(&liquid);
    for normal in &normals {
        assert_eq!(*normal, [0.0, 1.0, 0.0]);
    }
}

#[test]
fn wmo_liquid_geometry_index_pattern() {
    let liquid = wmo::WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 2,
            y_verts: 2,
            x_tiles: 1,
            y_tiles: 1,
            position: [0.0; 3],
            material_id: 0,
        },
        vertices: (0..4)
            .map(|_| WmoLiquidVertex {
                raw: [0; 4],
                height: 5.0,
            })
            .collect(),
        tiles: vec![WmoLiquidTile {
            liquid_type: 1,
            fishable: false,
            shared: false,
        }],
    };
    let (_, _, _, _, indices) = build_wmo_liquid_geometry(&liquid);
    assert_eq!(indices.len(), 6);
    assert_eq!(indices, vec![0, 1, 2, 2, 1, 3]);
}
