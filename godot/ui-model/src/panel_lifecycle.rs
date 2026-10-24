//! Authored root policies shared by declarations and native domain dispatch.
//! R UIParentPanelManager.lua1147–1162: bag/panel/special sweeps at AddOnPost.
//! R GameMenuEsc.lua: independent exclusive handlers, stable priority/first handled.
use ui_toolkit::lifecycle::{
    CloseGroup, EscapeBinding, EscapePriority, LifecycleRegistration, bind_registration,
};
use ui_toolkit::widget_def::Element;

/// Append-only roles; group membership does not imply an exclusive handler or priority.
/// An absent native instance never removes a registered domain's pending cancellation.
/// Existing group callback order and global equal-priority order remain unchanged.
pub const REGISTRATIONS: &[LifecycleRegistration<&str>] = &[
    LifecycleRegistration {
        hook: "bags",
        group: Some(CloseGroup::Bags),
        escape: None,
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
    // R ProfessionsRegistration.lua24–35: owner is both UIPanel and AddOn handler.
    LifecycleRegistration {
        hook: "professions",
        group: Some(CloseGroup::Panels),
        escape: Some(EscapeBinding {
            hook: "professions",
            priority: EscapePriority::AddOn,
        }),
    },
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
    // CustomerOrders/Registration.lua: ordinary UIPanel, not the owner handler.
    panel("professions_customer"),
    // Global handlers run after focused/native owners decline the key.
    exclusive("talent_loadout_dialog", EscapePriority::Dialog),
    exclusive("guild_invitation", EscapePriority::Dialog),
    exclusive("trading_slow_purchase", EscapePriority::Dialog),
    exclusive("ground_target", EscapePriority::Casting),
    // Existing native policy: ground -> carried cursor -> pet. Not a Lua registration.
    exclusive("bag_cursor", EscapePriority::Casting),
    exclusive("pet_move_to", EscapePriority::Casting),
    // R GameMenuFrame.lua36 precedes UIParentPanelManager.lua1162's all-close phase.
    exclusive("game_menu", EscapePriority::Framework),
    exclusive("all_windows", EscapePriority::AddOnPost),
    exclusive("clear_target", EscapePriority::World),
];

const fn panel(hook: &'static str) -> LifecycleRegistration<&'static str> {
    LifecycleRegistration {
        hook,
        group: Some(CloseGroup::Panels),
        escape: None,
    }
}

const fn exclusive(
    hook: &'static str,
    priority: EscapePriority,
) -> LifecycleRegistration<&'static str> {
    LifecycleRegistration {
        hook,
        group: None,
        escape: Some(EscapeBinding { hook, priority }),
    }
}

/// Bind the owning closeable root without inventing group/exclusive membership.
pub fn bind_close(element: Element, hook: &str) -> Element {
    let registration = REGISTRATIONS
        .iter()
        .find(|entry| entry.hook == hook)
        .unwrap_or_else(|| panic!("unknown authored lifecycle registration: {hook}"));
    bind_registration(element, registration)
}
