use bevy::prelude::*;
use shared::components::Health;

use crate::client_options::HudVisibilityToggles;
use crate::game_state::GameState;

pub struct HealthBarPlugin;

impl Plugin for HealthBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spawn_health_bars);
        app.add_systems(
            Update,
            (
                sync_health_bar_visibility,
                update_health_bars,
                billboard_health_bars,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

/// Marker for the health bar root entity (parent of background + foreground).
#[derive(Component)]
struct HealthBar;

/// Marker for the foreground (colored) bar quad.
#[derive(Component)]
struct HealthBarForeground;

const BAR_WIDTH: f32 = 1.0;
const BAR_HEIGHT: f32 = 0.1;
const BAR_Y_OFFSET: f32 = 2.5;

/// Compute the health bar color based on current/max HP.
pub fn health_bar_color(current: f32, max: f32) -> Color {
    let pct = if max > 0.0 {
        (current / max).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (r, g, b) = health_pct_to_rgb(pct);
    Color::srgb(r, g, b)
}

fn health_pct_to_rgb(pct: f32) -> (f32, f32, f32) {
    if pct >= 0.6 {
        // Green to yellow: 100% -> 60%
        let t = (pct - 0.6) / 0.4; // 1.0 at 100%, 0.0 at 60%
        let r = lerp(0.8, 0.0, t);
        (r, 0.8, 0.0)
    } else if pct >= 0.3 {
        // Yellow to red: 60% -> 30%
        let t = (pct - 0.3) / 0.3; // 1.0 at 60%, 0.0 at 30%
        let g = lerp(0.0, 0.8, t);
        (0.8, g, 0.0)
    } else {
        (0.8, 0.0, 0.0)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Observer: when Health is added to an entity with Transform, spawn a health bar child.
fn spawn_health_bars(
    trigger: On<Add, Health>,
    mut commands: Commands,
    query: Query<&Health>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let entity = trigger.entity;
    let Ok(health) = query.get(entity) else {
        return;
    };
    let pct = health_pct(health);
    let (bg_mesh, fg_mesh) = create_bar_meshes(&mut meshes);
    let (bg_material, fg_material) = create_bar_materials(&mut materials, health);
    let bar_root = spawn_bar_entity(
        &mut commands,
        bg_mesh,
        fg_mesh,
        bg_material,
        fg_material,
        pct,
    );
    commands.entity(entity).add_child(bar_root);
}

fn health_pct(health: &Health) -> f32 {
    if health.max > 0.0 {
        (health.current / health.max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn create_bar_meshes(meshes: &mut Assets<Mesh>) -> (Handle<Mesh>, Handle<Mesh>) {
    let half = Vec2::new(BAR_WIDTH / 2.0, BAR_HEIGHT / 2.0);
    let bg = meshes.add(Plane3d::new(Vec3::Z, half));
    let fg = meshes.add(Plane3d::new(Vec3::Z, half));
    (bg, fg)
}

fn create_bar_materials(
    materials: &mut Assets<StandardMaterial>,
    health: &Health,
) -> (Handle<StandardMaterial>, Handle<StandardMaterial>) {
    let bg = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.2),
        unlit: true,
        ..default()
    });
    let fg = materials.add(StandardMaterial {
        base_color: health_bar_color(health.current, health.max),
        unlit: true,
        ..default()
    });
    (bg, fg)
}

fn spawn_bar_entity(
    commands: &mut Commands,
    bg_mesh: Handle<Mesh>,
    fg_mesh: Handle<Mesh>,
    bg_material: Handle<StandardMaterial>,
    fg_material: Handle<StandardMaterial>,
    pct: f32,
) -> Entity {
    commands
        .spawn((
            HealthBar,
            Transform::from_xyz(0.0, BAR_Y_OFFSET, 0.0),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(bg_mesh),
                MeshMaterial3d(bg_material),
                Transform::from_xyz(0.0, 0.0, -0.001),
            ));
            parent.spawn((
                HealthBarForeground,
                Mesh3d(fg_mesh),
                MeshMaterial3d(fg_material),
                foreground_transform(pct),
            ));
        })
        .id()
}

/// Build the foreground bar transform: scale X by pct, shift left to keep left-aligned.
fn foreground_transform(pct: f32) -> Transform {
    let offset_x = -BAR_WIDTH * (1.0 - pct) / 2.0;
    Transform::from_xyz(offset_x, 0.0, 0.0).with_scale(Vec3::new(pct, 1.0, 1.0))
}

/// Update health bar foreground width and color when Health changes.
fn update_health_bars(
    health_query: Query<(&Health, &Children), Changed<Health>>,
    bar_query: Query<&Children, With<HealthBar>>,
    mut fg_query: Query<&mut Transform, With<HealthBarForeground>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mat_query: Query<&MeshMaterial3d<StandardMaterial>, With<HealthBarForeground>>,
) {
    for (health, entity_children) in health_query.iter() {
        let pct = health_pct(health);
        for child in entity_children.iter() {
            let Ok(bar_children) = bar_query.get(child) else {
                continue;
            };
            update_foreground(
                bar_children,
                pct,
                health,
                &mut fg_query,
                &mut materials,
                &mat_query,
            );
        }
    }
}

fn update_foreground(
    bar_children: &Children,
    pct: f32,
    health: &Health,
    fg_query: &mut Query<&mut Transform, With<HealthBarForeground>>,
    materials: &mut Assets<StandardMaterial>,
    mat_query: &Query<&MeshMaterial3d<StandardMaterial>, With<HealthBarForeground>>,
) {
    for bar_child in bar_children.iter() {
        if let Ok(mut fg_transform) = fg_query.get_mut(bar_child) {
            *fg_transform = foreground_transform(pct);
        }
        if let Ok(mat_handle) = mat_query.get(bar_child)
            && let Some(mat) = materials.get_mut(&mat_handle.0)
        {
            mat.base_color = health_bar_color(health.current, health.max);
        }
    }
}

/// Rotate health bars to always face the camera (billboard effect).
fn billboard_health_bars(
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    mut bar_query: Query<&mut Transform, With<HealthBar>>,
) {
    let Ok(camera_global) = camera_query.single() else {
        return;
    };
    let camera_pos = camera_global.translation();
    for mut transform in bar_query.iter_mut() {
        let dir = camera_pos - transform.translation;
        if dir.length_squared() > 0.001 {
            transform.look_to(Dir3::new(dir).unwrap_or(Dir3::Z), Dir3::Y);
        }
    }
}

fn sync_health_bar_visibility(
    hud_visibility: Option<Res<HudVisibilityToggles>>,
    mut query: Query<&mut Visibility, With<HealthBar>>,
) {
    let visible = hud_visibility.is_none_or(|toggles| toggles.show_health_bars);
    for mut visibility in &mut query {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_color_full_hp() {
        let c = health_bar_color(100.0, 100.0).to_srgba();
        assert!(c.red.abs() < 1e-4, "red should be ~0, got {}", c.red);
        assert!((c.green - 0.8).abs() < 1e-4);
        assert!(c.blue.abs() < 1e-4);
    }

    #[test]
    fn test_health_color_low_hp() {
        let color = health_bar_color(20.0, 100.0);
        assert_eq!(color, Color::srgb(0.8, 0.0, 0.0));
    }

    #[test]
    fn test_health_color_mid_hp() {
        let color = health_bar_color(50.0, 100.0);
        // 50% is in the 60-30% range: t = (0.5 - 0.3) / 0.3 = 0.667
        let expected_g = 0.0 + (0.8 - 0.0) * ((50.0 / 100.0 - 0.3) / 0.3);
        let c = color.to_srgba();
        assert!((c.red - 0.8).abs() < 1e-4);
        assert!((c.green - expected_g).abs() < 1e-4);
        assert!((c.blue - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_bar_width_scales_with_health() {
        let transform = foreground_transform(0.5);
        assert!((transform.scale.x - 0.5).abs() < 1e-6);
        assert!((transform.translation.x - (-0.25)).abs() < 1e-6);
    }

    #[test]
    fn test_health_color_at_60_boundary() {
        let color = health_bar_color(60.0, 100.0);
        assert_eq!(color, Color::srgb(0.8, 0.8, 0.0));
    }

    #[test]
    fn test_health_color_at_30_boundary() {
        let color = health_bar_color(30.0, 100.0);
        assert_eq!(color, Color::srgb(0.8, 0.0, 0.0));
    }

    #[test]
    fn test_health_color_zero_max() {
        let color = health_bar_color(0.0, 0.0);
        assert_eq!(color, Color::srgb(0.8, 0.0, 0.0));
    }
}
