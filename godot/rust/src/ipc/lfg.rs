//! Native follower/human LFG IPC uses the existing reliable LfgChannel contract.
use game_engine_network::ipc_wire::{GroupRole, Request, Response};
use shared::protocol::{
    DequeueFromLfg, GroupRoleSnapshot, QueryLfgStatus, QueueForLfg, RespondToLfgRoleCheck,
};

impl crate::GameClient {
    pub(crate) fn lfg_request(
        &mut self,
        request: Request,
        reply: super::Reply,
    ) -> Result<(), (Request, super::Reply)> {
        let result = match &request {
            Request::LfgStatus => {
                let result = self.account.send_lfg(QueryLfgStatus);
                result.map(|()| {
                    self.account.lfg.as_ref().map_or_else(
                        || "lfg: awaiting server snapshot".to_owned(),
                        |state| serde_json::to_string(state).expect("serialize LFG wire snapshot"),
                    )
                })
            }
            Request::LfgQueue { role, dungeon_ids } => self
                .account
                .send_lfg(QueueForLfg {
                    role: match role {
                        GroupRole::Tank => GroupRoleSnapshot::Tank,
                        GroupRole::Healer => GroupRoleSnapshot::Healer,
                        GroupRole::Damage => GroupRoleSnapshot::Damage,
                        GroupRole::None => GroupRoleSnapshot::None,
                    },
                    dungeon_ids: dungeon_ids.clone(),
                })
                .map(|()| {
                    "follower/dungeon queue request sent; lfg status reports server acceptance"
                        .to_owned()
                }),
            Request::LfgDequeue => self
                .account
                .send_lfg(DequeueFromLfg)
                .map(|()| "LFG dequeue requested".to_owned()),
            Request::LfgAccept | Request::LfgDecline => self
                .account
                .send_lfg(RespondToLfgRoleCheck {
                    accepted: matches!(request, Request::LfgAccept),
                })
                .map(|()| "LFG role-check response sent".to_owned()),
            _ => return Err((request, reply)),
        };
        let response = match result {
            Ok(text) => Response::Text(text),
            Err(error) => Response::Error(error.to_string()),
        };
        reply.send(response);
        Ok(())
    }
}
