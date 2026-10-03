//! Retail soft interact (`SoftTargetInteract`): the interactable unit or game object the
//! player faces, which the `INTERACTTARGET` binding uses (`InteractUnit("anyinteract")`,
//! Bindings_Standard.xml:1171-1173) and whose nameplate shows the unit's cursor icon
//! (`SoftTargetFrame`, Blizzard_NamePlates.xml:286-296).
//!
//! Default CVars (wow-ui-sim `cvars.yaml`, the values the Kiosk keyboard reset restores in
//! Blizzard_Gamepad/Core.lua:83-88): `SoftTargetInteractArc` 0 ("No yaw arc allowance, must
//! be directly in front"), `SoftTargetInteractRange` 10 ("limited to tab targeting and
//! individual interact ranges"), `SoftTargetIconInteract` 1, `SoftTargetIconGameObject` 0,
//! `SoftTargetLowPriorityIcons` 0. They live in `hud.softTarget` of the options file; the
//! Accessibility "Interact Key Icons" choice sets the icon ones.

use game_engine_core::input_bindings_data::InputAction;
use game_engine_core::soft_target_data::{SoftTargetArc, SoftTargetOptions};
use game_engine_session::SessionScreen;
use game_engine_ui_model::wow_cursor_data::ActiveWowCursor;
use godot::classes::{
    Camera3D, CanvasLayer, Control, TextureRect, control::MouseFilter, texture_rect::ExpandMode,
    texture_rect::StretchMode,
};
use godot::prelude::*;
use shared::components::Npc;
use shared::protocol::GameObjectInfo;

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::game_objects::game_object_cursor;
use crate::nameplates::plate_anchor;

/// Every interaction the client sends (NPC, mailbox, vault, chair) is limited to
/// TrinityCore `INTERACTION_DISTANCE` (ObjectDefines.h:24): `SoftTargetInteractRange` is
/// "limited to tab targeting and individual interact ranges".
const INTERACTION_DISTANCE: f32 = 5.0;
/// Width of "directly in front" for a unit: TrinityCore `HasInLine` widened by the
/// target's size (Position.cpp:192-200), a creature's `DEFAULT_PLAYER_COMBAT_REACH`
/// (ObjectDefines.h:40). The arc-0 width is not published; this is the server's model.
const UNIT_LINE_WIDTH: f32 = 1.5;
/// `SoftTargetNameplateSize` default, in pixels.
const ICON_SIZE: f32 = 19.0;
/// `SoftTargetFrame` BOTTOM to the plate's TOP at `y="-8"` (Blizzard_NamePlates.xml:288-290).
const ICON_PLATE_OVERLAP: f32 = 8.0;

/// `SoftTargetWorldtextSize` default, in pixels.
const WORLD_TEXT_ICON_SIZE: f32 = 32.0;
/// `SoftTargetWorldtextNearScale` default: the scale within `SoftTargetWorldtextNearDist`
/// (4 yd). Scaling between that and `SoftTargetWorldtextFarDist` (40 yd) is unpublished
/// and not applied; interaction range ends at 5 yd.
const WORLD_TEXT_NEAR_SCALE: f32 = 1.0;

/// A game object's size: `DEFAULT_PLAYER_BOUNDING_RADIUS`, "also currently used for any
/// non Unit world objects" (ObjectDefines.h:39).
const OBJECT_LINE_WIDTH: f32 = 0.389;

/// What a soft interact candidate is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SoftKind {
    /// A unit and the cursor it shows under the pointer.
    Unit(ActiveWowCursor),
    /// A game object the client can use (mailbox, Guild Vault, chair) and its cursor.
    GameObject(ActiveWowCursor),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SoftCandidate {
    pub id: u64,
    pub position: Vector3,
    pub kind: SoftKind,
}

/// The nearest interactable candidate within `SoftTargetInteractArc` of the player at `feet`
/// facing the horizontal unit vector `forward`, and within interact range.
pub(crate) fn soft_interact_target(
    feet: Vector3,
    forward: Vector3,
    candidates: &[SoftCandidate],
    options: &SoftTargetOptions,
) -> Option<u64> {
    let range = options.interact_range.min(INTERACTION_DISTANCE);
    candidates
        .iter()
        .filter(|candidate| {
            interactable(candidate.kind) && in_arc(options.interact_arc, feet, forward, candidate)
        })
        .map(|candidate| (feet.distance_to(candidate.position), candidate.id))
        .filter(|(distance, _)| *distance <= range)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// Units the interact key can use: not enemies (`Attack`) nor units with nothing to offer.
fn interactable(kind: SoftKind) -> bool {
    match kind {
        SoftKind::Unit(cursor) => {
            !matches!(cursor, ActiveWowCursor::Default | ActiveWowCursor::Attack)
        }
        SoftKind::GameObject(_) => true,
    }
}

/// `SoftTargetInteractArc`. Retail publishes only the CVar help text, no angles:
/// - 0, "directly in front": TrinityCore `HasInLine` with no extra width, the front half
///   (`HasInArc(M_PI)`) and the facing line within the target's size.
/// - 1, "in front yaw arc": TrinityCore's in-front test, `isInFront` with its default arc
///   `M_PI` (Object.h:371), the front half.
/// - 2, "anywhere in targeting area": no direction test.
fn in_arc(arc: SoftTargetArc, feet: Vector3, forward: Vector3, candidate: &SoftCandidate) -> bool {
    let offset = candidate.position - feet;
    let ground = Vector3::new(offset.x, 0.0, offset.z);
    let along = ground.dot(forward);
    let width = match candidate.kind {
        SoftKind::Unit(_) => UNIT_LINE_WIDTH,
        SoftKind::GameObject(_) => OBJECT_LINE_WIDTH,
    };
    match arc {
        SoftTargetArc::DirectlyInFront => {
            along >= 0.0 && (ground - forward * along).length() < width
        }
        SoftTargetArc::InFront => along >= 0.0,
        SoftTargetArc::Anywhere => true,
    }
}

/// The icon above the soft interact target, set by the Interact Key Icons CVars:
/// - a unit's cursor art (`SetUnitCursorTexture`) with `SoftTargetIconInteract`;
/// - a lootable corpse's only with `SoftTargetLowPriorityIcons` too, "Show interact icons
///   even when there is other visual indicators, such as quest or loot effects";
/// - a game object's cursor with `SoftTargetIconGameObject`, "Show icon for soft interact
///   game objects (interactable objects you cannot normally target)" (Wow.exe CVar help).
pub(crate) fn soft_target_icon(
    kind: SoftKind,
    options: &SoftTargetOptions,
) -> Option<ActiveWowCursor> {
    let shown = match kind {
        SoftKind::GameObject(_) => options.icon_game_object,
        SoftKind::Unit(ActiveWowCursor::Loot) => {
            options.icon_interact && options.low_priority_icons
        }
        SoftKind::Unit(_) => options.icon_interact,
    };
    let (SoftKind::GameObject(cursor) | SoftKind::Unit(cursor)) = kind;
    shown.then_some(cursor)
}

/// The icon's size in pixels: `SoftTargetNameplateSize` on a unit's plate, and the world
/// text icon's `SoftTargetWorldtextSize` at `SoftTargetWorldtextNearScale` over a game
/// object, which has no plate.
pub(crate) fn soft_icon_size(kind: SoftKind) -> f32 {
    match kind {
        SoftKind::Unit(_) => ICON_SIZE,
        SoftKind::GameObject(_) => WORLD_TEXT_ICON_SIZE * WORLD_TEXT_NEAR_SCALE,
    }
}

/// The soft interact target and its icon node.
#[derive(Default)]
pub(crate) struct SoftInteract {
    target: Option<(u64, SoftKind)>,
    icon: Option<Gd<TextureRect>>,
}

impl GameClient {
    /// Per frame after the merchant's right-click: pick the soft interact target, and
    /// `INTERACTTARGET` interacts with it.
    pub(super) fn update_soft_interact(&mut self) -> Result<(), FrameError> {
        let enabled = self.account.session.screen == SessionScreen::InWorld
            && self.client_options.hud.soft_target_interact;
        self.soft_interact.target = if enabled {
            self.pick_soft_interact()
        } else {
            None
        };
        let Some((id, _)) = self.soft_interact.target else {
            return Ok(());
        };
        let input = self.physical_input.gameplay_state(self.keyboard_free());
        let pressed = self
            .client_options
            .bindings
            .is_just_pressed(InputAction::InteractTarget, &input);
        if pressed && self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.interact_unit(id)?;
        }
        Ok(())
    }

    fn pick_soft_interact(&mut self) -> Option<(u64, SoftKind)> {
        let feet = self.world.local_player_transform()?.origin;
        let yaw = self.world.local_player_facing()?;
        let player = self.world.local_player_id();
        let ids: Vec<u64> = self
            .replica
            .units()
            .map(|unit| unit.server_id)
            .filter(|id| Some(*id) != player)
            .collect();
        let candidates: Vec<SoftCandidate> = ids
            .into_iter()
            .filter_map(|id| self.soft_candidate(id))
            .collect();
        let forward = Vector3::new(yaw.sin(), 0.0, yaw.cos());
        let options = self.client_options.hud.soft_target;
        let id = soft_interact_target(feet, forward, &candidates, &options)?;
        candidates
            .iter()
            .find(|candidate| candidate.id == id)
            .map(|candidate| (id, candidate.kind))
    }

    /// A drawn unit with its cursor, or a game object the client can use.
    fn soft_candidate(&mut self, id: u64) -> Option<SoftCandidate> {
        let unit = self.replica.unit(id)?;
        if let Some(info) = unit.get::<GameObjectInfo>() {
            return Some(SoftCandidate {
                id,
                position: self.game_objects.position(id)?,
                kind: SoftKind::GameObject(game_object_cursor(info.go_type)?),
            });
        }
        if !unit.has::<Npc>() {
            return None;
        }
        let position = self.world.unit_node(id)?.get_global_position();
        Some(SoftCandidate {
            id,
            position,
            kind: SoftKind::Unit(self.unit_cursor(id)?),
        })
    }

    /// Per frame after the nameplates: the soft target's icon over its plate, or where the
    /// plate would sit when the unit shows none.
    pub(super) fn sync_soft_interact_icon(&mut self) -> Result<(), FrameError> {
        let placed = self.soft_icon_placement();
        let texture = placed.and_then(|(_, cursor, _)| self.cursor_texture(cursor));
        let (Some((bottom, _, size)), Some(texture)) = (placed, texture) else {
            if let Some(icon) = self.soft_interact.icon.as_mut() {
                icon.set_visible(false);
            }
            return Ok(());
        };
        let icon = self.soft_interact_icon_node();
        icon.set_texture(&texture);
        icon.set_size(Vector2::splat(size));
        icon.set_position(bottom - Vector2::new(size / 2.0, size));
        icon.set_visible(true);
        Ok(())
    }

    /// The icon's bottom-centre on screen, its cursor and its size.
    fn soft_icon_placement(&self) -> Option<(Vector2, ActiveWowCursor, f32)> {
        let (id, kind) = self.soft_interact.target?;
        let cursor = soft_target_icon(kind, &self.client_options.hud.soft_target)?;
        let camera = self.base().get_viewport()?.get_camera_3d()?;
        let bottom = match kind {
            // The world text icon stands on top of the object's model bounds.
            SoftKind::GameObject(_) => {
                let anchor = self.game_objects.icon_anchor(id)?;
                if !camera.is_position_in_frustum(anchor) {
                    return None;
                }
                camera.unproject_position(anchor)
            }
            SoftKind::Unit(_) => self.unit_icon_bottom(&camera, id)?,
        };
        Some((bottom, cursor, soft_icon_size(kind)))
    }

    /// `SoftTargetFrame` on the unit's plate, or at the plate anchor when it shows none.
    fn unit_icon_bottom(&self, camera: &Gd<Camera3D>, id: u64) -> Option<Vector2> {
        let anchor = plate_anchor(&self.world.unit_node(id)?);
        if !camera.is_position_in_frustum(anchor) {
            return None;
        }
        let point = camera.unproject_position(anchor);
        Some(
            match self
                .nameplates
                .plate_rects()
                .find(|(plate, _)| *plate == id)
            {
                Some((_, rect)) => Vector2::new(point.x, rect.position.y + ICON_PLATE_OVERLAP),
                None => point,
            },
        )
    }

    pub(super) fn soft_interact_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let Some((id, kind)) = self.soft_interact.target else {
            return state;
        };
        state.set("target", id as i64);
        if let Some(cursor) = soft_target_icon(kind, &self.client_options.hud.soft_target) {
            state.set("icon", format!("{cursor:?}").as_str());
        }
        if let Some(icon) = self
            .soft_interact
            .icon
            .as_ref()
            .filter(|icon| icon.is_visible())
        {
            state.set("icon_rect", icon.get_global_rect());
        }
        state
    }

    fn soft_interact_icon_node(&mut self) -> &mut Gd<TextureRect> {
        if self.soft_interact.icon.is_none() {
            let mut layer = CanvasLayer::new_alloc();
            layer.set_name("SoftTargetFrame");
            // With the nameplates, below the registry UI layers.
            layer.set_layer(0);
            let mut icon = TextureRect::new_alloc();
            icon.set_expand_mode(ExpandMode::IGNORE_SIZE);
            icon.set_stretch_mode(StretchMode::SCALE);
            icon.upcast_mut::<Control>()
                .set_mouse_filter(MouseFilter::IGNORE);
            layer.add_child(&icon);
            self.base_mut().add_child(&layer);
            self.soft_interact.icon = Some(icon);
        }
        self.soft_interact
            .icon
            .as_mut()
            .expect("icon created above")
    }
}

#[cfg(test)]
mod tests {
    use game_engine_ui_model::wow_cursor_data::{NpcCursorView, npc_cursor};
    use shared::faction_reaction::Reaction;
    use shared::protocol::NpcFlags;

    use super::*;
    use game_engine_core::soft_target_data::InteractKeyIcons;

    /// A WoW world point `(x, y, z)` in a frame isometric to the client's (ground plane
    /// X/Z, height Y); selection only measures distances and angles.
    fn at(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3::new(x, z, y)
    }

    /// The facing of WoW `orientation` radians in the frame of [`at`].
    fn facing(orientation: f32) -> Vector3 {
        Vector3::new(orientation.cos(), 0.0, orientation.sin())
    }

    fn npc(id: u64, npcflag: u64, reaction: Reaction, position: Vector3) -> SoftCandidate {
        let cursor = npc_cursor(NpcCursorView {
            flags: NpcFlags(npcflag),
            dead: false,
            lootable: false,
            reaction,
        });
        SoftCandidate {
            id,
            position,
            kind: SoftKind::Unit(cursor),
        }
    }

    // world.db Northshire Abbey vendors (creature guid, npcflag 384/4224, faction 12).
    const DANIL: u64 = 79950;
    const DERMOT: u64 = 79951;
    const GODRIC: u64 = 79952;
    // world.db Goldshire Mailbox (gameobject guid 26784, entry 142075, type 19).
    const MAILBOX: u64 = 26784;

    fn northshire_vendors() -> [SoftCandidate; 3] {
        [
            npc(
                DANIL,
                384,
                Reaction::Friendly,
                at(-8901.59, -112.716, 82.0314),
            ),
            npc(
                DERMOT,
                4224,
                Reaction::Friendly,
                at(-8897.71, -115.328, 81.9982),
            ),
            npc(
                GODRIC,
                4224,
                Reaction::Friendly,
                at(-8898.23, -119.838, 82.016),
            ),
        ]
    }

    fn goldshire_mailbox() -> SoftCandidate {
        SoftCandidate {
            id: MAILBOX,
            position: at(-9455.99, 45.8229, 56.4395),
            kind: SoftKind::GameObject(ActiveWowCursor::Mail),
        }
    }

    #[test]
    fn nearest_vendor_in_line_wins() {
        // Brother Danil 1.21 yd and Dermot Johns 4.63 yd ahead, both within 1.15 yd of the
        // facing line; Godric Rothgar is 7.15 yd away.
        let feet = at(-8902.09, -113.82, 82.03);
        let target = soft_interact_target(
            feet,
            facing(355f32.to_radians()),
            &northshire_vendors(),
            &retail(),
        );
        assert_eq!(target, Some(DANIL));
    }

    #[test]
    fn a_vendor_beside_the_player_loses_to_one_in_front() {
        // Brother Danil is 3.0 yd away but 2.88 yd off the facing line; Dermot Johns is
        // 4.8 yd straight ahead.
        let feet = at(-8902.5, -115.6, 82.03);
        let target = soft_interact_target(feet, facing(0.0), &northshire_vendors(), &retail());
        assert_eq!(target, Some(DERMOT));
    }

    #[test]
    fn nothing_behind_or_beyond_interact_range() {
        let feet = at(-8902.09, -113.82, 82.03);
        let behind = soft_interact_target(
            feet,
            facing(175f32.to_radians()),
            &northshire_vendors(),
            &retail(),
        );
        assert_eq!(behind, None);
        // Dermot Johns straight ahead at 6.79 yd: inside SoftTargetInteractRange 10 but
        // past the 5 yd interaction distance.
        let far = soft_interact_target(
            at(-8904.5, -115.33, 82.03),
            facing(0.0),
            &northshire_vendors(),
            &retail(),
        );
        assert_eq!(far, None);
    }

    #[test]
    fn hostile_and_serviceless_units_are_not_interact_targets() {
        // A hostile Kobold Vermin (Attack cursor) and a friendly NPC with no services
        // (pointer cursor) stand nearer on the line than Dermot Johns.
        let feet = at(-8902.5, -115.6, 82.03);
        let mut candidates = northshire_vendors().to_vec();
        candidates.push(npc(6, 0, Reaction::Hostile, at(-8900.5, -115.5, 82.0)));
        candidates.push(npc(2, 0, Reaction::Friendly, at(-8899.5, -115.5, 82.0)));
        let target = soft_interact_target(feet, facing(0.0), &candidates, &retail());
        assert_eq!(target, Some(DERMOT));
    }

    #[test]
    fn a_mailbox_must_be_directly_in_front() {
        let feet = at(-9459.0, 45.8229, 56.44);
        let mailbox = [goldshire_mailbox()];
        assert_eq!(
            soft_interact_target(feet, facing(0.0), &mailbox, &retail()),
            Some(MAILBOX)
        );
        // 3 yd ahead but 0.5 yd off the line, past the object's 0.389 yd size.
        let off_line = at(-9459.0, 45.3229, 56.44);
        assert_eq!(
            soft_interact_target(off_line, facing(0.0), &mailbox, &retail()),
            None
        );
    }

    /// Retail defaults: Interact Key Icons "NPCs Only (Default)".
    fn retail() -> SoftTargetOptions {
        SoftTargetOptions::default()
    }

    fn icons(choice: InteractKeyIcons) -> SoftTargetOptions {
        let mut options = SoftTargetOptions::default();
        options.set_interact_key_icons(choice);
        options
    }

    fn goldshire_chair() -> SoftKind {
        // world.db Goldshire Wooden Chair (gameobject guid 26246, type 7).
        SoftKind::GameObject(ActiveWowCursor::Interact)
    }

    #[test]
    fn default_icons_show_npc_cursors_only() {
        let [danil, dermot, _] = northshire_vendors();
        assert_eq!(
            soft_target_icon(danil.kind, &retail()),
            Some(ActiveWowCursor::Buy)
        );
        assert_eq!(
            soft_target_icon(dermot.kind, &retail()),
            Some(ActiveWowCursor::Buy)
        );
        // Marshal McBride (npcflag 3: gossip + quest giver) shows the speak bubble.
        let mcbride = npc(79970, 3, Reaction::Friendly, Vector3::ZERO);
        assert_eq!(
            soft_target_icon(mcbride.kind, &retail()),
            Some(ActiveWowCursor::Speak)
        );
        // SoftTargetIconGameObject 0: no icon over the mailbox or a chair.
        assert_eq!(soft_target_icon(goldshire_mailbox().kind, &retail()), None);
        assert_eq!(soft_target_icon(goldshire_chair(), &retail()), None);
        // SoftTargetLowPriorityIcons 0: no icon over a corpse that already sparkles.
        let corpse = SoftKind::Unit(ActiveWowCursor::Loot);
        assert_eq!(soft_target_icon(corpse, &retail()), None);
    }

    #[test]
    fn show_all_adds_object_and_loot_icons_and_show_none_hides_all() {
        let all = icons(InteractKeyIcons::ShowAll);
        assert_eq!(
            soft_target_icon(goldshire_mailbox().kind, &all),
            Some(ActiveWowCursor::Mail)
        );
        assert_eq!(
            soft_target_icon(goldshire_chair(), &all),
            Some(ActiveWowCursor::Interact)
        );
        let corpse = SoftKind::Unit(ActiveWowCursor::Loot);
        assert_eq!(soft_target_icon(corpse, &all), Some(ActiveWowCursor::Loot));
        let none = icons(InteractKeyIcons::ShowNone);
        let [danil, _, _] = northshire_vendors();
        assert_eq!(soft_target_icon(danil.kind, &none), None);
        assert_eq!(soft_target_icon(goldshire_mailbox().kind, &none), None);
    }

    #[test]
    fn arc_1_takes_the_front_half_and_arc_2_any_direction() {
        // Brother Danil 3.0 yd away, 0.91 yd ahead and 2.88 yd beside the facing line;
        // Dermot Johns 4.8 yd straight ahead.
        let feet = at(-8902.5, -115.6, 82.03);
        let mut options = retail();
        options.interact_arc = SoftTargetArc::InFront;
        let target = soft_interact_target(feet, facing(0.0), &northshire_vendors(), &options);
        assert_eq!(target, Some(DANIL));
        // Facing away, both vendors are behind: only arc 2 still takes the nearest.
        let away = facing(std::f32::consts::PI);
        assert_eq!(
            soft_interact_target(feet, away, &northshire_vendors(), &options),
            None
        );
        options.interact_arc = SoftTargetArc::Anywhere;
        assert_eq!(
            soft_interact_target(feet, away, &northshire_vendors(), &options),
            Some(DANIL)
        );
    }

    #[test]
    fn interact_range_setting_limits_selection() {
        // Brother Danil 1.21 yd straight ahead.
        let feet = at(-8902.09, -113.82, 82.03);
        let forward = facing(355f32.to_radians());
        let mut options = retail();
        options.interact_range = 1.0;
        assert_eq!(
            soft_interact_target(feet, forward, &northshire_vendors(), &options),
            None
        );
        options.interact_range = 2.0;
        assert_eq!(
            soft_interact_target(feet, forward, &northshire_vendors(), &options),
            Some(DANIL)
        );
    }

    #[test]
    fn object_icons_use_world_text_size() {
        let [danil, _, _] = northshire_vendors();
        assert_eq!(soft_icon_size(danil.kind), 19.0);
        assert_eq!(soft_icon_size(goldshire_mailbox().kind), 32.0);
    }
}
