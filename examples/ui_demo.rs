//! UI system smoke test: renders colored rectangles as UI overlay on a 3D scene.
//!
//! Run: cargo run --example ui_demo

use bevy::prelude::*;
use game_engine::ui::anchor::{Anchor, AnchorPoint};
use game_engine::ui::plugin::{UiPlugin, UiState};
use game_engine::ui::strata::FrameStrata;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(UiPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ui_state: ResMut<UiState>,
) {
    // 3D scene: ground plane + light + camera
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(5.0, 8.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // UI frames
    let reg = &mut ui_state.registry;

    setup_health_bar(reg);
    setup_tooltip(reg);
    setup_action_bar(reg);

    println!("UI Demo: Created 4 frames");
    println!("  HealthBarBG (top-left, dark red)");
    println!("  HealthBarFill (75% green fill)");
    println!("  Tooltip (center, dark)");
    println!("  ActionBar (bottom center)");
}

fn setup_health_bar(reg: &mut game_engine::ui::registry::FrameRegistry) {
    // Health bar background (dark red, top-left corner)
    let bg_id = reg.create_frame("HealthBarBG", None);
    if let Some(f) = reg.get_mut(bg_id) {
        f.width = 300.0;
        f.height = 30.0;
        f.background_color = Some([0.2, 0.0, 0.0, 0.8]);
        f.anchors.push(Anchor {
            point: AnchorPoint::TopLeft,
            relative_to: None,
            relative_point: AnchorPoint::TopLeft,
            x_offset: 20.0,
            y_offset: -20.0,
        });
    }

    // Health bar fill (green, fills 75% of background)
    let fill_id = reg.create_frame("HealthBarFill", Some(bg_id));
    if let Some(f) = reg.get_mut(fill_id) {
        f.width = 225.0; // 75% of 300
        f.height = 30.0;
        f.background_color = Some([0.0, 0.8, 0.0, 1.0]);
        f.anchors.push(Anchor {
            point: AnchorPoint::Left,
            relative_to: None,
            relative_point: AnchorPoint::Left,
            x_offset: 0.0,
            y_offset: 0.0,
        });
    }
}

fn setup_tooltip(reg: &mut game_engine::ui::registry::FrameRegistry) {
    let tooltip_id = reg.create_frame("Tooltip", None);
    if let Some(f) = reg.get_mut(tooltip_id) {
        f.width = 200.0;
        f.height = 80.0;
        f.strata = FrameStrata::Tooltip;
        f.background_color = Some([0.1, 0.1, 0.1, 0.9]);
        f.anchors.push(Anchor {
            point: AnchorPoint::Center,
            relative_to: None,
            relative_point: AnchorPoint::Center,
            x_offset: 0.0,
            y_offset: 0.0,
        });
    }
}

fn setup_action_bar(reg: &mut game_engine::ui::registry::FrameRegistry) {
    let bar_id = reg.create_frame("ActionBar", None);
    if let Some(f) = reg.get_mut(bar_id) {
        f.width = 500.0;
        f.height = 50.0;
        f.background_color = Some([0.15, 0.15, 0.15, 0.85]);
        f.anchors.push(Anchor {
            point: AnchorPoint::Bottom,
            relative_to: None,
            relative_point: AnchorPoint::Bottom,
            x_offset: 0.0,
            y_offset: -10.0,
        });
    }
}
