//! One cursor/ReceiveDrag path for the main bar and all four extra bars.
use crate::GameClient;
use crate::bag_cursor::SpellDrag;
use crate::frame_error::FrameError;
use game_engine_ui_model::cursor_item::CursorItem;
use game_engine_ui_model::main_action_bar_component::parse_action_button;
use game_engine_ui_model::merchant::Click;
use game_engine_ui_model::spellbook_frame_component::ACTION_SPELLBOOK_CAST;
use godot::prelude::*;
use shared::protocol::ActionRef;

/// Same logical-pixel threshold as container item dragging.
const DRAG_THRESHOLD: f32 = 4.0;

impl GameClient {
    pub(crate) fn spell_cursor_press(
        &mut self,
        owner: i64,
        action: &str,
        click: Click,
        at: Option<Vector2>,
    ) -> Result<bool, FrameError> {
        let book = self
            .spells
            .book_ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner);
        if book {
            self.press_spellbook_cursor(owner, action, click, at)?;
            return Ok(true);
        }
        let bar = self
            .spells
            .bar_ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner);
        if bar && !click.right {
            if !self.assign_cursor_to_action_button(action)?
                && let Some((bar, index)) = parse_action_button(action)
            {
                self.use_action_button(bar, index)?;
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn press_spellbook_cursor(
        &mut self,
        owner: i64,
        action: &str,
        click: Click,
        at: Option<Vector2>,
    ) -> Result<(), FrameError> {
        if let Some(raw) = action.strip_prefix(ACTION_SPELLBOOK_CAST)
            && let Some(origin) = at.filter(|_| !click.right)
        {
            let spell_id = raw
                .parse()
                .map_err(|_| format!("Bad spellbook spell {action}"))?;
            let icon = self
                .spells
                .catalog()
                .and_then(|data| data.get(spell_id))
                .map_or(0, |spell| spell.icon_fdid);
            let icon_fdid = self.drawable_fdid(icon);
            self.bags.cursor.spell_drag = Some(SpellDrag {
                owner,
                origin,
                spell_id,
                icon_fdid,
            });
            return Ok(());
        }
        self.apply_spellbook_action(action)
    }

    pub(crate) fn finish_spell_drag(
        &mut self,
        owner: i64,
        at: Vector2,
        physical_at: Vector2,
    ) -> Result<bool, FrameError> {
        let Some(drag) = self
            .bags
            .cursor
            .spell_drag
            .as_ref()
            .filter(|drag| drag.owner == owner)
        else {
            return Ok(false);
        };
        let spell_id = drag.spell_id;
        let distance = drag.origin.distance_to(at);
        if distance < DRAG_THRESHOLD {
            self.bags.cursor.spell_drag = None;
            self.cast_spell(spell_id)?;
        } else {
            let hit = self.ui_hit_at(physical_at)?;
            if let Some(action) = hit.and_then(|hit| hit.action) {
                self.assign_cursor_to_action_button(&action)?;
            }
            self.bags.cursor.spell_drag = None;
        }
        Ok(true)
    }

    /// Item drops copy the item action; they never move or consume the bag stack.
    pub(crate) fn assign_cursor_to_action_button(
        &mut self,
        action: &str,
    ) -> Result<bool, FrameError> {
        let Some((bar, index)) = parse_action_button(action) else {
            return Ok(false);
        };
        let payload = self
            .bags
            .cursor
            .spell_drag
            .as_ref()
            .map(|drag| ActionRef::Spell(drag.spell_id))
            .or_else(|| self.toybox.drag.as_ref().map(|drag| drag.action.to_slot()))
            .or_else(|| match &self.bags.cursor.item {
                CursorItem::Inventory { item_id, .. } => Some(ActionRef::Item(*item_id)),
                _ => None,
            });
        let Some(payload) = payload else {
            return Ok(false);
        };
        let request = bar
            .assignment(index, payload, self.bonus_bar_offset())
            .ok_or("Action button outside supported slot range")?;
        self.account.send_set_action_button(request.clone())?;
        self.account.spells.apply_assignment(&request);
        self.bags.cursor.item = CursorItem::Empty;
        self.bags.cursor.spell_drag = None;
        self.toybox.drag = None;
        Ok(true)
    }
}
