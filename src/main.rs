use std::f32::consts::PI;
use std::path::PathBuf;
use std::time::Duration;

use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use wow_engine::ipc::IpcPlugin;

mod asset;
mod camera;

use camera::{Player, WowCamera, WowCameraPlugin};

const DEFAULT_M2: &str =
    "/syncthing/Sync/Projects/wow/reference-addons.new/TomTom/Images/Arrow.m2";

#[derive(Resource)]
struct DumpTreeFlag;

#[derive(Resource)]
struct ScreenshotRequest {
    output: PathBuf,
    frames_remaining: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let screenshot = parse_screenshot_args(&args);
    let dump_tree = args.iter().any(|a| a == "--dump-tree");

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

    if let Some(req) = screenshot {
        app.insert_resource(req);
        app.add_systems(Update, take_screenshot);
    }

    app.run();
}

/// Parse `screenshot <output> [model]` from args. Returns None if not a screenshot command.
fn parse_screenshot_args(args: &[String]) -> Option<ScreenshotRequest> {
    if args.first().map(|s| s.as_str()) != Some("screenshot") {
        return None;
    }
    let output = args.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("screenshot.webp"));
    Some(ScreenshotRequest { output, frames_remaining: 3 })
}

/// Find the model path from CLI args.
/// Normal: `wow-engine [model.m2] [--flags]`
/// Screenshot: `wow-engine screenshot [output.webp] [model.m2]`
fn parse_model_path() -> PathBuf {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("screenshot") {
        // Third arg (index 2) is the model path
        args.get(2).map(PathBuf::from)
    } else {
        args.iter().find(|a| !a.starts_with("--")).map(PathBuf::from)
    }
    .unwrap_or_else(|| PathBuf::from(DEFAULT_M2))
}

fn take_screenshot(mut commands: Commands, req: Option<ResMut<ScreenshotRequest>>) {
    let Some(mut req) = req else { return };
    if req.frames_remaining > 0 {
        req.frames_remaining -= 1;
        return;
    }
    commands.remove_resource::<ScreenshotRequest>();
    let output = req.output.clone();
    commands
        .spawn(Screenshot::primary_window())
        .observe(move |trigger: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            save_screenshot(&trigger.image, &output);
            exit.write(AppExit::Success);
        });
}

fn save_screenshot(img: &bevy::image::Image, output: &PathBuf) {
    let Some(data) = img.data.as_ref() else {
        eprintln!("Screenshot has no pixel data");
        return;
    };
    let size = img.size();
    let encoder = webp::Encoder::from_rgba(data, size.x, size.y);
    let webp_data = encoder.encode(15.0);
    std::fs::write(output, &*webp_data)
        .unwrap_or_else(|e| eprintln!("Failed to write {}: {e}", output.display()));
    println!("Saved {} ({} bytes)", output.display(), webp_data.len());
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
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
    let m2_path = parse_model_path();

    match asset::m2::load_m2(&m2_path) {
        Ok(model) => {
            let name = m2_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("m2_model");
            commands
                .spawn((
                    Name::new(name.to_owned()),
                    Player,
                    Transform::from_xyz(0.0, 0.5, 0.0),
                    Visibility::default(),
                ))
                .with_children(|parent| {
                    for (i, batch) in model.batches.into_iter().enumerate() {
                        let material =
                            load_batch_material(&batch, i, &mut images, &mut materials);
                        parent.spawn((
                            Mesh3d(meshes.add(batch.mesh)),
                            MeshMaterial3d(material),
                        ));
                    }
                });
        }
        Err(e) => eprintln!("Failed to load M2 {}: {e}", m2_path.display()),
    }
}

const PLACEHOLDER_COLORS: &[Color] = &[
    Color::srgb(0.8, 0.5, 0.3), // skin tone
    Color::srgb(0.3, 0.5, 0.8), // blue
    Color::srgb(0.3, 0.8, 0.3), // green
    Color::srgb(0.8, 0.3, 0.3), // red
    Color::srgb(0.7, 0.7, 0.3), // yellow
    Color::srgb(0.6, 0.3, 0.7), // purple
];

fn load_batch_material(
    batch: &asset::m2::M2RenderBatch,
    index: usize,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    let texture_dir = PathBuf::from("data/textures");
    if let Some(fdid) = batch.texture_fdid {
        let blp_path = texture_dir.join(format!("{fdid}.blp"));
        if blp_path.exists() {
            match asset::blp::load_blp_to_image(&blp_path) {
                Ok(image) => {
                    return materials.add(StandardMaterial {
                        base_color_texture: Some(images.add(image)),
                        ..default()
                    });
                }
                Err(e) => eprintln!("Failed to load BLP {}: {e}", blp_path.display()),
            }
        } else {
            eprintln!("Missing texture: data/textures/{fdid}.blp (download with casc-extract)");
        }
    }
    let color = PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()];
    materials.add(StandardMaterial {
        base_color: color,
        ..default()
    })
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
