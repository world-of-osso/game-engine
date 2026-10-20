//! Mainline GuildRoster.lua direct-assignment decisions, using wire zero-based ranks.
use crate::guild_ranks::GuildRanksSession;
use shared::protocol::*;
use std::ops::RangeInclusive;

impl GuildRanksSession {
    pub fn member_rank_range(&self, name: &str) -> Option<RangeInclusive<u8>> {
        let state = self.state()?;
        let member = state
            .members
            .iter()
            .find(|member| member.character_name == name)?;
        state.ranks.get(usize::from(member.rank))?;
        if member.rank <= state.own_rank {
            return None;
        }
        let lowest = u8::try_from(state.ranks.len() - 1).expect("guild has at most ten ranks");
        let promote = self.can(GUILD_RIGHT_PROMOTE);
        let demote = self.can(GUILD_RIGHT_DEMOTE);
        let can_change =
            (promote && member.rank > state.own_rank + 1) || (demote && member.rank < lowest);
        if !can_change {
            return None;
        }
        let highest = if promote {
            state.own_rank + 1
        } else {
            member.rank
        };
        let lowest = if demote { lowest } else { member.rank };
        Some(highest..=lowest)
    }

    pub fn build_member_rank_assignment(&self, name: &str, rank: u8) -> Option<GuildRankRequest> {
        if self.awaiting_state || self.profile_waiting {
            return None;
        }
        if !self.member_rank_range(name)?.contains(&rank) {
            return None;
        }
        let member = self
            .state()?
            .members
            .iter()
            .find(|member| member.character_name == name)?;
        if member.rank == rank {
            return None;
        }
        Some(GuildRankRequest::SetMemberRank {
            character_name: member.character_name.clone(),
            rank,
        })
    }
}
