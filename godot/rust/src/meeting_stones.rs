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
        self.group_frames
            .popups
            .hide(game_engine_ui_model::summon::CONFIRM_SUMMON);
        self.summon = Default::default();
        let remaining = Duration::from_millis(u64::from(request.time_left_ms));
        self.pending_summon = Some((request, remaining));
        self.update_summon_popup(Duration::ZERO);
    }

    pub(super) fn update_summon_popup(&mut self, delta: Duration) {
        let combat = self.summon_in_combat();
        self.summon
            .update(delta, &mut self.group_frames.popups, combat);
        self.update_pending_summon_popup(delta, combat);
    }

    fn update_pending_summon_popup(&mut self, delta: Duration, combat: bool) {
        let Some((request, remaining)) = self.pending_summon.as_mut() else {
            return;
        };
        *remaining = remaining.saturating_sub(delta);
        // Not loaded is distinct from a loaded catalog lacking this zone.
        let Some(zone_name) = self.minimap.area_name(request.zone_id) else {
            return;
        };
        let (mut request, remaining) = self.pending_summon.take().expect("pending summon");
        request.time_left_ms = remaining.as_nanos().div_ceil(1_000_000) as u32;
        self.summon.receive(
            request,
            zone_name.as_deref().unwrap_or_default(),
            &mut self.group_frames.popups,
            combat,
        );
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
