use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::status::InspectStatusSnapshot;

#[derive(Resource, Default)]
pub(super) struct InWorldEscapeStack {
    order: Vec<InWorldEscapePanel>,
    open: HashSet<InWorldEscapePanel>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum InWorldEscapePanel {
    Achievement,
    Bag,
    Calendar,
    Character,
    EncounterJournal,
    Friends,
    Guild,
    Inspect,
    LootRules,
    Mail,
    Merchant,
    Professions,
    Talent,
    WorldMap,
}

impl InWorldEscapeStack {
    pub(super) fn sync(&mut self, panel: InWorldEscapePanel, is_open: bool) {
        if is_open {
            if self.open.insert(panel) {
                self.order.push(panel);
            }
            return;
        }
        self.remove(panel);
    }

    pub(super) fn topmost(&self) -> Option<InWorldEscapePanel> {
        self.order.last().copied()
    }

    pub(super) fn remove(&mut self, panel: InWorldEscapePanel) {
        if !self.open.remove(&panel) {
            return;
        }
        self.order.retain(|entry| *entry != panel);
    }

    #[cfg(test)]
    pub(super) fn ordered_panels(&self) -> &[InWorldEscapePanel] {
        &self.order
    }

    #[cfg(test)]
    pub(super) fn with_open_order(order: &[InWorldEscapePanel]) -> Self {
        let mut stack = Self::default();
        for panel in order {
            stack.sync(*panel, true);
        }
        stack
    }
}

#[derive(SystemParam)]
pub(super) struct InWorldEscapePanelState<'w> {
    achievement: Option<Res<'w, crate::scenes::achievement_frame::AchievementFrameOpen>>,
    bag: Option<Res<'w, crate::scenes::bag_frame::BagFrameOpenState>>,
    calendar: Option<Res<'w, crate::scenes::calendar_frame::CalendarFrameOpen>>,
    character: Option<Res<'w, crate::scenes::character_frame::CharacterFrameOpen>>,
    encounter_journal:
        Option<Res<'w, crate::scenes::encounter_journal_frame::EncounterJournalFrameOpen>>,
    friends: Option<Res<'w, crate::scenes::friends_frame::FriendsFrameOpen>>,
    guild: Option<Res<'w, crate::scenes::guild_frame::GuildFrameOpen>>,
    inspect: Option<Res<'w, InspectStatusSnapshot>>,
    loot_rules: Option<Res<'w, crate::scenes::loot_rules_frame::LootRulesFrameOpen>>,
    mail: Option<Res<'w, crate::scenes::mail_frame::MailFrameOpen>>,
    merchant: Option<Res<'w, game_engine::merchant_data::MerchantState>>,
    professions: Option<Res<'w, crate::scenes::professions_frame::ProfessionsFrameOpen>>,
    talent: Option<Res<'w, crate::scenes::talent_frame::TalentFrameOpen>>,
    world_map: Option<Res<'w, crate::scenes::world_map_frame::WorldMapFrameOpen>>,
}

#[derive(SystemParam)]
pub(super) struct InWorldEscapePanelMut<'w> {
    achievement: Option<ResMut<'w, crate::scenes::achievement_frame::AchievementFrameOpen>>,
    bag: Option<ResMut<'w, crate::scenes::bag_frame::BagFrameOpenState>>,
    calendar: Option<ResMut<'w, crate::scenes::calendar_frame::CalendarFrameOpen>>,
    character: Option<ResMut<'w, crate::scenes::character_frame::CharacterFrameOpen>>,
    encounter_journal:
        Option<ResMut<'w, crate::scenes::encounter_journal_frame::EncounterJournalFrameOpen>>,
    friends: Option<ResMut<'w, crate::scenes::friends_frame::FriendsFrameOpen>>,
    guild: Option<ResMut<'w, crate::scenes::guild_frame::GuildFrameOpen>>,
    inspect: Option<ResMut<'w, InspectStatusSnapshot>>,
    loot_rules: Option<ResMut<'w, crate::scenes::loot_rules_frame::LootRulesFrameOpen>>,
    mail: Option<ResMut<'w, crate::scenes::mail_frame::MailFrameOpen>>,
    merchant: Option<ResMut<'w, game_engine::merchant_data::MerchantState>>,
    professions: Option<ResMut<'w, crate::scenes::professions_frame::ProfessionsFrameOpen>>,
    talent: Option<ResMut<'w, crate::scenes::talent_frame::TalentFrameOpen>>,
    world_map: Option<ResMut<'w, crate::scenes::world_map_frame::WorldMapFrameOpen>>,
}

pub(super) fn sync_inworld_escape_stack(
    mut escape_stack: ResMut<InWorldEscapeStack>,
    panels: InWorldEscapePanelState,
) {
    sync_flag_panels(&mut escape_stack, &panels);
    sync_bag_panel(&mut escape_stack, &panels);
    sync_inspect_panel(&mut escape_stack, &panels);
    sync_merchant_panel(&mut escape_stack, &panels);
}

fn sync_open_flag(escape_stack: &mut InWorldEscapeStack, panel: InWorldEscapePanel, is_open: bool) {
    escape_stack.sync(panel, is_open);
}

fn sync_flag_panels(escape_stack: &mut InWorldEscapeStack, panels: &InWorldEscapePanelState) {
    sync_primary_flag_panels(escape_stack, panels);
    sync_secondary_flag_panels(escape_stack, panels);
}

fn sync_primary_flag_panels(
    escape_stack: &mut InWorldEscapeStack,
    panels: &InWorldEscapePanelState,
) {
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Achievement,
        panels.achievement.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Calendar,
        panels.calendar.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Character,
        panels.character.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::EncounterJournal,
        panels.encounter_journal.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Friends,
        panels.friends.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Guild,
        panels.guild.as_ref().is_some_and(|open| open.0),
    );
}

fn sync_secondary_flag_panels(
    escape_stack: &mut InWorldEscapeStack,
    panels: &InWorldEscapePanelState,
) {
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::LootRules,
        panels.loot_rules.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Mail,
        panels.mail.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Professions,
        panels.professions.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Talent,
        panels.talent.as_ref().is_some_and(|open| open.0),
    );
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::WorldMap,
        panels.world_map.as_ref().is_some_and(|open| open.0),
    );
}

fn sync_bag_panel(escape_stack: &mut InWorldEscapeStack, panels: &InWorldEscapePanelState) {
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Bag,
        panels.bag.as_ref().is_some_and(|open| open.any_open()),
    );
}

fn sync_inspect_panel(escape_stack: &mut InWorldEscapeStack, panels: &InWorldEscapePanelState) {
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Inspect,
        panels
            .inspect
            .as_ref()
            .and_then(|snapshot| snapshot.target_name.as_ref())
            .is_some(),
    );
}

fn sync_merchant_panel(escape_stack: &mut InWorldEscapeStack, panels: &InWorldEscapePanelState) {
    sync_open_flag(
        escape_stack,
        InWorldEscapePanel::Merchant,
        panels
            .merchant
            .as_ref()
            .is_some_and(|merchant| merchant.is_open()),
    );
}

pub(super) fn close_topmost_tracked_panel(
    escape_stack: &mut InWorldEscapeStack,
    mut close_panel: impl FnMut(InWorldEscapePanel) -> bool,
) -> Option<InWorldEscapePanel> {
    while let Some(panel) = escape_stack.topmost() {
        escape_stack.remove(panel);
        if close_panel(panel) {
            return Some(panel);
        }
    }
    None
}

pub(super) fn close_tracked_panel(
    panel: InWorldEscapePanel,
    panels: &mut InWorldEscapePanelMut,
) -> bool {
    match panel {
        InWorldEscapePanel::Achievement
        | InWorldEscapePanel::Calendar
        | InWorldEscapePanel::Character
        | InWorldEscapePanel::EncounterJournal
        | InWorldEscapePanel::Friends
        | InWorldEscapePanel::Guild
        | InWorldEscapePanel::LootRules
        | InWorldEscapePanel::Mail
        | InWorldEscapePanel::Professions
        | InWorldEscapePanel::Talent
        | InWorldEscapePanel::WorldMap => close_flag_panel(panel, panels),
        InWorldEscapePanel::Bag => close_bag_panel(panels.bag.as_deref_mut()),
        InWorldEscapePanel::Inspect => close_inspect_panel(panels.inspect.as_deref_mut()),
        InWorldEscapePanel::Merchant => close_merchant_panel(panels.merchant.as_deref_mut()),
    }
}

fn close_flag_panel(panel: InWorldEscapePanel, panels: &mut InWorldEscapePanelMut) -> bool {
    flag_panel_closer(panel).is_some_and(|close| close(panels))
}

type FlagPanelCloser = fn(&mut InWorldEscapePanelMut) -> bool;

fn flag_panel_closer(panel: InWorldEscapePanel) -> Option<FlagPanelCloser> {
    match panel {
        InWorldEscapePanel::Achievement => Some(close_achievement_panel),
        InWorldEscapePanel::Calendar => Some(close_calendar_panel),
        InWorldEscapePanel::Character => Some(close_character_panel),
        InWorldEscapePanel::EncounterJournal => Some(close_encounter_journal_panel),
        InWorldEscapePanel::Friends => Some(close_friends_panel),
        InWorldEscapePanel::Guild => Some(close_guild_panel),
        InWorldEscapePanel::LootRules => Some(close_loot_rules_panel),
        InWorldEscapePanel::Mail => Some(close_mail_panel),
        InWorldEscapePanel::Professions => Some(close_professions_panel),
        InWorldEscapePanel::Talent => Some(close_talent_panel),
        InWorldEscapePanel::WorldMap => Some(close_world_map_panel),
        InWorldEscapePanel::Bag | InWorldEscapePanel::Inspect | InWorldEscapePanel::Merchant => {
            None
        }
    }
}

fn close_achievement_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.achievement.as_deref_mut().map(|open| &mut open.0))
}

fn close_calendar_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.calendar.as_deref_mut().map(|open| &mut open.0))
}

fn close_character_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.character.as_deref_mut().map(|open| &mut open.0))
}

fn close_encounter_journal_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(
        panels
            .encounter_journal
            .as_deref_mut()
            .map(|open| &mut open.0),
    )
}

fn close_friends_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.friends.as_deref_mut().map(|open| &mut open.0))
}

fn close_guild_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.guild.as_deref_mut().map(|open| &mut open.0))
}

fn close_loot_rules_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.loot_rules.as_deref_mut().map(|open| &mut open.0))
}

fn close_mail_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.mail.as_deref_mut().map(|open| &mut open.0))
}

fn close_professions_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.professions.as_deref_mut().map(|open| &mut open.0))
}

fn close_talent_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.talent.as_deref_mut().map(|open| &mut open.0))
}

fn close_world_map_panel(panels: &mut InWorldEscapePanelMut) -> bool {
    close_open_flag(panels.world_map.as_deref_mut().map(|open| &mut open.0))
}

fn close_open_flag(open: Option<&mut bool>) -> bool {
    let Some(open) = open else { return false };
    if !*open {
        return false;
    }
    *open = false;
    true
}

pub(super) fn close_bag_panel(
    open: Option<&mut crate::scenes::bag_frame::BagFrameOpenState>,
) -> bool {
    let Some(open) = open else { return false };
    if !open.any_open() {
        return false;
    }
    open.close_all();
    true
}

pub(super) fn close_inspect_panel(snapshot: Option<&mut InspectStatusSnapshot>) -> bool {
    let Some(snapshot) = snapshot else {
        return false;
    };
    if snapshot.target_name.is_none() {
        return false;
    }
    *snapshot = InspectStatusSnapshot::default();
    true
}

fn close_merchant_panel(state: Option<&mut game_engine::merchant_data::MerchantState>) -> bool {
    let Some(state) = state else { return false };
    if !state.is_open() {
        return false;
    }
    state.close();
    true
}
