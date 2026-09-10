//! Login caret geometry comes from the same shaped layout that renders field text.

use std::hash::{Hash, Hasher};

use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::text::ComputedTextBlock;
use parley::editing::{Affinity, Cursor};

use super::form::LoginFieldId;
use super::native::LoginSession;

#[cfg(test)]
#[path = "native_caret_tests.rs"]
mod tests;

const CARET_WIDTH: f32 = 2.0;
const BLINK_INTERVAL: f64 = 0.5;

#[derive(Clone, Copy, PartialEq, Eq)]
struct EditSnapshot {
    focused: bool,
    cursor: usize,
    text_hash: u64,
}

#[derive(Component)]
struct LoginCaret {
    text: Entity,
    field: LoginFieldId,
    last_edit: Option<EditSnapshot>,
    blink_started: f64,
}

/// Parent the overlay to the existing field clip, after its text child.
pub(super) fn spawn_login_caret(
    commands: &mut Commands,
    text: Entity,
    clip: Entity,
    field: LoginFieldId,
) -> Entity {
    commands
        .spawn((
            Name::new(match field {
                LoginFieldId::Username => "UsernameCaret",
                LoginFieldId::Password => "PasswordCaret",
            }),
            Node {
                position_type: PositionType::Absolute,
                width: px(CARET_WIDTH),
                height: px(20),
                ..default()
            },
            BackgroundColor(Color::NONE),
            bevy::ui::FocusPolicy::Pass,
            bevy::picking::Pickable::IGNORE,
            LoginCaret {
                text,
                field,
                last_edit: None,
                blink_started: 0.0,
            },
            ChildOf(clip),
        ))
        .id()
}

/// Run after `UiSystems::PostLayout`: both shaping and inherited clipping are ready.
/// Update computed geometry directly so cursor movement does not wait another layout frame.
pub(super) fn sync_login_carets(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let modal = world.contains_resource::<crate::scenes::game_menu::UiModalOpen>();
    let carets: Vec<_> = world
        .query::<(Entity, &LoginCaret)>()
        .iter(world)
        .map(|(entity, caret)| (entity, caret.text, caret.field))
        .collect();
    for (entity, text, field) in carets {
        let Some(snapshot) = edit_snapshot(world, field, modal) else {
            world.get_mut::<BackgroundColor>(entity).unwrap().0 = Color::NONE;
            continue;
        };
        let visible = {
            let mut caret = world.get_mut::<LoginCaret>(entity).unwrap();
            if caret.last_edit != Some(snapshot) {
                caret.last_edit = Some(snapshot);
                caret.blink_started = now;
            }
            snapshot.focused
                && (now - caret.blink_started).rem_euclid(2.0 * BLINK_INTERVAL) < BLINK_INTERVAL
        };
        let geometry = caret_geometry(world, text, snapshot.cursor);
        let color = world.get::<TextColor>(text).map(|color| color.0);
        world.get_mut::<BackgroundColor>(entity).unwrap().0 =
            match (visible, geometry.is_some(), color) {
                (true, true, Some(color)) => color,
                _ => Color::NONE,
            };
        if let Some((transform, size, inverse_scale_factor)) = geometry {
            *world.get_mut::<UiGlobalTransform>(entity).unwrap() = transform;
            let mut node = world.get_mut::<ComputedNode>(entity).unwrap();
            node.size = size;
            node.unrounded_size = size;
            node.inverse_scale_factor = inverse_scale_factor;
        }
    }
}

fn edit_snapshot(world: &World, field: LoginFieldId, modal: bool) -> Option<EditSnapshot> {
    let session = world.get_resource::<LoginSession>()?;
    let value = session.form.field(field);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.text.hash(&mut hasher);
    Some(EditSnapshot {
        focused: !modal && session.focus == Some(field),
        // Password presentation has one ASCII '*' per raw byte, so this is also
        // the displayed-text byte offset. Username offsets already index UTF-8.
        cursor: value.cursor_position,
        text_hash: hasher.finish(),
    })
}

fn caret_geometry(
    world: &World,
    text: Entity,
    byte_index: usize,
) -> Option<(UiGlobalTransform, Vec2, f32)> {
    let block = world.get::<ComputedTextBlock>(text)?;
    let node = world.get::<ComputedNode>(text)?;
    let transform = world.get::<UiGlobalTransform>(text)?;
    let scale = node.inverse_scale_factor.recip();
    let width = CARET_WIDTH * scale;
    let rect = if world.get::<Text>(text)?.0.is_empty() {
        // Empty text has no shaped line. Its origin is exact; use the declared
        // font height, centered on that origin, without inventing glyph advances.
        let FontSize::Px(height) = world.get::<TextFont>(text)?.font_size else {
            return None;
        };
        Rect::from_corners(
            Vec2::new(0.0, -height * scale / 2.0),
            Vec2::new(width, height * scale / 2.0),
        )
    } else {
        let layout = block.buffer();
        if layout.is_empty() {
            return None;
        }
        let bounds = Cursor::from_byte_index(layout, byte_index, Affinity::Downstream)
            .geometry(layout, width);
        Rect::from_corners(
            Vec2::new(bounds.x0 as f32, bounds.y0 as f32),
            Vec2::new(bounds.x1 as f32, bounds.y1 as f32),
        )
    };
    let offset = rect.center() - node.size / 2.0;
    Some((
        UiGlobalTransform::from(Affine2::from(transform) * Affine2::from_translation(offset)),
        rect.size(),
        node.inverse_scale_factor,
    ))
}
