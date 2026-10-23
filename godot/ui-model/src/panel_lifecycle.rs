//! Authored close membership shared by root declarations and native domain dispatch.
//! R UIParentPanelManager.lua1147–1162: bags, panels, special frames; AddOnPost.
//! Existing project projections split Collections/Guild surfaces into separate domain hooks.
use ui_toolkit::lifecycle::{
    CloseGroup, CloseRegistration, EscapePriority, bind_close_registration,
};
use ui_toolkit::widget_def::Element;

/// Append-only registrations; no mounted instance is required to cancel a pending domain.
/// Order within each group retains existing domain close/error ordering.
pub const CLOSE_REGISTRATIONS: &[CloseRegistration<&str>] = &[
    CloseRegistration {
        hook: "bags",
        group: CloseGroup::Bags,
        priority: EscapePriority::AddOnPost,
    },
    panel("housing"),
    panel("toybox"),
    panel("spellbook"),
    panel("world_map"),
    panel("mailbox"),
    panel("auction"),
    panel("merchant"),
    panel("trade"),
    panel("quests"),
    panel("character"),
    panel("guild_ranks"),
    panel("guild_charter"),
    panel("guild_tabard"),
    panel("professions"),
    panel("pet_journal"),
    panel("mount_journal"),
    panel("adventure_guide"),
    panel("trading_post"),
    panel("trainer"),
    panel("transmog"),
    panel("achievements"),
    panel("mythic_mentor"),
    panel("raid_info"),
    panel("combat_stable"),
    panel("bank"),
    panel("guild_bank"),
    panel("flight_map"),
];

const fn panel(hook: &'static str) -> CloseRegistration<&'static str> {
    CloseRegistration {
        hook,
        group: CloseGroup::Panels,
        priority: EscapePriority::AddOnPost,
    }
}

pub fn bind_close(element: Element, hook: &str) -> Element {
    let registration = CLOSE_REGISTRATIONS
        .iter()
        .find(|entry| entry.hook == hook)
        .unwrap_or_else(|| panic!("unknown authored panel close registration: {hook}"));
    bind_close_registration(element, registration)
}
