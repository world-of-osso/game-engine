//! Floating combat text renderer: overlay labels above each unit's `FloatingCombatTextStack`,
//! rising and fading with the stack's timing. Labels are owned by the unit, so they despawn
//! with it; labels beyond the live text count are despawned when texts expire.

use bevy::camera::visibility::{RenderLayers, VisibilitySystems};
use bevy::prelude::*;
use bevy::sprite::Text2dShadow;
use bevy::transform::TransformSystems;
use game_engine::floating_combat_text::{FloatingCombatText, FloatingCombatTextStack};
use ui_toolkit::render::{UI_RENDER_LAYER, UiCamera};

use crate::game::inworld_scene_stage::{InWorldSceneStage, inworld_scene_stage_allows_ui};
use crate::rendering::nameplate::{
    NameplateFontAssets, nameplate_state_active, viewport_to_overlay,
};

/// World-space height above the unit origin where texts spawn (above the nameplate).
const ANCHOR_HEIGHT: f32 = 3.5;
const FONT_SIZE: f32 = 20.0;

pub struct FloatingCombatTextPlugin;

impl Plugin for FloatingCombatTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, tick_combat_text.run_if(nameplate_state_active));
        app.add_systems(
            PostUpdate,
            draw_combat_text
                .after(TransformSystems::Propagate)
                .after(VisibilitySystems::VisibilityPropagate)
                .before(VisibilitySystems::CheckVisibility)
                .run_if(nameplate_state_active),
        );
    }
}

#[derive(Component)]
struct CombatTextLabel;

#[derive(Component)]
#[relationship(relationship_target = CombatTextLabels)]
struct CombatTextLabelOwner(Entity);

#[derive(Component)]
#[relationship_target(relationship = CombatTextLabelOwner, linked_spawn)]
struct CombatTextLabels(Vec<Entity>);

fn tick_combat_text(time: Res<Time>, mut stacks: Query<&mut FloatingCombatTextStack>) {
    let dt = time.delta_secs();
    for mut stack in &mut stacks {
        if stack.active_count() > 0 {
            stack.tick(dt);
        }
    }
}

type LabelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut TextFont,
        &'static mut TextColor,
        &'static mut Transform,
        &'static mut GlobalTransform,
        &'static mut Visibility,
    ),
    (With<CombatTextLabel>, Without<FloatingCombatTextStack>),
>;
type CameraQuery<'w, 's, M> =
    Query<'w, 's, (&'static Camera, &'static GlobalTransform), (With<M>, Without<CombatTextLabel>)>;

fn draw_combat_text(
    mut commands: Commands,
    owners: Query<(
        Entity,
        &FloatingCombatTextStack,
        &GlobalTransform,
        &InheritedVisibility,
        Option<&CombatTextLabels>,
    )>,
    mut labels: LabelQuery,
    world_camera: CameraQuery<Camera3d>,
    overlay_camera: CameraQuery<UiCamera>,
    mut art: NameplateFontAssets,
    stage: Option<Res<InWorldSceneStage>>,
    disabled: Option<Res<crate::client_options::UiDisabled>>,
) {
    let enabled = inworld_scene_stage_allows_ui(stage, disabled);
    for (owner, stack, global, inherited, owned) in &owners {
        let anchor = (enabled && inherited.get())
            .then(|| project_anchor(global, &world_camera, &overlay_camera))
            .flatten();
        let existing = owned.map_or(&[][..], |owned| owned.0.as_slice());
        for (index, text) in stack.texts.iter().enumerate() {
            let placement = anchor.map(|anchor| label_position(anchor, text));
            match existing.get(index) {
                Some(&label) => {
                    if let Ok(parts) = labels.get_mut(label) {
                        update_label(parts, text, placement);
                    }
                }
                None => spawn_label(&mut commands, owner, text, placement, art.load_font()),
            }
        }
        for &surplus in existing.iter().skip(stack.texts.len()) {
            commands.entity(surplus).despawn();
        }
    }
}

fn project_anchor(
    owner: &GlobalTransform,
    world_camera: &CameraQuery<Camera3d>,
    overlay_camera: &CameraQuery<UiCamera>,
) -> Option<Vec2> {
    let (world_camera, world_transform) = world_camera.single().ok()?;
    let (overlay_camera, overlay_transform) = overlay_camera.single().ok()?;
    if !world_camera.is_active || !overlay_camera.is_active {
        return None;
    }
    let point = owner.translation() + Vec3::Y * ANCHOR_HEIGHT;
    let viewport = world_camera
        .world_to_viewport(world_transform, point)
        .ok()?;
    viewport_to_overlay(world_camera, overlay_camera, overlay_transform, viewport)
}

/// Overlay space is y-up, so the rise offset is added.
fn label_position(anchor: Vec2, text: &FloatingCombatText) -> Vec2 {
    anchor + Vec2::new(text.x_offset, text.y_offset())
}

fn label_color(text: &FloatingCombatText) -> Color {
    let [r, g, b, a] = text.kind.color();
    Color::srgba(r, g, b, a * text.alpha())
}

fn label_font_size(text: &FloatingCombatText) -> FontSize {
    FontSize::Px(FONT_SIZE * text.font_scale())
}

fn label_visibility(placement: Option<Vec2>) -> Visibility {
    if placement.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn spawn_label(
    commands: &mut Commands,
    owner: Entity,
    text: &FloatingCombatText,
    placement: Option<Vec2>,
    font: Handle<Font>,
) {
    let transform = Transform::from_translation(placement.unwrap_or_default().extend(2.0));
    commands.spawn((
        CombatTextLabel,
        CombatTextLabelOwner(owner),
        Name::new("FloatingCombatText"),
        RenderLayers::layer(UI_RENDER_LAYER),
        Text2d::new(text.display_text()),
        TextFont {
            font_size: label_font_size(text),
            font: font.into(),
            ..default()
        },
        TextColor(label_color(text)),
        Text2dShadow {
            offset: Vec2::new(1.0, -1.0),
            color: Color::BLACK,
        },
        transform,
        GlobalTransform::from(transform),
        label_visibility(placement),
    ));
}

fn update_label(
    (mut label, mut font, mut color, mut transform, mut global, mut visibility): (
        Mut<Text2d>,
        Mut<TextFont>,
        Mut<TextColor>,
        Mut<Transform>,
        Mut<GlobalTransform>,
        Mut<Visibility>,
    ),
    text: &FloatingCombatText,
    placement: Option<Vec2>,
) {
    visibility.set_if_neq(label_visibility(placement));
    let Some(position) = placement else {
        return;
    };
    let display = text.display_text();
    if label.0 != display {
        label.0 = display;
    }
    let size = label_font_size(text);
    if font.font_size != size {
        font.font_size = size;
    }
    color.set_if_neq(TextColor(label_color(text)));
    let desired = Transform::from_translation(position.extend(2.0));
    transform.set_if_neq(desired);
    // Unparented overlay roots: propagation already ran this frame.
    global.set_if_neq(GlobalTransform::from(desired));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::nameplate::tests::{app_with_cameras, settle, visible};
    use bevy::time::TimeUpdateStrategy;
    use game_engine::floating_combat_text::CombatTextKind;
    use std::time::Duration;

    fn fct_app() -> App {
        let (mut app, _) = app_with_cameras(1.0);
        // The camera fixture already finished plugin setup.
        FloatingCombatTextPlugin.build(&mut app);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app
    }

    fn unit_with_texts(app: &mut App, texts: Vec<FloatingCombatText>) -> Entity {
        let mut stack = FloatingCombatTextStack::default();
        for text in texts {
            stack.push(text);
        }
        app.world_mut()
            .spawn((
                Transform::from_xyz(100.0, 0.0, 0.0),
                Visibility::Visible,
                stack,
            ))
            .id()
    }

    fn labels_of(app: &App, owner: Entity) -> Vec<Entity> {
        app.world()
            .get::<CombatTextLabels>(owner)
            .map(|labels| labels.0.clone())
            .unwrap_or_default()
    }

    #[test]
    fn crit_spell_text_draws_large_yellow_above_unit_and_rises() {
        let mut app = fct_app();
        let owner = unit_with_texts(
            &mut app,
            vec![FloatingCombatText::critical(
                CombatTextKind::SpellDamage,
                1234,
            )],
        );
        settle(&mut app);
        let labels = labels_of(&app, owner);
        assert_eq!(labels.len(), 1);
        let label = labels[0];
        assert!(visible(&app, label));
        assert_eq!(app.world().get::<Text2d>(label).unwrap().0, "1234");
        assert_eq!(
            app.world().get::<TextFont>(label).unwrap().font_size,
            FontSize::Px(FONT_SIZE * 1.5)
        );
        let color = app.world().get::<TextColor>(label).unwrap().0.to_srgba();
        assert_eq!((color.red, color.green, color.blue), (1.0, 1.0, 0.0));
        let first_y = app.world().get::<Transform>(label).unwrap().translation.y;
        app.update();
        let later_y = app.world().get::<Transform>(label).unwrap().translation.y;
        assert!(later_y > first_y, "{later_y} should rise above {first_y}");
    }

    #[test]
    fn expired_texts_despawn_labels_and_unit_despawn_removes_them() {
        let mut app = fct_app();
        let owner = unit_with_texts(
            &mut app,
            vec![
                FloatingCombatText::new(CombatTextKind::PhysicalDamage, 50),
                FloatingCombatText::new(CombatTextKind::Heal, 70),
            ],
        );
        settle(&mut app);
        let labels = labels_of(&app, owner);
        assert_eq!(labels.len(), 2);
        let heal = labels[1];
        let heal_color = app.world().get::<TextColor>(heal).unwrap().0.to_srgba();
        assert_eq!((heal_color.red, heal_color.green), (0.0, 1.0));
        // Lifetime 1.5 s at 0.1 s per frame.
        for _ in 0..17 {
            app.update();
        }
        assert!(labels_of(&app, owner).is_empty());
        assert!(labels.iter().all(|&l| app.world().get_entity(l).is_err()));

        app.world_mut()
            .get_mut::<FloatingCombatTextStack>(owner)
            .unwrap()
            .push(FloatingCombatText::new(CombatTextKind::Miss, 0));
        settle(&mut app);
        let miss = labels_of(&app, owner)[0];
        assert_eq!(app.world().get::<Text2d>(miss).unwrap().0, "Miss");
        app.world_mut().despawn(owner);
        assert!(app.world().get_entity(miss).is_err());
    }

    #[test]
    fn off_screen_unit_hides_its_text() {
        let mut app = fct_app();
        let owner = unit_with_texts(
            &mut app,
            vec![FloatingCombatText::new(CombatTextKind::PhysicalDamage, 9)],
        );
        settle(&mut app);
        let label = labels_of(&app, owner)[0];
        assert!(visible(&app, label));
        app.world_mut()
            .get_mut::<Transform>(owner)
            .unwrap()
            .translation
            .x = -5000.0;
        settle(&mut app);
        assert!(!visible(&app, label));
    }
}
