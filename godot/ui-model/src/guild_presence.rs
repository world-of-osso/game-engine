//! Mainline TimeUtil.lua24–43 elapsed recency, not a calendar date.
//! English labels: authenticated local-CASC GlobalStrings10842/11461/11518–11520,
//! build12.1.5.70448 (recency-string-source.json in the Last Online proof ledger).
use shared::protocol::GuildMemberSnapshot;

pub(crate) fn read_current_last_online_text(member: &GuildMemberSnapshot) -> String {
    last_online_at(
        member.is_online,
        member.last_logout_unix_secs,
        crate::chat_data::now_timestamp() as u64,
    )
}

fn last_online_at(online: bool, observed_logout: Option<u64>, now: u64) -> String {
    if online {
        return "Online".into();
    }
    let Some(logout) = observed_logout else {
        return "Offline".into();
    };
    let elapsed = now.saturating_sub(logout);
    // TimeUtil.lua10–11 defines a month as30days and a year as12months.
    for (seconds, unit) in [
        (360 * 86400, "year"),
        (30 * 86400, "month"),
        (86400, "day"),
        (3600, "hour"),
    ] {
        let count = elapsed / seconds;
        if count > 0 {
            return format!("{count} {unit}{}", if count == 1 { "" } else { "s" });
        }
    }
    "< an hour".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_last_online_source_unit_boundaries_are_deterministic() {
        let now = 2_000_000_000;
        for (elapsed, expected) in [
            (0, "< an hour"),
            (3599, "< an hour"),
            (3600, "1 hour"),
            (2 * 3600, "2 hours"),
            (86400 - 1, "23 hours"),
            (86400, "1 day"),
            (30 * 86400 - 1, "29 days"),
            (30 * 86400, "1 month"),
            (360 * 86400 - 1, "11 months"),
            (360 * 86400, "1 year"),
            (720 * 86400, "2 years"),
        ] {
            assert_eq!(last_online_at(false, Some(now - elapsed), now), expected);
        }
        assert_eq!(last_online_at(false, None, now), "Offline");
        assert_eq!(last_online_at(true, Some(now - 86400), now), "Online");
    }
}
