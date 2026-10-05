//! HUD button tooltips: the minimap cluster (`Blizzard_Minimap/Mainline/Minimap.lua`,
//! `GameTime.lua`, `Blizzard_TimeManager.lua`) and the bag bar
//! (`Blizzard_MainMenuBarBagButtons/Mainline/MainMenuBarBagButtons.lua`). `AddLine` without
//! a colour is `NORMAL_FONT_COLOR`; the first line uses the header font.

use game_engine_core::minimap_data::ZonePvp;

use super::GameTooltip;
use crate::damage_meter_data::break_up_large_number;
use crate::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE, TooltipLineState, TooltipPresentation,
    description_lines,
};

/// `NORMAL_FONT_COLOR`.
const NORMAL: [f32; 4] = TOOLTIP_DESCRIPTION_COLOR;

fn text_tooltip(
    title: impl Into<String>,
    color: [f32; 4],
    lines: Vec<TooltipLineState>,
) -> GameTooltip {
    GameTooltip::new(
        TooltipPresentation {
            title: title.into(),
            title_color: color,
            lines,
            ..TooltipPresentation::hidden()
        },
        None,
    )
}

/// `MinimapZoneTextButtonMixin:OnEnter` + `Minimap_SetTooltip` (Minimap.lua:126-136,
/// 213-249): the white zone; the subzone (empty when it equals the zone) in the PvP colour,
/// with `SANCTUARY_TERRITORY` or `FACTION_CONTROLLED_TERRITORY` "(%s Territory)" below it
/// (a friendly or hostile zone with no faction name shows neither); then
/// `MicroButtonTooltipText(WORLDMAP_BUTTON, "TOGGLEWORLDMAP")` "World Map (M)" in gold.
pub fn zone_tooltip(
    zone: &str,
    subzone: &str,
    pvp: ZonePvp,
    controlling_faction: Option<&str>,
    world_map_key: Option<&str>,
) -> GameTooltip {
    let subzone = if subzone == zone { "" } else { subzone };
    let color = pvp.text_color();
    let line = |text: String| TooltipLineState::colored(text, color);
    let mut lines = match (pvp, controlling_faction) {
        (ZonePvp::Sanctuary, _) => vec![line(subzone.into()), line("(Sanctuary)".into())],
        (ZonePvp::Friendly | ZonePvp::Hostile, Some(faction)) => {
            vec![line(subzone.into()), line(format!("({faction} Territory)"))]
        }
        (ZonePvp::Friendly | ZonePvp::Hostile, None) => Vec::new(),
        (ZonePvp::Normal, _) => vec![line(subzone.into())],
    };
    let button = match world_map_key {
        Some(key) => format!("World Map ({key})"),
        None => "World Map".to_owned(),
    };
    lines.push(TooltipLineState::colored(button, NORMAL));
    text_tooltip(zone, TOOLTIP_WHITE, lines)
}

/// `MiniMapTrackingButtonMixin:OnEnter` (Minimap.lua:800-805): `TRACKING` and the wrapped
/// `MINIMAP_TRACKING_TOOLTIP_NONE`.
pub fn tracking_tooltip() -> GameTooltip {
    text_tooltip(
        "Tracking",
        TOOLTIP_WHITE,
        description_lines("Click to enable and disable tracking types.", NORMAL),
    )
}

/// `MinimapZoomInButtonMixin:OnEnter` / `ZoomOut` with `UberTooltips` 1: `ZOOM_IN` /
/// `ZOOM_OUT` at the default anchor.
pub fn zoom_tooltip(zoom_in: bool) -> GameTooltip {
    text_tooltip(
        if zoom_in { "Zoom In" } else { "Zoom Out" },
        TOOLTIP_WHITE,
        Vec::new(),
    )
}

/// `TimeManagerClockButton_UpdateTooltip` → `GameTime_UpdateTooltip`: `TIMEMANAGER_TOOLTIP_TITLE`,
/// the realm and local times, a blank line and `GAMETIME_TOOLTIP_TOGGLE_CLOCK`.
pub fn clock_tooltip(realm_time: &str, local_time: &str) -> GameTooltip {
    let time = |label: &str, value: &str| TooltipLineState {
        left_color: NORMAL,
        right_color: TOOLTIP_WHITE,
        ..TooltipLineState::pair(label, value)
    };
    text_tooltip(
        "Time Info",
        TOOLTIP_WHITE,
        vec![
            time("Realm time:", realm_time),
            time("Local time:", local_time),
            TooltipLineState::new(String::new()),
            TooltipLineState::colored("Click to show clock settings.", NORMAL),
        ],
    )
}

/// `GameTime_GetFormattedTime(hour, minute, true)`: `TIME_TWELVEHOURAM`/`PM` "%d:%02d AM".
pub fn twelve_hour_time(hour: u32, minute: u32) -> String {
    let suffix = if hour % 24 >= 12 { "PM" } else { "AM" };
    let hour = match hour % 12 {
        0 => 12,
        hour => hour,
    };
    format!("{hour}:{minute:02} {suffix}")
}

/// `GameTimeFrame_OnUpdate` with the clock shown and no invites: `GAMETIME_TOOLTIP_TOGGLE_CALENDAR`.
pub fn calendar_tooltip() -> GameTooltip {
    text_tooltip("Click to show the calendar.", NORMAL, Vec::new())
}

/// `Minimap_OnUpdate`: `SetOwner(UIParent, "ANCHOR_CURSOR")` and `SetMinimapMouseover`, one
/// white line per unit whose blip is under the cursor; none without blips.
pub fn minimap_mouseover_tooltip(names: &[String]) -> Option<GameTooltip> {
    let (first, rest) = names.split_first()?;
    let lines = rest
        .iter()
        .map(|name| TooltipLineState::colored(name.clone(), TOOLTIP_WHITE))
        .collect();
    Some(text_tooltip(first.clone(), TOOLTIP_WHITE, lines).at_cursor())
}

/// `MainMenuBarBackpackMixin:OnEnterInternal`: `BACKPACK_TOOLTIP` and `NUM_FREE_SLOTS`.
pub fn backpack_tooltip(free_slots: usize) -> GameTooltip {
    let slots = if free_slots == 1 { "Slot" } else { "Slots" };
    text_tooltip(
        "Backpack",
        TOOLTIP_WHITE,
        vec![TooltipLineState::colored(
            format!("{free_slots} Empty {slots} (Total)"),
            NORMAL,
        )],
    )
}

/// `BaseBagSlotButtonMixin:OnEnterInternal` for an empty bag slot: `EQUIP_CONTAINER`.
/// `PaperDollItemSlotButton_OnEnter` without an item: `GameTooltip:SetText` of the slot's
/// `<SLOT>SLOT` global ("Head"), in the default white.
pub fn empty_paperdoll_slot_tooltip(label: &str) -> GameTooltip {
    text_tooltip(label, TOOLTIP_WHITE, Vec::new())
}

pub fn empty_bag_slot_tooltip() -> GameTooltip {
    text_tooltip("Equip Container", TOOLTIP_WHITE, Vec::new())
}

/// `MiniMapMailFrameMixin:OnEnter` / `FormatUnreadMailTooltip`: one white `SetText` of
/// `HAVE_MAIL_FROM` and a line per sender, or `HAVE_MAIL` without senders.
pub fn unread_mail_tooltip(senders: &[String]) -> GameTooltip {
    let (header, senders) = crate::minimap::mail_tooltip_lines(senders);
    let lines = senders
        .into_iter()
        .map(|sender| TooltipLineState::colored(sender, TOOLTIP_WHITE))
        .collect();
    text_tooltip(header, TOOLTIP_WHITE, lines)
}

/// `ExpBarMixin:OnEnter` → `ExhaustionTickMixin:ExhaustionToolTipText` (ExpBar.lua:72-88,
/// ExpBarOverrides.lua:20-36) at the default anchor: `XP_TEXT` "%s / %s  ( %d%% )\n\n" in
/// white with `BreakUpLargeNumbers` and `math.ceil` percent, then `EXHAUST_TOOLTIP1`: the
/// `GetRestState` name in gold and "%d%% of normal experience\ngained from monsters." in
/// white (GlobalStrings 19675, 11839). `GetRestState` is 1 "Rested" ×2 with a rested pool,
/// 2 "Normal" ×1 without; `EXHAUST_TOOLTIP2` belongs to states 4-5, which Retail never sets.
pub fn xp_tooltip(xp: u32, next_level_xp: u32, rested_xp: u32) -> GameTooltip {
    let percent = (u64::from(xp) * 100).div_ceil(u64::from(next_level_xp));
    let (state, multiplier) = if rested_xp > 0 {
        ("Rested", 200)
    } else {
        ("Normal", 100)
    };
    let title = format!(
        "{} / {}  ( {percent}% )",
        break_up_large_number(xp.into()),
        break_up_large_number(next_level_xp.into())
    );
    text_tooltip(
        title,
        TOOLTIP_WHITE,
        vec![
            TooltipLineState::colored(String::new(), TOOLTIP_WHITE),
            TooltipLineState::colored(state, NORMAL),
            TooltipLineState::colored(
                format!("{multiplier}% of normal experience"),
                TOOLTIP_WHITE,
            ),
            TooltipLineState::colored("gained from monsters.", TOOLTIP_WHITE),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(tooltip: &GameTooltip) -> Vec<(&str, &str)> {
        std::iter::once((tooltip.content.title.as_str(), ""))
            .chain(
                tooltip
                    .content
                    .lines
                    .iter()
                    .map(|line| (line.left_text.as_str(), line.right_text.as_str())),
            )
            .collect()
    }

    #[test]
    fn the_xp_bar_shows_progress_and_the_rest_state() {
        let normal = xp_tooltip(1234, 5000, 0);
        assert_eq!(
            rows(&normal),
            [
                ("1,234 / 5,000  ( 25% )", ""),
                ("", ""),
                ("Normal", ""),
                ("100% of normal experience", ""),
                ("gained from monsters.", ""),
            ]
        );
        assert_eq!(normal.content.title_color, TOOLTIP_WHITE);
        assert_eq!(normal.content.lines[1].left_color, NORMAL);
        assert_eq!(normal.content.lines[2].left_color, TOOLTIP_WHITE);
        // `math.ceil`: 1 XP of 250 reads 1%; a rested pool doubles monster XP.
        let rested = xp_tooltip(1, 250, 300);
        assert_eq!(rows(&rested)[0], ("1 / 250  ( 1% )", ""));
        assert_eq!(rows(&rested)[2..4], [("Rested", ""), ("200% of normal experience", "")]);
    }

    #[test]
    fn an_empty_paperdoll_slot_shows_only_its_white_slot_name() {
        let head = empty_paperdoll_slot_tooltip("Head");
        assert_eq!(rows(&head), [("Head", "")]);
        assert_eq!(head.content.title_color, TOOLTIP_WHITE);
    }

    #[test]
    fn the_zone_button_names_zone_subzone_and_the_world_map_key() {
        super::super::set_test_data_root();
        let elwynn = zone_tooltip(
            "Elwynn Forest",
            "Northshire Valley",
            ZonePvp::Friendly,
            Some("Alliance"),
            Some("M"),
        );
        assert_eq!(
            rows(&elwynn),
            [
                ("Elwynn Forest", ""),
                ("Northshire Valley", ""),
                ("(Alliance Territory)", ""),
                ("World Map (M)", ""),
            ]
        );
        assert_eq!(elwynn.content.title_color, TOOLTIP_WHITE);
        assert_eq!(elwynn.content.lines[0].left_color, [0.1, 1.0, 0.1, 1.0]);
        assert_eq!(elwynn.content.lines[2].left_color, NORMAL);
        let city = zone_tooltip(
            "Stormwind City",
            "Stormwind City",
            ZonePvp::Normal,
            None,
            None,
        );
        assert_eq!(rows(&city)[1..], [("", ""), ("World Map", "")]);
        assert_eq!(city.content.lines[0].left_color, NORMAL);
        let goldshire = zone_tooltip("Elwynn Forest", "Goldshire", ZonePvp::Sanctuary, None, None);
        assert_eq!(rows(&goldshire)[2], ("(Sanctuary)", ""));
    }

    #[test]
    fn the_clock_lists_realm_and_local_time_then_the_hint() {
        super::super::set_test_data_root();
        let tooltip = clock_tooltip("6:07 PM", "6:07 PM");
        assert_eq!(
            rows(&tooltip),
            [
                ("Time Info", ""),
                ("Realm time:", "6:07 PM"),
                ("Local time:", "6:07 PM"),
                ("", ""),
                ("Click to show clock settings.", ""),
            ]
        );
        assert_eq!(tooltip.content.lines[0].left_color, NORMAL);
        assert_eq!(tooltip.content.lines[0].right_color, TOOLTIP_WHITE);
    }

    #[test]
    fn minimap_blips_list_their_units_at_the_cursor() {
        super::super::set_test_data_root();
        assert_eq!(minimap_mouseover_tooltip(&[]), None);
        let names = ["Brother Danil".to_owned(), "Marshal McBride".to_owned()];
        let tooltip = minimap_mouseover_tooltip(&names).expect("blips");
        assert_eq!(
            rows(&tooltip),
            [("Brother Danil", ""), ("Marshal McBride", "")]
        );
        assert_eq!(tooltip.anchor, super::super::TooltipAnchor::Cursor);
    }

    #[test]
    fn times_read_in_twelve_hours() {
        assert_eq!(twelve_hour_time(18, 7), "6:07 PM");
        assert_eq!(twelve_hour_time(0, 30), "12:30 AM");
        assert_eq!(twelve_hour_time(12, 0), "12:00 PM");
    }

    #[test]
    fn bag_bar_buttons_count_free_slots_and_name_empty_slots() {
        super::super::set_test_data_root();
        assert_eq!(
            rows(&backpack_tooltip(12))[1],
            ("12 Empty Slots (Total)", "")
        );
        assert_eq!(rows(&backpack_tooltip(1))[1], ("1 Empty Slot (Total)", ""));
        assert_eq!(rows(&empty_bag_slot_tooltip()), [("Equip Container", "")]);
        assert_eq!(rows(&zoom_tooltip(true)), [("Zoom In", "")]);
        assert_eq!(rows(&tracking_tooltip())[0], ("Tracking", ""));
    }

    #[test]
    fn unread_mail_lists_its_senders_in_white_under_the_header() {
        super::super::set_test_data_root();
        let senders = ["Fbalpha".to_owned(), "Auction House".to_owned()];
        let tooltip = unread_mail_tooltip(&senders);
        assert_eq!(
            rows(&tooltip),
            [
                ("Unread mail from:", ""),
                ("Fbalpha", ""),
                ("Auction House", "")
            ]
        );
        assert_eq!(tooltip.content.title_color, TOOLTIP_WHITE);
        assert!(
            tooltip
                .content
                .lines
                .iter()
                .all(|l| l.left_color == TOOLTIP_WHITE)
        );
        assert_eq!(
            rows(&unread_mail_tooltip(&[])),
            [("You have unread mail", "")]
        );
    }
}
