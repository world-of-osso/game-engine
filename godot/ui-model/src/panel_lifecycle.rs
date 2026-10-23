//! Authored close membership shared by root declarations and native domain dispatch.
//! R UIParentPanelManager.lua1147–1162: bags, panels, special frames; AddOnPost.
//! Existing project projections split Collections/Guild surfaces into separate domain hooks.
use ui_toolkit::lifecycle::{
    CloseGroup, CloseRegistration, EscapeBinding, EscapePriority, bind_close_registration,
    bind_escape_registration,
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

/// Global handlers left after focused/native keyboard owners declined the key.
/// R GameMenuEsc.lua: stable priority order, first handled stops; existing domain tie order.
/// GameMenuFrame.lua36 runs Framework before UIParentPanelManager.lua1162 AddOnPost.
/// Earlier popup/editor keyboard owners remain in the native priority-input route.
pub const ESCAPE_BINDINGS: &[EscapeBinding<&str>] = &[
    escape_handler("talent_loadout_dialog", EscapePriority::Dialog),
    escape_handler("guild_invitation", EscapePriority::Dialog),
    escape_handler("trading_slow_purchase", EscapePriority::Dialog),
    escape_handler("ground_target", EscapePriority::Casting),
    // Project cursor cleanup retains its existing position between ground and pet targeting.
    // This is native input policy, not an invented Retail Lua registration.
    escape_handler("bag_cursor", EscapePriority::Casting),
    escape_handler("pet_move_to", EscapePriority::Casting),
    escape_handler("game_menu", EscapePriority::Framework),
    escape_handler("all_windows", EscapePriority::AddOnPost),
    escape_handler("clear_target", EscapePriority::World),
];

const fn escape_handler(
    hook: &'static str,
    priority: EscapePriority,
) -> EscapeBinding<&'static str> {
    EscapeBinding { priority, hook }
}

const fn panel(hook: &'static str) -> CloseRegistration<&'static str> {
    CloseRegistration {
        hook,
        group: CloseGroup::Panels,
        priority: EscapePriority::AddOnPost,
    }
}

pub fn bind_escape(element: Element, hook: &str) -> Element {
    let registration = ESCAPE_BINDINGS
        .iter()
        .find(|entry| entry.hook == hook)
        .unwrap_or_else(|| panic!("unknown authored Escape registration: {hook}"));
    bind_escape_registration(element, registration)
}

pub fn bind_close(element: Element, hook: &str) -> Element {
    let registration = CLOSE_REGISTRATIONS
        .iter()
        .find(|entry| entry.hook == hook)
        .unwrap_or_else(|| panic!("unknown authored panel close registration: {hook}"));
    bind_close_registration(element, registration)
}
