use std::f32::consts::PI;
use std::path::PathBuf;
use std::time::Duration;

use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;
use wow_engine::ipc::IpcPlugin;

mod asset;
mod camera;

use camera::{Player, WowCamera, WowCameraPlugin};

const DEFAULT_M2: &str =
    "/syncthing/Sync/Projects/wow/reference-addons.new/TomTom/Images/Arrow.m2";

#[derive(Resource)]
struct DumpTreeFlag;

fn main() {
    let dump_tree = std::env::args().any(|a| a == "--dump-tree");

    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(IpcPlugin)
        .add_plugins(WowCameraPlugin)
        .add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                refresh_interval: Duration::from_millis(500),
                ..default()
            },
        })
        .add_systems(Startup, setup);

    if dump_tree {
        app.insert_resource(DumpTreeFlag);
        app.add_systems(PostStartup, dump_tree_and_exit);
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera with WoW-style orbit controller
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
        WowCamera::default(),
        AmbientLight {
            color: Color::WHITE,
            brightness: 150.0,
            ..default()
        },
    ));

    // Directional light (sun)
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-PI / 4.0)),
    ));

    // Ground plane (100x100)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
    ));

    // Load M2 model from CLI arg or default path
    let m2_path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_M2));

    match asset::m2::load_m2(&m2_path) {
        Ok(mesh) => {
            let name = m2_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("m2_model");
            commands.spawn((
                Name::new(name.to_owned()),
                Player,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.8, 0.4, 0.2),
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.5, 0.0),
            ));
        }
        Err(e) => eprintln!("Failed to load M2 {}: {e}", m2_path.display()),
    }
}

#[allow(clippy::type_complexity)]
fn dump_tree_and_exit(
    tree_query: Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: Query<&ChildOf>,
    mut exit: MessageWriter<AppExit>,
) {
    let tree = wow_engine::dump::build_tree(&tree_query, &parent_query, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}
