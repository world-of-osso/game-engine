//! Mainline CommunitiesMemberList.lua871–896 structural ordering, with approved policies.
use shared::protocol::{GuildMemberSnapshot, GuildRankMember};
use std::cmp::Ordering;

type Member<'a> = (&'a GuildRankMember, Option<&'a GuildMemberSnapshot>);

/// Project policy: recent observed logout first ascending, not Retail relative counters/ties.
const GUILD_ZONE_SORT_OFFLINE_BY_LOGOUT_SECONDS: fn(u64, u64) -> Ordering =
    |lhs, rhs| rhs.cmp(&lhs);

pub(super) fn compare(lhs: Member<'_>, rhs: Member<'_>, descending: bool) -> Ordering {
    let primary = match (offline_logout(lhs.1), offline_logout(rhs.1)) {
        (Some(lhs), Some(rhs)) => GUILD_ZONE_SORT_OFFLINE_BY_LOGOUT_SECONDS(lhs, rhs),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => compare_zones(online_zone(lhs.1), online_zone(rhs.1)),
    };
    let primary = if descending {
        primary.reverse()
    } else {
        primary
    };
    // Approved existing name order stays fixed when the primary comparison ties.
    primary.then_with(|| lhs.0.character_name.cmp(&rhs.0.character_name))
}

fn offline_logout(profile: Option<&GuildMemberSnapshot>) -> Option<u64> {
    profile
        .filter(|member| !member.is_online)
        .and_then(|member| member.last_logout_unix_secs)
}

fn online_zone(profile: Option<&GuildMemberSnapshot>) -> Option<&str> {
    profile
        .filter(|member| member.is_online)
        .and_then(|member| member.online_zone.as_deref())
}

fn compare_zones(lhs: Option<&str>, rhs: Option<&str>) -> Ordering {
    match (lhs, rhs) {
        // Explicitly allowed aso historical behavior-prose packet; not a verified Lua binding.
        (Some(lhs), Some(rhs)) => crate::guild_note_sort::compare_storm_historical(lhs, rhs),
        // Mainline CompareMembersByAttribute: a known string precedes a missing value here.
        _ => lhs.is_none().cmp(&rhs.is_none()),
    }
}
