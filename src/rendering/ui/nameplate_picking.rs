//! Screen-space hit testing for the actual rendered nameplate parts.
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::{TextBounds, TextLayoutInfo};
use ui_toolkit::render::UiCamera;

use crate::networking::LocalPlayer;

#[derive(Component, Clone, Copy)]
pub(crate) struct NameplateHitTarget(pub Entity);

type PlateVisuals<'w, 's> = Query<
    'w,
    's,
    (
        &'static NameplateHitTarget,
        &'static GlobalTransform,
        &'static Visibility,
        &'static InheritedVisibility,
        &'static Anchor,
        Option<&'static Sprite>,
        Option<&'static TextLayoutInfo>,
        Option<&'static TextBounds>,
        Option<&'static TextColor>,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct NameplatePicker<'w, 's> {
    camera: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<UiCamera>>,
    visuals: PlateVisuals<'w, 's>,
    owners: Query<
        'w,
        's,
        (&'static GlobalTransform, &'static InheritedVisibility),
        Without<LocalPlayer>,
    >,
}

impl NameplatePicker<'_, '_> {
    pub fn pick(&self, cursor: Vec2, world_camera: Vec3) -> Option<Entity> {
        let (camera, camera_pose) = self.camera.single().ok()?;
        if !camera.is_active || !camera.logical_viewport_rect()?.contains(cursor) {
            return None;
        }
        let candidates = self.visuals.iter().filter_map(
            |(target, pose, visibility, inherited, anchor, sprite, text, bounds, text_color)| {
                if *visibility == Visibility::Hidden || !inherited.get() {
                    return None;
                }
                let (owner, owner_visibility) = self.owners.get(target.0).ok()?;
                if !owner_visibility.get() {
                    return None;
                }
                let size = visual_size(sprite, text, bounds, text_color)?;
                let rectangle = project_rectangle(camera, camera_pose, pose, *anchor, size)?;
                rectangle
                    .contains(cursor)
                    .then_some((owner.translation().distance_squared(world_camera), target.0))
            },
        );
        candidates
            .min_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then_with(|| a.1.to_bits().cmp(&b.1.to_bits()))
            })
            .map(|(_, owner)| owner)
    }
}

fn visual_size(
    sprite: Option<&Sprite>,
    text: Option<&TextLayoutInfo>,
    bounds: Option<&TextBounds>,
    text_color: Option<&TextColor>,
) -> Option<Vec2> {
    let size = if let Some(sprite) = sprite {
        if sprite.color.alpha() == 0.0 {
            return None;
        }
        sprite.custom_size?
    } else {
        if text_color?.0.alpha() == 0.0 {
            return None;
        }
        let text = text?;
        // Text2d layout already converts size to logical units; only glyphs retain DPI scaling.
        Vec2::new(
            bounds
                .and_then(|bounds| bounds.width)
                .unwrap_or(text.size.x),
            bounds
                .and_then(|bounds| bounds.height)
                .unwrap_or(text.size.y),
        )
    };
    (size.is_finite() && size.min_element() > 0.0).then_some(size)
}

fn project_rectangle(
    camera: &Camera,
    camera_pose: &GlobalTransform,
    pose: &GlobalTransform,
    anchor: Anchor,
    size: Vec2,
) -> Option<Rect> {
    let center = -anchor.as_vec() * size;
    let half = size / 2.0;
    let mut minimum = Vec2::splat(f32::INFINITY);
    let mut maximum = Vec2::splat(f32::NEG_INFINITY);
    for corner in [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(-half.x, half.y),
        Vec2::new(half.x, half.y),
    ] {
        let point = pose.transform_point((center + corner).extend(0.0));
        let pixel = camera.world_to_viewport(camera_pose, point).ok()?;
        minimum = minimum.min(pixel);
        maximum = maximum.max(pixel);
    }
    Some(Rect::from_corners(minimum, maximum))
}
