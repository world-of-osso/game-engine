//! Unit tooltip networking (docs/specs/unit-tooltip.md): the tooltip asks for a
//! creature entry's data once ([`CreatureTooltipCache::request`]), the query goes
//! out on `TooltipChannel`, and the server's `CreatureTooltip` is cached per entry.
//! `AppearanceCollectionUpdate` replaces [`AccountAppearances`], so marks follow
//! newly learned appearances. Both reset when leaving the world.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use game_engine::network_events::{
    add_message_route, register_message_handler, register_outgoing_handler,
};
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use shared::protocol::{
    AppearanceCollectionUpdate, CreatureTooltip, CreatureTooltipQuery, TooltipChannel,
};
use shared::transmog::AppearanceCollection;

use crate::game_state::GameState;

/// Server tooltip data by creature entry, and the entries still to ask for.
#[derive(Resource, Default)]
pub(crate) struct CreatureTooltipCache {
    entries: HashMap<u32, CreatureTooltip>,
    requested: HashSet<u32>,
    pending: Vec<u32>,
}

impl CreatureTooltipCache {
    pub(crate) fn get(&self, entry: u32) -> Option<&CreatureTooltip> {
        self.entries.get(&entry)
    }

    /// Ask the server for `entry` unless it was already asked for.
    pub(crate) fn request(&mut self, entry: u32) {
        if self.requested.insert(entry) {
            self.pending.push(entry);
        }
    }
}

/// The account's learned appearances, as the server last sent them.
#[derive(Resource, Default)]
pub(crate) struct AccountAppearances(pub AppearanceCollection);

pub struct UnitTooltipNetworkPlugin;

impl Plugin for UnitTooltipNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreatureTooltipCache>()
            .init_resource::<AccountAppearances>();
        let handler =
            register_message_handler::<CreatureTooltip, _>(app, receive_tooltip_data, |_| true);
        add_message_route::<AppearanceCollectionUpdate>(app, handler);
        register_outgoing_handler(app, send_tooltip_queries, |world| {
            !world.resource::<CreatureTooltipCache>().pending.is_empty()
        });
        app.add_systems(OnExit(GameState::InWorld), reset_tooltip_data);
    }
}

fn receive_tooltip_data(
    mut tooltips: MessageReceivers<CreatureTooltip>,
    mut collections: MessageReceivers<AppearanceCollectionUpdate>,
    mut cache: ResMut<CreatureTooltipCache>,
    mut appearances: ResMut<AccountAppearances>,
) {
    for inbox in tooltips.iter_mut() {
        for tooltip in inbox.receive() {
            cache.entries.insert(tooltip.entry, tooltip);
        }
    }
    for inbox in collections.iter_mut() {
        for update in inbox.receive() {
            let mut collection = AppearanceCollection::default();
            collection.learn_many(update.appearances);
            appearances.0 = collection;
        }
    }
}

fn send_tooltip_queries(
    mut cache: ResMut<CreatureTooltipCache>,
    mut senders: MessageSenders<CreatureTooltipQuery>,
) {
    for entry in std::mem::take(&mut cache.pending) {
        for mut sender in senders.iter_mut() {
            sender.send::<TooltipChannel>(CreatureTooltipQuery { entry });
        }
    }
}

fn reset_tooltip_data(
    mut cache: ResMut<CreatureTooltipCache>,
    mut appearances: ResMut<AccountAppearances>,
) {
    *cache = CreatureTooltipCache::default();
    *appearances = AccountAppearances::default();
}

#[cfg(test)]
#[path = "unit_tooltip_tests.rs"]
mod tests;
