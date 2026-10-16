//! Read-only receipt of real owner spell messages and current chat input; no fixture injection.
use crate::GameClient;
use godot::prelude::*;

#[godot_api(secondary)]
impl GameClient {
    #[func]
    pub fn guild_perks_observation(&self) -> GString {
        let sent = self
            .spells_snapshot()
            .get("sent")
            .expect("actual spell automation sent receipt")
            .to::<PackedInt64Array>()
            .to_vec();
        let player = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id));
        let speed = player.and_then(|unit| unit.get::<shared::components::MovementSpeed>());
        let mounted = player.and_then(|unit| unit.get::<shared::components::Mounted>());
        let motion = player.and_then(|unit| unit.get::<shared::components::PlayerMotion>());
        let position = player.and_then(|unit| unit.get::<shared::components::Position>());
        let input = self.physical_input.gameplay_state(self.keyboard_free());
        let json = serde_json::to_string(&serde_json::json!({
            // Installed Hearthstone8690 uses charge category2309.
            "hearth_charges": self.account.spells.charges(2309).map(|charges| charges.snapshot()),
            "movement_speed": speed.map(|speed| speed.0),
            "position": position.map(|position| [position.x, position.y, position.z]),
            "startup_loading": self.asset_startup.is_some(),
            "keyboard_free": self.keyboard_free(),
            "jump_held": self.client_options.bindings.is_pressed(game_engine_core::input_bindings_data::InputAction::Jump, &input),
            "forward_held": self.client_options.bindings.is_pressed(game_engine_core::input_bindings_data::InputAction::MoveForward, &input),
            "mount_id": mounted.map(|mount| mount.mount_id),
            "can_fly": motion.map(|motion| motion.contains(shared::components::PlayerMotion::CAN_FLY)),
            "can_adv_fly": motion.map(|motion| motion.contains(shared::components::PlayerMotion::CAN_ADV_FLY)),
            "flying": motion.map(|motion| motion.contains(shared::components::PlayerMotion::FLYING)),
            "falling": motion.map(|motion| motion.contains(shared::components::PlayerMotion::FALLING)),
            "sent": sent,
            "known_spells": self.account.spells.known(),
            "chat_open": self.chat.model.state.input_open,
            "draft_visible": self.chat.draft.visible(),
            "draft_wire": self.chat.draft.wire(),
        }))
        .expect("serialize actual perk knowledge/chat receipt");
        GString::from(json.as_str())
    }
}
