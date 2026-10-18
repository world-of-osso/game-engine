//! Mainline public-note ordering with explicitly qualified historical comparison.
use shared::protocol::{GuildMemberSnapshot, GuildRankMember};
use std::cmp::Ordering;

type Member<'a> = (&'a GuildRankMember, Option<&'a GuildMemberSnapshot>);

/// Historical Storm clean-room prose, not verified Lua binding or Retail parity.
/// Replace only when the native behaviour packet establishes the required change.
const GUILD_NOTE_COMPARE_STORM_HISTORICAL: fn(&str, &str) -> Ordering = compare_storm_historical;

pub(super) fn compare(lhs: Member<'_>, rhs: Member<'_>, descending: bool) -> Ordering {
    let lhs_note = lhs.1.map(|profile| profile.public_note.as_str());
    let rhs_note = rhs.1.map(|profile| profile.public_note.as_str());
    let primary = match (lhs_note, rhs_note) {
        (Some(lhs), Some(rhs)) => GUILD_NOTE_COMPARE_STORM_HISTORICAL(lhs, rhs),
        // Source nil handling is opposite to string direction: nil first on first click.
        _ => lhs_note.is_none().cmp(&rhs_note.is_none()),
    };
    let primary = if descending {
        primary.reverse()
    } else {
        primary
    };
    // SortMemberInfoWithOverride never reverses the base comparison for equal notes.
    primary.then_with(|| compare_base(lhs, rhs))
}

/// GUILD_ROSTER_CLUB_FIELDS_ABSENT: use represented fields only, in source order.
fn compare_base(lhs: Member<'_>, rhs: Member<'_>) -> Ordering {
    presence_sort_key(lhs.1)
        .cmp(&presence_sort_key(rhs.1))
        .then_with(|| match (lhs.1, rhs.1) {
            (Some(lhs), Some(rhs)) => rhs.level.cmp(&lhs.level),
            _ => Ordering::Equal,
        })
        .then_with(|| lhs.0.rank.cmp(&rhs.0.rank))
        .then_with(|| lhs.0.character_name.cmp(&rhs.0.character_name))
}

fn presence_sort_key(profile: Option<&GuildMemberSnapshot>) -> u8 {
    match profile {
        Some(profile) if profile.is_online => 0,
        Some(_) => 1,
        None => 2,
    }
}

fn compare_storm_historical(lhs: &str, rhs: &str) -> Ordering {
    let map = |character: char| {
        let point = u32::from(character);
        let point = match point {
            0x0430..=0x044f => point - 0x20,
            _ => point,
        };
        match point {
            0x0061..=0x007a | 0x00e0..=0x00fe => point - 0x20,
            0x0153 => 0x0152,
            0x0410..=0x0415 => point - 1,
            0x0401 | 0x0451 => 0x0415,
            0x10000.. => 0xfffd,
            _ => point,
        }
    };
    lhs.chars().map(map).cmp(rhs.chars().map(map))
}
