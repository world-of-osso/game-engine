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
        let json = serde_json::to_string(&serde_json::json!({
            "movement_speed": speed.map(|speed| speed.0),
            "mount_id": mounted.map(|mount| mount.mount_id),
            "can_fly": motion.map(|motion| motion.contains(shared::components::PlayerMotion::CAN_FLY)),
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
