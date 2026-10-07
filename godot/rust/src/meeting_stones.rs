//! Stone/ritual use retains the current player target; summon offers use StaticPopup.
use crate::{
    GameClient,
    frame_error::{FrameError, SessionError},
    replicated::UnitFields,
};
use game_engine_ui_model::popup::PopupResult;
use shared::protocol::{
    GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_RITUAL, GameObjectInfo, SummonRequest,
    UseGameObject,
};
use std::time::Duration;

pub(crate) fn meeting_stone_request(id: u64, info: &GameObjectInfo) -> Option<UseGameObject> {
    matches!(
        info.go_type,
        GAMEOBJECT_TYPE_MEETINGSTONE | GAMEOBJECT_TYPE_RITUAL
    )
    .then_some(UseGameObject { object: id })
}

impl GameClient {
    pub(crate) fn use_meeting_stone(&mut self, id: u64) -> Result<bool, FrameError> {
        let request = self
            .replica
            .unit(id)
            .and_then(|unit| unit.get::<GameObjectInfo>())
            .and_then(|info| meeting_stone_request(id, info));
        let Some(request) = request.filter(|_| self.game_objects.contains(id)) else {
            return Ok(false);
        };
        // The server checks distance to the model box, not to the model origin.
        self.account.send_use_game_object(request.object)?;
        Ok(true)
    }

    pub(super) fn receive_summon(&mut self, request: SummonRequest) {
        let combat = self.summon_in_combat();
        self.summon
            .receive(request, &mut self.group_frames.popups, combat);
    }

    pub(super) fn update_summon_popup(&mut self, delta: Duration) {
        let combat = self.summon_in_combat();
        self.summon
            .update(delta, &mut self.group_frames.popups, combat);
    }

    fn summon_in_combat(&self) -> bool {
        self.world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .is_some_and(UnitFields::in_combat)
    }

    pub(super) fn dispatch_summon_popup_results(
        &mut self,
        results: &[PopupResult],
    ) -> Result<(), SessionError> {
        if let Some(response) = self.summon.popup_results(results) {
            self.account.send_summon_response(response)?;
        }
        Ok(())
    }
}
